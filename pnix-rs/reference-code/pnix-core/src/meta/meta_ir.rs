//! MetaFx IR: Self-description IR for meta-circular preparation
//!
//! pnix이 자기 자신을 기술하는 IR.
//! 실행 ❌, 검증 ❌, 순수 구조 변환만.
//!
//! 헌법 준수:
//! - P0-1: 순수 구조 변환 (의미 변경 불가)
//! - E1: 실행 계약은 참조만 (재해석 금지)

use serde::{Deserialize, Serialize};

use crate::contracts::effect::Effect;
use crate::core::{CostHint, EdgeCond, FxCoreModule, NodeKind, ScopePolicy, SkipPolicy};

/// MetaFx 모듈: FxCore의 구조적 자기 기술
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFxModule {
  /// 원본 FxCore 이름
  pub name: String,
  /// 원본 스테이지
  pub stage: u8,
  /// 원본 replay_hash (있으면)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub replay_hash: Option<String>,
  /// 타입 선언 (문자열 목록)
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub types: Vec<String>,
  /// 입력 선언 (Stage-2)
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub inputs: Vec<MetaFxInput>,
  /// morphism 선언 (extern)
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub morphisms: Vec<MetaFxMorphism>,
  /// 노드 구조 기술
  pub nodes: Vec<MetaFxNode>,
  /// 엣지 구조 기술
  pub edges: Vec<MetaFxEdge>,
  /// 스코프 구조 기술
  pub scopes: Vec<MetaFxScope>,
  /// 통계 정보 (구조적)
  pub stats: MetaFxStats,
}

/// MetaFx 입력: Stage-2 입력 선언
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFxInput {
  /// 입력 이름
  pub name: String,
  /// 입력 타입
  pub ty: String,
}

/// MetaFx 포트: morphism 입출력 포트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFxMorphismPort {
  /// 포트 이름
  pub name: String,
  /// 포트 타입
  pub ty: String,
}

/// MetaFx Morphism: extern 선언의 구조적 기술
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFxMorphism {
  /// morphism 이름 (e.g., "py.normalize")
  pub name: String,
  /// Stage-1 입력 타입
  pub input: String,
  /// Stage-1 출력 타입
  pub output: String,
  /// Stage-2 입력 포트
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub inputs: Vec<MetaFxMorphismPort>,
  /// Stage-2 출력 포트
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub outputs: Vec<MetaFxMorphismPort>,
  /// Effect (문자열: "Pure" | "World")
  pub effect: String,
}

/// MetaFx 실행 계약: core::ExecutionContract의 문자열 미러
///
/// core 타입에 직접 의존하지 않음 (self-description 독립성)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetaExecutionContract {
  /// 필수 입력 포트 목록
  pub required_inputs: Vec<String>,
  /// 스킵 가능 여부
  pub may_skip: bool,
  /// 스킵 정책 (문자열: "error" | "skip")
  pub skip_policy: String,
}

/// MetaFx 노드: 노드의 구조적 기술
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFxNode {
  /// 노드 이름
  pub name: String,
  /// morphism 참조
  pub uses: String,
  /// 종류 (normal/gate) - 문자열로 정규화
  pub kind: String,
  /// optional 여부
  pub optional: bool,
  /// 소속 스코프
  pub scope: String,
  /// 비용 힌트 (문자열로 정규화)
  pub cost: String,
  /// 우선순위
  pub priority: i32,
  /// Execution Contract (문자열 미러, core 타입 비의존)
  pub contract: MetaExecutionContract,
}

/// MetaFx 엣지: 엣지의 구조적 기술
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFxEdge {
  /// 소스 (노드명 또는 "input")
  pub from: String,
  /// 타겟 노드명
  pub to: String,
  /// 조건 (when/unless/onfail) - 문자열로 정규화
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub cond: Option<String>,
  /// 포트 정보 (있으면)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub port_info: Option<MetaFxPortInfo>,
}

/// MetaFx 포트 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFxPortInfo {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub from_port: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub to_port: Option<String>,
}

/// MetaFx 스코프: 스코프의 구조적 기술
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFxScope {
  /// 스코프 이름
  pub name: String,
  /// 소속 노드 목록
  pub node_count: usize,
  /// 정책 (문자열로 정규화)
  pub policy: String,
}

/// MetaFx 통계: 구조적 통계 (의미 없는 숫자)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFxStats {
  pub node_count: usize,
  pub edge_count: usize,
  pub scope_count: usize,
  pub gate_count: usize,
  pub optional_count: usize,
  pub conditional_edge_count: usize,
}

