//! 테스트 러너 구현 (Y11c)
//!
//! 테스트 선언을 수집하고 실행하여 결과를 요약합니다.

use crate::apply::{self, ApplyStatus, AuditReason, BackendConfig, NodeStatus};
use crate::model::{
  Effect, FxCoreMeta, FxCoreModule, FxEdge, FxInput, FxMorphism, FxNode, FxPort, NodeKind,
  FXCORE_VERSION,
};
use crate::plan;
use anyhow::Result;
use pnix_core::ast::{parse_module, AstItem};
use pnix_core::contracts::ResourceLimits;
use pnix_core::diagnostics::Diagnostics;
use pnix_core::spec::builtin::{BuiltinCatalog, BuiltinDecl};
use pnix_runtime_api::{EvalConfig, EvalEngine};
use pnix_runtime_legacy::ir::LegacyModule;
use pnix_runtime_legacy::ssa_eval::LegacyEvalEngine;
use serde_json::Value as JsonValue;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// 테스트 케이스 정보
#[derive(Debug, Clone)]
pub enum TestCaseKind {
  Expr(String),
  Node { uses: String },
}

#[derive(Debug, Clone)]
pub struct TestCase {
  pub name: String,
  pub kind: TestCaseKind,
  #[allow(dead_code)] // 향후 사용 예정
  pub source_file: Option<PathBuf>,
}

/// 테스트 결과
#[derive(Debug, Clone)]
pub struct TestResult {
  pub name: String,
  pub passed: bool,
  pub error: Option<String>,
}

/// 테스트 요약
#[derive(Debug, Clone)]
pub struct TestSummary {
  pub total: usize,
  pub passed: usize,
  pub failed: usize,
  pub skipped: usize,
  pub results: Vec<TestResult>,
}

#[derive(Debug, Clone)]
pub struct GraphTestConfig {
  pub clojure_url: String,
  pub python_url: String,
  pub deno_url: String,
  pub blenderpy_url: String,
  pub rpc_timeout_ms: u64,
  pub rpc_retry_attempts: usize,
  pub rpc_retry_backoff_ms: u64,
  pub rpc_retry_seed: u64,
  pub use_batch_apply: bool,
  pub allow_non_atomic_effects: bool,
  pub resource_limits: ResourceLimits,
}

fn normalize_signature_type(token: &str) -> &str {
  let token = token.trim();
  if let Some(idx) = token.rfind('⇒') {
    return token[idx + '⇒'.len_utf8()..].trim();
  }
  token
}

fn signature_arg_types(signature: &str, arity: usize) -> Vec<String> {
  let raw_parts: Vec<&str> = if signature.contains('→') {
    signature.split('→').collect()
  } else {
    signature.split("->").collect()
  };

  let mut args: Vec<String> = raw_parts
    .iter()
    .take(raw_parts.len().saturating_sub(1))
    .map(|t| normalize_signature_type(t).to_string())
    .collect();

  while args.len() < arity {
    args.push("Any".to_string());
  }

  args.truncate(arity);
  args
}

fn default_value_for_type(ty: &str) -> &'static str {
  let ty = ty.trim();
  if ty.contains("Bool") {
    "true"
  } else if ty.contains("Int") {
    "1"
  } else if ty.contains("Float") {
    "1.0"
  } else if ty.contains("Num") {
    "1"
  } else if ty.contains("String") {
    "\"x\""
  } else if ty.contains("List") {
    "[1]"
  } else if ty.contains("AttrSet") {
    "{ a = 1; }"
  } else if ty.contains("Any") || ty == "a" {
    "1"
  } else {
    "null"
  }
}

fn resolve_builtin_decl<'a>(
  uses: &'a str,
  catalog: &'a BuiltinCatalog,
) -> Option<(&'a BuiltinDecl, Cow<'a, str>)> {
  if let Some(name) = crate::builtins::resolve_builtin_name(uses) {
    let decl = catalog.get(name.as_ref())?;
    return Some((decl, name));
  }

  if let Some(decl) = catalog.get(uses) {
    return Some((decl, Cow::Borrowed(uses)));
  }

  None
}

fn base_op_name(uses: &str) -> &str {
  uses.rsplit('.').next().unwrap_or(uses)
}

fn default_args_for_builtin(builtin: &str, decl: &BuiltinDecl) -> Vec<String> {
  let arity = decl.arity.unwrap_or(0);
  let arg_types = signature_arg_types(&decl.signature, arity);
  let mut args: Vec<String> = arg_types
    .iter()
    .map(|ty| default_value_for_type(ty).to_string())
    .collect();

  match builtin {
    "map" => {
      if !args.is_empty() {
        args[0] = "x: x".to_string();
      }
    }
    "filter" | "find" => {
      if !args.is_empty() {
        args[0] = "x: true".to_string();
      }
    }
    "fold" | "foldl'" => {
      if !args.is_empty() {
        args[0] = "acc: x: acc".to_string();
      }
      if args.len() >= 2 {
        args[1] = "0".to_string();
      }
    }
    _ => {}
  }

  args
}

