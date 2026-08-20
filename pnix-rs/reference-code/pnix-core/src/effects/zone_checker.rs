//! ZoneChecker: FxCoreModule effect zone 검증
//!
//! 그래프의 노드/엣지 effect를 추적하고 격리 정책을 검증.
//!
//! ## 검증 항목
//!
//! 1. Morphism effect → Node zone 전파
//! 2. Edge를 통한 zone 전파
//! 3. Scope 격리 정책 준수

use super::EffectZone;
use crate::core::{FxCoreModule, ScopePolicy};
use std::collections::HashMap;
use thiserror::Error;

/// Zone 검증 결과
#[derive(Debug)]
pub struct ZoneCheckResult {
  /// 검증 성공 여부
  pub success: bool,
  /// 발견된 에러들
  pub errors: Vec<ZoneError>,
  /// 발견된 경고들
  pub warnings: Vec<ZoneWarning>,
  /// 추론된 노드 zone
  pub node_zones: HashMap<String, EffectZone>,
  /// 모듈 전체 zone (최상위)
  pub module_zone: EffectZone,
}

impl ZoneCheckResult {
  fn new() -> Self {
    Self {
      success: true,
      errors: Vec::new(),
      warnings: Vec::new(),
      node_zones: HashMap::new(),
      module_zone: EffectZone::Pure,
    }
  }

  fn add_error(&mut self, error: ZoneError) {
    self.success = false;
    self.errors.push(error);
  }

  fn add_warning(&mut self, warning: ZoneWarning) {
    self.warnings.push(warning);
  }
}

/// Zone 에러
///
/// # Example
/// ```rust
/// use pnix_core::effects::ZoneError;
/// let err = ZoneError::UnknownMorphism { name: "missing".to_string() };
/// assert!(matches!(err, ZoneError::UnknownMorphism { .. }));
/// ```
#[derive(Debug, Error)]
pub enum ZoneError {
  #[error(
    "Zone violation: node {node} has zone {node_zone} but scope {scope} requires {required}"
  )]
  ScopeViolation {
    node: String,
    scope: String,
    node_zone: EffectZone,
    required: EffectZone,
  },

  #[error("Effect escalation: edge from {from} ({from_zone}) to {to} requires {to} to be at least {from_zone}")]
  EffectEscalation {
    from: String,
    to: String,
    from_zone: EffectZone,
  },

  #[error("Unknown morphism: {name}")]
  UnknownMorphism { name: String },
}

/// Zone 경고
#[derive(Debug)]
pub enum ZoneWarning {
  /// 높은 effect zone의 노드
  HighEffectZone { node: String, zone: EffectZone },

  /// World effect를 사용하는 morphism
  WorldEffect { morphism: String },

  /// Scope 내 effect 불일치
  ScopeEffectMix {
    scope: String,
    zones: Vec<EffectZone>,
  },

  /// Effect escalation: downstream zone이 upstream보다 낮아서 자동 상향됨
  EffectEscalation {
    from: String,
    to: String,
    from_zone: EffectZone,
    original_to_zone: EffectZone,
    reason: String,
  },
}

/// Zone 체커
pub struct ZoneChecker {
  /// Morphism별 effect zone
  morphism_zones: HashMap<String, EffectZone>,
}

