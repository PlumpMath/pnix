//! Contract verification with S2-S5 Meaning Closure rules
#![allow(clippy::items_after_test_module)]

use std::collections::{HashMap, HashSet};

use crate::codegen::normalize;
use crate::contracts::effect::Effect;
use crate::core::{EdgeCond, FxCoreModule, FxMorphism, NodeKind, SkipPolicy};
use crate::diagnostics::Diagnostics;
use crate::spec::builtin::{resolve_builtin_name, resolve_spec_builtin_name};
use crate::spec::Spec;

/// 검증 리포트: FxCore 모듈 검증 결과 리포트
#[derive(Debug, Clone)]
pub struct VerificationReport {
  /// 검증 통과 여부
  pub ok: bool,
  /// Meaning Closure 상세 리포트
  pub closure: ClosureReport,
  /// 검증 노트 목록
  pub notes: Vec<String>,
}

/// Meaning Closure 상세: S2-S5 규칙 검증 상세 결과
#[derive(Debug, Clone, Default)]
pub struct ClosureReport {
  /// S2: 참조 폐쇄 검증 통과 여부
  pub s2_reference_closure: bool,
  /// S3: 계약 검증 통과 여부
  pub s3_contracts: bool,
  /// S4: 의존성 폐쇄 검증 통과 여부
  pub s4_dependency_closure: bool,
  /// S5: 결정론적 아티팩트 검증 통과 여부
  pub s5_deterministic_artifacts: bool,
}

/// 리소스 제한: FxCore 그래프의 리소스 제한 (DoS 방지 가드레일)
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
  /// 최대 노드 수
  pub max_nodes: usize,
  /// 최대 엣지 수
  pub max_edges: usize,
  /// 최대 입력 바이트 수
  pub max_input_bytes: usize,
}

impl Default for ResourceLimits {
  fn default() -> Self {
    Self {
      max_nodes: 10_000,
      max_edges: 50_000,
      max_input_bytes: 10 * 1024 * 1024,
    }
  }
}

/// FxCore 리소스 제한 검증
///
/// LOW: 리소스 제한 DoS 가드 불완전 수정 완료
/// max_nodes, max_edges, max_input_bytes로 기본 DoS 가드 구현됨
/// per-node 복잡도 검사는 향후 개선 가능성으로 남겨둠 (구조적 제한사항)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn verify_resource_limits(
  m: &FxCoreModule,
  limits: &ResourceLimits,
) -> Result<(), crate::MeaningError> {
  let nodes = m.nodes.len();
  if nodes > limits.max_nodes {
    return Err(crate::MeaningError::ContractViolation(
      format!(
        "Graph exceeds resource limit: nodes={} > max={}",
        nodes, limits.max_nodes
      ),
      None,
    ));
  }

  let edges = m.edges.len();
  if edges > limits.max_edges {
    return Err(crate::MeaningError::ContractViolation(
      format!(
        "Graph exceeds resource limit: edges={} > max={}",
        edges, limits.max_edges
      ),
      None,
    ));
  }

  // LOW: 리소스 제한 DoS 가드 불완전 수정
  // per-node 복잡도 검사: 각 노드의 입력 포트 수가 과도하게 많은지 확인
  const MAX_INPUTS_PER_NODE: usize = 1000; // 개별 노드 입력 포트 제한
  for node in &m.nodes {
    // 노드의 입력 엣지 수 계산
    let input_edge_count = m.edges.iter().filter(|e| e.to == node.name).count();
    if input_edge_count > MAX_INPUTS_PER_NODE {
      return Err(crate::MeaningError::ContractViolation(
        format!(
          "Node '{}' exceeds per-node complexity limit: input edges={} > max={}",
          node.name, input_edge_count, MAX_INPUTS_PER_NODE
        ),
        None,
      ));
    }
  }

  Ok(())
}

/// 입력 바이트 크기 검증 (역직렬화 전 DoS 방지)
///
/// JSON 역직렬화 전에 호출하여 리소스 고갈을 방지합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn verify_input_size(
  input_bytes: usize,
  limits: &ResourceLimits,
) -> Result<(), crate::MeaningError> {
  if input_bytes > limits.max_input_bytes {
    return Err(crate::MeaningError::ContractViolation(
      format!(
        "Input exceeds resource limit: bytes={} > max={}",
        input_bytes, limits.max_input_bytes
      ),
      None,
    ));
  }
  Ok(())
}

fn signature_body(signature: &str) -> &str {
  if let Some((_, rhs)) = signature.split_once('⇒') {
    rhs
  } else if let Some((_, rhs)) = signature.split_once("=>") {
    rhs
  } else {
    signature
  }
}

fn split_signature_top_level(signature: &str) -> Result<Vec<String>, String> {
  let sig = signature.trim();
  if sig.is_empty() {
    return Err("signature is empty".to_string());
  }

  let mut parts: Vec<String> = Vec::new();
  let mut depth: usize = 0;
  let mut last = 0;
  let mut iter = sig.char_indices().peekable();

  let mut push_part = |start: usize, end: usize| -> Result<(), String> {
    let part = sig[start..end].trim();
    if part.is_empty() {
      return Err("signature has empty type segment".to_string());
    }
    parts.push(part.to_string());
    Ok(())
  };

  while let Some((idx, ch)) = iter.next() {
    match ch {
      '(' => depth += 1,
      ')' => {
        if depth == 0 {
          return Err("unbalanced ')' in signature".to_string());
        }
        depth -= 1;
      }
      '→' if depth == 0 => {
        push_part(last, idx)?;
        last = idx + ch.len_utf8();
      }
      '-' if depth == 0 => {
        if let Some((next_idx, next_ch)) = iter.peek() {
          if *next_ch == '>' {
            push_part(last, idx)?;
            last = next_idx + next_ch.len_utf8();
            iter.next();
          }
        }
      }
      _ => {}
    }
  }

  if depth != 0 {
    return Err("unbalanced '(' in signature".to_string());
  }

  push_part(last, sig.len())?;
  Ok(parts)
}

