//! FRP 패치: 함수형 반응형 프로그래밍 그래프 수정 작업

use anyhow::{anyhow, bail, Result};
use serde::Deserialize;

use crate::state_mutation_contract::enforce_state_mutation_contract;
use pnix_runtime_legacy::frp::{
  BinOp, FrpPatch, GateCondition, LegacyFrpGraph, PatchResult, SignalExpr, SignalId, SignalKind,
  UnaryOp,
};

/// FRP 패치 파일: FRP 그래프 수정 작업 목록
#[derive(Debug, Deserialize)]
pub struct FrpPatchFile {
  /// 패치 버전
  pub version: u32,
  /// Patch identity (optional in compat mode)
  #[serde(default)]
  pub patch_id: Option<String>,
  /// Idempotency key (optional in compat mode)
  #[serde(default)]
  pub idempotency_key: Option<String>,
  /// Commit actor (must be `sequencer` when present)
  #[serde(default)]
  pub committer: Option<String>,
  /// 패치 작업 목록
  #[serde(default)]
  pub patches: Vec<FrpPatchOp>,
}

impl FrpPatchFile {
  pub fn from_json_str(input: &str) -> Result<Self> {
    let patch: FrpPatchFile = serde_json::from_str(input)?;
    if patch.version != 1 {
      bail!("unsupported frp patch version {}", patch.version);
    }
    Ok(patch)
  }
}

/// FRP 패치 작업: FRP 그래프를 수정하는 작업
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum FrpPatchOp {
  /// 시그널 추가
  AddSignal {
    /// 시그널 이름
    name: String,
    /// 시그널 종류
    kind: FrpSignalKindSpec,
  },
  /// 시그널 제거
  RemoveSignal {
    /// 시그널 이름
    name: String,
  },
  /// 시그널 업데이트
  UpdateSignal {
    /// 시그널 이름
    name: String,
    /// 새 표현식 (선택적)
    expr: Option<FrpExprSpec>,
  },
  /// 상수 값 설정
  SetConstant {
    /// 시그널 이름
    name: String,
    /// 상수 값
    value: f64,
  },
  /// 입력 값 설정
  SetInput {
    /// 시그널 이름
    name: String,
    /// 입력 값
    value: f64,
  },
  /// 상태 리셋
  ResetState {
    /// 시그널 이름
    name: String,
  },
  /// 게이트 설정
  SetGate {
    /// 시그널 이름
    signal_name: String,
    /// 게이트 조건
    gate: FrpGateSpec,
  },
}

/// FRP 시그널 종류 스펙: 시그널 타입 정의
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrpSignalKindSpec {
  /// 상수 시그널
  Constant {
    /// 상수 값
    value: f64,
  },
  /// 입력 시그널
  Input {
    /// 기본값 (선택적)
    #[serde(default)]
    default: Option<f64>,
  },
  /// 시간 시그널
  Time,
  /// 델타 시간 시그널
  DeltaTime,
}

/// FRP 게이트 스펙: 게이트 조건 정의
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrpGateSpec {
  /// 항상 활성화
  Always,
  /// 시그널이 true일 때 활성화
  WhenTrue {
    /// 참조할 시그널 이름
    signal: String,
  },
  /// 시그널이 false일 때 활성화
  WhenFalse {
    /// 참조할 시그널 이름
    signal: String,
  },
}

/// FRP 표현식 스펙: FRP 표현식 정의
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FrpExprSpec {
  /// 상수 값
  Const {
    /// 값
    value: f64,
  },
  /// 시그널 참조
  Ref {
    /// 시그널 이름
    signal: String,
  },
  /// 시간 시그널
  Time,
  /// 델타 시간 시그널
  DeltaTime,
  /// 단항 연산
  Unary {
    /// 연산자
    op: FrpUnaryOp,
    /// 피연산자 표현식
    expr: Box<FrpExprSpec>,
  },
  /// 이항 연산
  Binary {
    /// 연산자
    op: FrpBinaryOp,
    /// 왼쪽 피연산자 표현식
    lhs: Box<FrpExprSpec>,
    /// 오른쪽 피연산자 표현식
    rhs: Box<FrpExprSpec>,
  },
  /// 변수 참조
  Var {
    /// 변수 이름
    name: String,
  },
}

