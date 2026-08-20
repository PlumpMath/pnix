//! TypeChecker: FxCoreModule 타입 검증
//!
//! 그래프의 노드/엣지 타입 호환성을 검증.
//!
//! ## 검증 항목
//!
//! 1. Morphism 시그니처 일관성
//! 2. Edge 연결 타입 호환성
//! 3. Optional 노드 출력 처리
//! 4. Ported morphism 포트 매칭

use super::{CoreType, SchemaArrow, SubtypingChecker};
use crate::core::FxCoreModule;
use std::collections::HashMap;
use thiserror::Error;

/// 타입 검증 결과
#[derive(Debug)]
pub struct TypeCheckResult {
  /// 검증 성공 여부
  pub success: bool,
  /// 발견된 에러들
  pub errors: Vec<TypeError>,
  /// 발견된 경고들
  pub warnings: Vec<TypeWarning>,
  /// 추론된 노드 타입
  pub node_types: HashMap<String, NodeTypeInfo>,
}

impl TypeCheckResult {
  /// 새 타입 검증 결과 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  fn new() -> Self {
    Self {
      success: true,
      errors: Vec::new(),
      warnings: Vec::new(),
      node_types: HashMap::new(),
    }
  }

  fn add_error(&mut self, error: TypeError) {
    self.success = false;
    self.errors.push(error);
  }

  fn add_warning(&mut self, warning: TypeWarning) {
    self.warnings.push(warning);
  }
}

/// 노드 타입 정보
#[derive(Debug, Clone)]
pub struct NodeTypeInfo {
  /// 입력 타입
  pub input: CoreType,
  /// 출력 타입
  pub output: CoreType,
  /// Optional 노드인 경우 출력이 Optional로 래핑됨
  pub optional_wrapped: bool,
}

/// 타입 에러
///
/// # Example
/// ```rust
/// use pnix_core::types::TypeError;
/// let err = TypeError::UnknownMorphism { name: "add".to_string() };
/// assert!(matches!(err, TypeError::UnknownMorphism { .. }));
/// ```
/// 타입 에러: 타입 검증 에러 타입
#[derive(Debug, Error)]
pub enum TypeError {
  #[error("Edge type mismatch: {from_node}.{from_port:?} ({from_type}) -> {to_node}.{to_port:?} ({to_type})")]
  EdgeTypeMismatch {
    /// 출발 노드 이름
    from_node: String,
    /// 출발 포트 이름 (선택)
    from_port: Option<String>,
    /// 출발 타입
    from_type: CoreType,
    /// 도착 노드 이름
    to_node: String,
    /// 도착 포트 이름 (선택)
    to_port: Option<String>,
    /// 도착 타입
    to_type: CoreType,
  },

  #[error("Unknown morphism: {name}")]
  UnknownMorphism {
    /// Morphism 이름
    name: String,
  },

  #[error("Port not found: {port} on morphism {morphism}")]
  PortNotFound {
    /// Morphism 이름
    morphism: String,
    /// 포트 이름
    port: String,
  },

  #[error("Missing required input: {node} requires {input}")]
  MissingInput {
    /// 노드 이름
    node: String,
    /// 필요한 입력 이름
    input: String,
  },

  #[error("Cyclic dependency detected: {cycle:?}")]
  CyclicDependency {
    /// 순환 의존성 노드 목록
    cycle: Vec<String>,
  },
}

/// 타입 경고: 타입 검증 경고 타입
#[derive(Debug)]
pub enum TypeWarning {
  /// Optional 노드 출력이 non-optional 입력에 연결됨
  OptionalToRequired {
    /// 출발 노드 이름
    from_node: String,
    /// 도착 노드 이름
    to_node: String,
  },

  /// 암시적 타입 변환 발생
  ImplicitCoercion {
    /// 출발 노드 이름
    from_node: String,
    /// 도착 노드 이름
    to_node: String,
    arrow: SchemaArrow,
  },