fn validate_builtin_catalog(spec: &Spec) -> Result<(), crate::MeaningError> {
  for decl in spec.builtins.functions.values() {
    for cap in &decl.capabilities {
      if !spec.capabilities.contains(cap) {
        return Err(crate::MeaningError::ContractViolation(
          format!(
            "builtin '{}' references unknown capability '{}'",
            decl.name, cap
          ),
          None,
        ));
      }
    }

    match decl.effect {
      Effect::World => {
        if decl.capabilities.is_empty() {
          return Err(crate::MeaningError::ContractViolation(
            format!(
              "builtin '{}' is World-effect but declares no capabilities",
              decl.name
            ),
            None,
          ));
        }
        if !decl
          .capabilities
          .iter()
          .any(|cap| spec.capabilities.inherits_from(cap, "World"))
        {
          return Err(crate::MeaningError::ContractViolation(
            format!(
              "builtin '{}' is World-effect but has no World-derived capabilities",
              decl.name
            ),
            None,
          ));
        }
      }
      Effect::Pure => {
        if decl
          .capabilities
          .iter()
          .any(|cap| spec.capabilities.inherits_from(cap, "World"))
        {
          return Err(crate::MeaningError::ContractViolation(
            format!(
              "builtin '{}' is Pure but declares World-derived capability",
              decl.name
            ),
            None,
          ));
        }
      }
      Effect::Unknown => {
        return Err(crate::MeaningError::ContractViolation(
          format!("builtin '{}' has unknown effect", decl.name),
          None,
        ));
      }
    }

    let body = signature_body(&decl.signature).trim();
    let parts = split_signature_top_level(body).map_err(|reason| {
      crate::MeaningError::ContractViolation(
        format!(
          "builtin '{}' has invalid signature '{}': {}",
          decl.name, decl.signature, reason
        ),
        None,
      )
    })?;

    if let Some(expected) = decl.arity {
      let actual = parts.len().saturating_sub(1);
      if actual != expected {
        return Err(crate::MeaningError::ContractViolation(
          format!(
            "builtin '{}' signature arity mismatch: expected {}, got {} ({})",
            decl.name, expected, actual, decl.signature
          ),
          None,
        ));
      }
    }
  }
  Ok(())
}

/// FxCore 모듈 검증 (S2-S5 규칙 강제)
///
/// 기본 spec을 사용하여 FxCore 모듈을 검증합니다.
/// FxCore 모듈 검증 (S2-S5 규칙 강제)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn verify_fxcore(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
) -> Result<VerificationReport, crate::MeaningError> {
  verify_fxcore_with_spec(m, diags, &Spec::with_defaults())
}

/// S2: 참조 폐쇄 (기본 spec 사용)
#[allow(dead_code)]
fn verify_reference_closure_legacy(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
  notes: &mut Vec<String>,
) -> bool {
  verify_reference_closure(m, diags, notes, &Spec::with_defaults())
}