fn build_node_test_expr(uses: &str, catalog: &BuiltinCatalog) -> Option<String> {
  let (decl, builtin_name) = resolve_builtin_decl(uses, catalog)?;

  let args = default_args_for_builtin(builtin_name.as_ref(), decl);
  if args.is_empty() {
    Some(format!("builtins.{}", builtin_name))
  } else {
    Some(format!("builtins.{} {}", builtin_name, args.join(" ")))
  }
}

fn normalize_test_expr(expr: &str) -> String {
  let trimmed = expr.trim();
  if let Some(rest) = trimmed.strip_prefix("assert") {
    let boundary = rest.chars().next();
    let is_assert_keyword = matches!(boundary, Some(' ') | Some('\t') | Some('('));
    if is_assert_keyword && !trimmed.contains(';') {
      let cond = rest.trim_start();
      if !cond.is_empty() {
        return format!("assert {}; true", cond);
      }
    }
  }

  trimmed.to_string()
}

fn default_json_for_type(ty: &str) -> JsonValue {
  let ty = ty.trim();
  if ty.contains("Bool") {
    JsonValue::Bool(true)
  } else if ty.contains("Int") || ty.contains("Num") {
    JsonValue::from(1)
  } else if ty.contains("Float") {
    JsonValue::from(1.0)
  } else if ty.contains("String") {
    JsonValue::from("x")
  } else if ty.contains("List") {
    JsonValue::from(vec![JsonValue::from(1)])
  } else if ty.contains("AttrSet") {
    let mut obj = serde_json::Map::new();
    obj.insert("a".to_string(), JsonValue::from(1));
    JsonValue::Object(obj)
  } else if ty.contains("Any") || ty == "a" {
    JsonValue::from(1)
  } else {
    JsonValue::Null
  }
}

fn default_port_names(count: usize) -> Vec<String> {
  if count == 1 {
    return vec!["in".to_string()];
  }

  let mut names = Vec::new();
  for idx in 0..count {
    let name = match idx {
      0 => "a".to_string(),
      1 => "b".to_string(),
      2 => "c".to_string(),
      3 => "d".to_string(),
      _ => format!("in{}", idx + 1),
    };
    names.push(name);
  }
  names
}

fn build_node_test_module(
  name: &str,
  uses: &str,
  catalog: &BuiltinCatalog,
) -> (FxCoreModule, HashMap<String, JsonValue>) {
  let base = base_op_name(uses);
  let (arity, arg_types) = if let Some(decl) = catalog.get(base) {
    let arity = decl.arity.unwrap_or(1);
    (arity, signature_arg_types(&decl.signature, arity))
  } else {
    (1, vec!["Any".to_string()])
  };
  let port_names = default_port_names(arity);
  let input_ports: Vec<FxPort> = port_names
    .iter()
    .zip(arg_types.iter())
    .map(|(name, ty)| FxPort {
      name: name.clone(),
      ty: ty.clone(),
    })
    .collect();

  let inputs: Vec<FxInput> = input_ports
    .iter()
    .map(|port| FxInput {
      name: port.name.clone(),
      ty: port.ty.clone(),
    })
    .collect();
  let mut input_values = HashMap::new();
  for port in &input_ports {
    input_values.insert(port.name.clone(), default_json_for_type(&port.ty));
  }

  let morphism = FxMorphism {
    name: uses.to_string(),
    input: input_ports
      .first()
      .map(|p| p.ty.clone())
      .unwrap_or_else(|| "Any".to_string()),
    output: "Any".to_string(),
    inputs: input_ports.clone(),
    outputs: vec![FxPort {
      name: "out".to_string(),
      ty: "Any".to_string(),
    }],
    effect: Effect::Pure,
  };

  let node = FxNode {
    name: name.to_string(),
    uses: uses.to_string(),
    kind: NodeKind::Normal,
    meta: None,
    ..Default::default()
  };

  let edges: Vec<FxEdge> = input_ports
    .iter()
    .map(|port| FxEdge::from_input(port.name.clone(), name.to_string(), Some(port.name.clone())))
    .collect();

  let meta = FxCoreMeta {
    version: FXCORE_VERSION.to_string(),
    stage: 2,
    ..Default::default()
  };

  let module = FxCoreModule {
    meta,
    name: format!("test_node_{}", name),
    types: Vec::new(),
    adt_types: Vec::new(),
    adttypes: Vec::new(),
    inputs,
    morphisms: vec![morphism],
    nodes: vec![node],
    edges,
    scopes: Vec::new(),
  };

  (module, input_values)
}