/// FRP 단항 연산자: 단항 연산자 타입
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum FrpUnaryOp {
  /// 부호 반전 (-)
  Neg,
  /// 사인 함수
  Sin,
  /// 코사인 함수
  Cos,
  /// 내림 (floor)
  Floor,
  /// 올림 (ceil)
  Ceil,
  /// 절댓값
  Abs,
  /// 제곱근
  Sqrt,
}

/// FRP 이항 연산자: 이항 연산자 타입
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum FrpBinaryOp {
  /// 덧셈 (+)
  Add,
  /// 뺄셈 (-)
  Sub,
  /// 곱셈 (*)
  Mul,
  /// 나눗셈 (/)
  Div,
  /// 나머지 (%)
  Mod,
}

pub fn apply_patches(graph: &mut LegacyFrpGraph, patch: FrpPatchFile) -> Result<Vec<PatchResult>> {
  enforce_state_mutation_contract(
    "frp_patch",
    patch.patch_id.as_deref(),
    patch.idempotency_key.as_deref(),
    patch.committer.as_deref(),
  )?;

  let mut results = Vec::new();
  for op in patch.patches {
    apply_patch_op(graph, op, &mut results)?;
  }
  Ok(results)
}

fn apply_patch_op(
  graph: &mut LegacyFrpGraph,
  op: FrpPatchOp,
  results: &mut Vec<PatchResult>,
) -> Result<()> {
  match op {
    FrpPatchOp::AddSignal { name, kind } => {
      let (signal_kind, default_input) = match kind {
        FrpSignalKindSpec::Constant { value } => (SignalKind::Constant(value), None),
        FrpSignalKindSpec::Input { default } => (SignalKind::Input, default),
        FrpSignalKindSpec::Time => (SignalKind::Time, None),
        FrpSignalKindSpec::DeltaTime => (SignalKind::DeltaTime, None),
      };

      let result = graph.runtime.apply_patch(&FrpPatch::AddSignal {
        name: name.clone(),
        kind: signal_kind,
      });
      results.push(result.clone());

      if result.success {
        if let Some(default) = default_input {
          let r = graph.runtime.apply_patch(&FrpPatch::SetInput {
            name,
            value: default,
          });
          results.push(r);
        }
      }
    }
    FrpPatchOp::RemoveSignal { name } => {
      let result = graph.runtime.apply_patch(&FrpPatch::RemoveSignal { name });
      results.push(result);
    }
    FrpPatchOp::UpdateSignal { name, expr } => {
      let new_expr = match expr {
        Some(spec) => Some(resolve_expr(graph, &spec)?),
        None => None,
      };
      let result = graph
        .runtime
        .apply_patch(&FrpPatch::UpdateSignal { name, new_expr });
      results.push(result);
    }
    FrpPatchOp::SetConstant { name, value } => {
      let result = graph
        .runtime
        .apply_patch(&FrpPatch::SetConstant { name, value });
      results.push(result);
    }
    FrpPatchOp::SetInput { name, value } => {
      let result = graph
        .runtime
        .apply_patch(&FrpPatch::SetInput { name, value });
      results.push(result);
    }
    FrpPatchOp::ResetState { name } => {
      let result = graph.runtime.apply_patch(&FrpPatch::ResetState { name });
      results.push(result);
    }
    FrpPatchOp::SetGate { signal_name, gate } => {
      let condition = resolve_gate(graph, gate)?;
      let result = graph.runtime.apply_patch(&FrpPatch::SetGate {
        signal_name,
        gate: condition,
      });
      results.push(result);
    }
  }

  Ok(())
}