  /// 사용되지 않는 노드 출력
  UnusedOutput { node: String },
}

/// 타입 체커: FxCoreModule의 타입 검증을 수행하는 체커
pub struct TypeChecker {
  /// Morphism 시그니처 저장소 (morphism 이름 → 시그니처 매핑)
  morphism_sigs: HashMap<String, MorphismSig>,
  /// Subtyping 검사기 (타입 간 subtyping 관계 검사)
  subtyping: SubtypingChecker,
}

/// Morphism 시그니처: Morphism의 입출력 타입 정보
#[derive(Debug, Clone)]
pub struct MorphismSig {
  /// 단순 입력 타입 (Stage-1 호환)
  pub input: CoreType,
  /// 단순 출력 타입 (Stage-1 호환)
  pub output: CoreType,
  /// 포트 기반 입력 (Stage-2, 포트 이름 → 타입 매핑)
  pub inputs: Vec<(String, CoreType)>,
  /// 포트 기반 출력 (Stage-2, 포트 이름 → 타입 매핑)
  pub outputs: Vec<(String, CoreType)>,
}

impl MorphismSig {
  /// Stage-1 단순 시그니처
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn simple(input: CoreType, output: CoreType) -> Self {
    Self {
      input,
      output,
      inputs: Vec::new(),
      outputs: Vec::new(),
    }
  }

  /// Stage-2 포트 기반 시그니처
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ported(
    inputs: Vec<(impl Into<String>, CoreType)>,
    outputs: Vec<(impl Into<String>, CoreType)>,
  ) -> Self {
    let inputs: Vec<_> = inputs.into_iter().map(|(k, v)| (k.into(), v)).collect();
    let outputs: Vec<_> = outputs.into_iter().map(|(k, v)| (k.into(), v)).collect();

    // 전체 입출력 타입은 포트들의 Record 타입
    let input = if inputs.is_empty() {
      CoreType::Unit
    } else {
      CoreType::Record(inputs.clone())
    };

    let output = if outputs.is_empty() {
      CoreType::Unit
    } else {
      CoreType::Record(outputs.clone())
    };

    Self {
      input,
      output,
      inputs,
      outputs,
    }
  }

  /// 입력 포트 타입 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get_input_port(&self, name: &str) -> Option<&CoreType> {
    self.inputs.iter().find(|(n, _)| n == name).map(|(_, t)| t)
  }

  /// 출력 포트 타입 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get_output_port(&self, name: &str) -> Option<&CoreType> {
    self.outputs.iter().find(|(n, _)| n == name).map(|(_, t)| t)
  }

  /// Stage-2 포트 기반인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_ported(&self) -> bool {
    !self.inputs.is_empty() || !self.outputs.is_empty()
  }
}

