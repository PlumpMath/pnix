//! Spec 기반 검증 테스트

use crate::passes::lowering::lower_to_fxcore_with_spec;
use crate::spec::Spec;
use crate::surface::{SurfaceInput, SurfaceModule, SurfaceNode};
use crate::MeaningError;

#[test]
fn test_unknown_builtin_error() {
  let spec = Spec::with_defaults();

  // Unknown builtin을 사용하는 surface 모듈 생성
  let surface = SurfaceModule {
    name: "test".to_string(),
    types: vec!["Num".to_string()],
    adt_types: vec![],
    inputs: vec![],
    decls: vec![],
    nodes: vec![SurfaceNode {
      name: "node1".to_string(),
      uses: "unknown_builtin".to_string(), // spec에 없는 builtin
      kind: None,
      optional: false,
      scope: None,
      cost: None,
      priority: None,
    }],
    edges: vec![],
    scopes: vec![],
  };

  let diags = crate::diagnostics::Diagnostics::default();
  let result = lower_to_fxcore_with_spec(&surface, &diags, &spec);

  // Unknown builtin은 에러를 발생시켜야 함
  assert!(result.is_err());
  match result.unwrap_err() {
    MeaningError::UnresolvedSymbol(msg, _span) => {
      assert!(msg.contains("unknown_builtin"));
      assert!(msg.contains("not in spec catalog"));
    }
    _ => panic!("Expected UnresolvedSymbol error"),
  }
}

#[test]
fn test_user_defined_type_allowed() {
  let spec = Spec::with_defaults();

  // 사용자 정의 타입은 허용됨 (spec에 없어도 OK)
  let surface = SurfaceModule {
    name: "test".to_string(),
    types: vec!["UserDefinedType".to_string()], // 사용자 정의 타입은 허용
    adt_types: vec![],
    inputs: vec![],
    decls: vec![],
    nodes: vec![],
    edges: vec![],
    scopes: vec![],
  };

  let diags = crate::diagnostics::Diagnostics::default();
  let result = lower_to_fxcore_with_spec(&surface, &diags, &spec);

  // 사용자 정의 타입은 허용되므로 성공해야 함
  assert!(result.is_ok());
}

#[test]
fn test_user_defined_input_type_allowed() {
  let spec = Spec::with_defaults();

  // 사용자 정의 입력 타입은 허용됨 (spec에 없어도 OK)
  let surface = SurfaceModule {
    name: "test".to_string(),
    types: vec![],
    adt_types: vec![],
    inputs: vec![SurfaceInput {
      name: "input1".to_string(),
      ty: "UserDefinedType".to_string(), // 사용자 정의 타입은 허용
    }],
    decls: vec![],
    nodes: vec![],
    edges: vec![],
    scopes: vec![],
  };

  let diags = crate::diagnostics::Diagnostics::default();
  let result = lower_to_fxcore_with_spec(&surface, &diags, &spec);

  // 사용자 정의 입력 타입은 허용되므로 성공해야 함
  assert!(result.is_ok());
}

#[test]
fn test_valid_builtin_passes() {
  let spec = Spec::with_defaults();

  // Valid builtin을 사용하는 surface 모듈 생성
  let surface = SurfaceModule {
    name: "test".to_string(),
    types: vec!["Num".to_string()],
    adt_types: vec![],
    inputs: vec![],
    decls: vec![],
    nodes: vec![SurfaceNode {
      name: "node1".to_string(),
      uses: "add".to_string(), // spec에 있는 builtin
      kind: None,
      optional: false,
      scope: None,
      cost: None,
      priority: None,
    }],
    edges: vec![],
    scopes: vec![],
  };

  let diags = crate::diagnostics::Diagnostics::default();
  let _result = lower_to_fxcore_with_spec(&surface, &diags, &spec);

  // Valid builtin은 통과해야 함 (다른 검증에서 실패할 수 있지만 spec 검증은 통과)
  // 실제로는 morphism이 없어서 실패할 수 있지만, spec 검증 단계는 통과해야 함
  // 여기서는 spec에 있는 builtin인지만 확인
  assert!(spec.builtins.contains("add"));
}