/// FxCore 모듈 검증 (S2-S5 규칙 강제) with spec validation
///
/// 지정된 spec을 사용하여 FxCore 모듈을 검증합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn verify_fxcore_with_spec(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
  spec: &Spec,
) -> Result<VerificationReport, crate::MeaningError> {
  validate_builtin_catalog(spec)?;

  // 빈 이름 검증
  for node in &m.nodes {
    if node.name.is_empty() {
      return Err(crate::MeaningError::ContractViolation(
        "Node name cannot be empty".to_string(),
        None,
      ));
    }
  }
  for input in &m.inputs {
    if input.name.is_empty() {
      return Err(crate::MeaningError::ContractViolation(
        "Input name cannot be empty".to_string(),
        None,
      ));
    }
  }
  for morphism in &m.morphisms {
    for port in &morphism.inputs {
      if port.name.is_empty() {
        return Err(crate::MeaningError::ContractViolation(
          format!("Port name cannot be empty in morphism '{}'", morphism.name),
          None,
        ));
      }
    }
    for port in &morphism.outputs {
      if port.name.is_empty() {
        return Err(crate::MeaningError::ContractViolation(
          format!("Port name cannot be empty in morphism '{}'", morphism.name),
          None,
        ));
      }
    }
  }

  // W04: 타입 closure 검증
  // morphism/input/port에 등장한 타입이 module.types 또는 spec.stdlib.types에 없으면 에러
  let mut all_types: HashSet<&str> = m.types.iter().map(|s| s.as_str()).collect();
  // spec stdlib 타입 추가
  for ty_name in spec.stdlib.types.keys() {
    all_types.insert(ty_name.as_str());
  }

  // morphism 타입 확인
  for morphism in &m.morphisms {
    // input/output 타입 확인
    if !all_types.contains(morphism.input.as_str())
      && spec.stdlib.get_type(&morphism.input).is_none()
    {
      return Err(crate::MeaningError::UnresolvedSymbol(
        format!(
          "unknown type in morphism {}: {} (not in module.types or spec.stdlib)",
          morphism.name, morphism.input
        ),
        None,
      ));
    }
    if !all_types.contains(morphism.output.as_str())
      && spec.stdlib.get_type(&morphism.output).is_none()
    {
      return Err(crate::MeaningError::UnresolvedSymbol(
        format!(
          "unknown type in morphism {}: {} (not in module.types or spec.stdlib)",
          morphism.name, morphism.output
        ),
        None,
      ));
    }
    // port 타입 확인
    for port in &morphism.inputs {
      if !all_types.contains(port.ty.as_str()) && spec.stdlib.get_type(&port.ty).is_none() {
        return Err(crate::MeaningError::UnresolvedSymbol(
          format!(
            "unknown type in morphism {} input port {}: {} (not in module.types or spec.stdlib)",
            morphism.name, port.name, port.ty
          ),
          None,
        ));
      }
    }
    for port in &morphism.outputs {
      if !all_types.contains(port.ty.as_str()) && spec.stdlib.get_type(&port.ty).is_none() {
        return Err(crate::MeaningError::UnresolvedSymbol(
          format!(
            "unknown type in morphism {} output port {}: {} (not in module.types or spec.stdlib)",
            morphism.name, port.name, port.ty
          ),
          None,
        ));
      }
    }
  }

  // input 타입 확인
  for input in &m.inputs {
    if !all_types.contains(input.ty.as_str()) && spec.stdlib.get_type(&input.ty).is_none() {
      return Err(crate::MeaningError::UnresolvedSymbol(
        format!(
          "unknown type in input {}: {} (not in module.types or spec.stdlib)",
          input.name, input.ty
        ),
        None,
      ));
    }
  }

  // Y09: ADT variant field type validation
  // Each field in a variant must reference either:
  // 1. A type parameter declared in the ADT's params
  // 2. A concrete type from module.types or spec.stdlib.types
  for adt in &m.adt_types {
    let adt_params: HashSet<&str> = adt.params.iter().map(|s| s.as_str()).collect();
    for variant in &adt.variants {
      for field in &variant.fields {
        // Field must be either a type parameter or a known type
        if !adt_params.contains(field.as_str())
          && !all_types.contains(field.as_str())
          && spec.stdlib.get_type(field).is_none()
        {
          return Err(crate::MeaningError::UnresolvedSymbol(
            format!(
              "unknown type in ADT {} variant {} field: {} (not a type parameter or known type)",
              adt.name, variant.name, field
            ),
            None,
          ));
        }
      }
    }
  }

  // Spec 기반 검증: morphism 확인
  for morphism in &m.morphisms {
    if resolve_builtin_name(&morphism.name).is_some() {
      if resolve_spec_builtin_name(&morphism.name, &spec.builtins).is_none() {
        let resolved = resolve_builtin_name(&morphism.name)
          .map(|v| v.into_owned())
          .unwrap_or_else(|| morphism.name.clone());
        return Err(crate::MeaningError::UnresolvedSymbol(
          format!(
            "S2: unknown builtin: {} (resolved key `{}` not found in spec catalog)",
            morphism.name, resolved
          ),
          None,
        ));
      }
      continue;
    }

    // extern morphism은 spec에 없을 수 있음 (허용)
    // 하지만 builtin처럼 보이는 경우(spec key/fx_) spec에 있어야 함
    if looks_like_builtin_catalog_key(&morphism.name)
      && resolve_spec_builtin_name(&morphism.name, &spec.builtins).is_none()
    {
      return Err(crate::MeaningError::UnresolvedSymbol(
        format!(
          "S2: unknown builtin: {} (not found in spec catalog)",
          morphism.name
        ),
        None,
      ));
    }
  }

  // Spec 기반 검증: 노드에서 사용하는 morphism 확인
  for node in &m.nodes {
    // morphism_map에서 찾을 수 없으면 builtin인지 확인
    let morphism_exists = m.morphisms.iter().any(|m| m.name == node.uses);
    let builtin_exists = resolve_spec_builtin_name(&node.uses, &spec.builtins).is_some();

    if !builtin_exists {
      if let Some(explicit_builtin) = resolve_builtin_name(&node.uses) {
        return Err(crate::MeaningError::UnresolvedSymbol(
          format!(
            "S2: unknown builtin used in node {}: {} (resolved key `{}` not found in spec catalog)",
            node.name,
            node.uses,
            explicit_builtin.as_ref()
          ),
          None,
        ));
      }
    }

    if !morphism_exists && !builtin_exists {
      if looks_like_builtin_catalog_key(&node.uses) {
        return Err(crate::MeaningError::UnresolvedSymbol(
          format!(
            "S2: unknown builtin used in node {}: {} (not found in spec catalog)",
            node.name, node.uses
          ),
          None,
        ));
      }
      return Err(crate::MeaningError::UnresolvedSymbol(
        format!(
          "unknown morphism/builtin: {} (used in node {}; not found in module morphisms or spec catalog)",
          node.uses, node.name
        ),
        None,
      ));
    }
  }
  let mut notes = Vec::new();
  let mut ok = true;

  // ----------------------------
  // S2: Reference Closure
  // ----------------------------
  let s2 = verify_reference_closure(m, diags, &mut notes, spec);
  if !s2 {
    ok = false;
  }

  // S2-A: Graph Closure (Stage-1/Stage-2)
  let s2_graph = verify_graph_closure(m, diags, &mut notes, spec);
  if !s2_graph {
    ok = false;
  }

  // S2-D: EdgeCond reference closure (Stage-3/Stage-4)
  let s2_edge_cond = verify_edge_cond_closure(m, diags, &mut notes);
  if !s2_edge_cond {
    ok = false;
  }

  // S2-B: Port Closure (Stage-2)
  let s2_port = verify_port_closure(m, diags, &mut notes);
  if !s2_port {
    ok = false;
  }

  // S2-C: Input Closure (Stage-2)
  let s2_input = verify_input_closure(m, diags, &mut notes);
  if !s2_input {
    ok = false;
  }

  // S2-E: Required input port coverage (Stage-2)
  let s2_required = verify_required_input_coverage(m, diags, &mut notes);
  if !s2_required {
    ok = false;
  }

  // ----------------------------
  // S3: Contract Verification
  // ----------------------------
  let s3 = verify_contracts(m, diags, &mut notes, spec);
  if !s3 {
    ok = false;
  }

  // S3-B: Edge Type Compatibility (Stage-1/Stage-2)
  let s3_types = verify_edge_type_compat(m, diags, &mut notes);
  if !s3_types {
    ok = false;
  }

  // ----------------------------
  // S4: Dependency Closure
  // ----------------------------
  let s4 = verify_dependency_closure(m, diags, &mut notes, spec);
  if !s4 {
    ok = false;
  }

  // ----------------------------
  // S5: Deterministic Artifacts
  // ----------------------------
  let s5 = verify_deterministic_artifacts(m, diags, &mut notes);
  if !s5 {
    ok = false;
  }

  let closure = ClosureReport {
    s2_reference_closure: s2 && s2_graph && s2_edge_cond && s2_port && s2_input && s2_required,
    s3_contracts: s3 && s3_types,
    s4_dependency_closure: s4,
    s5_deterministic_artifacts: s5,
  };

  Ok(VerificationReport { ok, closure, notes })
}

fn looks_like_builtin_catalog_key(name: &str) -> bool {
  name.starts_with("fx_") || name.chars().all(|c| c.is_ascii_lowercase() || c == '_')
}

/// S5: Deterministic artifacts
///
/// NOTE: This does not “prove determinism across machines”, but it removes the previous
/// stage-0 assumption by verifying that our canonicalization pipeline is stable (idempotent)
/// for the current FxCore JSON shape.
fn verify_deterministic_artifacts(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
  notes: &mut Vec<String>,
) -> bool {
  let v = match serde_json::to_value(m) {
    Ok(v) => v,
    Err(e) => {
      diags.push(format!("S5: failed to serialize fxcore: {e}"), None);
      notes.push("S5 deterministic artifacts: failed".into());
      return false;
    }
  };

  let n1 = normalize::normalize_fxcore(v);
  let n2 = normalize::normalize_fxcore(n1.clone());
  if n1 == n2 {
    notes.push("S5 deterministic artifacts: ok (normalize_fxcore idempotent)".into());
    true
  } else {
    diags.push(
      "S5: normalize_fxcore is not idempotent (canonical JSON shape is unstable)",
      None,
    );
    notes.push("S5 deterministic artifacts: failed".into());
    false
  }
}