fn build_backend_config(
  graph_config: &GraphTestConfig,
  inputs: HashMap<String, JsonValue>,
) -> BackendConfig {
  let inputs: BTreeMap<String, JsonValue> = inputs.into_iter().collect();
  BackendConfig {
    clojure_url: graph_config.clojure_url.clone(),
    python_url: graph_config.python_url.clone(),
    deno_url: graph_config.deno_url.clone(),
    blenderpy_url: graph_config.blenderpy_url.clone(),
    rpc_timeout_ms: graph_config.rpc_timeout_ms,
    rpc_retry_attempts: graph_config.rpc_retry_attempts,
    rpc_retry_backoff_ms: graph_config.rpc_retry_backoff_ms,
    rpc_retry_seed: graph_config.rpc_retry_seed,
    use_batch_apply: graph_config.use_batch_apply,
    allow_non_atomic_effects: graph_config.allow_non_atomic_effects,
    inputs,
    resource_limits: graph_config.resource_limits,
  }
}

fn eval_expr_test(test: &TestCase, expr: &str, config: &EvalConfig) -> Result<TestResult> {
  let wrapped_expr = format!("let result = ({}); in result", expr);
  let module = LegacyModule::from_source(wrapped_expr);

  let mut engine = LegacyEvalEngine::new();
  match engine.eval(&module, config) {
    Ok(_) => Ok(TestResult {
      name: test.name.clone(),
      passed: true,
      error: None,
    }),
    Err(err) => Ok(TestResult {
      name: test.name.clone(),
      passed: false,
      error: Some(err.to_string()),
    }),
  }
}

fn eval_node_test_graph(
  test: &TestCase,
  uses: &str,
  graph_config: &GraphTestConfig,
) -> Result<TestResult> {
  let catalog = BuiltinCatalog::with_defaults();
  let (fx, inputs) = build_node_test_module(&test.name, uses, &catalog);
  let plan = plan::build_plan(&fx)?;
  let backend_config = build_backend_config(graph_config, inputs);

  let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .map_err(|e| anyhow::anyhow!("failed to init test runtime: {}", e))?;
  let result = rt.block_on(async { apply::apply_graph(&fx, &plan, "test", &backend_config).await });

  match result {
    Ok(result) => {
      let ok = result.nodes_failed == 0 && result.status == ApplyStatus::Ok;
      if ok {
        return Ok(TestResult {
          name: test.name.clone(),
          passed: true,
          error: None,
        });
      }

      let err = result
        .trace
        .iter()
        .find(|entry| matches!(entry.status, NodeStatus::Failed))
        .and_then(|entry| match &entry.audit {
          AuditReason::Failed { error, .. } => Some(error.clone()),
          _ => None,
        })
        .unwrap_or_else(|| "node execution failed".to_string());

      Ok(TestResult {
        name: test.name.clone(),
        passed: false,
        error: Some(err),
      })
    }
    Err(err) => Ok(TestResult {
      name: test.name.clone(),
      passed: false,
      error: Some(err.to_string()),
    }),
  }
}

fn is_ident_like(name: &str) -> bool {
  !name.is_empty()
    && name
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn collect_tests_by_scan(source: &str, source_path: Option<&Path>) -> Vec<TestCase> {
  let mut tests = Vec::new();

  for raw in source.lines() {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
      continue;
    }

    if let Some(rest) = line.strip_prefix("test ") {
      let parts: Vec<&str> = rest.splitn(2, '=').collect();
      if parts.len() != 2 {
        continue;
      }
      let name = parts[0].trim();
      let expr = parts[1].trim();
      if !is_ident_like(name) || expr.is_empty() {
        continue;
      }
      tests.push(TestCase {
        name: name.to_string(),
        kind: TestCaseKind::Expr(expr.to_string()),
        source_file: source_path.map(|p| p.to_path_buf()),
      });
      continue;
    }

    if let Some(rest) = line.strip_prefix("@test ") {
      if let Some(node_rest) = rest.strip_prefix("node ") {
        let parts: Vec<&str> = node_rest.split_whitespace().collect();
        if parts.len() < 3 || parts[1] != "uses" {
          continue;
        }
        let name = parts[0].trim();
        let uses = parts[2].trim();
        if !is_ident_like(name) || !is_ident_like(uses) {
          continue;
        }
        tests.push(TestCase {
          name: name.to_string(),
          kind: TestCaseKind::Node {
            uses: uses.to_string(),
          },
          source_file: source_path.map(|p| p.to_path_buf()),
        });
        continue;
      }

      let expr = rest.trim();
      if expr.is_empty() {
        continue;
      }
      tests.push(TestCase {
        name: format!("test_{}", tests.len()),
        kind: TestCaseKind::Expr(expr.to_string()),
        source_file: source_path.map(|p| p.to_path_buf()),
      });
    }
  }

  tests
}