impl TypeChecker {
  /// 새 타입 체커 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      morphism_sigs: HashMap::new(),
      subtyping: SubtypingChecker::new(),
    }
  }

  /// Morphism 시그니처 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register_morphism(&mut self, name: impl Into<String>, sig: MorphismSig) {
    self.morphism_sigs.insert(name.into(), sig);
  }

  /// FxCoreModule에서 morphism 추출하여 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 파싱 및 등록만, 값 계산 없음
  pub fn register_from_module(&mut self, module: &FxCoreModule) {
    for morphism in &module.morphisms {
      let sig = if morphism.inputs.is_empty() && morphism.outputs.is_empty() {
        // Stage-1 단순 시그니처
        MorphismSig::simple(
          CoreType::parse(&morphism.input),
          CoreType::parse(&morphism.output),
        )
      } else {
        // Stage-2 포트 기반 시그니처
        let inputs: Vec<_> = morphism
          .inputs
          .iter()
          .map(|p| (p.name.clone(), CoreType::parse(&p.ty)))
          .collect();
        let outputs: Vec<_> = morphism
          .outputs
          .iter()
          .map(|p| (p.name.clone(), CoreType::parse(&p.ty)))
          .collect();
        MorphismSig::ported(inputs, outputs)
      };

      self.morphism_sigs.insert(morphism.name.clone(), sig);
    }
  }

  /// FxCoreModule 타입 검증: FxCoreModule의 타입 호환성 검증
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check(&mut self, module: &FxCoreModule) -> TypeCheckResult {
    let mut result = TypeCheckResult::new();

    // 1. 모듈에서 morphism 등록
    self.register_from_module(module);

    // 2. 노드 타입 추론
    for node in &module.nodes {
      if let Some(sig) = self.morphism_sigs.get(&node.uses) {
        let mut output = sig.output.clone();

        // Optional 노드는 출력이 Optional로 래핑됨
        let optional_wrapped = node.optional;
        if optional_wrapped && !output.is_optional() {
          output = CoreType::optional(output);
        }

        result.node_types.insert(
          node.name.clone(),
          NodeTypeInfo {
            input: sig.input.clone(),
            output,
            optional_wrapped,
          },
        );
      } else {
        result.add_error(TypeError::UnknownMorphism {
          name: node.uses.clone(),
        });
      }
    }

    // 3. 엣지 타입 호환성 검증
    for edge in &module.edges {
      self.check_edge(module, edge, &mut result);
    }

    // 4. 사용되지 않는 출력 경고
    self.check_unused_outputs(module, &mut result);

    result
  }

  fn check_edge(
    &mut self,
    module: &FxCoreModule,
    edge: &crate::core::FxEdge,
    result: &mut TypeCheckResult,
  ) {
    // Get source type
    let from_type = self.get_edge_source_type(module, edge, result);

    // Get target type
    let to_type = self.get_edge_target_type(module, edge, result);

    let (from_type, to_type) = match (from_type, to_type) {
      (Some(f), Some(t)) => (f, t),
      _ => return, // Error already reported
    };

    // Warn if optional output connects to required input
    if let Some(from_info) = result.node_types.get(&edge.from) {
      if from_info.optional_wrapped && !to_type.is_optional() {
        result.add_warning(TypeWarning::OptionalToRequired {
          from_node: edge.from.clone(),
          to_node: edge.to.clone(),
        });
      }
    }

    // Check subtyping
    if let Some(ev) = self.subtyping.check(&from_type, &to_type) {
      // Subtyping succeeded
      if !matches!(ev, super::SubtypingEvidence::Reflexive) {
        // Implicit coercion happened
        result.add_warning(TypeWarning::ImplicitCoercion {
          from_node: edge.from.clone(),
          to_node: edge.to.clone(),
          arrow: SchemaArrow::Subtyping {
            from: from_type.clone(),
            to: to_type.clone(),
            evidence: ev,
          },
        });
      }
    } else {
      // Type mismatch
      result.add_error(TypeError::EdgeTypeMismatch {
        from_node: edge.from.clone(),
        from_port: edge.from_port.clone(),
        from_type: from_type.clone(),
        to_node: edge.to.clone(),
        to_port: edge.to_port.clone(),
        to_type: to_type.clone(),
      });
    }
  }

  fn get_edge_source_type(
    &self,
    module: &FxCoreModule,
    edge: &crate::core::FxEdge,
    result: &mut TypeCheckResult,
  ) -> Option<CoreType> {
    let from_name = edge.from_input.as_deref().unwrap_or(edge.from.as_str());

    // Check if it's an input reference
    if let Some(input) = module.inputs.iter().find(|i| i.name == from_name) {
      return Some(CoreType::parse(&input.ty));
    }

    // Otherwise it's a node
    let node = module.nodes.iter().find(|n| n.name == from_name)?;
    let sig = self.morphism_sigs.get(&node.uses)?;

    if let Some(port_name) = edge.from_port.as_deref() {
      // Ported output
      if let Some(ty) = sig.get_output_port(port_name) {
        Some(ty.clone())
      } else {
        result.add_error(TypeError::PortNotFound {
          morphism: node.uses.clone(),
          port: port_name.to_string(),
        });
        None
      }
    } else {
      // Simple output (stage-1 compat: allow single-port default)
      let mut output = if sig.outputs.len() == 1 {
        sig.outputs[0].1.clone()
      } else {
        sig.output.clone()
      };
      if node.optional && !output.is_optional() {
        output = CoreType::optional(output);
      }
      Some(output)
    }
  }

  fn get_edge_target_type(
    &self,
    module: &FxCoreModule,
    edge: &crate::core::FxEdge,
    result: &mut TypeCheckResult,
  ) -> Option<CoreType> {
    let info = result.node_types.get(&edge.to)?;

    if let Some(port_name) = edge.to_port.as_deref() {
      // Ported input - need to look up morphism sig
      let node = module.nodes.iter().find(|n| n.name == edge.to)?;
      let sig = self.morphism_sigs.get(&node.uses)?;

      if let Some(ty) = sig.get_input_port(port_name) {
        Some(ty.clone())
      } else {
        result.add_error(TypeError::PortNotFound {
          morphism: node.uses.clone(),
          port: port_name.to_string(),
        });
        None
      }
    } else {
      let node = module.nodes.iter().find(|n| n.name == edge.to)?;
      let sig = self.morphism_sigs.get(&node.uses)?;
      if sig.inputs.len() == 1 {
        Some(sig.inputs[0].1.clone())
      } else {
        Some(info.input.clone())
      }
    }
  }

  fn check_unused_outputs(&self, module: &FxCoreModule, result: &mut TypeCheckResult) {
    let mut used_outputs: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for edge in &module.edges {
      used_outputs.insert(&edge.from);
    }

    for node in &module.nodes {
      if !used_outputs.contains(node.name.as_str()) && !node.optional {
        // Check if this is a terminal node (no outgoing edges expected)
        // For now, just add as warning
        result.add_warning(TypeWarning::UnusedOutput {
          node: node.name.clone(),
        });
      }
    }
  }
}