/// FxCore를 MetaFx로 변환 (순수 구조 변환)
///
/// 실행 ❌, 검증 ❌, 의미 재해석 ❌
/// 오직 구조적 데이터 매핑만 수행
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn describe_fxcore(fx: &FxCoreModule) -> MetaFxModule {
  // 타입 변환 (그대로 복사)
  let types: Vec<String> = fx.types.clone();

  // 입력 변환 (Stage-2)
  let inputs: Vec<MetaFxInput> = fx
    .inputs
    .iter()
    .map(|i| MetaFxInput {
      name: i.name.clone(),
      ty: i.ty.clone(),
    })
    .collect();

  // morphism 변환 (extern)
  let morphisms: Vec<MetaFxMorphism> = fx
    .morphisms
    .iter()
    .map(|m| MetaFxMorphism {
      name: m.name.clone(),
      input: m.input.clone(),
      output: m.output.clone(),
      inputs: m
        .inputs
        .iter()
        .map(|p| MetaFxMorphismPort {
          name: p.name.clone(),
          ty: p.ty.clone(),
        })
        .collect(),
      outputs: m
        .outputs
        .iter()
        .map(|p| MetaFxMorphismPort {
          name: p.name.clone(),
          ty: p.ty.clone(),
        })
        .collect(),
      effect: match m.effect {
        Effect::Pure => "Pure".into(),
        Effect::World => "World".into(),
        Effect::Unknown => "Unknown".into(),
      },
    })
    .collect();

  // 노드 변환 (구조만)
  let nodes: Vec<MetaFxNode> = fx
    .nodes
    .iter()
    .map(|n| MetaFxNode {
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
      contract: MetaExecutionContract {
        required_inputs: n.contract.required_inputs.clone(),
        may_skip: n.contract.may_skip,
        skip_policy: match n.contract.skip_policy {
          SkipPolicy::Error => "error".into(),
          SkipPolicy::Skip => "skip".into(),
        },
      },
    })
    .collect();

  // 엣지 변환 (구조만)
  let edges: Vec<MetaFxEdge> = fx
    .edges
    .iter()
    .map(|e| {
      let cond = e.cond.as_ref().map(|c| match c {
        EdgeCond::When(g) => format!("when:{}", g),
        EdgeCond::Unless(g) => format!("unless:{}", g),
        EdgeCond::OnFail(n) => format!("onfail:{}", n),
        EdgeCond::WhenUnless { when, unless } => format!("when_unless:{}:{}", when, unless),
        EdgeCond::AllWhen(gates) => format!("all_when:{}", gates.join(",")),
        EdgeCond::AllUnless(gates) => format!("all_unless:{}", gates.join(",")),
        EdgeCond::Unknown => "unknown".into(),
      });

      let port_info = if e.from_port.is_some() || e.to_port.is_some() {
        Some(MetaFxPortInfo {
          from_port: e.from_port.clone(),
          to_port: e.to_port.clone(),
        })
      } else {
        None
      };

      MetaFxEdge {
        from: e.from.clone(),
        to: e.to.clone(),
        cond,
        port_info,
      }
    })
    .collect();

  // 스코프 변환 (구조만)
  let scopes: Vec<MetaFxScope> = fx
    .scopes
    .iter()
    .map(|s| MetaFxScope {
      name: s.name.clone(),
      node_count: s.nodes.len(),
      policy: match s.policy {
        ScopePolicy::FailFast => "fail_fast".into(),
        ScopePolicy::Isolate => "isolate".into(),
        ScopePolicy::BestEffort => "best_effort".into(),
      },
    })
    .collect();

  // 통계 계산 (구조적 숫자)
  let gate_count = fx.nodes.iter().filter(|n| n.kind == NodeKind::Gate).count();
  let optional_count = fx.nodes.iter().filter(|n| n.optional).count();
  let conditional_edge_count = fx.edges.iter().filter(|e| e.cond.is_some()).count();

  let stats = MetaFxStats {
    node_count: fx.nodes.len(),
    edge_count: fx.edges.len(),
    scope_count: fx.scopes.len(),
    gate_count,
    optional_count,
    conditional_edge_count,
  };

  MetaFxModule {
    name: fx.name.clone(),
    stage: fx.meta.stage,
    replay_hash: fx.meta.replay_hash.clone(),
    types,
    inputs,
    morphisms,
    nodes,
    edges,
    scopes,
    stats,
  }
}

