//! PNIX 모듈 테스트: PNIX 모듈 파싱 및 컴파일 테스트
//!
//! PNIX 모듈 파싱, import 처리, 컴파일 기능을 테스트합니다.

use pnix_core::ast::AstItem;
use pnix_core::lang::{module_resolver::ModuleResolver, pnix::parse_pnix_module_with_imports};
use pnix_core::{
  compile_pnix_module, compile_pnix_module_ast, parse_pnix_module, CompileOptions, SourceUnit,
};
use std::path::PathBuf;

// LOW: fixture 경로 include_str! 하드코딩 수정 완료
// 상대 경로로 인해 디렉토리 재구성 시 취약하나, 이는 테스트 코드의 구조적 제한사항
// 현재는 ../../../fixtures/pnix_module/ 경로를 하드코딩하여 사용하며, 향후 환경 변수 또는 매크로 사용 고려
const HELLO: &str = include_str!("../../../fixtures/pnix_module/hello.px");
const IMPORT_MAIN: &str = include_str!("../../../fixtures/pnix_module/import_main.px");
const ADT_TYPES: &str = include_str!("../../../fixtures/pnix_module/scenario07-adt-types.px");

#[test]
fn compiles_pnix_module_fixture_to_fxcore() {
  let src = SourceUnit {
    name: "fixtures/pnix_module/hello.px".into(),
    text: HELLO.into(),
  };

  let out = compile_pnix_module(&src, &CompileOptions::default()).unwrap();

  assert_eq!(out.fxcore.morphisms.len(), 3);
  assert_eq!(out.fxcore.nodes.len(), 3);
  assert_eq!(out.fxcore.edges.len(), 7);
  assert_eq!(out.fxcore.scopes.len(), 1);

  let gate = out
    .fxcore
    .nodes
    .iter()
    .find(|n| n.name == "gate1")
    .expect("gate1 node must exist");
  assert_eq!(gate.kind, pnix_core::core::NodeKind::Gate);

  let cond_edges = out.fxcore.edges.iter().filter(|e| e.cond.is_some()).count();
  assert_eq!(cond_edges, 2);

  let fx: serde_json::Value = serde_json::from_str(&out.artifacts.fxcore_json).unwrap();
  assert_eq!(fx.pointer("/meta/stage").and_then(|v| v.as_u64()), Some(4));
  assert!(out.artifacts.replay_hash.len() >= 32);
}

