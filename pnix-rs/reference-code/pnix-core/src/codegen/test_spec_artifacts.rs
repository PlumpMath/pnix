//! Spec artifacts 테스트 (W03b)

use crate::build_ir::BuildIr;
use crate::codegen::emit::emit_all_with_spec;
use crate::core::{FxCoreModule, FxNode};
use crate::diagnostics::Diagnostics;
use crate::spec::Spec;
use crate::ssa::{SSAValue, SsaBlock, SsaModule};

#[test]
fn test_spec_canon_json_included() {
  let spec = Spec::with_defaults();
  let fx = FxCoreModule {
    meta: Default::default(),
    name: "test".to_string(),
    types: vec!["Num".to_string()],
    adt_types: vec![],
    adttypes: vec![],
    inputs: vec![],
    morphisms: vec![],
    nodes: vec![FxNode {
      name: "node1".to_string(),
      uses: "add".to_string(),
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
  let ssa = SsaModule {
    name: "test".to_string(),
    blocks: vec![SsaBlock {
      label: "entry".into(),
      ops: vec![],
      ret: SSAValue(0),
    }],
  };
  let bir = BuildIr::from_fxcore(
    &fx,
    crate::build_ir::Os::Linux,
    crate::build_ir::Arch::X86_64,
  );
  let mut diags = Diagnostics::default();

  let artifacts = emit_all_with_spec(&fx, &ssa, &bir, &mut diags, &spec).unwrap();

  // spec_canon_json이 포함되어야 함
  assert!(artifacts.spec_canon_json.is_some());
  let spec_json = artifacts.spec_canon_json.as_ref().unwrap();
  assert!(spec_json.contains("\"version\""));
  assert!(spec_json.contains("\"builtins\""));
}

#[test]
fn test_used_spec_canon_json_included() {
  let spec = Spec::with_defaults();
  let fx = FxCoreModule {
    meta: Default::default(),
    name: "test".to_string(),
    types: vec!["Num".to_string()],
    adt_types: vec![],
    adttypes: vec![],
    inputs: vec![],
    morphisms: vec![],
    nodes: vec![FxNode {
      name: "node1".to_string(),
      uses: "add".to_string(),
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
  let ssa = SsaModule {
    name: "test".to_string(),
    blocks: vec![SsaBlock {
      label: "entry".into(),
      ops: vec![],
      ret: SSAValue(0),
    }],
  };
  let bir = BuildIr::from_fxcore(
    &fx,
    crate::build_ir::Os::Linux,
    crate::build_ir::Arch::X86_64,
  );
  let mut diags = Diagnostics::default();

  let artifacts = emit_all_with_spec(&fx, &ssa, &bir, &mut diags, &spec).unwrap();

  // used_spec_canon_json이 포함되어야 함
  assert!(artifacts.used_spec_canon_json.is_some());
  let used_spec_json = artifacts.used_spec_canon_json.as_ref().unwrap();
  assert!(used_spec_json.contains("\"used_builtins\""));
}

#[test]
fn test_replay_hash_includes_spec() {
  let spec = Spec::with_defaults();
  let fx = FxCoreModule {
    meta: Default::default(),
    name: "test".to_string(),
    types: vec![],
    adt_types: vec![],
    adttypes: vec![],
    inputs: vec![],
    morphisms: vec![],
    nodes: vec![],
    edges: vec![],
    scopes: vec![],
  };
  let ssa = SsaModule {
    name: "test".to_string(),
    blocks: vec![SsaBlock {
      label: "entry".into(),
      ops: vec![],
      ret: SSAValue(0),
    }],
  };
  let bir = BuildIr::from_fxcore(
    &fx,
    crate::build_ir::Os::Linux,
    crate::build_ir::Arch::X86_64,
  );
  let mut diags = Diagnostics::default();

  let artifacts1 = emit_all_with_spec(&fx, &ssa, &bir, &mut diags, &spec).unwrap();
  let mut diags2 = Diagnostics::default();
  let artifacts2 = emit_all_with_spec(&fx, &ssa, &bir, &mut diags2, &spec).unwrap();

  // 동일 입력 2회 컴파일 시 replay hash가 동일해야 함
  assert_eq!(artifacts1.replay_hash, artifacts2.replay_hash);
  assert_eq!(artifacts1.spec_canon_json, artifacts2.spec_canon_json);
}