impl ZoneChecker {
  /// 새 Zone 체커 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      morphism_zones: HashMap::new(),
    }
  }

  /// Morphism의 effect zone 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register_morphism(&mut self, name: impl Into<String>, zone: EffectZone) {
    self.morphism_zones.insert(name.into(), zone);
  }

  /// FxCoreModule에서 morphism zone 추출
  fn register_from_module(&mut self, module: &FxCoreModule) {
    for morphism in &module.morphisms {
      let zone = EffectZone::from_effect(morphism.effect);
      self.morphism_zones.insert(morphism.name.clone(), zone);
    }
  }

  /// FxCoreModule zone 검증
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn check(&mut self, module: &FxCoreModule) -> ZoneCheckResult {
    let mut result = ZoneCheckResult::new();

    // 1. Morphism zone 등록
    self.register_from_module(module);

    // 2. 노드 zone 추론
    for node in &module.nodes {
      let zone = self
        .morphism_zones
        .get(&node.uses)
        .copied()
        .unwrap_or_else(|| {
          result.add_error(ZoneError::UnknownMorphism {
            name: node.uses.clone(),
          });
          EffectZone::Pure
        });

      result.node_zones.insert(node.name.clone(), zone);

      // High effect zone 경고
      if zone >= EffectZone::Interop {
        result.add_warning(ZoneWarning::HighEffectZone {
          node: node.name.clone(),
          zone,
        });
      }
    }

    // 3. Edge를 통한 zone 전파 검증
    // (downstream 노드는 upstream 노드보다 낮은 zone을 가질 수 없음)
    for edge in &module.edges {
      if let (Some(&from_zone), Some(&to_zone)) = (
        result.node_zones.get(&edge.from),
        result.node_zones.get(&edge.to),
      ) {
        // CRITICAL: downstream이 upstream보다 낮으면 위반
        // downstream은 upstream의 결과를 받으므로, 최소한 from_zone 수준이어야 함
        if to_zone.is_strictly_below(from_zone) {
          // to_zone을 from_zone으로 업데이트하여 제약 강제
          // (안전한 최소값으로 상향)
          result.node_zones.insert(edge.to.clone(), from_zone);

          // 경고: zone이 자동으로 상향되었음을 기록
          result.add_warning(ZoneWarning::EffectEscalation {
            from: edge.from.clone(),
            to: edge.to.clone(),
            from_zone,
            original_to_zone: to_zone,
            reason: format!(
              "Downstream node '{}' had zone {} which is below upstream '{}' zone {}. Automatically upgraded to {}.",
              edge.to, to_zone, edge.from, from_zone, from_zone
            ),
          });
        }
      }
    }

    // 4. Scope 정책 검증
    for scope in &module.scopes {
      let scope_nodes: Vec<_> = module
        .nodes
        .iter()
        .filter(|n| n.scope == scope.name)
        .collect();

      if scope_nodes.is_empty() {
        continue;
      }

      let scope_zones: Vec<EffectZone> = scope_nodes
        .iter()
        .filter_map(|n| result.node_zones.get(&n.name).copied())
        .collect();

      let max_zone = EffectZone::join_all(scope_zones.iter().copied());

      // FailFast scope는 Pure zone만 허용
      if scope.policy == ScopePolicy::FailFast && max_zone > EffectZone::Pure {
        for node in scope_nodes {
          if let Some(&zone) = result.node_zones.get(&node.name) {
            if zone > EffectZone::Pure {
              result.add_error(ZoneError::ScopeViolation {
                node: node.name.clone(),
                scope: scope.name.clone(),
                node_zone: zone,
                required: EffectZone::Pure,
              });
            }
          }
        }
      }

      // 다양한 zone이 섞여 있으면 경고
      let unique_zones: std::collections::HashSet<_> = scope_zones.iter().collect();
      if unique_zones.len() > 1 {
        result.add_warning(ZoneWarning::ScopeEffectMix {
          scope: scope.name.clone(),
          zones: scope_zones,
        });
      }
    }

    // 5. 모듈 전체 zone 계산
    result.module_zone = EffectZone::join_all(result.node_zones.values().copied());

    result
  }
}

impl Default for ZoneChecker {
  fn default() -> Self {
    Self::new()
  }
}

/// 단순 zone 추론 (morphism effect 기반)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 값 계산 없음
pub fn infer_module_zone(module: &FxCoreModule) -> EffectZone {
  EffectZone::join_all(
    module
      .morphisms
      .iter()
      .map(|m| EffectZone::from_effect(m.effect)),
  )
}