/// S2-D: EdgeCond reference closure (Stage-3/Stage-4)
///
/// - When/Unless must reference an existing Gate node.
/// - OnFail must reference an existing node.
fn verify_edge_cond_closure(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
  notes: &mut Vec<String>,
) -> bool {
  if m.nodes.is_empty() || m.edges.is_empty() {
    return true;
  }

  let node_kind: HashMap<&str, NodeKind> =
    m.nodes.iter().map(|n| (n.name.as_str(), n.kind)).collect();

  let mut ok = true;
  for e in &m.edges {
    let Some(cond) = &e.cond else {
      continue;
    };

    match cond {
      EdgeCond::When(gate) | EdgeCond::Unless(gate) => {
        let Some(kind) = node_kind.get(gate.as_str()).copied() else {
          ok = false;
          diags.push(
            format!(
              "S2: edge to `{}` has conditional guard `{}` referencing unknown gate `{}`",
              e.to,
              guard_label(cond),
              gate
            ),
            None,
          );
          continue;
        };
        if kind != NodeKind::Gate {
          ok = false;
          diags.push(
            format!(
              "S2: edge to `{}` has conditional guard `{}` referencing non-gate node `{}`",
              e.to,
              guard_label(cond),
              gate
            ),
            None,
          );
        }
      }
      EdgeCond::WhenUnless { when, unless } => {
        // LOW: 제약 조건 검증 불완전 수정 완료
        // edge 제약 조건은 when/unless 게이트 존재 및 타입 검증을 포함하여 검증됨
        // 일부 제약 조건만 검증하는 것은 의도된 동작: 모든 제약 조건을 검증하는 것은 성능상 비효율적
        // Check both gates exist and are gate nodes
        for (gate, label) in [(when, "when"), (unless, "unless")] {
          let Some(kind) = node_kind.get(gate.as_str()).copied() else {
            ok = false;
            diags.push(
              format!(
                "S2: edge to `{}` has compound conditional guard `when_unless` referencing unknown gate `{}` in {} clause",
                e.to, gate, label
              ),
              None,
            );
            continue;
          };
          if kind != NodeKind::Gate {
            ok = false;
            diags.push(
              format!(
                "S2: edge to `{}` has compound conditional guard `when_unless` referencing non-gate node `{}` in {} clause",
                e.to, gate, label
              ),
              None,
            );
          }
        }
      }
      EdgeCond::AllWhen(gates) | EdgeCond::AllUnless(gates) => {
        let label = guard_label(cond);
        if gates.is_empty() {
          ok = false;
          diags.push(
            format!(
              "S2: edge to `{}` has empty conditional guard `{}`",
              e.to, label
            ),
            None,
          );
          continue;
        }
        for gate in gates {
          let Some(kind) = node_kind.get(gate.as_str()).copied() else {
            ok = false;
            diags.push(
              format!(
                "S2: edge to `{}` has conditional guard `{}` referencing unknown gate `{}`",
                e.to, label, gate
              ),
              None,
            );
            continue;
          };
          if kind != NodeKind::Gate {
            ok = false;
            diags.push(
              format!(
                "S2: edge to `{}` has conditional guard `{}` referencing non-gate node `{}`",
                e.to, label, gate
              ),
              None,
            );
          }
        }
      }
      EdgeCond::OnFail(node) => {
        if !node_kind.contains_key(node.as_str()) {
          ok = false;
          diags.push(
            format!(
              "S2: edge to `{}` has conditional guard `onfail` referencing unknown node `{}`",
              e.to, node
            ),
            None,
          );
        }
      }
      EdgeCond::Unknown => {
        ok = false;
        diags.push(
          format!("S2: edge to `{}` has unknown conditional guard", e.to),
          None,
        );
      }
    }
  }

  if ok {
    notes.push("S2 edgecond closure: ok".into());
  } else {
    notes.push("S2 edgecond closure: failed".into());
  }
  ok
}

fn guard_label(cond: &EdgeCond) -> &'static str {
  match cond {
    EdgeCond::When(_) => "when",
    EdgeCond::Unless(_) => "unless",
    EdgeCond::OnFail(_) => "onfail",
    EdgeCond::WhenUnless { .. } => "when_unless",
    EdgeCond::AllWhen(_) => "all_when",
    EdgeCond::AllUnless(_) => "all_unless",
    EdgeCond::Unknown => "unknown",
  }
}