fn resolve_gate(graph: &LegacyFrpGraph, gate: FrpGateSpec) -> Result<GateCondition> {
  match gate {
    FrpGateSpec::Always => Ok(GateCondition::Always),
    FrpGateSpec::WhenTrue { signal } => {
      let id = resolve_signal(graph, &signal)?;
      Ok(GateCondition::WhenTrue(id))
    }
    FrpGateSpec::WhenFalse { signal } => {
      let id = resolve_signal(graph, &signal)?;
      Ok(GateCondition::WhenFalse(id))
    }
  }
}

/// Maximum recursion depth for expression resolution to prevent stack overflow
const MAX_EXPR_DEPTH: usize = 1000;

fn resolve_expr(graph: &LegacyFrpGraph, expr: &FrpExprSpec) -> Result<SignalExpr> {
  resolve_expr_with_depth(graph, expr, 0)
}

fn resolve_expr_with_depth(
  graph: &LegacyFrpGraph,
  expr: &FrpExprSpec,
  depth: usize,
) -> Result<SignalExpr> {
  if depth > MAX_EXPR_DEPTH {
    bail!(
      "expression nesting depth exceeds maximum ({})",
      MAX_EXPR_DEPTH
    );
  }
  match expr {
    FrpExprSpec::Const { value } => Ok(SignalExpr::Const(*value)),
    FrpExprSpec::Ref { signal } => Ok(SignalExpr::Ref(resolve_signal(graph, signal)?)),
    FrpExprSpec::Time => Ok(SignalExpr::Time),
    FrpExprSpec::DeltaTime => Ok(SignalExpr::DeltaTime),
    FrpExprSpec::Unary { op, expr } => {
      let inner = resolve_expr_with_depth(graph, expr, depth + 1)?;
      Ok(SignalExpr::UnaryOp(map_unary(*op), Box::new(inner)))
    }
    FrpExprSpec::Binary { op, lhs, rhs } => {
      let left = resolve_expr_with_depth(graph, lhs, depth + 1)?;
      let right = resolve_expr_with_depth(graph, rhs, depth + 1)?;
      Ok(SignalExpr::BinOp(
        Box::new(left),
        map_binary(*op),
        Box::new(right),
      ))
    }
    FrpExprSpec::Var { name } => Ok(SignalExpr::Var(name.clone())),
  }
}

fn resolve_signal(graph: &LegacyFrpGraph, name: &str) -> Result<SignalId> {
  graph
    .runtime
    .find_signal_by_name(name)
    .ok_or_else(|| anyhow!("signal '{}' not found", name))
}

fn map_unary(op: FrpUnaryOp) -> UnaryOp {
  match op {
    FrpUnaryOp::Neg => UnaryOp::Neg,
    FrpUnaryOp::Sin => UnaryOp::Sin,
    FrpUnaryOp::Cos => UnaryOp::Cos,
    FrpUnaryOp::Floor => UnaryOp::Floor,
    FrpUnaryOp::Ceil => UnaryOp::Ceil,
    FrpUnaryOp::Abs => UnaryOp::Abs,
    FrpUnaryOp::Sqrt => UnaryOp::Sqrt,
  }
}

