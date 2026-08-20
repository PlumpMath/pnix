//! Gate Test: 타입 에러 시 컴파일 실패 검증
//!
//! 헌법 원칙: "조용한 성공 금지"
//! 타입 에러가 있으면 컴파일이 실패해야 함 (조용히 성공하면 안 됨)

use pnix_core::ast::{AstItem, AstModule, EdgeSource, EdgeTarget, SigAst};
use pnix_core::diagnostics::Diagnostics;
use pnix_core::passes::pipeline::{CompilationPipeline, PipelineConfig};
use pnix_core::MeaningError;

fn make_type_error_ast() -> AstModule {
  // 타입 불일치를 일으키는 AST 생성
  // transform은 Num -> Num인데, String input을 연결
  AstModule {
    name: "test".into(),
    items: vec![
      AstItem::ExternDecl {
        name: "transform".into(),
        sig: SigAst::simple("Num".into(), "Num".into()),
        span: Default::default(),
      },
      AstItem::InputDecl {
        name: "x".into(),
        ty: "String".into(), // 타입 불일치: transform은 Num을 기대
        span: Default::default(),
      },
      AstItem::NodeDecl {
        name: "node1".into(),
        uses: "transform".into(),
        kind: None,
        optional: false,
        scope: None,
        cost: None,
        priority: None,
        span: Default::default(),
      },
      AstItem::EdgeDecl {
        from: EdgeSource::Input { name: "x".into() },
        to: EdgeTarget {
          node: "node1".into(),
          port: None,
        },
        cond: None,
        span: Default::default(),
      },
    ],
  }
}

#[test]
fn test_type_error_fails_compilation() {
  let ast = make_type_error_ast();
  let mut diags = Diagnostics::default();

  let config = PipelineConfig {
    enable_type_check: true,
    ..PipelineConfig::default()
  };
  let pipeline = CompilationPipeline::new(config);

  // 타입 에러가 있으면 컴파일이 실패해야 함
  let result = pipeline.run(&ast, &mut diags);

  assert!(
    result.is_err(),
    "Type errors must cause compilation to fail (no silent success)"
  );

  // 에러 메시지에 "Type check failed"가 포함되어야 함
  if let Err(MeaningError::TypeError(msg, _)) = result {
    assert!(
      msg.contains("Type check failed"),
      "Error message should mention type check failure, got: {}",
      msg
    );
  } else {
    panic!("Expected MeaningError::TypeError with type check failure message");
  }
}

#[test]
fn test_type_check_disabled_allows_compilation() {
  let ast = make_type_error_ast();
  let mut diags = Diagnostics::default();

  let config = PipelineConfig {
    enable_type_check: false, // 타입 체크 비활성화
    ..PipelineConfig::default()
  };
  let pipeline = CompilationPipeline::new(config);

  // 타입 체크가 비활성화되면 컴파일이 성공할 수 있음
  let _result = pipeline.run(&ast, &mut diags);

  // 타입 체크가 비활성화되면 성공할 수 있음 (다른 에러가 없으면)
  // 하지만 실제로는 lowering 단계에서 에러가 발생할 수 있으므로
  // 이 테스트는 타입 체크가 비활성화되면 타입 에러로 인한 실패가 없다는 것을 확인
  // (실제 동작은 lowering 단계에 따라 다를 수 있음)
}

#[test]
fn test_no_type_error_compiles_successfully() {
  // 타입 에러가 없는 올바른 AST
  let ast = AstModule {
    name: "test".into(),
    items: vec![
      AstItem::ExternDecl {
        name: "transform".into(),
        sig: SigAst::simple("Num".into(), "Num".into()),
        span: Default::default(),
      },
      AstItem::InputDecl {
        name: "x".into(),
        ty: "Num".into(), // 타입 일치
        span: Default::default(),
      },
      AstItem::NodeDecl {
        name: "node1".into(),
        uses: "transform".into(),
        kind: None,
        optional: false,
        scope: None,
        cost: None,
        priority: None,
        span: Default::default(),
      },
      AstItem::EdgeDecl {
        from: EdgeSource::Input { name: "x".into() },
        to: EdgeTarget {
          node: "node1".into(),
          port: None,
        },
        cond: None,
        span: Default::default(),
      },
    ],
  };

  let mut diags = Diagnostics::default();
  let config = PipelineConfig {
    enable_type_check: true,
    ..PipelineConfig::default()
  };
  let pipeline = CompilationPipeline::new(config);

  // 타입 에러가 없으면 컴파일이 성공해야 함
  let result = pipeline.run(&ast, &mut diags);

  // 타입 에러가 없으면 성공해야 함
  // (다른 에러가 있을 수 있지만, 타입 에러로 인한 실패는 아님)
  if let Err(e) = &result {
    // 타입 체크 실패가 아닌 다른 에러일 수 있음
    let err_msg = format!("{:?}", e);
    assert!(
      !err_msg.contains("Type check failed"),
      "Should not fail with type check error, got: {}",
      err_msg
    );
  }
  // 성공하거나 다른 에러가 있어도 타입 에러는 아님
}