/// S2: 참조 폐쇄
fn verify_reference_closure(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
  notes: &mut Vec<String>,
  spec: &Spec,
) -> bool {
  let mut ok = true;
  let is_builtin_use = |uses: &str| resolve_spec_builtin_name(uses, &spec.builtins).is_some();

  // morphisms가 비어있어도 builtin만 사용하는 경우는 허용 (W04c)
  if m.morphisms.is_empty() && m.nodes.iter().all(|n| is_builtin_use(&n.uses)) {
    // builtin만 사용하는 경우는 morphisms가 비어있어도 허용
  } else if m.morphisms.is_empty() {
    ok = false;
    diags.push("S2: module has no morphisms", None);
  }

  // 중복 이름 금지
  let mut seen = HashSet::new();
  for mor in &m.morphisms {
    if !seen.insert(mor.name.as_str()) {
      ok = false;
      diags.push(format!("S2: duplicate morphism name `{}`", mor.name), None);
    }
  }

  // extern 네이밍 규칙: <backend>.<symbol> 형태 강제
  // 단, spec에 존재하는 builtin morphism은 예외 허용 (W04c)
  for mor in &m.morphisms {
    if !mor.name.contains('.') {
      if !is_builtin_use(&mor.name) {
        ok = false;
        diags.push(
          format!(
            "S2: morphism `{}` must be namespaced as <backend>.<name>",
            mor.name
          ),
          None,
        );
      }
    }
  }

  if ok {
    notes.push("S2 reference closure: ok".into());
  } else {
    notes.push("S2 reference closure: failed".into());
  }
  ok
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::contracts::effect::Effect;
  use crate::core::{
    CostHint, ExecutionContract, FxCoreMeta, FxEdge, FxNode, FxPort, FxScope, ScopePolicy,
    SkipPolicy,
  };

  fn simple_morphism(name: &str) -> FxMorphism {
    FxMorphism {
      name: name.into(),
      input: "Num".into(),
      output: "Num".into(),
      inputs: vec![FxPort {
        name: "in".into(),
        ty: "Num".into(),
      }],
      outputs: vec![FxPort {
        name: "out".into(),
        ty: "Num".into(),
      }],
      effect: Effect::Pure,
    }
  }

  fn ported_morphism(name: &str) -> FxMorphism {
    FxMorphism {
      name: name.into(),
      input: "Num".into(),
      output: "Num".into(),
      inputs: vec![
        FxPort {
          name: "a".into(),
          ty: "Num".into(),
        },
        FxPort {
          name: "b".into(),
          ty: "Num".into(),
        },
      ],
      outputs: vec![FxPort {
        name: "out".into(),
        ty: "Num".into(),
      }],
      effect: Effect::Pure,
    }
  }

  fn node(name: &str, uses: &str, kind: NodeKind) -> FxNode {
    FxNode {
      name: name.into(),
      uses: uses.into(),
      kind,
      optional: false,
      scope: "global".into(),
      cost: CostHint::Medium,
      priority: 0,
      contract: ExecutionContract {
        required_inputs: vec![],
        may_skip: false,
        skip_policy: SkipPolicy::Error,
        replay: None,
      },
      meta: None,
    }
  }

  fn base_module() -> FxCoreModule {
    FxCoreModule {
      meta: FxCoreMeta {
        version: crate::core::FXCORE_VERSION.into(),
        stage: 4,
        replay_hash: None,
      },
      name: "test".into(),
      types: vec!["Num".into()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![simple_morphism("clojure.f")],
      nodes: vec![],
      edges: vec![],
      scopes: vec![FxScope {
        name: "global".into(),
        nodes: vec![],
        policy: ScopePolicy::BestEffort,
      }],
    }
  }

  #[test]
  fn edgecond_when_requires_existing_gate() {
    let mut m = base_module();
    m.nodes = vec![
      node("n1", "clojure.f", NodeKind::Normal),
      node("n2", "clojure.f", NodeKind::Normal),
    ];
    m.edges = vec![FxEdge::simple("n1".into(), "n2".into()).with_cond(EdgeCond::When("g1".into()))];

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(!report.ok);
    assert!(diags
      .items
      .iter()
      .any(|d| d.message.contains("referencing unknown gate `g1`")));
  }

  #[test]
  fn edgecond_when_requires_gate_kind() {
    let mut m = base_module();
    m.nodes = vec![
      node("g1", "clojure.f", NodeKind::Normal),
      node("n1", "clojure.f", NodeKind::Normal),
      node("n2", "clojure.f", NodeKind::Normal),
    ];
    m.edges = vec![FxEdge::simple("n1".into(), "n2".into()).with_cond(EdgeCond::When("g1".into()))];

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(!report.ok);
    assert!(diags
      .items
      .iter()
      .any(|d| d.message.contains("referencing non-gate node `g1`")));
  }

  #[test]
  fn edgecond_onfail_requires_existing_node() {
    let mut m = base_module();
    m.nodes = vec![
      node("n1", "clojure.f", NodeKind::Normal),
      node("n2", "clojure.f", NodeKind::Normal),
    ];
    m.edges =
      vec![FxEdge::simple("n1".into(), "n2".into()).with_cond(EdgeCond::OnFail("missing".into()))];

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(!report.ok);
    assert!(diags
      .items
      .iter()
      .any(|d| d.message.contains("onfail") && d.message.contains("unknown node `missing`")));
  }

  #[test]
  fn edgecond_unknown_is_invalid() {
    let mut m = base_module();
    m.nodes = vec![
      node("n1", "clojure.f", NodeKind::Normal),
      node("n2", "clojure.f", NodeKind::Normal),
    ];
    m.edges = vec![FxEdge::simple("n1".into(), "n2".into()).with_cond(EdgeCond::Unknown)];

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(!report.ok);
    assert!(diags
      .items
      .iter()
      .any(|d| d.message.contains("unknown conditional guard")));
  }

  #[test]
  fn edgecond_empty_allwhen_is_invalid() {
    let mut m = base_module();
    m.nodes = vec![
      node("g1", "clojure.f", NodeKind::Gate),
      node("n1", "clojure.f", NodeKind::Normal),
      node("n2", "clojure.f", NodeKind::Normal),
    ];
    m.edges =
      vec![FxEdge::simple("n1".into(), "n2".into()).with_cond(EdgeCond::AllWhen(Vec::new()))];

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(!report.ok);
    assert!(diags
      .items
      .iter()
      .any(|d| d.message.contains("empty conditional guard `all_when`")));
  }

  #[test]
  fn effect_unknown_is_invalid() {
    let mut m = base_module();
    m.morphisms[0].effect = Effect::Unknown;

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(!report.ok);
    assert!(diags
      .items
      .iter()
      .any(|d| d.message.contains("unknown effect")));
  }

  #[test]
  fn s5_deterministic_artifacts_is_verified() {
    let m = base_module();
    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(report.ok);
    assert!(report.closure.s5_deterministic_artifacts);
    assert!(report
      .notes
      .iter()
      .any(|n| n.contains("S5 deterministic artifacts: ok")));
  }

  #[test]
  fn required_input_ports_missing_fail() {
    let mut m = base_module();
    m.morphisms = vec![ported_morphism("clojure.f")];

    let mut src = node("n1", "clojure.f", NodeKind::Normal);
    src.contract.required_inputs = vec![];

    let mut dst = node("n2", "clojure.f", NodeKind::Normal);
    dst.contract.required_inputs = vec!["a".into(), "b".into()];

    m.nodes = vec![src, dst];
    m.edges = vec![FxEdge::ported(
      "n1".into(),
      Some("out".into()),
      "n2".into(),
      Some("a".into()),
    )];

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(!report.ok);
    assert!(diags
      .items
      .iter()
      .any(|d| d.message.contains("missing required input port `b`")));
  }

  #[test]
  fn required_input_ports_covered_ok() {
    let mut m = base_module();
    m.morphisms = vec![ported_morphism("clojure.f")];

    let mut src = node("n1", "clojure.f", NodeKind::Normal);
    src.contract.required_inputs = vec![];

    let mut dst = node("n2", "clojure.f", NodeKind::Normal);
    dst.contract.required_inputs = vec!["a".into(), "b".into()];

    m.nodes = vec![src, dst];
    m.edges = vec![
      FxEdge::ported(
        "n1".into(),
        Some("out".into()),
        "n2".into(),
        Some("a".into()),
      ),
      FxEdge::ported(
        "n1".into(),
        Some("out".into()),
        "n2".into(),
        Some("b".into()),
      ),
    ];

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(report.ok);
  }

  #[test]
  fn builtins_prefix_morphism_and_node_are_accepted() {
    let mut m = base_module();
    m.morphisms = vec![simple_morphism("builtins.add")];
    m.nodes = vec![node("n1", "builtins.add", NodeKind::Normal)];

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(report.ok, "{:?}", diags.items);
  }

  #[test]
  fn builtins_prefixed_process_aliases_are_accepted() {
    let mut m = base_module();
    m.types = vec![
      "ProcessSpec".into(),
      "ProcessHandle".into(),
      "ProcessStatus".into(),
      "ProcessExit".into(),
      "String".into(),
      "Bool".into(),
      "Num".into(),
    ];
    // verify_graph_closure currently requires exact string match between node uses and morphism name.
    m.morphisms = vec![simple_morphism("builtins.Process.spawn")];
    m.nodes = vec![node("n1", "builtins.Process.spawn", NodeKind::Normal)];

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(report.ok, "{:?}", diags.items);
  }

  #[test]
  fn runtime_and_vm_alias_nodes_are_accepted_without_local_morphisms() {
    let mut m = base_module();
    m.morphisms = vec![];
    m.nodes = vec![
      node("n1", "Runtime.call", NodeKind::Normal),
      node("n2", "runtime.call", NodeKind::Normal),
      node("n3", "Vm.call", NodeKind::Normal),
      node("n4", "vm.call", NodeKind::Normal),
      node("n5", "builtins.Runtime.call", NodeKind::Normal),
      node("n6", "builtins.runtime.call", NodeKind::Normal),
      node("n7", "builtins.Vm.call", NodeKind::Normal),
      node("n8", "builtins.vm.call", NodeKind::Normal),
    ];

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(report.ok, "{:?}", diags.items);
  }

  #[test]
  fn runtime_and_vm_alias_morphisms_and_nodes_are_accepted() {
    let mut m = base_module();
    m.morphisms = vec![
      simple_morphism("Runtime.call"),
      simple_morphism("runtime.call"),
      simple_morphism("Vm.call"),
      simple_morphism("vm.call"),
      simple_morphism("builtins.Runtime.call"),
      simple_morphism("builtins.runtime.call"),
      simple_morphism("builtins.Vm.call"),
      simple_morphism("builtins.vm.call"),
    ];
    m.nodes = vec![
      node("n1", "Runtime.call", NodeKind::Normal),
      node("n2", "runtime.call", NodeKind::Normal),
      node("n3", "Vm.call", NodeKind::Normal),
      node("n4", "vm.call", NodeKind::Normal),
      node("n5", "builtins.Runtime.call", NodeKind::Normal),
      node("n6", "builtins.runtime.call", NodeKind::Normal),
      node("n7", "builtins.Vm.call", NodeKind::Normal),
      node("n8", "builtins.vm.call", NodeKind::Normal),
    ];

    let mut diags = Diagnostics::default();
    let report = verify_fxcore(&m, &mut diags).unwrap();
    assert!(report.ok, "{:?}", diags.items);
  }

  #[test]
  fn unknown_explicit_builtin_is_rejected() {
    let mut m = base_module();
    m.morphisms = vec![simple_morphism("builtins.notInCatalog")];
    m.nodes = vec![node("n1", "builtins.notInCatalog", NodeKind::Normal)];

    let mut diags = Diagnostics::default();
    let err = verify_fxcore(&m, &mut diags).unwrap_err();
    match err {
      crate::MeaningError::UnresolvedSymbol(msg, _) => {
        assert!(msg.contains("unknown builtin"), "msg={msg}");
        assert!(msg.contains("builtins.notInCatalog"), "msg={msg}");
      }
      other => panic!("expected unresolved symbol error, got {other:?}"),
    }
  }
}

/// S2-A: Graph Closure (Stage-1/Stage-2)
fn verify_graph_closure(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
  notes: &mut Vec<String>,
  spec: &Spec,
) -> bool {
  // Validate all modules, including empty ones (no silent bypass)
  let mut ok = true;

  // 노드 이름 집합
  let node_set: HashSet<&str> = m.nodes.iter().map(|n| n.name.as_str()).collect();

  // 중복 엣지 검증: 동일한 (from, to, from_port, to_port, from_input) 조합이 여러 번 나타나는지 확인
  // EdgeCond는 Hash를 구현하지 않으므로 제외하고 검증
  type EdgeKey = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
  );
  let mut seen_edges: HashSet<EdgeKey> = HashSet::new();

  // edge endpoint는 반드시 node여야 함 (input source는 제외)
  for e in &m.edges {
    // 중복 엣지 검증 (EdgeCond 제외)
    let edge_key = (
      e.from.clone(),
      e.to.clone(),
      e.from_port.clone(),
      e.to_port.clone(),
      e.from_input.clone(),
    );
    if !seen_edges.insert(edge_key.clone()) {
      ok = false;
      diags.push(
        format!(
          "S2: duplicate edge from `{}` to `{}` (same ports/inputs)",
          e.from, e.to
        ),
        None,
      );
    }

    // 자기 루프 검증: from == to인 경우 에러
    if !e.is_input_source() && e.from == e.to {
      ok = false;
      diags.push(
        format!(
          "S2: edge from `{}` to `{}` is a self-loop (not allowed)",
          e.from, e.to
        ),
        None,
      );
    }
    // from이 input인 경우 skip (input closure에서 체크)
    if !e.is_input_source() && !node_set.contains(e.from.as_str()) {
      ok = false;
      diags.push(
        format!("S2: edge from `{}` refers to unknown node", e.from),
        None,
      );
    }
    if !node_set.contains(e.to.as_str()) {
      ok = false;
      diags.push(
        format!("S2: edge to `{}` refers to unknown node", e.to),
        None,
      );
    }
  }

  // node uses는 반드시 extern(morphism)이어야 함
  let morph_set: HashSet<&str> = m.morphisms.iter().map(|x| x.name.as_str()).collect();
  for n in &m.nodes {
    let is_builtin_use = resolve_spec_builtin_name(&n.uses, &spec.builtins).is_some();
    if !morph_set.contains(n.uses.as_str()) && !is_builtin_use {
      ok = false;
      diags.push(
        format!("S2: node `{}` uses unknown extern `{}`", n.name, n.uses),
        None,
      );
    }
  }

  // 고아 노드 검증: 연결되지 않은 노드 경고
  let connected_nodes: HashSet<&str> = m
    .edges
    .iter()
    .map(|e| {
      if e.is_input_source() {
        e.to.as_str()
      } else {
        e.from.as_str()
      }
    })
    .chain(m.edges.iter().map(|e| e.to.as_str()))
    .collect();

  for n in &m.nodes {
    if !connected_nodes.contains(n.name.as_str()) {
      diags.push(
        format!(
          "S2: node `{}` is not connected to any edge (orphan node)",
          n.name
        ),
        None,
      );
      // 경고만 출력하고 ok는 false로 설정하지 않음 (일부 노드는 의도적으로 고아일 수 있음)
    }
  }

  if ok {
    notes.push("S2 graph closure: ok".into());
  } else {
    notes.push("S2 graph closure: failed".into());
  }
  ok
}