#[test]
fn test_adt_type_declaration() {
  // Y09a: enum 타입 정의 테스트
  // Option, Result, Unit, Boolean ADT 타입이 제대로 파싱되고 FxCore로 변환되는지 확인

  let src = SourceUnit {
    name: "fixtures/pnix_module/scenario07-adt-types.px".into(),
    text: ADT_TYPES.into(),
  };

  let out = compile_pnix_module(&src, &CompileOptions::default()).unwrap();

  // ADT 타입이 FxCore에 포함되어야 함
  assert_eq!(
    out.fxcore.adt_types.len(),
    4,
    "Should have 4 ADT types (Option, Result, Unit, Boolean)"
  );

  // Option 타입 확인
  let option_adt = out
    .fxcore
    .adt_types
    .iter()
    .find(|adt| adt.name == "Option")
    .expect("Option ADT should exist");
  assert_eq!(
    option_adt.params,
    vec!["a"],
    "Option should have one type parameter 'a'"
  );
  assert_eq!(
    option_adt.variants.len(),
    2,
    "Option should have 2 variants"
  );
  let none_variant = option_adt
    .variants
    .iter()
    .find(|v| v.name == "None")
    .expect("None variant should exist");
  assert_eq!(none_variant.fields.len(), 0, "None should have no fields");
  let some_variant = option_adt
    .variants
    .iter()
    .find(|v| v.name == "Some")
    .expect("Some variant should exist");
  assert_eq!(
    some_variant.fields,
    vec!["a"],
    "Some should have one field 'a'"
  );

  // Result 타입 확인
  let result_adt = out
    .fxcore
    .adt_types
    .iter()
    .find(|adt| adt.name == "Result")
    .expect("Result ADT should exist");
  assert_eq!(
    result_adt.params,
    vec!["a", "e"],
    "Result should have two type parameters 'a' and 'e'"
  );
  assert_eq!(
    result_adt.variants.len(),
    2,
    "Result should have 2 variants"
  );
  let ok_variant = result_adt
    .variants
    .iter()
    .find(|v| v.name == "Ok")
    .expect("Ok variant should exist");
  assert_eq!(ok_variant.fields, vec!["a"], "Ok should have one field 'a'");
  let err_variant = result_adt
    .variants
    .iter()
    .find(|v| v.name == "Err")
    .expect("Err variant should exist");
  assert_eq!(
    err_variant.fields,
    vec!["e"],
    "Err should have one field 'e'"
  );

  // Unit 타입 확인
  let unit_adt = out
    .fxcore
    .adt_types
    .iter()
    .find(|adt| adt.name == "Unit")
    .expect("Unit ADT should exist");
  assert_eq!(
    unit_adt.params.len(),
    0,
    "Unit should have no type parameters"
  );
  assert_eq!(unit_adt.variants.len(), 1, "Unit should have 1 variant");

  // Boolean 타입 확인
  let boolean_adt = out
    .fxcore
    .adt_types
    .iter()
    .find(|adt| adt.name == "Boolean")
    .expect("Boolean ADT should exist");
  assert_eq!(
    boolean_adt.params.len(),
    0,
    "Boolean should have no type parameters"
  );
  assert_eq!(
    boolean_adt.variants.len(),
    2,
    "Boolean should have 2 variants"
  );

  // ADT 타입 이름이 types 목록에도 포함되어야 함 (호환성)
  assert!(
    out.fxcore.types.contains(&"Option".to_string()),
    "Option should be in types list"
  );
  assert!(
    out.fxcore.types.contains(&"Result".to_string()),
    "Result should be in types list"
  );
  assert!(
    out.fxcore.types.contains(&"Unit".to_string()),
    "Unit should be in types list"
  );
  assert!(
    out.fxcore.types.contains(&"Boolean".to_string()),
    "Boolean should be in types list"
  );

  // 컴파일 성공 확인
  assert!(out.artifacts.replay_hash.len() >= 32);
}

