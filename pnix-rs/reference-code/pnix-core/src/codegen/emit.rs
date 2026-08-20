//! Text emission with replay hash
//!
//! 텍스트 생성만 허용
//! 컴파일러/링커/툴체인 호출 금지

use crate::build_ir::BuildIr;
use crate::codegen::normalize;
use crate::core::{FxCoreMeta, FxCoreModule, FXCORE_VERSION};
use crate::diagnostics::Diagnostics;
use crate::spec::{self, fxcore_link::SpecInjection, Spec};
use crate::ssa::SsaModule;
use crate::MeaningResult;

/// 생성된 산출물 (모두 텍스트 + replay hash)
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
#[derive(Debug, Clone, Default)]
pub struct Artifacts {
  /// FxCore 모듈 JSON (정규화됨)
  pub fxcore_json: String,
  /// SSA 모듈 JSON (정규화됨)
  pub ssa_json: String,
  /// Build IR JSON (정규화됨)
  pub build_ir_json: String,
  /// Replay 해시 (결정론 보장용)
  pub replay_hash: String,
  /// Spec canonical JSON (W03b)
  pub spec_canon_json: Option<String>,
  /// Used spec canonical JSON (W03b, W05b)
  pub used_spec_canon_json: Option<String>,
}

/// FxCore 그래프의 stage 레벨 계산
/// - Stage-1: 기본 (nodes/edges만)
/// - Stage-2: inputs/ports
/// - Stage-3: conditional edges (when/unless), gates, optional
/// - Stage-4: scopes, onfail, cost/priority
fn compute_stage(fx: &FxCoreModule) -> u8 {
  // Stage-4 체크: scopes 또는 onfail
  if !fx.scopes.is_empty() {
    return 4;
  }
  if fx.edges.iter().any(|e| {
    e.cond
      .as_ref()
      .map(|c| matches!(c, crate::core::EdgeCond::OnFail(_)))
      .unwrap_or(false)
  }) {
    return 4;
  }

  // Stage-3 체크: conditional edges (when/unless), gates, optional
  if fx.edges.iter().any(|e| e.cond.is_some()) {
    return 3;
  }
  if fx.nodes.iter().any(|n| n.optional) {
    return 3;
  }
  if fx
    .nodes
    .iter()
    .any(|n| n.kind == crate::core::NodeKind::Gate)
  {
    return 3;
  }

  // Stage-2 체크: inputs 또는 ports
  if !fx.inputs.is_empty() {
    return 2;
  }
  if fx
    .edges
    .iter()
    .any(|e| e.from_port.is_some() || e.to_port.is_some())
  {
    return 2;
  }

  // 기본 Stage-1
  1
}

/// 모든 산출물 생성
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn emit_all(
  fx: &FxCoreModule,
  ssa: &SsaModule,
  bir: &BuildIr,
  _diags: &mut Diagnostics,
) -> MeaningResult<Artifacts> {
  emit_all_with_spec(fx, ssa, bir, _diags, &Spec::with_defaults())
}

/// 모든 산출물 생성 (spec 포함)
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn emit_all_with_spec(
  fx: &FxCoreModule,
  ssa: &SsaModule,
  bir: &BuildIr,
  _diags: &mut Diagnostics,
  spec: &Spec,
) -> MeaningResult<Artifacts> {
  // Clone and inject meta information
  let mut fx_with_meta = fx.clone();
  fx_with_meta.meta = FxCoreMeta {
    version: FXCORE_VERSION.to_string(),
    stage: compute_stage(fx),
    replay_hash: None, // Will not include in JSON (skip_serializing_if)
  };

  // 1) serialize
  let fx_v = serde_json::to_value(&fx_with_meta)
    .map_err(|e| crate::MeaningError::Internal(format!("fxcore json: {e}"), None))?;
  let ssa_v = serde_json::to_value(ssa)
    .map_err(|e| crate::MeaningError::Internal(format!("ssa json: {e}"), None))?;
  let bir_v = serde_json::to_value(bir)
    .map_err(|e| crate::MeaningError::Internal(format!("build_ir json: {e}"), None))?;

  // 2) normalize + canonicalize
  let fx_n = normalize::normalize_fxcore(fx_v);
  let ssa_n = normalize::normalize_ssa(ssa_v);
  let bir_n = normalize::normalize_build_ir(bir_v);

  // 3) stable strings
  let fxcore_json = normalize::to_pretty(&fx_n);
  let ssa_json = normalize::to_pretty(&ssa_n);
  let build_ir_json = normalize::to_pretty(&bir_n);

  // 4) spec canonical JSON (W03b)
  let spec_canon_json = spec::emit_spec_canonical(spec)
    .map_err(|e| crate::MeaningError::Internal(format!("spec canonical json: {e}"), None))?;
  let spec_hash = spec::spec_hash(spec)
    .map_err(|e| crate::MeaningError::Internal(format!("spec hash: {e}"), None))?;

  // 5) used spec (W05b)
  let used_spec = fx.extract_used_spec(spec);
  let used_spec_v = serde_json::to_value(&used_spec)
    .map_err(|e| crate::MeaningError::Internal(format!("used spec json: {e}"), None))?;
  let used_spec_n = normalize::canonicalize(used_spec_v);
  let used_spec_canon_json = Some(normalize::to_pretty(&used_spec_n));

  // 6) replay hash = H(fx_n, ssa_n, bir_n, spec_hash) (W03b: spec_hash 포함)
  let replay_hash = replay_hash_with_spec(&fx_n, &ssa_n, &bir_n, &spec_hash)?;

  Ok(Artifacts {
    fxcore_json,
    ssa_json,
    build_ir_json,
    replay_hash,
    spec_canon_json: Some(spec_canon_json),
    used_spec_canon_json,
  })
}