/// 소스 파일에서 테스트 선언 수집
pub fn collect_tests_from_source(
  source: &str,
  source_path: Option<&Path>,
) -> Result<Vec<TestCase>> {
  let mut diags = Diagnostics::default();
  let module = parse_module(source, "test", &mut diags)
    .map_err(|e| anyhow::anyhow!("Failed to parse module for test collection: {}", e))?;

  let mut tests = Vec::new();

  for item in &module.items {
    match item {
      AstItem::TestDecl { name, expr, .. } => {
        tests.push(TestCase {
          name: name.clone(),
          kind: TestCaseKind::Expr(expr.clone()),
          source_file: source_path.map(|p| p.to_path_buf()),
        });
      }
      AstItem::NodeDecl {
        name, uses, kind, ..
      } if kind.as_deref() == Some("test") => {
        tests.push(TestCase {
          name: name.clone(),
          kind: TestCaseKind::Node { uses: uses.clone() },
          source_file: source_path.map(|p| p.to_path_buf()),
        });
      }
      _ => {}
    }
  }

  if tests.is_empty() {
    let scanned = collect_tests_by_scan(source, source_path);
    if !scanned.is_empty() {
      return Ok(scanned);
    }
  }

  Ok(tests)
}

/// 단일 테스트 실행
pub fn run_single_test(
  test: &TestCase,
  filter: Option<&str>,
  config: &EvalConfig,
  graph_config: Option<&GraphTestConfig>,
) -> Result<TestResult> {
  // 필터 적용
  if let Some(filter_pattern) = filter {
    if !test.name.contains(filter_pattern) {
      return Ok(TestResult {
        name: test.name.clone(),
        passed: false, // 필터링된 테스트는 스킵 (실패로 표시하여 결과에 포함)
        error: Some("skipped (filtered)".to_string()),
      });
    }
  }

  match &test.kind {
    TestCaseKind::Expr(expr) => {
      let expr = normalize_test_expr(expr);
      eval_expr_test(test, &expr, config)
    }
    TestCaseKind::Node { uses } => {
      let catalog = BuiltinCatalog::with_defaults();
      if let Some(expr) = build_node_test_expr(uses, &catalog) {
        let expr = normalize_test_expr(&expr);
        return eval_expr_test(test, &expr, config);
      }

      let graph_config = match graph_config {
        Some(config) => config,
        None => {
          return Ok(TestResult {
            name: test.name.clone(),
            passed: false,
            error: Some(format!(
              "unsupported @test node uses '{}': external backends disabled",
              uses
            )),
          });
        }
      };

      eval_node_test_graph(test, uses, graph_config)
    }
  }
}

/// 여러 테스트 실행 (결정론적 순서 유지)
pub fn run_tests(
  tests: Vec<TestCase>,
  filter: Option<&str>,
  config: &EvalConfig,
  graph_config: Option<&GraphTestConfig>,
) -> TestSummary {
  let mut results = Vec::new();
  let mut passed = 0;
  let mut failed = 0;
  let mut skipped = 0;

  // 테스트를 이름순으로 정렬하여 결정론적 실행 순서 보장
  let mut sorted_tests = tests;
  sorted_tests.sort_by(|a, b| a.name.cmp(&b.name));

  for test in sorted_tests {
    match run_single_test(&test, filter, config, graph_config) {
      Ok(result) => {
        if result.error.as_deref() == Some("skipped (filtered)") {
          skipped += 1;
        } else if result.passed {
          passed += 1;
        } else {
          failed += 1;
        }
        results.push(result);
      }
      Err(err) => {
        failed += 1;
        results.push(TestResult {
          name: test.name.clone(),
          passed: false,
          error: Some(format!("Test execution error: {}", err)),
        });
      }
    }
  }

  TestSummary {
    total: results.len(),
    passed,
    failed,
    skipped,
    results,
  }
}

/// 테스트 요약 출력
pub fn print_test_summary(summary: &TestSummary) {
  println!("\nTest Results:");
  println!("  Total: {}", summary.total);
  println!("  Passed: {}", summary.passed);
  println!("  Failed: {}", summary.failed);
  if summary.skipped > 0 {
    println!("  Skipped: {}", summary.skipped);
  }

  if summary.failed > 0 {
    println!("\nFailed Tests:");
    for result in &summary.results {
      if !result.passed {
        println!(
          "  - {}: {}",
          result.name,
          result.error.as_deref().unwrap_or("unknown error")
        );
      }
    }
  }

  if summary.passed > 0 {
    println!("\nPassed Tests:");
    for result in &summary.results {
      if result.passed {
        println!("  - {}", result.name);
      }
    }
  }
}