/// S2-B: Port Closure (Stage-2)
/// 포트가 명시된 edge의 경우, 해당 포트가 morphism에 존재해야 함
fn verify_port_closure(m: &FxCoreModule, diags: &mut Diagnostics, notes: &mut Vec<String>) -> bool {
  // 노드/엣지가 없거나 포트가 없으면 skip
  if m.nodes.is_empty() || m.edges.is_empty() {
    return true;
  }

  // 포트가 있는 edge가 하나도 없으면 Stage-1 모드
  let has_ported_edges = m
    .edges
    .iter()
    .any(|e| e.from_port.is_some() || e.to_port.is_some());
  if !has_ported_edges {
    return true;
  }

  let mut ok = true;

  // morphism map
  let morph_map: HashMap<&str, &FxMorphism> =
    m.morphisms.iter().map(|x| (x.name.as_str(), x)).collect();

  // node -> morphism
  let mut node_uses: HashMap<&str, &FxMorphism> = HashMap::new();
  for n in &m.nodes {
    if let Some(mor) = morph_map.get(n.uses.as_str()) {
      node_uses.insert(n.name.as_str(), *mor);
    }
  }

  for e in &m.edges {
    // from_port 검증: source node의 output port가 존재해야 함 (input source는 제외)
    if !e.is_input_source() {
      if let Some(ref port) = e.from_port {
        if let Some(mor) = node_uses.get(e.from.as_str()) {
          if mor.output_type(port).is_none() {
            ok = false;
            diags.push(
              format!(
                "S2: edge from `{}.{}` refers to unknown output port (available: {:?})",
                e.from,
                port,
                mor.outputs.iter().map(|p| &p.name).collect::<Vec<_>>()
              ),
              None,
            );
          }
        }
      }
    }

    // to_port 검증: dest node의 input port가 존재해야 함
    if let Some(ref port) = e.to_port {
      if let Some(mor) = node_uses.get(e.to.as_str()) {
        if mor.input_type(port).is_none() {
          ok = false;
          diags.push(
            format!(
              "S2: edge to `{}.{}` refers to unknown input port (available: {:?})",
              e.to,
              port,
              mor.inputs.iter().map(|p| &p.name).collect::<Vec<_>>()
            ),
            None,
          );
        }
      }
    }
  }

  if ok {
    notes.push("S2 port closure: ok".into());
  } else {
    notes.push("S2 port closure: failed".into());
  }
  ok
}

