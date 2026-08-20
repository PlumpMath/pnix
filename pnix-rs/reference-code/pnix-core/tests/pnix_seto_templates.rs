//! PNIX SETO 템플릿 테스트: PNIX SETO 템플릿 처리 테스트
//!
//! PNIX SETO 템플릿 파일을 로드하고 처리하는 기능을 테스트합니다.

use std::fs;
use std::path::PathBuf;

use pnix_core::lang::layer::{
  load_layer_grammar_for_package_with, load_layer_laws_for_package_with,
  load_layer_meaning_for_package_with, load_layer_schema_pnix_with_base, LayerEngine,
  LayerParseError,
};
use pnix_core::lang::pnix::{parse_expr, parse_layer_expr, PnixExpr};

fn template_path(rel: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("..")
    .join("..")
    .join(rel)
}

fn read_template(rel: &str) -> String {
  let path = template_path(rel);
  fs::read_to_string(&path).unwrap_or_else(|err| {
    panic!("failed to read {}: {}", path.display(), err);
  })
}

fn read_file(path: &std::path::Path) -> Result<String, LayerParseError> {
  fs::read_to_string(path).map_err(|err| LayerParseError::Io {
    path: path.display().to_string(),
    message: err.to_string(),
  })
}

#[test]
fn parse_layer_schema_template() {
  let src = read_template("docs/spec/pnix-seto/templates/layer_schema_template.px");
  parse_expr(&src).unwrap();
  assert!(src.contains("seto_layers"));
}

#[test]
fn parse_layer_math_rat_template() {
  let src = read_template("docs/spec/pnix-seto/templates/layer_math_rat.px");
  parse_expr(&src).unwrap();
  assert!(src.contains("math.rat"));
}

#[test]
fn parse_rat_test_case_and_expected_output() {
  let src = read_template("docs/spec/pnix-seto/templates/test_case_rat.px");
  parse_expr(&src).unwrap();
  assert!(src.contains("seto.math.rat"));
  assert!(src.contains("{ type = \"rat\"; num = 5; den = 6; }"));
}

#[test]
fn parse_frp_test_cases() {
  let cases = [
    "docs/spec/pnix-seto/templates/test_case_frp_pendulum.px",
    "docs/spec/pnix-seto/templates/test_case_frp_ik_chain.px",
    "docs/spec/pnix-seto/templates/test_case_frp_wind_curve.px",
    "docs/spec/pnix-seto/templates/test_case_frp_ui_drag.px",
    "docs/spec/pnix-seto/templates/test_case_frp_mathml_layout.px",
  ];
  for case in cases {
    let src = read_template(case);
    parse_expr(&src).unwrap();
    assert!(src.contains("seto.frp"));
  }
}

#[test]
fn parse_physics_test_cases() {
  let cases = [
    "docs/spec/pnix-seto/templates/test_case_physics_newton.px",
    "docs/spec/pnix-seto/templates/test_case_physics_relativity.px",
    "docs/spec/pnix-seto/templates/test_case_physics_quantum.px",
    "docs/spec/pnix-seto/templates/test_case_physics_string.px",
  ];
  for case in cases {
    let src = read_template(case);
    parse_expr(&src).unwrap();
    assert!(src.contains("seto.physics"));
  }
}

#[test]
fn load_frp_layer_from_schema_template() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/layer_schema_template.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry
    .packages
    .get("frp.core")
    .expect("missing frp.core in layer schema");
  let base_dir = schema_path.parent().unwrap();
  load_layer_grammar_for_package_with(package, base_dir, &read_file).unwrap();
  load_layer_meaning_for_package_with(package, base_dir, &read_file).unwrap();
  load_layer_laws_for_package_with(package, base_dir, &read_file).unwrap();
}

#[test]
fn load_physics_layer_from_schema_template() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/layer_schema_template.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry
    .packages
    .get("physics.core")
    .expect("missing physics.core in layer schema");
  let base_dir = schema_path.parent().unwrap();
  load_layer_grammar_for_package_with(package, base_dir, &read_file).unwrap();
  load_layer_meaning_for_package_with(package, base_dir, &read_file).unwrap();
  load_layer_laws_for_package_with(package, base_dir, &read_file).unwrap();
}