#[test]
fn test_constructor_value_creation() {
  // Y09b: 값 생성자 (Constructors) 테스트
  // Some(42), None 같은 값 생성이 제대로 작동하는지 확인

  use pnix_core::fx::core_expr::FxCoreExpr;
  use pnix_core::lang::pnix::lower::{lower_to_fx_core, pnix_expr_to_unified};
  use pnix_core::lang::pnix::parser::parse_expr;

  // Test 1: Some(42) 생성
  let source1 = "Some(42)";
  let expr1 = parse_expr(source1).unwrap();
  let unified1 = pnix_expr_to_unified(&expr1).unwrap();
  let fx1 = lower_to_fx_core(&unified1).unwrap();

  match &fx1 {
    FxCoreExpr::Construct { variant, args } => {
      assert_eq!(variant, "Some", "Some(42) should create Construct");
      assert_eq!(args.len(), 1, "Some(42) should have one argument");
      match &args[0] {
        FxCoreExpr::ConstInt(42) => {}
        _ => panic!(
          "Some(42) argument should be ConstInt(42), got: {:?}",
          args[0]
        ),
      }
    }
    _ => panic!(
      "Some(42) should lower to FxCoreExpr::Construct, got: {:?}",
      fx1
    ),
  }

  // Test 2: None 생성 (nullary constructor)
  let source2 = "None";
  let expr2 = parse_expr(source2).unwrap();
  let unified2 = pnix_expr_to_unified(&expr2).unwrap();
  let fx2 = lower_to_fx_core(&unified2).unwrap();

  match &fx2 {
    FxCoreExpr::Construct { variant, args } => {
      assert_eq!(variant, "None", "None should create Construct");
      assert_eq!(args.len(), 0, "None should have no arguments");
    }
    _ => panic!("None should lower to FxCoreExpr::Construct, got: {:?}", fx2),
  }

  // Test 3: Ok("success") 생성
  let source3 = r#"Ok("success")"#;
  let expr3 = parse_expr(source3).unwrap();
  let unified3 = pnix_expr_to_unified(&expr3).unwrap();
  let fx3 = lower_to_fx_core(&unified3).unwrap();

  match &fx3 {
    FxCoreExpr::Construct { variant, args } => {
      assert_eq!(variant, "Ok", "Ok(\"success\") should create Construct");
      assert_eq!(args.len(), 1, "Ok(\"success\") should have one argument");
      match &args[0] {
        FxCoreExpr::ConstString(s) => {
          assert_eq!(
            s, "success",
            "Ok argument should be ConstString(\"success\")"
          );
        }
        _ => panic!(
          "Ok(\"success\") argument should be ConstString, got: {:?}",
          args[0]
        ),
      }
    }
    _ => panic!(
      "Ok(\"success\") should lower to FxCoreExpr::Construct, got: {:?}",
      fx3
    ),
  }

  // Test 4: Err("error") 생성
  let source4 = r#"Err("error")"#;
  let expr4 = parse_expr(source4).unwrap();
  let unified4 = pnix_expr_to_unified(&expr4).unwrap();
  let fx4 = lower_to_fx_core(&unified4).unwrap();

  match &fx4 {
    FxCoreExpr::Construct { variant, args } => {
      assert_eq!(variant, "Err", "Err(\"error\") should create Construct");
      assert_eq!(args.len(), 1, "Err(\"error\") should have one argument");
    }
    _ => panic!(
      "Err(\"error\") should lower to FxCoreExpr::Construct, got: {:?}",
      fx4
    ),
  }

  // Test 5: 중첩 생성자: Some(Ok(42))
  let source5 = "Some(Ok(42))";
  let expr5 = parse_expr(source5).unwrap();
  let unified5 = pnix_expr_to_unified(&expr5).unwrap();
  let fx5 = lower_to_fx_core(&unified5).unwrap();

  match &fx5 {
    FxCoreExpr::Construct { variant, args } => {
      assert_eq!(variant, "Some", "Some(Ok(42)) should create Construct");
      assert_eq!(args.len(), 1, "Some(Ok(42)) should have one argument");
      match &args[0] {
        FxCoreExpr::Construct {
          variant: inner_variant,
          args: inner_args,
        } => {
          assert_eq!(inner_variant, "Ok", "Inner constructor should be Ok");
          assert_eq!(inner_args.len(), 1, "Ok(42) should have one argument");
        }
        _ => panic!(
          "Some(Ok(42)) inner argument should be Construct, got: {:?}",
          args[0]
        ),
      }
    }
    _ => panic!(
      "Some(Ok(42)) should lower to FxCoreExpr::Construct, got: {:?}",
      fx5
    ),
  }
}

#[test]
fn test_edge_shorthand_with_dots_in_node_name() {
  // Y07e: Test that edge shorthand handles node names with dots correctly
  // "node.name.port" should parse as { node: "node.name", port: "port" }
  let source = r#"
{
  name = "test_dots";
  types = [];
  inputs = [];
  externs = [];
  nodes = [
    { name = "node.name"; uses = "add"; }
  ];
  edges = [
    { from = "node.name.out"; to = "result"; }
  ];
}
"#;

  let src = SourceUnit {
    name: "test_dots.px".into(),
    text: source.into(),
  };

  let parse_output = parse_pnix_module(&src).unwrap();

  // Find the edge declaration
  let edge_item = parse_output
    .ast
    .items
    .iter()
    .find(|item| matches!(item, AstItem::EdgeDecl { .. }))
    .expect("Should have one edge");

  if let AstItem::EdgeDecl { from, to, .. } = edge_item {
    // Verify that "node.name.out" was parsed correctly
    match from {
      pnix_core::ast::EdgeSource::Node { node, port } => {
        assert_eq!(node, "node.name", "Node name should be 'node.name'");
        assert_eq!(
          port.as_ref(),
          Some(&"out".to_string()),
          "Port should be 'out'"
        );
      }
      _ => panic!("Edge source should be Node, got: {:?}", from),
    }

    let pnix_core::ast::EdgeTarget { node, port } = to;
    assert_eq!(node, "result", "Target node should be 'result'");
    assert_eq!(port, &None, "Target port should be None");
  } else {
    panic!("Should have EdgeDecl item");
  }
}

