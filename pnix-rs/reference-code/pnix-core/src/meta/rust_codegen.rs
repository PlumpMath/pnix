//! MetaFx → Rust Struct Codegen
//!
//! Meta-circular Stage 1: pnix이 자기 구조를 Rust 코드로 설명
//!
//! 실행 ❌ / 의미 검증 ❌ / 컴파일에 사용 ❌
//! 순수 텍스트 생성만 (읽기 전용, 비교/검사용)
//!
//! 금지 사항:
//! - 실제 FxNode, FxEdge, ExecutionContract import 금지
//! - pnix-core 내부 타입 재사용 금지
//! - verify / lowering 호출 금지

use super::{MetaFxEdge, MetaFxModule, MetaFxNode, MetaFxScope};

/// Generate Rust code that describes the FxCore structure (read-only).
///
/// - Pure text generation
/// - Deterministic output
/// - NOT used for compilation
///
/// 입력: MetaFxModule (self-description IR)
/// 출력: Rust struct 정의 코드 문자열
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn generate_rust_structs(meta: &MetaFxModule) -> String {
  let mut out = String::new();

  // 1. Header (고정)
  out.push_str(HEADER);
  out.push('\n');

  // 2. Imports
  out.push_str("use serde::{Serialize, Deserialize};\n\n");

  // 3. Struct definitions (고정 순서)
  out.push_str(&gen_module_struct(meta));
  out.push('\n');
  out.push_str(&gen_stats_struct());
  out.push('\n');
  out.push_str(&gen_node_struct());
  out.push('\n');
  out.push_str(&gen_contract_struct());
  out.push('\n');
  out.push_str(&gen_edge_struct());
  out.push('\n');
  out.push_str(&gen_edge_cond_enum());
  out.push('\n');
  out.push_str(&gen_scope_struct());
  out.push('\n');

  // 4. Instance data as const (정렬된 순서로)
  out.push_str(&gen_instance_data(meta));

  out
}

const HEADER: &str = r#"// ============================================================
//  AUTO-GENERATED FROM MetaFx (self-description)
//  DO NOT EDIT
//
//  This code is NOT used by pnix-core compilation.
//  It exists for inspection, comparison, and future meta-circular work.
// ============================================================
"#;

fn gen_module_struct(meta: &MetaFxModule) -> String {
  format!(
    r#"/// Generated module structure for: {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFxModule {{
    pub name: String,
    pub stage: u8,
    pub replay_hash: Option<String>,
    pub stats: GeneratedFxStats,
    pub nodes: Vec<GeneratedFxNode>,
    pub edges: Vec<GeneratedFxEdge>,
    pub scopes: Vec<GeneratedFxScope>,
}}
"#,
    meta.name
  )
}

fn gen_stats_struct() -> String {
  r#"#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFxStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub gate_count: usize,
    pub scope_count: usize,
    pub optional_count: usize,
    pub conditional_edge_count: usize,
}
"#
  .to_string()
}

fn gen_node_struct() -> String {
  r#"#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFxNode {
    pub name: String,
    pub uses: String,
    pub kind: String,          // "normal" | "gate"
    pub optional: bool,
    pub scope: String,
    pub cost: String,          // "tiny" | "light" | "medium" | "heavy" | "xheavy"
    pub priority: i32,

    // ExecutionContract is copied structurally (NO reinterpretation)
    pub contract: GeneratedExecutionContract,
}
"#
  .to_string()
}

fn gen_contract_struct() -> String {
  r#"#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedExecutionContract {
    pub required_inputs: Vec<String>,
    pub may_skip: bool,
    pub skip_policy: String,   // "error" | "skip"
}
"#
  .to_string()
}

fn gen_edge_struct() -> String {
  r#"#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFxEdge {
    pub from: String,
    pub to: String,

    // Optional port info (Stage-2)
    pub from_port: Option<String>,
    pub to_port: Option<String>,

    // Optional condition (Stage-3/4)
    pub cond: Option<GeneratedEdgeCond>,
}
"#
  .to_string()
}

fn gen_edge_cond_enum() -> String {
  r#"#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeneratedEdgeCond {
    When(String),
    Unless(String),
    OnFail(String),
}
"#
  .to_string()
}

fn gen_scope_struct() -> String {
  r#"#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFxScope {
    pub name: String,
    pub policy: String,     // "fail_fast" | "isolate" | "best_effort"
    pub node_count: usize,
}
"#
  .to_string()
}