impl Default for TypeChecker {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::contracts::effect::Effect;
  use crate::core::{CostHint, FxEdge, FxInput, FxMorphism, FxNode, FxPort, NodeKind};

  fn make_test_module() -> FxCoreModule {
    FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      types: vec!["Position".into(), "Velocity".into(), "Force".into()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![
        FxMorphism {
          name: "physics.integrate".into(),
          input: "Position * Velocity".into(),
          output: "Position".into(),
          inputs: vec![],
          outputs: vec![],
          effect: Effect::Pure,
        },
        FxMorphism {
          name: "physics.force".into(),
          input: "Position".into(),
          output: "Force".into(),
          inputs: vec![],
          outputs: vec![],
          effect: Effect::Pure,
        },
      ],
      nodes: vec![
        FxNode {
          name: "integrate".into(),
          uses: "physics.integrate".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: Default::default(),

          meta: None,
        },
        FxNode {
          name: "force".into(),
          uses: "physics.force".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Light,
          priority: 0,
          contract: Default::default(),

          meta: None,
        },
      ],
      edges: vec![FxEdge {
        from: "integrate".into(),
        to: "force".into(),
        cond: None,
        from_port: None,
        to_port: None,
        from_input: None,
      }],
      scopes: vec![],
    }
  }

  #[test]
  fn test_type_checker_basic() {
    let module = make_test_module();
    let mut checker = TypeChecker::new();

    let result = checker.check(&module);

    // integrate: Position * Velocity -> Position
    // force: Position -> Force
    // Edge: integrate.output (Position) -> force.input (Position) ✓
    assert!(
      result.success,
      "Type check should succeed: {:?}",
      result.errors
    );
  }

  #[test]
  fn test_type_mismatch() {
    let mut module = make_test_module();

    // Change force to expect Velocity instead of Position
    module.morphisms[1].input = "Velocity".into();

    let mut checker = TypeChecker::new();
    let result = checker.check(&module);

    assert!(!result.success);
    assert!(result
      .errors
      .iter()
      .any(|e| matches!(e, TypeError::EdgeTypeMismatch { .. })));
  }