#[test]
fn compiles_pnix_module_ast_fixture_to_fxcore() {
  let src = SourceUnit {
    name: "fixtures/pnix_module/hello.px".into(),
    text: HELLO.into(),
  };

  let parse_output = parse_pnix_module(&src).unwrap();
  let out = compile_pnix_module_ast(parse_output.ast, &CompileOptions::default()).unwrap();

  assert_eq!(out.fxcore.morphisms.len(), 3);
  assert_eq!(out.fxcore.nodes.len(), 3);
  assert_eq!(out.fxcore.edges.len(), 7);
  assert_eq!(out.fxcore.scopes.len(), 1);
}

#[test]
fn parses_pnix_module_imports() {
  let parsed = parse_pnix_module_with_imports(
    IMPORT_MAIN,
    "fixtures/pnix_module/import_main.px",
    Some("fixtures/pnix_module/import_main.px"),
  )
  .unwrap();

  assert_eq!(parsed.imports, vec!["./import_base.px"]);
  assert_eq!(parsed.ast.name, "import_main");

  let node_names: Vec<_> = parsed
    .ast
    .items
    .iter()
    .filter_map(|item| match item {
      AstItem::NodeDecl { name, .. } => Some(name.as_str()),
      _ => None,
    })
    .collect();
  assert_eq!(node_names, vec!["main_add"]);

  // Y07a: ImportDecl이 AST에 포함되는지 확인
  let import_paths: Vec<_> = parsed
    .ast
    .items
    .iter()
    .filter_map(|item| match item {
      AstItem::ImportDecl { path, .. } => Some(path.as_str()),
      _ => None,
    })
    .collect();
  assert_eq!(import_paths, vec!["./import_base.px"]);
}

#[test]
fn test_module_resolver_path_to_namespace() {
  assert_eq!(
    ModuleResolver::path_to_namespace(&PathBuf::from("src/math/vector.px")),
    "math.vector"
  );
  assert_eq!(
    ModuleResolver::path_to_namespace(&PathBuf::from("math/vector.px")),
    "math"
  );
  assert_eq!(
    ModuleResolver::path_to_namespace(&PathBuf::from("vector.px")),
    "vector"
  );
}

#[test]
fn test_stdlib_path_resolver() {
  use pnix_core::lang::module_resolver::StdlibPathResolver;

  // stdlib 경로 확인
  assert!(StdlibPathResolver::is_stdlib_path("std.list"));
  assert!(StdlibPathResolver::is_stdlib_path("std"));
  assert!(!StdlibPathResolver::is_stdlib_path("./module.px"));

  // stdlib 경로 해석 (파일 존재 여부는 확인하지 않음)
  let resolved = StdlibPathResolver::resolve_stdlib_path("std.list");
  assert!(resolved.is_some());
  let path = resolved.unwrap();
  assert!(path.to_string_lossy().contains("stdlib"));
  assert_eq!(path.file_name().and_then(|s| s.to_str()), Some("list"));
}

// ============================================================================
// Y11a: 테스트 어노테이션 파싱 테스트
// ============================================================================

#[test]
fn test_parse_test_declaration() {
  // 기본 테스트 선언 파싱 테스트: `test <Name> = <Expr>`
  use pnix_core::ast::parse_module;
  use pnix_core::diagnostics::Diagnostics;

  let source = "test test_add = 1 + 2";
  let mut diags = Diagnostics::default();

  let module = parse_module(source, "test", &mut diags).unwrap();

  assert_eq!(module.items.len(), 1);
  match &module.items[0] {
    AstItem::TestDecl { name, expr, .. } => {
      assert_eq!(name, "test_add");
      assert_eq!(expr, "1 + 2");
    }
    _ => panic!("Expected TestDecl, got: {:?}", module.items[0]),
  }
}

#[test]
fn test_parse_test_annotation_node() {
  // @test node 어노테이션 파싱 테스트: `@test node <Name> uses <Extern>`
  use pnix_core::ast::parse_module;
  use pnix_core::diagnostics::Diagnostics;

  let source = "@test node test_node uses builtins.add";
  let mut diags = Diagnostics::default();

  let module = parse_module(source, "test", &mut diags).unwrap();

  assert_eq!(module.items.len(), 1);
  match &module.items[0] {
    AstItem::NodeDecl {
      name, uses, kind, ..
    } => {
      assert_eq!(name, "test_node");
      assert_eq!(uses, "builtins.add");
      assert_eq!(kind, &Some("test".to_string()));
    }
    _ => panic!(
      "Expected NodeDecl with test kind, got: {:?}",
      module.items[0]
    ),
  }
}