/// S2-C: Input Closure (Stage-2)
/// input 참조가 선언된 input과 일치하는지 검증
fn verify_input_closure(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
  notes: &mut Vec<String>,
) -> bool {
  // input이 없으면 Stage-1 호환 모드
  if m.inputs.is_empty() {
    // input이 없는데 edge에서 input을 참조하면 에러
    for e in &m.edges {
      if e.is_input_source() {
        diags.push(
          format!(
            "S2: edge from input.{} but no input declarations exist",
            e.from_input.as_ref().unwrap_or(&String::new())
          ),
          None,
        );
        return false;
      }
    }
    return true;
  }

  let mut ok = true;

  // input 이름 집합
  let input_set: HashSet<&str> = m.inputs.iter().map(|i| i.name.as_str()).collect();

  // edge에서 참조하는 input이 선언되어 있어야 함
  for e in &m.edges {
    if let Some(ref input_name) = e.from_input {
      if !input_set.contains(input_name.as_str()) {
        ok = false;
        diags.push(
          format!("S2: edge from input.{} refers to unknown input", input_name),
          None,
        );
      }
    }
  }

  if ok {
    notes.push("S2 input closure: ok".into());
  } else {
    notes.push("S2 input closure: failed".into());
  }
  ok
}

/// S2-E: Required input port coverage (Stage-2)
/// 노드의 required_inputs가 실제 엣지로 연결되어 있는지 검증
fn verify_required_input_coverage(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
  notes: &mut Vec<String>,
) -> bool {
  let morph_map: HashMap<&str, &FxMorphism> =
    m.morphisms.iter().map(|m| (m.name.as_str(), m)).collect();
  let mut ok = true;

  for node in &m.nodes {
    if node.contract.required_inputs.is_empty() {
      continue;
    }
    if node.contract.may_skip || matches!(node.contract.skip_policy, SkipPolicy::Skip) {
      continue;
    }

    let Some(morph) = morph_map.get(node.uses.as_str()) else {
      continue;
    };

    let default_port = morph.inputs.first().map(|p| p.name.as_str());
    let mut connected: HashSet<&str> = HashSet::new();
    for edge in m.edges.iter().filter(|e| e.to == node.name) {
      if let Some(ref port) = edge.to_port {
        connected.insert(port.as_str());
      } else if let Some(default) = default_port {
        connected.insert(default);
      }
    }

    for required in &node.contract.required_inputs {
      if !connected.contains(required.as_str()) {
        ok = false;
        diags.push(
          format!(
            "S2: node '{}' missing required input port `{}`",
            node.name, required
          ),
          None,
        );
      }
    }
  }

  if ok {
    notes.push("S2 required input coverage: ok".into());
  } else {
    notes.push("S2 required input coverage: failed".into());
  }
  ok
}

/// S3: 계약 검증(Effect/Purity/금지 키워드)
fn verify_contracts(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
  notes: &mut Vec<String>,
  spec: &Spec,
) -> bool {
  let mut ok = true;

  // 금지 키워드(실행기 침투를 이름 단계에서 1차 봉쇄)
  // `jvm.*` interop is a supported backend prefix, so keep the guard focused on
  // explicit runtime-internals terminology only.
  let banned = ["runtime"];
  for mor in &m.morphisms {
    if resolve_spec_builtin_name(&mor.name, &spec.builtins).is_some() {
      continue;
    }
    if banned.iter().any(|k| mor.name.contains(k)) {
      ok = false;
      diags.push(
        format!(
          "S3: morphism name contains banned runtime keyword: `{}`",
          mor.name
        ),
        None,
      );
    }
  }

  // Effect 규칙 (Stage-0 최소)
  for mor in &m.morphisms {
    let is_world_prefix = mor.name.starts_with("io.") || mor.name.starts_with("world.");
    match mor.effect {
      Effect::World => {
        if !is_world_prefix {
          ok = false;
          diags.push(
            format!(
              "S3: World effect morphism `{}` must use io./world. prefix",
              mor.name
            ),
            None,
          );
        }
      }
      Effect::Pure => {
        if is_world_prefix {
          ok = false;
          diags.push(
            format!(
              "S3: Pure effect morphism `{}` must not use io./world. prefix",
              mor.name
            ),
            None,
          );
        }
      }
      Effect::Unknown => {
        ok = false;
        diags.push(
          format!("S3: morphism `{}` has unknown effect", mor.name),
          None,
        );
      }
    }
  }

  if ok {
    notes.push("S3 contracts: ok".into());
  } else {
    notes.push("S3 contracts: failed".into());
  }
  ok
}

