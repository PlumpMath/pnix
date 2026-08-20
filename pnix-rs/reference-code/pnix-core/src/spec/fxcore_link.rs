//! FxCoreModule ↔ Spec 연결점
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외

use crate::core::FxCoreModule;
use crate::spec::builtin::resolve_spec_builtin_name;
use crate::spec::Spec;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// FxCoreModule에서 사용된 spec 정보
///
/// 컴파일 산출물에 포함되는 spec 범위를 명시합니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsedSpec {
  /// 사용된 builtin 함수들 (이름 → 선언)
  pub used_builtins: BTreeMap<String, crate::spec::builtin::BuiltinDecl>,
  /// 사용된 타입들 (이름 → 선언)
  pub used_types: BTreeMap<String, crate::spec::stdlib::TypeDecl>,
  /// 사용된 morphism들 (이름 → 선언, extern 제외)
  pub used_morphisms: BTreeMap<String, crate::spec::lowering::LoweringRule>,
}

impl UsedSpec {
  /// 빈 UsedSpec 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      used_builtins: BTreeMap::new(),
      used_types: BTreeMap::new(),
      used_morphisms: BTreeMap::new(),
    }
  }

  /// FxCoreModule에서 사용된 spec 추출
  ///
  /// 규칙:
  /// - `FxCoreModule.types`: "선언된 타입" (사용된 타입)
  /// - `FxCoreModule.morphisms`: "선언된 morphism" (extern 포함)
  /// - `FxCoreModule.nodes.uses`: "사용된 morphism/builtin"
  ///
  /// "사용된 것"만 추출하여 spec에 포함합니다.
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn from_fxcore_module(module: &FxCoreModule, spec: &Spec) -> Self {
    let mut used = Self::new();

    // 사용된 타입 추출
    for ty_name in &module.types {
      if let Some(ty_decl) = spec.stdlib.get_type(ty_name) {
        used.used_types.insert(ty_name.clone(), ty_decl.clone());
      }
    }

    // 사용된 builtin 추출 (nodes에서 사용)
    for node in &module.nodes {
      if let Some(builtin_name) = resolve_spec_builtin_name(&node.uses, &spec.builtins) {
        let key = builtin_name.as_ref();
        if let Some(builtin_decl) = spec.builtins.get(key) {
          used
            .used_builtins
            .insert(key.to_string(), builtin_decl.clone());
        }
      }
    }

    // 사용된 morphism 추출 (lowering 규칙에서)
    for morphism in &module.morphisms {
      // extern이 아닌 경우 lowering 규칙 확인
      if !morphism.name.contains(".") {
        // builtin → morphism 매핑 규칙 찾기
        let builtin_name = morphism.name.strip_prefix("fx_").unwrap_or(&morphism.name);
        for rule in spec
          .lowering_rules
          .find_by_source(&format!("builtin:{}", builtin_name))
        {
          if rule.kind == crate::spec::lowering::LoweringRuleKind::BuiltinToMorphism {
            used
              .used_morphisms
              .insert(morphism.name.clone(), rule.clone());
          }
        }
      }
    }

    used
  }
}

impl Default for UsedSpec {
  fn default() -> Self {
    Self::new()
  }
}

/// FxCoreModule에 spec 정보 주입
///
/// 컴파일 산출물에 포함되는 spec 범위를 결정합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 값 계산 없음
pub trait SpecInjection {
  /// 사용된 spec 추출
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  fn extract_used_spec(&self, spec: &Spec) -> UsedSpec;
}