#[allow(dead_code)] // W03b: 기존 호환성을 위해 유지
fn replay_hash(
  fx: &serde_json::Value,
  ssa: &serde_json::Value,
  bir: &serde_json::Value,
) -> crate::MeaningResult<String> {
  // 기본 spec hash 사용 (W03b: spec_hash 포함)
  let spec = Spec::with_defaults();
  let spec_hash = spec::spec_hash(&spec)
    .map_err(|e| crate::MeaningError::Internal(format!("spec hash failed: {e}"), None))?;
  replay_hash_with_spec(fx, ssa, bir, &spec_hash)
}

fn replay_hash_with_spec(
  fx: &serde_json::Value,
  ssa: &serde_json::Value,
  bir: &serde_json::Value,
  spec_hash: &str,
) -> crate::MeaningResult<String> {
  use pnix_hash::{Digest, Sha256};

  // HIGH: Serialization 실패 시 빈 벡터로 대체하지 않고 에러 전파
  // 실패한 serialization은 잘못된 해시를 생성하여 캐시 무효화 버그 발생 가능
  let fx_b = serde_json::to_vec(fx).map_err(|e| {
    crate::MeaningError::Internal(format!("fxcore serialization failed: {e}"), None)
  })?;
  let ssa_b = serde_json::to_vec(ssa)
    .map_err(|e| crate::MeaningError::Internal(format!("ssa serialization failed: {e}"), None))?;
  let bir_b = serde_json::to_vec(bir).map_err(|e| {
    crate::MeaningError::Internal(format!("build_ir serialization failed: {e}"), None)
  })?;

  let mut hasher = Sha256::new();
  hasher.update(&fx_b);
  hasher.update(&ssa_b);
  hasher.update(&bir_b);
  hasher.update(spec_hash.as_bytes()); // W03b: spec_hash 포함

  Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
  use super::compute_stage;
  use crate::core::{
    CostHint, EdgeCond, Effect, ExecutionContract, FxCoreMeta, FxCoreModule, FxEdge, FxInput,
    FxMorphism, FxNode, NodeKind, FXCORE_VERSION,
  };

  fn base_module() -> FxCoreModule {
    FxCoreModule {
      meta: FxCoreMeta {
        version: FXCORE_VERSION.to_string(),
        stage: 1,
        replay_hash: None,
      },
      name: "test".to_string(),
      types: Vec::new(),
      adt_types: Vec::new(),
      adttypes: Vec::new(),
      inputs: Vec::new(),
      morphisms: vec![FxMorphism::simple(
        "noop".to_string(),
        "Unit".to_string(),
        "Unit".to_string(),
        Effect::Pure,
      )],
      nodes: vec![FxNode {
        name: "n1".to_string(),
        uses: "noop".to_string(),
        kind: NodeKind::Normal,
        optional: false,
        scope: "global".to_string(),
        cost: CostHint::default(),
        priority: 0,
        contract: ExecutionContract::default(),

        meta: None,
      }],
      edges: Vec::new(),
      scopes: Vec::new(),
    }
  }

  #[test]
  fn compute_stage_defaults_to_stage1() {
    let fx = base_module();
    assert_eq!(compute_stage(&fx), 1);
  }

  #[test]
  fn compute_stage_detects_stage2_inputs() {
    let mut fx = base_module();
    fx.inputs.push(FxInput {
      name: "x".to_string(),
      ty: "Num".to_string(),
    });
    assert_eq!(compute_stage(&fx), 2);
  }

  #[test]
  fn compute_stage_detects_stage3_gate_node() {
    let mut fx = base_module();
    fx.nodes[0].kind = NodeKind::Gate;
    assert_eq!(compute_stage(&fx), 3);
  }

  #[test]
  fn compute_stage_detects_stage3_conditional_edge() {
    let mut fx = base_module();
    fx.edges.push(FxEdge {
      from: "n1".to_string(),
      to: "n1".to_string(),
      from_port: None,
      to_port: None,
      from_input: None,
      cond: Some(EdgeCond::When("g1".to_string())),
    });
    assert_eq!(compute_stage(&fx), 3);
  }

  #[test]
  fn compute_stage_detects_stage4_onfail_edge() {
    let mut fx = base_module();
    fx.edges.push(FxEdge {
      from: "n1".to_string(),
      to: "n1".to_string(),
      from_port: None,
      to_port: None,
      from_input: None,
      cond: Some(EdgeCond::OnFail("n2".to_string())),
    });
    assert_eq!(compute_stage(&fx), 4);
  }
}