/// Generate instance data as Rust code
/// 노드/엣지/스코프는 이름 기준 정렬하여 결정성 보장
fn gen_instance_data(meta: &MetaFxModule) -> String {
  let mut out = String::new();

  out.push_str("// ============================================================\n");
  out.push_str("//  INSTANCE DATA\n");
  out.push_str("// ============================================================\n\n");

  // Module info
  out.push_str(&format!(
    r#"pub const MODULE_NAME: &str = "{}";
pub const MODULE_STAGE: u8 = {};
"#,
    meta.name, meta.stage
  ));

  if let Some(ref hash) = meta.replay_hash {
    out.push_str(&format!(
      "pub const MODULE_REPLAY_HASH: Option<&str> = Some(\"{}\");\n",
      hash
    ));
  } else {
    out.push_str("pub const MODULE_REPLAY_HASH: Option<&str> = None;\n");
  }
  out.push('\n');

  // Stats
  out.push_str(&format!(
    r#"pub const STATS: GeneratedFxStats = GeneratedFxStats {{
    node_count: {},
    edge_count: {},
    gate_count: {},
    scope_count: {},
    optional_count: {},
    conditional_edge_count: {},
}};
"#,
    meta.stats.node_count,
    meta.stats.edge_count,
    meta.stats.gate_count,
    meta.stats.scope_count,
    meta.stats.optional_count,
    meta.stats.conditional_edge_count
  ));
  out.push('\n');

  // Nodes (정렬)
  let mut sorted_nodes: Vec<&MetaFxNode> = meta.nodes.iter().collect();
  sorted_nodes.sort_by(|a, b| a.name.cmp(&b.name));

  out.push_str("/// Node definitions (sorted by name)\n");
  out.push_str("pub fn nodes() -> Vec<GeneratedFxNode> {\n");
  out.push_str("    vec![\n");
  for node in &sorted_nodes {
    out.push_str(&format_node_literal(node));
  }
  out.push_str("    ]\n");
  out.push_str("}\n\n");

  // Edges (from, to 기준 정렬)
  let mut sorted_edges: Vec<&MetaFxEdge> = meta.edges.iter().collect();
  sorted_edges.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));

  out.push_str("/// Edge definitions (sorted by from, to)\n");
  out.push_str("pub fn edges() -> Vec<GeneratedFxEdge> {\n");
  out.push_str("    vec![\n");
  for edge in &sorted_edges {
    out.push_str(&format_edge_literal(edge));
  }
  out.push_str("    ]\n");
  out.push_str("}\n\n");

  // Scopes (정렬)
  let mut sorted_scopes: Vec<&MetaFxScope> = meta.scopes.iter().collect();
  sorted_scopes.sort_by(|a, b| a.name.cmp(&b.name));

  out.push_str("/// Scope definitions (sorted by name)\n");
  out.push_str("pub fn scopes() -> Vec<GeneratedFxScope> {\n");
  out.push_str("    vec![\n");
  for scope in &sorted_scopes {
    out.push_str(&format_scope_literal(scope));
  }
  out.push_str("    ]\n");
  out.push_str("}\n");

  out
}

fn format_node_literal(n: &MetaFxNode) -> String {
  // MetaExecutionContract는 이미 문자열 기반 (core 타입 비의존)
  let skip_policy = &n.contract.skip_policy;

  let required_inputs: Vec<String> = n
    .contract
    .required_inputs
    .iter()
    .map(|s| format!("\"{}\".to_string()", s))
    .collect();

  format!(
    r#"        GeneratedFxNode {{
            name: "{}".to_string(),
            uses: "{}".to_string(),
            kind: "{}".to_string(),
            optional: {},
            scope: "{}".to_string(),
            cost: "{}".to_string(),
            priority: {},
            contract: GeneratedExecutionContract {{
                required_inputs: vec![{}],
                may_skip: {},
                skip_policy: "{}".to_string(),
            }},
        }},
"#,
    n.name,
    n.uses,
    n.kind,
    n.optional,
    n.scope,
    n.cost,
    n.priority,
    required_inputs.join(", "),
    n.contract.may_skip,
    skip_policy
  )
}

fn format_edge_literal(e: &MetaFxEdge) -> String {
  let from_port = match &e.port_info {
    Some(p) => match &p.from_port {
      Some(fp) => format!("Some(\"{}\".to_string())", fp),
      None => "None".to_string(),
    },
    None => "None".to_string(),
  };

  let to_port = match &e.port_info {
    Some(p) => match &p.to_port {
      Some(tp) => format!("Some(\"{}\".to_string())", tp),
      None => "None".to_string(),
    },
    None => "None".to_string(),
  };

  let cond = match &e.cond {
    Some(c) => {
      if let Some(gate) = c.strip_prefix("when:") {
        format!("Some(GeneratedEdgeCond::When(\"{}\".to_string()))", gate)
      } else if let Some(gate) = c.strip_prefix("unless:") {
        format!("Some(GeneratedEdgeCond::Unless(\"{}\".to_string()))", gate)
      } else if let Some(node) = c.strip_prefix("onfail:") {
        format!("Some(GeneratedEdgeCond::OnFail(\"{}\".to_string()))", node)
      } else {
        "None".to_string()
      }
    }
    None => "None".to_string(),
  };

  format!(
    r#"        GeneratedFxEdge {{
            from: "{}".to_string(),
            to: "{}".to_string(),
            from_port: {},
            to_port: {},
            cond: {},
        }},