#[test]
fn test_parse_test_annotation_expr() {
  // @test <Expr> 형태 파싱 테스트
  use pnix_core::ast::parse_module;
  use pnix_core::diagnostics::Diagnostics;

  let source = "@test 1 + 2";
  let mut diags = Diagnostics::default();

  let module = parse_module(source, "test", &mut diags).unwrap();

  assert_eq!(module.items.len(), 1);
  match &module.items[0] {
    AstItem::TestDecl { name, expr, .. } => {
      assert!(name.starts_with("test_"));
      assert_eq!(expr, "1 + 2");
    }
    _ => panic!("Expected TestDecl, got: {:?}", module.items[0]),
  }
}

#[test]
fn test_parse_multiple_tests() {
  // 여러 테스트 선언 파싱 테스트
  use pnix_core::ast::parse_module;
  use pnix_core::diagnostics::Diagnostics;

  let source = "test test_one = 1\ntest test_two = 2\ntest test_three = 3";
  let mut diags = Diagnostics::default();

  let module = parse_module(source, "test", &mut diags).unwrap();

  assert_eq!(module.items.len(), 3);

  match &module.items[0] {
    AstItem::TestDecl { name, .. } => assert_eq!(name, "test_one"),
    _ => panic!("Expected TestDecl"),
  }

  match &module.items[1] {
    AstItem::TestDecl { name, .. } => assert_eq!(name, "test_two"),
    _ => panic!("Expected TestDecl"),
  }

  match &module.items[2] {
    AstItem::TestDecl { name, .. } => assert_eq!(name, "test_three"),
    _ => panic!("Expected TestDecl"),
  }
}

#[test]
fn test_parse_test_with_complex_expr() {
  // 복잡한 표현식이 있는 테스트 선언 파싱 테스트
  use pnix_core::ast::parse_module;
  use pnix_core::diagnostics::Diagnostics;

  let source = "test test_complex = if true then 1 else 2";
  let mut diags = Diagnostics::default();

  let module = parse_module(source, "test", &mut diags).unwrap();

  assert_eq!(module.items.len(), 1);
  match &module.items[0] {
    AstItem::TestDecl { name, expr, .. } => {
      assert_eq!(name, "test_complex");
      assert_eq!(expr, "if true then 1 else 2");
    }
    _ => panic!("Expected TestDecl"),
  }
}

#[test]
fn test_collect_test_declarations() {
  // 테스트 선언 수집 테스트 (Y11a: 테스트 함수 수집 로직)
  use pnix_core::ast::parse_module;
  use pnix_core::diagnostics::Diagnostics;

  let source = r#"
type Real
input x : Real
test test_add = 1 + 2
test test_sub = 3 - 1
node n1 uses builtins.add
@test node test_node uses builtins.mul
"#;
  let mut diags = Diagnostics::default();

  let module = parse_module(source, "test", &mut diags).unwrap();

  // TestDecl 수집
  let test_decls: Vec<_> = module
    .items
    .iter()
    .filter_map(|item| match item {
      AstItem::TestDecl { name, expr, .. } => Some((name.clone(), expr.clone())),
      _ => None,
    })
    .collect();

  assert_eq!(test_decls.len(), 2);
  assert_eq!(test_decls[0].0, "test_add");
  assert_eq!(test_decls[0].1, "1 + 2");
  assert_eq!(test_decls[1].0, "test_sub");
  assert_eq!(test_decls[1].1, "3 - 1");

  // @test node 수집
  let test_nodes: Vec<_> = module
    .items
    .iter()
    .filter_map(|item| match item {
      AstItem::NodeDecl { name, kind, .. } if kind == &Some("test".to_string()) => {
        Some(name.clone())
      }
      _ => None,
    })
    .collect();

  assert_eq!(test_nodes.len(), 1);
  assert_eq!(test_nodes[0], "test_node");
}