fn map_binary(op: FrpBinaryOp) -> BinOp {
  match op {
    FrpBinaryOp::Add => BinOp::Add,
    FrpBinaryOp::Sub => BinOp::Sub,
    FrpBinaryOp::Mul => BinOp::Mul,
    FrpBinaryOp::Div => BinOp::Div,
    FrpBinaryOp::Mod => BinOp::Mod,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn extract_docs_legacy_frp_patch_envelope_json() -> String {
    const DOC: &str = include_str!(concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/../../docs/patch-schema.md"
    ));
    let marker = "## Legacy FRP patch (executor legacy-frp mode)";
    let start = DOC
      .find(marker)
      .unwrap_or_else(|| panic!("docs marker not found: {marker}"));
    let tail = &DOC[start..];
    let fence_open = "```json";
    let fence_start = tail
      .find(fence_open)
      .unwrap_or_else(|| panic!("docs legacy-frp envelope code fence not found"));
    let after_open = &tail[(fence_start + fence_open.len())..];
    let fence_end = after_open
      .find("```")
      .unwrap_or_else(|| panic!("docs legacy-frp envelope code fence not closed"));
    after_open[..fence_end].trim().to_string()
  }

  fn make_graph() -> LegacyFrpGraph {
    let mut graph = LegacyFrpGraph::new();
    graph.runtime.register_constant("x", 1.0);
    graph.runtime.register_input("y");
    graph.runtime.register_constant("cond", 1.0);
    graph
  }

  fn get_value(graph: &LegacyFrpGraph, name: &str) -> Option<f64> {
    graph
      .runtime
      .all_named_values_deterministic()
      .expect("named values")
      .into_iter()
      .find(|(n, _)| n == name)
      .map(|(_, v)| v)
  }

  #[test]
  fn test_set_constant_patch() {
    let mut graph = make_graph();
    let patch = FrpPatchFile::from_json_str(
      r#"{
                "version": 1,
                "patches": [
                    { "op": "set_constant", "name": "x", "value": 42.0 }
                ]
            }"#,
    )
    .unwrap();

    let results = apply_patches(&mut graph, patch).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].success);
    assert_eq!(get_value(&graph, "x"), Some(42.0));
  }

  #[test]
  fn test_add_input_with_default() {
    let mut graph = make_graph();
    let patch = FrpPatchFile::from_json_str(
      r#"{
                "version": 1,
                "patches": [
                    { "op": "add_signal", "name": "z", "kind": { "type": "input", "default": 7.0 } }
                ]
            }"#,
    )
    .unwrap();

    let results = apply_patches(&mut graph, patch).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results[0].success);
    assert_eq!(get_value(&graph, "z"), Some(7.0));
  }

  #[test]
  fn test_set_gate() {
    let mut graph = make_graph();
    let patch = FrpPatchFile::from_json_str(
            r#"{
                "version": 1,
                "patches": [
                    { "op": "set_gate", "signal_name": "x", "gate": { "type": "when_true", "signal": "cond" } }
                ]
            }"#,
        )
        .unwrap();

    let results = apply_patches(&mut graph, patch).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].success);
  }

  #[test]
  fn test_patch_rejects_non_sequencer_committer() {
    let mut graph = make_graph();
    let patch = FrpPatchFile::from_json_str(
      r#"{
        "version": 1,
        "committer": "manual",
        "patches": []
      }"#,
    )
    .expect("parse patch");

    let err = apply_patches(&mut graph, patch).expect_err("non-sequencer must fail");
    assert!(err
      .to_string()
      .contains("STATE_MUTATION_NON_SEQUENCER_COMMIT"));
  }

  #[test]
  fn u05_docs_schema_legacy_frp_example_parses_and_applies() {
    let envelope_json = extract_docs_legacy_frp_patch_envelope_json();
    let patch = FrpPatchFile::from_json_str(&envelope_json).expect("parse docs envelope json");

    // The docs example references these pre-existing signals.
    let mut graph = LegacyFrpGraph::new();
    graph.runtime.register_external_input("mouse_x", 0.0);
    graph.runtime.register_constant("speed_plus_dt", 0.0);
    graph.runtime.register_constant("unused", 0.0);

    let results = apply_patches(&mut graph, patch).expect("apply docs patch");
    assert!(
      !results.is_empty(),
      "docs patch must contain at least one op"
    );
    for result in &results {
      assert!(result.success, "patch op failed: {result:?}");
    }

    assert_eq!(get_value(&graph, "mouse_x"), Some(100.0));
    assert_eq!(get_value(&graph, "speed"), Some(20.0));
    assert_eq!(get_value(&graph, "new_input"), Some(0.0));
    assert!(graph.runtime.find_signal_by_name("unused").is_none());
  }
}