impl SpecInjection for FxCoreModule {
  fn extract_used_spec(&self, spec: &Spec) -> UsedSpec {
    UsedSpec::from_fxcore_module(self, spec)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::contracts::effect::Effect;
  use crate::core::{FxMorphism, FxNode};

  #[test]
  fn test_used_spec_extraction() {
    let spec = Spec::with_defaults();

    let module = FxCoreModule {
      meta: Default::default(),
      name: "test".to_string(),
      types: vec!["Num".to_string(), "Bool".to_string()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![FxNode {
        name: "node1".to_string(),
        uses: "add".to_string(), // builtin
        kind: crate::core::NodeKind::Normal,
        optional: false,
        scope: "global".to_string(),
        cost: crate::core::CostHint::Medium,
        priority: 0,
        contract: crate::core::ExecutionContract {
          required_inputs: vec![],
          may_skip: false,
          skip_policy: crate::core::SkipPolicy::Error,
          replay: None,
        },

        meta: None,
      }],
      edges: vec![],
      scopes: vec![],
    };

    let used = module.extract_used_spec(&spec);

    // 사용된 타입 확인
    assert!(used.used_types.contains_key("Num"));
    assert!(used.used_types.contains_key("Bool"));

    // 사용된 builtin 확인
    assert!(used.used_builtins.contains_key("add"));
  }

  #[test]
  fn test_used_spec_with_extern() {
    let spec = Spec::with_defaults();

    let module = FxCoreModule {
      meta: Default::default(),
      name: "test".to_string(),
      types: vec!["Num".to_string()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![FxMorphism {
        name: "py.numpy.add".to_string(), // extern
        input: "Num".to_string(),
        output: "Num".to_string(),
        inputs: vec![],
        outputs: vec![],
        effect: Effect::Pure,
      }],
      nodes: vec![FxNode {
        name: "node1".to_string(),
        uses: "py.numpy.add".to_string(), // extern morphism
        kind: crate::core::NodeKind::Normal,
        optional: false,
        scope: "global".to_string(),
        cost: crate::core::CostHint::Medium,
        priority: 0,
        contract: crate::core::ExecutionContract {
          required_inputs: vec![],
          may_skip: false,
          skip_policy: crate::core::SkipPolicy::Error,
          replay: None,
        },

        meta: None,
      }],
      edges: vec![],
      scopes: vec![],
    };

    let used = module.extract_used_spec(&spec);

    // extern morphism은 used_morphisms에 포함되지 않음 (lowering 규칙에 없음)
    assert!(!used.used_morphisms.contains_key("py.numpy.add"));
  }

  #[test]
  fn test_used_spec_builtin_not_dropped_when_morphism_exists() {
    let spec = Spec::with_defaults();

    let module = FxCoreModule {
      meta: Default::default(),
      name: "test".to_string(),
      types: vec!["Num".to_string()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![FxMorphism {
        name: "add".to_string(),
        input: "Num".to_string(),
        output: "Num".to_string(),
        inputs: vec![],
        outputs: vec![],
        effect: Effect::Pure,
      }],
      nodes: vec![FxNode {
        name: "node1".to_string(),
        uses: "builtins.add".to_string(),
        kind: crate::core::NodeKind::Normal,
        optional: false,
        scope: "global".to_string(),
        cost: crate::core::CostHint::Medium,
        priority: 0,
        contract: crate::core::ExecutionContract {
          required_inputs: vec![],
          may_skip: false,
          skip_policy: crate::core::SkipPolicy::Error,
          replay: None,
        },

        meta: None,
      }],
      edges: vec![],
      scopes: vec![],
    };

    let used = module.extract_used_spec(&spec);
    assert!(used.used_builtins.contains_key("add"));
  }

  #[test]
  fn test_used_spec_resolves_stdlib_alias_to_builtin_key() {
    let spec = Spec::with_defaults();
    let module = FxCoreModule {
      meta: Default::default(),
      name: "test".to_string(),
      types: vec!["String".to_string()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![FxNode {
        name: "node1".to_string(),
        uses: "String.length".to_string(),
        kind: crate::core::NodeKind::Normal,
        optional: false,
        scope: "global".to_string(),
        cost: crate::core::CostHint::Medium,
        priority: 0,
        contract: crate::core::ExecutionContract {
          required_inputs: vec![],
          may_skip: false,
          skip_policy: crate::core::SkipPolicy::Error,
          replay: None,
        },

        meta: None,
      }],
      edges: vec![],
      scopes: vec![],
    };

    let used = module.extract_used_spec(&spec);
    assert!(used.used_builtins.contains_key("stringLength"));
  }

  #[test]
  fn test_used_spec_collapses_runtime_and_vm_alias_family_to_runtime_call() {
    let spec = Spec::with_defaults();
    let module = FxCoreModule {
      meta: Default::default(),
      name: "test".to_string(),
      types: vec!["Num".to_string()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![
        FxNode {
          name: "n1".to_string(),
          uses: "Runtime.call".to_string(),
          kind: crate::core::NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: crate::core::CostHint::Medium,
          priority: 0,
          contract: crate::core::ExecutionContract {
            required_inputs: vec![],
            may_skip: false,
            skip_policy: crate::core::SkipPolicy::Error,
            replay: None,
          },
          meta: None,
        },
        FxNode {
          name: "n2".to_string(),
          uses: "runtime.call".to_string(),
          kind: crate::core::NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: crate::core::CostHint::Medium,
          priority: 0,
          contract: crate::core::ExecutionContract {
            required_inputs: vec![],
            may_skip: false,
            skip_policy: crate::core::SkipPolicy::Error,
            replay: None,
          },
          meta: None,
        },
        FxNode {
          name: "n3".to_string(),
          uses: "Vm.call".to_string(),
          kind: crate::core::NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: crate::core::CostHint::Medium,
          priority: 0,
          contract: crate::core::ExecutionContract {
            required_inputs: vec![],
            may_skip: false,
            skip_policy: crate::core::SkipPolicy::Error,
            replay: None,
          },
          meta: None,
        },
        FxNode {
          name: "n4".to_string(),
          uses: "vm.call".to_string(),
          kind: crate::core::NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: crate::core::CostHint::Medium,
          priority: 0,
          contract: crate::core::ExecutionContract {
            required_inputs: vec![],
            may_skip: false,
            skip_policy: crate::core::SkipPolicy::Error,
            replay: None,
          },
          meta: None,
        },
        FxNode {
          name: "n5".to_string(),
          uses: "builtins.Runtime.call".to_string(),
          kind: crate::core::NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: crate::core::CostHint::Medium,
          priority: 0,
          contract: crate::core::ExecutionContract {
            required_inputs: vec![],
            may_skip: false,
            skip_policy: crate::core::SkipPolicy::Error,
            replay: None,
          },
          meta: None,
        },
        FxNode {
          name: "n6".to_string(),
          uses: "builtins.runtime.call".to_string(),
          kind: crate::core::NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: crate::core::CostHint::Medium,
          priority: 0,
          contract: crate::core::ExecutionContract {
            required_inputs: vec![],
            may_skip: false,
            skip_policy: crate::core::SkipPolicy::Error,
            replay: None,
          },
          meta: None,
        },
        FxNode {
          name: "n7".to_string(),
          uses: "builtins.Vm.call".to_string(),
          kind: crate::core::NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: crate::core::CostHint::Medium,
          priority: 0,
          contract: crate::core::ExecutionContract {
            required_inputs: vec![],
            may_skip: false,
            skip_policy: crate::core::SkipPolicy::Error,
            replay: None,
          },
          meta: None,
        },
        FxNode {
          name: "n8".to_string(),
          uses: "builtins.vm.call".to_string(),
          kind: crate::core::NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: crate::core::CostHint::Medium,
          priority: 0,
          contract: crate::core::ExecutionContract {
            required_inputs: vec![],
            may_skip: false,
            skip_policy: crate::core::SkipPolicy::Error,
            replay: None,
          },
          meta: None,
        },
      ],
      edges: vec![],
      scopes: vec![],
    };

    let used = module.extract_used_spec(&spec);
    assert_eq!(used.used_builtins.len(), 1);
    assert!(used.used_builtins.contains_key("runtimeCall"));
  }
}