"#,
    e.from, e.to, from_port, to_port, cond
  )
}

fn format_scope_literal(s: &MetaFxScope) -> String {
  format!(
    r#"        GeneratedFxScope {{
            name: "{}".to_string(),
            policy: "{}".to_string(),
            node_count: {},
        }},
"#,
    s.name, s.policy, s.node_count
  )
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
          name: "gate-check".into(),
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
        FxEdge::simple("solve".into(), "gate-check".into()),
        FxEdge::simple("gate-check".into(), "render".into())
          .with_cond(EdgeCond::When("gate-check".into())),
      ],
      scopes: vec![FxScope {
        name: "output".into(),
        nodes: vec!["render".into()],
        policy: ScopePolicy::Isolate,
      }],
    }
  }

  #[test]
  fn rust_codegen_contains_expected_structs() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let code = generate_rust_structs(&meta);

    assert!(code.contains("pub struct GeneratedFxModule"));
    assert!(code.contains("pub struct GeneratedFxNode"));
    assert!(code.contains("pub struct GeneratedExecutionContract"));
    assert!(code.contains("pub enum GeneratedEdgeCond"));
    assert!(code.contains("pub struct GeneratedFxEdge"));
    assert!(code.contains("pub struct GeneratedFxScope"));
    assert!(code.contains("pub struct GeneratedFxStats"));
  }

  #[test]
  fn rust_codegen_is_deterministic() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);

    let code1 = generate_rust_structs(&meta);
    let code2 = generate_rust_structs(&meta);

    assert_eq!(code1, code2, "Rust codegen must be deterministic");
  }

  #[test]
  fn rust_codegen_contains_header() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let code = generate_rust_structs(&meta);

    assert!(code.contains("AUTO-GENERATED FROM MetaFx"));
    assert!(code.contains("DO NOT EDIT"));
    assert!(code.contains("NOT used by pnix-core compilation"));
  }

  #[test]
  fn rust_codegen_contains_instance_data() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let code = generate_rust_structs(&meta);

    assert!(code.contains("MODULE_NAME"));
    assert!(code.contains("\"test-graph\""));
    assert!(code.contains("MODULE_STAGE: u8 = 4"));
    assert!(code.contains("MODULE_REPLAY_HASH"));
  }

  #[test]
  fn rust_codegen_nodes_are_sorted() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let code = generate_rust_structs(&meta);

    // 정렬 순서: gate-check < render < solve
    let gate_pos = code.find("name: \"gate-check\"").unwrap();
    let render_pos = code.find("name: \"render\"").unwrap();
    let solve_pos = code.find("name: \"solve\"").unwrap();

    assert!(
      gate_pos < render_pos,
      "gate-check should come before render"
    );
    assert!(render_pos < solve_pos, "render should come before solve");
  }

  #[test]
  fn rust_codegen_edges_are_sorted() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let code = generate_rust_structs(&meta);

    // 정렬 순서: (gate-check, render) < (solve, gate-check)
    let first_edge = code.find("from: \"gate-check\"").unwrap();
    let second_edge = code.find("from: \"solve\"").unwrap();

    assert!(
      first_edge < second_edge,
      "edges should be sorted by (from, to)"
    );
  }

  #[test]
  fn rust_codegen_includes_edge_conditions() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let code = generate_rust_structs(&meta);

    assert!(code.contains("GeneratedEdgeCond::When"));
    assert!(code.contains("\"gate-check\""));
  }

  #[test]
  fn rust_codegen_includes_contract_data() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let code = generate_rust_structs(&meta);

    assert!(code.contains("required_inputs:"));
    assert!(code.contains("may_skip:"));
    assert!(code.contains("skip_policy:"));
    assert!(code.contains("\"data\""));
  }

  #[test]
  fn rust_codegen_includes_scope_data() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let code = generate_rust_structs(&meta);

    assert!(code.contains("GeneratedFxScope"));
    assert!(code.contains("name: \"output\""));
    assert!(code.contains("policy: \"isolate\""));
  }

  #[test]
  fn rust_codegen_uses_serde() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let code = generate_rust_structs(&meta);

    assert!(code.contains("use serde::{Serialize, Deserialize}"));
    assert!(code.contains("#[derive(Debug, Clone, Serialize, Deserialize)]"));
  }
}
