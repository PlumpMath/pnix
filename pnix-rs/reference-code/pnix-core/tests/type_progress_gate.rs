//! Gate test: Type progress theorem smoke.
//!
//! Goal: well-typed modules must make forward progress through compilation,
//! and type failures must be explicit errors.

use pnix_core::ast::{AstItem, AstModule, EdgeSource, EdgeTarget, SigAst};
use pnix_core::diagnostics::{Diagnostics, Span};
use pnix_core::passes::pipeline::{CompilationPipeline, PipelineConfig};
use pnix_core::MeaningError;

fn well_typed_ast() -> AstModule {
  AstModule {
    name: "type_progress_ok".to_string(),
    items: vec![
      AstItem::ExternDecl {
        name: "id".to_string(),
        sig: SigAst::simple("Num".to_string(), "Num".to_string()),
        span: Span::default(),
      },
      AstItem::InputDecl {
        name: "x".to_string(),
        ty: "Num".to_string(),
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
          name: "x".to_string(),
        },
        to: EdgeTarget {
          node: "n1".to_string(),
          port: None,
        },
        cond: None,
        span: Span::default(),
      },
    ],
  }
}

fn ill_typed_ast() -> AstModule {
  AstModule {
    name: "type_progress_err".to_string(),
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
  }
}

#[test]
fn gate_progress_well_typed_module_produces_ir_artifacts() {
  let ast = well_typed_ast();
  let mut diags = Diagnostics::default();
  let pipeline = CompilationPipeline::new(PipelineConfig {
    enable_type_check: true,
    ..PipelineConfig::default()
  });

  let result = pipeline
    .run(&ast, &mut diags)
    .expect("well-typed module should compile");
  let type_result = result
    .type_result
    .expect("type-check result should exist when enabled");

  assert!(
    type_result.success,
    "well-typed module must pass type check"
  );
  assert!(
    !result.fxcore.nodes.is_empty(),
    "well-typed module should progress to non-empty FxCore"
  );
  assert!(
    !result.ssa.blocks.is_empty(),
    "well-typed module should progress to non-empty SSA"
  );
}

#[test]
fn gate_progress_type_failure_is_explicit_error() {
  let ast = ill_typed_ast();
  let mut diags = Diagnostics::default();
  let pipeline = CompilationPipeline::new(PipelineConfig {
    enable_type_check: true,
    ..PipelineConfig::default()
  });

  let result = pipeline.run(&ast, &mut diags);
  assert!(
    matches!(result, Err(MeaningError::TypeError(_, _))),
    "ill-typed module must fail with explicit MeaningError::TypeError"
  );
}