/// 노드별 zone 맵 추론
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 값 계산 없음
pub fn infer_node_zones(module: &FxCoreModule) -> HashMap<String, EffectZone> {
  let morphism_zones: HashMap<_, _> = module
    .morphisms
    .iter()
    .map(|m| (m.name.as_str(), EffectZone::from_effect(m.effect)))
    .collect();

  module
    .nodes
    .iter()
    .map(|n| {
      let zone = morphism_zones
        .get(n.uses.as_str())
        .copied()
        .unwrap_or(EffectZone::Pure);
      (n.name.clone(), zone)
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::contracts::effect::Effect;
  use crate::core::{CostHint, FxEdge, FxMorphism, FxNode, FxScope, NodeKind};

  fn make_test_module() -> FxCoreModule {
    FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![
        FxMorphism {
          name: "pure.op".into(),
          input: "A".into(),
          output: "B".into(),
          inputs: vec![],
          outputs: vec![],
          effect: Effect::Pure,
        },
        FxMorphism {
          name: "world.op".into(),
          input: "B".into(),
          output: "C".into(),
          inputs: vec![],
          outputs: vec![],
          effect: Effect::World,
        },
      ],
      nodes: vec![
        FxNode {
          name: "n1".into(),
          uses: "pure.op".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: Default::default(),

          meta: None,
        },
        FxNode {
          name: "n2".into(),
          uses: "world.op".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: Default::default(),

          meta: None,
        },
      ],
      edges: vec![FxEdge {
        from: "n1".into(),
        to: "n2".into(),
        cond: None,
        from_port: None,
        to_port: None,
        from_input: None,
      }],
      scopes: vec![],
    }
  }

  #[test]
  fn test_zone_checker_basic() {
    let module = make_test_module();
    let mut checker = ZoneChecker::new();

    let result = checker.check(&module);

    assert!(result.success);
    assert_eq!(result.node_zones.get("n1"), Some(&EffectZone::Pure));
    assert_eq!(result.node_zones.get("n2"), Some(&EffectZone::World));
    assert_eq!(result.module_zone, EffectZone::World);
  }

  #[test]
  fn test_zone_checker_high_effect_warning() {
    let module = make_test_module();
    let mut checker = ZoneChecker::new();

    let result = checker.check(&module);

    // World effect should trigger a warning
    assert!(
            result
                .warnings
                .iter()
                .any(|w| matches!(w, ZoneWarning::HighEffectZone { node, zone } if node == "n2" && *zone == EffectZone::World))
        );
  }

  #[test]
  fn test_infer_module_zone() {
    let module = make_test_module();
    let zone = infer_module_zone(&module);

    assert_eq!(zone, EffectZone::World);
  }

  #[test]
  fn test_infer_node_zones() {
    let module = make_test_module();
    let zones = infer_node_zones(&module);

    assert_eq!(zones.get("n1"), Some(&EffectZone::Pure));
    assert_eq!(zones.get("n2"), Some(&EffectZone::World));
  }

  #[test]
  fn test_scope_policy_failfast() {
    let mut module = make_test_module();

    // Add a FailFast scope with a World effect node
    module.scopes.push(FxScope {
      name: "strict".into(),
      nodes: vec!["n2".into()],
      policy: ScopePolicy::FailFast,
    });
    module.nodes[1].scope = "strict".into();

    let mut checker = ZoneChecker::new();
    let result = checker.check(&module);

    // Should fail because FailFast scope has World effect
    assert!(!result.success);
    assert!(result
      .errors
      .iter()
      .any(|e| matches!(e, ZoneError::ScopeViolation { .. })));
  }

  #[test]
  fn test_scope_effect_mix_warning() {
    let mut module = make_test_module();

    // Both nodes in same scope with different effects
    module.scopes.push(FxScope {
      name: "mixed".into(),
      nodes: vec!["n1".into(), "n2".into()],
      policy: ScopePolicy::BestEffort,
    });
    module.nodes[0].scope = "mixed".into();
    module.nodes[1].scope = "mixed".into();

    let mut checker = ZoneChecker::new();
    let result = checker.check(&module);

    assert!(result.success);
    assert!(result
      .warnings
      .iter()
      .any(|w| matches!(w, ZoneWarning::ScopeEffectMix { scope, .. } if scope == "mixed")));
  }
}