/// MetaFx를 JSON으로 직렬화
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn meta_to_json(meta: &MetaFxModule) -> Result<String, serde_json::Error> {
  serde_json::to_string_pretty(meta)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::{CostHint, ExecutionContract, FxEdge, FxNode, FxScope};

  fn make_test_graph() -> FxCoreModule {
    FxCoreModule {
      meta: crate::core::FxCoreMeta {
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
          name: "n1".into(),
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
          name: "g1".into(),
          uses: "gate.check".into(),
          kind: NodeKind::Gate,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Tiny,
          priority: 10,
          contract: ExecutionContract::default(),

          meta: None,
        },
        FxNode {
          name: "n2".into(),
          uses: "deno.render".into(),
          kind: NodeKind::Normal,
          optional: true,
          scope: "render".into(),
          cost: CostHint::Heavy,
          priority: 0,
          contract: ExecutionContract {
            required_inputs: vec!["data".into()],
            may_skip: true,
            skip_policy: crate::core::SkipPolicy::Skip,
            replay: None,
          },

          meta: None,
        },
      ],
      edges: vec![
        FxEdge::simple("n1".into(), "g1".into()),
        FxEdge::simple("g1".into(), "n2".into()).with_cond(EdgeCond::When("g1".into())),
      ],
      scopes: vec![FxScope {
        name: "render".into(),
        nodes: vec!["n2".into()],
        policy: ScopePolicy::Isolate,
      }],
    }
  }

  #[test]
  fn test_describe_fxcore_basic() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);

    assert_eq!(meta.name, "test-graph");
    assert_eq!(meta.stage, 4);
    assert_eq!(meta.replay_hash, Some("test_hash_123".into()));
  }

  #[test]
  fn test_describe_fxcore_nodes() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);

    assert_eq!(meta.nodes.len(), 3);

    let n1 = meta.nodes.iter().find(|n| n.name == "n1").unwrap();
    assert_eq!(n1.kind, "normal");
    assert!(!n1.optional);

    let g1 = meta.nodes.iter().find(|n| n.name == "g1").unwrap();
    assert_eq!(g1.kind, "gate");

    let n2 = meta.nodes.iter().find(|n| n.name == "n2").unwrap();
    assert!(n2.optional);
    assert_eq!(n2.scope, "render");
  }

  #[test]
  fn test_describe_fxcore_edges() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);

    assert_eq!(meta.edges.len(), 2);

    let cond_edge = meta.edges.iter().find(|e| e.cond.is_some()).unwrap();
    assert_eq!(cond_edge.cond.as_deref(), Some("when:g1"));
  }

  #[test]
  fn test_describe_fxcore_scopes() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);

    assert_eq!(meta.scopes.len(), 1);
    let scope = &meta.scopes[0];
    assert_eq!(scope.name, "render");
    assert_eq!(scope.policy, "isolate");
    assert_eq!(scope.node_count, 1);
  }

  #[test]
  fn test_describe_fxcore_stats() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);

    assert_eq!(meta.stats.node_count, 3);
    assert_eq!(meta.stats.edge_count, 2);
    assert_eq!(meta.stats.scope_count, 1);
    assert_eq!(meta.stats.gate_count, 1);
    assert_eq!(meta.stats.optional_count, 1);
    assert_eq!(meta.stats.conditional_edge_count, 1);
  }

  #[test]
  fn test_meta_to_json() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let json = meta_to_json(&meta).unwrap();

    // JSON에 핵심 필드 존재 확인
    assert!(json.contains("\"name\": \"test-graph\""));
    assert!(json.contains("\"stage\": 4"));
    assert!(json.contains("\"gate_count\": 1"));
  }

  #[test]
  fn test_describe_is_pure_structural() {
    // 동일 입력 → 동일 출력 (순수 함수)
    let fx = make_test_graph();
    let meta1 = describe_fxcore(&fx);
    let meta2 = describe_fxcore(&fx);

    let json1 = meta_to_json(&meta1).unwrap();
    let json2 = meta_to_json(&meta2).unwrap();

    assert_eq!(json1, json2, "describe_fxcore must be deterministic");
  }

  #[test]
  fn test_contract_reference_preserved() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);

    let n2 = meta.nodes.iter().find(|n| n.name == "n2").unwrap();
    assert!(n2.contract.may_skip);
    assert_eq!(n2.contract.required_inputs, vec!["data"]);
  }
}