  #[test]
  fn test_optional_widening() {
    let mut module = make_test_module();

    // Make force expect Position? (optional)
    module.morphisms[1].input = "Position?".into();

    let mut checker = TypeChecker::new();
    let result = checker.check(&module);

    // Should succeed with implicit widening
    assert!(result.success);
    assert!(result
      .warnings
      .iter()
      .any(|w| matches!(w, TypeWarning::ImplicitCoercion { .. })));
  }

  #[test]
  fn test_morphism_sig_ported() {
    let sig = MorphismSig::ported(
      vec![
        ("pos", CoreType::named("Position")),
        ("vel", CoreType::named("Velocity")),
      ],
      vec![("result", CoreType::named("Position"))],
    );

    assert!(sig.is_ported());
    assert_eq!(
      sig.get_input_port("pos"),
      Some(&CoreType::named("Position"))
    );
    assert_eq!(
      sig.get_output_port("result"),
      Some(&CoreType::named("Position"))
    );
    assert_eq!(sig.get_input_port("unknown"), None);
  }

  #[test]
  fn test_input_edge_type_mismatch() {
    let module = FxCoreModule {
      meta: Default::default(),
      name: "input-mismatch".into(),
      types: vec!["Matrix".into(), "Vector".into()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![FxInput {
        name: "M".into(),
        ty: "Vector".into(),
      }],
      morphisms: vec![FxMorphism {
        name: "solve".into(),
        input: "Matrix".into(),
        output: "Vector".into(),
        inputs: vec![FxPort {
          name: "m".into(),
          ty: "Matrix".into(),
        }],
        outputs: vec![FxPort {
          name: "v".into(),
          ty: "Vector".into(),
        }],
        effect: Effect::Pure,
      }],
      nodes: vec![FxNode {
        name: "solve".into(),
        uses: "solve".into(),
        kind: NodeKind::Normal,
        optional: false,
        scope: "global".into(),
        cost: CostHint::Medium,
        priority: 0,
        contract: Default::default(),

        meta: None,
      }],
      edges: vec![FxEdge::from_input(
        "M".into(),
        "solve".into(),
        Some("m".into()),
      )],
      scopes: vec![],
    };

    let mut checker = TypeChecker::new();
    let result = checker.check(&module);

    assert!(!result.success);
    assert!(result
      .errors
      .iter()
      .any(|e| matches!(e, TypeError::EdgeTypeMismatch { .. })));
  }

  #[test]
  fn test_port_not_found_on_target() {
    let module = FxCoreModule {
      meta: Default::default(),
      name: "port-missing".into(),
      types: vec!["Matrix".into()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![FxInput {
        name: "M".into(),
        ty: "Matrix".into(),
      }],
      morphisms: vec![FxMorphism {
        name: "solve".into(),
        input: "Matrix".into(),
        output: "Matrix".into(),
        inputs: vec![FxPort {
          name: "m".into(),
          ty: "Matrix".into(),
        }],
        outputs: vec![FxPort {
          name: "v".into(),
          ty: "Matrix".into(),
        }],
        effect: Effect::Pure,
      }],
      nodes: vec![FxNode {
        name: "solve".into(),
        uses: "solve".into(),
        kind: NodeKind::Normal,
        optional: false,
        scope: "global".into(),
        cost: CostHint::Medium,
        priority: 0,
        contract: Default::default(),

        meta: None,
      }],
      edges: vec![FxEdge::from_input(
        "M".into(),
        "solve".into(),
        Some("bad".into()),
      )],
      scopes: vec![],
    };

    let mut checker = TypeChecker::new();
    let result = checker.check(&module);

    assert!(!result.success);
    assert!(result
      .errors
      .iter()
      .any(|e| matches!(e, TypeError::PortNotFound { .. })));
  }
}