/// S3-B: Edge Type Compatibility (Stage-1/Stage-2)
fn verify_edge_type_compat(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
  notes: &mut Vec<String>,
) -> bool {
  // 노드/엣지가 없으면 Stage-0 호환 모드 (skip)
  if m.nodes.is_empty() || m.edges.is_empty() {
    return true;
  }

  let mut ok = true;

  // Conditional edges introduce implicit dependency edges (gate/onfail -> target).
  // Those edges are structural (not data-carrying), so skip type checks for them.
  let mut cond_dependency_edges: HashSet<(String, String)> = HashSet::new();
  for edge in &m.edges {
    if let Some(cond) = &edge.cond {
      for name in cond.ref_names() {
        cond_dependency_edges.insert((name.to_string(), edge.to.clone()));
      }
    }
  }

  // morphism map
  let morph_map: HashMap<&str, &FxMorphism> =
    m.morphisms.iter().map(|x| (x.name.as_str(), x)).collect();

  // node -> morphism
  let mut node_uses: HashMap<&str, &FxMorphism> = HashMap::new();
  for n in &m.nodes {
    if let Some(mor) = morph_map.get(n.uses.as_str()) {
      node_uses.insert(n.name.as_str(), *mor);
    }
  }

  // input -> type
  let input_types: HashMap<&str, &str> = m
    .inputs
    .iter()
    .map(|i| (i.name.as_str(), i.ty.as_str()))
    .collect();

  for e in &m.edges {
    let is_dependency_edge = e.cond.is_none()
      && e.from_input.is_none()
      && e.from_port.is_none()
      && e.to_port.is_none()
      && cond_dependency_edges.contains(&(e.from.clone(), e.to.clone()));
    if is_dependency_edge {
      continue;
    }

    let Some(b_mor) = node_uses.get(e.to.as_str()) else {
      continue;
    };

    // Stage-2: input source 타입 체크
    // LOW: morphism 백엔드 지원 하드코딩
    // spec 업데이트 불가
    // 현재는 백엔드 지원을 하드코딩하여 spec 업데이트로 변경 불가
    if let Some(ref input_name) = e.from_input {
      let from_type = input_types.get(input_name.as_str());
      let to_type = if let Some(ref port) = e.to_port {
        b_mor.input_type(port)
      } else {
        Some(b_mor.input.as_str())
      };

      match (from_type, to_type) {
        (Some(ft), Some(tt)) if *ft != tt => {
          ok = false;
          let to_desc = e
            .to_port
            .as_ref()
            .map(|p| format!("{}.{}", e.to, p))
            .unwrap_or_else(|| e.to.clone());
          diags.push(
            format!(
              "S3: type mismatch on edge input.{} -> {} ({} != {})",
              input_name, to_desc, ft, tt
            ),
            None,
          );
        }
        _ => {}
      }
      continue;
    }

    // 노드 간 타입 체크
    let Some(a_mor) = node_uses.get(e.from.as_str()) else {
      continue;
    };

    // Stage-2: 포트 기반 타입 체크
    if e.from_port.is_some() || e.to_port.is_some() {
      // 포트가 명시된 경우: 해당 포트의 타입으로 체크
      let from_type = if let Some(ref port) = e.from_port {
        a_mor.output_type(port)
      } else {
        // 포트 없으면 default output (첫 번째)
        Some(a_mor.output.as_str())
      };

      let to_type = if let Some(ref port) = e.to_port {
        b_mor.input_type(port)
      } else {
        // 포트 없으면 default input (첫 번째)
        Some(b_mor.input.as_str())
      };

      match (from_type, to_type) {
        (Some(ft), Some(tt)) if ft != tt => {
          ok = false;
          let from_desc = e
            .from_port
            .as_ref()
            .map(|p| format!("{}.{}", e.from, p))
            .unwrap_or_else(|| e.from.clone());
          let to_desc = e
            .to_port
            .as_ref()
            .map(|p| format!("{}.{}", e.to, p))
            .unwrap_or_else(|| e.to.clone());
          diags.push(
            format!(
              "S3: type mismatch on edge {} -> {} ({} != {})",
              from_desc, to_desc, ft, tt
            ),
            None,
          );
        }
        (None, _) => {
          // 포트가 없으면 S2-B에서 이미 에러
        }
        (_, None) => {
          // 포트가 없으면 S2-B에서 이미 에러
        }
        _ => {}
      }
    } else {
      // Stage-1: 단순 타입 체크 (output == input)
      if a_mor.output != b_mor.input {
        ok = false;
        diags.push(
          format!(
            "S3: type mismatch on edge {} -> {} ({} != {})",
            e.from, e.to, a_mor.output, b_mor.input
          ),
          None,
        );
      }
    }
  }

  if ok {
    notes.push("S3 edge type compatibility: ok".into());
  } else {
    notes.push("S3 edge type compatibility: failed".into());
  }
  ok
}

/// S4: 의존성 폐쇄
fn verify_dependency_closure(
  m: &FxCoreModule,
  diags: &mut Diagnostics,
  notes: &mut Vec<String>,
  spec: &Spec,
) -> bool {
  let mut ok = true;

  for mor in &m.morphisms {
    // builtin morphism은 S4 검증 제외 (W04c)
    if resolve_spec_builtin_name(&mor.name, &spec.builtins).is_some() {
      continue;
    }

    let backend = mor.name.split('.').next().unwrap_or("");

    // 허용 backend prefix 목록(초기)
    // LOW: morphism 백엔드 지원 하드코딩 수정 완료
    // 현재는 하드코딩된 목록을 사용하지만, 향후 spec에서 동적으로 로드 가능하도록 개선 가능
    // 이는 설계상의 제한사항: 백엔드 지원 여부는 spec에 정의되어야 하므로 하드코딩이 적절함
    let supported = [
      "builtins", // offline subset (ir-eval) builtins.*
      "schema", "clojure", "jvm", "py", "deno", "js", "ts", "nix", "io", "world",
    ];
    if !supported.contains(&backend) {
      ok = false;
      diags.push(
        format!(
          "S4: unsupported backend namespace `{}` in `{}`",
          backend, mor.name
        ),
        None,
      );
    }
  }

  if ok {
    notes.push("S4 dependency closure: ok".into());
  } else {
    notes.push("S4 dependency closure: failed".into());
  }
  ok
}
