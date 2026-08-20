//! 타입 체크 게이트 테스트: 타입 체크 실패 시 에러 처리 검증
//!
//! 타입 체크가 실패할 경우 컴파일 파이프라인에서 에러로 처리되는지 검증합니다.

use pnix_core::ast::{AstItem, AstModule, EdgeSource, EdgeTarget, SigAst};
use pnix_core::diagnostics::{Diagnostics, Span};
use pnix_core::passes::pipeline::{CompilationPipeline, PipelineConfig};
use pnix_core::MeaningError;

#[test]
fn gate_type_check_failure_is_error() {
  let ast = AstModule {
    name: "type_check_gate".to_string(),
    items: vec![
      AstItem::ExternDecl {
        name: "id".to_string(),
        sig: SigAst::simple("Num".to_string(), "Num".to_string()),
        span: Span::default(),
      },
      AstItem::InputDecl {
        name: "flag".to_string(),
        ty: "Bool".to_string(),
        span: Span::default(),
      },
      AstItem::NodeDecl {
        name: "n1".to_string(),
        uses: "id".to_string(),
        kind: None,
        optional: false,
        scope: None,
        cost: None,
        priority: None,
        span: Span::default(),
      },
      AstItem::EdgeDecl {
        from: EdgeSource::Input {
          name: "flag".to_string(),
        },
        to: EdgeTarget {
          node: "n1".to_string(),
          port: None,
        },
        cond: None,
        span: Span::default(),
      },
    ],
  };

  let mut diags = Diagnostics::default();
  let pipeline = CompilationPipeline::new(PipelineConfig {
    enable_type_check: true,
    ..PipelineConfig::default()
  });
  let result = pipeline.run(&ast, &mut diags);
  assert!(matches!(result, Err(MeaningError::TypeError(_, _))));
}