#[test]
fn load_schema_and_validate_operator_laws() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/layer_schema_template.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry.packages.get("math.rat").unwrap();
  let base_dir = schema_path.parent().unwrap();
  let laws = load_layer_laws_for_package_with(package, base_dir, &read_file).unwrap();
  load_layer_grammar_for_package_with(package, base_dir, &read_file).unwrap();
  load_layer_meaning_for_package_with(package, base_dir, &read_file).unwrap();

  let engine = LayerEngine::new();
  let prepared = engine.prepare_with_laws(package, Some(&laws));
  assert!(prepared.report.is_ok());
  let mut pipeline = engine.build_pipeline(&prepared);
  assert!(pipeline.report.is_ok());
  assert!(pipeline.typing.rules.contains_key("/"));
  let expr = parse_layer_expr("Rat.make 1 2 / Rat.make 3 4", Some(&pipeline)).unwrap();
  let is_rat_make = |expr: &PnixExpr| {
    matches!(
      expr,
      PnixExpr::Select { base, attr }
        if (matches!(**base, PnixExpr::Var(ref name) if name == "Rat")
          || matches!(**base, PnixExpr::Construct { ref variant, ref args }
            if variant == "Rat" && args.is_empty()))
          && attr == "make"
    ) || matches!(expr, PnixExpr::Var(name) if name == "Rat.make" || name == "rat.make")
  };
  match expr {
    PnixExpr::Apply { func, arg } => match *func {
      PnixExpr::Apply { func, arg: lhs } => {
        assert!(matches!(*func, PnixExpr::Var(ref name) if name == "Rat.div"));
        match *lhs {
          PnixExpr::Apply { func, arg: lhs_arg } => match *func {
            PnixExpr::Apply {
              func,
              arg: lhs_head,
            } => {
              assert!(is_rat_make(&func));
              assert!(matches!(*lhs_head, PnixExpr::Int(1)));
              assert!(matches!(*lhs_arg, PnixExpr::Int(2)));
            }
            _ => panic!("expected curried apply for Rat.make (lhs)"),
          },
          _ => panic!("expected Rat.make application for lhs"),
        }
        match *arg {
          PnixExpr::Apply { func, arg: rhs_arg } => match *func {
            PnixExpr::Apply {
              func,
              arg: rhs_head,
            } => {
              assert!(is_rat_make(&func));
              assert!(matches!(*rhs_head, PnixExpr::Int(3)));
              assert!(matches!(*rhs_arg, PnixExpr::Int(4)));
            }
            _ => panic!("expected curried apply for Rat.make (rhs)"),
          },
          _ => panic!("expected Rat.make application for rhs"),
        }
      }
      _ => panic!("expected curried apply for Rat.div"),
    },
    _ => panic!("expected desugared apply"),
  }

  pipeline
    .desugar
    .rules
    .insert("++".to_string(), "String.concat".to_string());
  let expr = parse_layer_expr("a ++ b", Some(&pipeline)).unwrap();
  match expr {
    PnixExpr::Apply { func, arg } => match *func {
      PnixExpr::Apply { func, arg: lhs } => {
        assert!(matches!(*func, PnixExpr::Var(ref name) if name == "String.concat"));
        assert!(matches!(*lhs, PnixExpr::Var(ref name) if name == "a"));
        assert!(matches!(*arg, PnixExpr::Var(ref name) if name == "b"));
      }
      _ => panic!("expected curried apply for String.concat"),
    },
    _ => panic!("expected desugared apply"),
  }

  pipeline
    .normalize
    .rules
    .insert("Rat.make".to_string(), "Rat.normalize".to_string());
  let expr = parse_layer_expr("Rat.make 2 4", Some(&pipeline)).unwrap();
  match expr {
    PnixExpr::Apply { func, arg } => match *func {
      PnixExpr::Apply { func, arg: lhs } => {
        let _ = func;
        assert!(matches!(*lhs, PnixExpr::Int(1)), "lhs: {:?}", lhs);
        assert!(matches!(*arg, PnixExpr::Int(2)), "rhs: {:?}", arg);
      }
      _ => panic!("expected curried apply for Rat.make"),
    },
    _ => panic!("expected normalized apply"),
  }
}
