//! Y15d: AI 친화적 IR 쿼리 API
//!
//! IR을 프로그래매틱하게 탐색하고 수정할 수 있는 쿼리 인터페이스

use pnix_core::core::FxCoreModule;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Query 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct QueryParams {
  pub ir: Value,
  pub query: String,
  #[serde(default)]
  pub timeout_ms: Option<u64>,
  #[serde(flatten)]
  pub query_args: HashMap<String, Value>,
}

/// Query 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct QueryResult {
  pub results: Vec<Value>,
}

/// Query 핸들러 타입
#[allow(dead_code)] // 향후 사용 예정
type QueryHandler =
  fn(&FxCoreModule, &HashMap<String, Value>, &QueryContext) -> Result<Vec<Value>, String>;

/// Query 레지스트리
#[allow(dead_code)] // 향후 사용 예정
struct QueryRegistry {
  handlers: HashMap<String, QueryHandlerEntry>,
}

struct QueryHandlerEntry {
  handler: QueryHandler,
  allowed_args: &'static [&'static str],
}

const DEFAULT_QUERY_TIMEOUT_MS: u64 = 5_000;

struct QueryContext {
  deadline: Option<Instant>,
}

impl QueryContext {
  fn new(deadline: Option<Instant>) -> Self {
    Self { deadline }
  }

  fn check(&self) -> Result<(), String> {
    if let Some(deadline) = self.deadline {
      if Instant::now() > deadline {
        return Err("Query timed out".to_string());
      }
    }
    Ok(())
  }
}

impl QueryRegistry {
  fn new() -> Self {
    let mut registry = Self {
      handlers: HashMap::new(),
    };

    // 기본 쿼리 핸들러 등록
    registry.register("find_morphism", &["name"], query_find_morphism);
    registry.register("list_morphisms", &[], query_list_morphisms);
    registry.register("find_node", &["name"], query_find_node);
    registry.register("list_nodes", &[], query_list_nodes);
    registry.register("find_edge", &["from", "to"], query_find_edge);
    registry.register("list_edges", &[], query_list_edges);
    registry.register("get_inputs", &[], query_get_inputs);
    registry.register("get_outputs", &["node"], query_get_outputs);

    registry
  }

  fn register(&mut self, name: &str, allowed_args: &'static [&'static str], handler: QueryHandler) {
    self.handlers.insert(
      name.to_string(),
      QueryHandlerEntry {
        handler,
        allowed_args,
      },
    );
  }

  fn handle(
    &self,
    query: &str,
    module: &FxCoreModule,
    args: &HashMap<String, Value>,
    context: &QueryContext,
  ) -> Result<Vec<Value>, String> {
    let entry = self
      .handlers
      .get(query)
      .ok_or_else(|| format!("Unknown query: {}", query))?;
    validate_query_args(query, args, entry.allowed_args)?;
    context.check()?;
    (entry.handler)(module, args, context)
  }

  #[allow(dead_code)] // 향후 사용 예정
  fn list_queries(&self) -> Vec<String> {
    let mut queries: Vec<String> = self.handlers.keys().cloned().collect();
    queries.sort(); // Stable ordering
    queries
  }
}

static QUERY_REGISTRY: OnceLock<QueryRegistry> = OnceLock::new();

fn query_registry() -> &'static QueryRegistry {
  QUERY_REGISTRY.get_or_init(QueryRegistry::new)
}

/// Query 요청 처리
pub fn handle_query(params: QueryParams) -> Result<QueryResult, String> {
  // 빈 쿼리 문자열 검증
  if params.query.is_empty() {
    return Err("Query string cannot be empty".to_string());
  }

  let timeout_ms = params.timeout_ms.unwrap_or(DEFAULT_QUERY_TIMEOUT_MS);
  if timeout_ms == 0 {
    return Err("Query timeout must be >= 1".to_string());
  }
  let deadline = Instant::now().checked_add(Duration::from_millis(timeout_ms));
  let context = QueryContext::new(deadline);

  // IR을 FxCoreModule로 파싱
  let module: FxCoreModule =
    // MEDIUM: 쿼리 에러 메시지 내부 상태 노출 수정 완료
    // 파싱 에러 메시지를 안전하게 처리
    serde_json::from_value(params.ir.clone()).map_err(|e| {
      let error_msg = e.to_string();
      // 에러 메시지에서 특수 문자 이스케이프
      let safe_msg: String = error_msg
        .chars()
        .flat_map(|c| match c {
          '\n' => "\\n".chars().collect::<Vec<_>>(),
          '\r' => "\\r".chars().collect::<Vec<_>>(),
          '\t' => "\\t".chars().collect::<Vec<_>>(),
          '"' => "\\\"".chars().collect::<Vec<_>>(),
          '\\' => "\\\\".chars().collect::<Vec<_>>(),
          _ => {
            if c.is_control() {
              format!("\\u{:04x}", c as u32).chars().collect()
            } else {
              vec![c]
            }
          }
        })
        .collect();
      format!("Failed to parse IR: {}", safe_msg)
    })?;

  context.check()?;

  // 쿼리 실행
  let results = query_registry().handle(&params.query, &module, &params.query_args, &context)?;

  Ok(QueryResult { results })
}

fn to_json_value<T: Serialize>(context: &str, value: &T) -> Result<Value, String> {
  serde_json::to_value(value).map_err(|e| format!("Failed to serialize {}: {}", context, e))
}

fn validate_query_args(
  query: &str,
  args: &HashMap<String, Value>,
  allowed_args: &'static [&'static str],
) -> Result<(), String> {
  if args.is_empty() {
    return Ok(());
  }

  let allowed: HashSet<&str> = allowed_args.iter().copied().collect();
  let mut unknown: Vec<String> = args
    .keys()
    .filter(|key| !allowed.contains(key.as_str()))
    .cloned()
    .collect();
  if unknown.is_empty() {
    return Ok(());
  }

  unknown.sort();
  Err(format!(
    "Unknown parameter(s) for query '{}': {}",
    query,
    unknown.join(", ")
  ))
}

/// 사용 가능한 쿼리 목록 반환
#[allow(dead_code)] // 향후 사용 예정
pub fn list_queries() -> Vec<String> {
  query_registry().list_queries()
}

// ============================================================
// Query 핸들러 구현
// ============================================================

/// find_morphism: 이름으로 morphism 찾기
fn query_find_morphism(
  module: &FxCoreModule,
  args: &HashMap<String, Value>,
  context: &QueryContext,
) -> Result<Vec<Value>, String> {
  let name = args
    .get("name")
    .and_then(|v| v.as_str())
    .ok_or_else(|| "Missing required parameter: name".to_string())?;

  let mut results = Vec::new();
  for morphism in &module.morphisms {
    context.check()?;
    if morphism.name == name {
      results.push(to_json_value(
        &format!("morphism '{}'", morphism.name),
        morphism,
      )?);
    }
  }

  Ok(results)
}

/// list_morphisms: 모든 morphism 목록 반환
fn query_list_morphisms(
  module: &FxCoreModule,
  _args: &HashMap<String, Value>,
  context: &QueryContext,
) -> Result<Vec<Value>, String> {
  let mut results: BTreeMap<String, Value> = BTreeMap::new();
  for morphism in &module.morphisms {
    context.check()?;
    if results.contains_key(&morphism.name) {
      continue;
    }
    let value = to_json_value(&format!("morphism '{}'", morphism.name), morphism)?;
    results.insert(morphism.name.clone(), value);
  }

  Ok(results.into_values().collect())
}

/// find_node: 이름으로 노드 찾기
fn query_find_node(
  module: &FxCoreModule,
  args: &HashMap<String, Value>,
  context: &QueryContext,
) -> Result<Vec<Value>, String> {
  let name = args
    .get("name")
    .and_then(|v| v.as_str())
    .ok_or_else(|| "Missing required parameter: name".to_string())?;

  let mut results = Vec::new();
  for node in &module.nodes {
    context.check()?;
    if node.name == name {
      results.push(to_json_value(&format!("node '{}'", node.name), node)?);
    }
  }

  Ok(results)
}

/// list_nodes: 모든 노드 목록 반환
fn query_list_nodes(
  module: &FxCoreModule,
  _args: &HashMap<String, Value>,
  context: &QueryContext,
) -> Result<Vec<Value>, String> {
  let mut results = Vec::new();
  for node in &module.nodes {
    context.check()?;
    results.push(to_json_value(&format!("node '{}'", node.name), node)?);
  }

  // Stable ordering: 이름으로 정렬
  results.sort_by_key(|v| {
    v.get("name")
      .and_then(|n| n.as_str())
      .unwrap_or("")
      .to_string()
  });

  Ok(results)
}

/// find_edge: 특정 엣지 찾기
fn query_find_edge(
  module: &FxCoreModule,
  args: &HashMap<String, Value>,
  context: &QueryContext,
) -> Result<Vec<Value>, String> {
  let from = args.get("from").and_then(|v| v.as_str());
  let to = args.get("to").and_then(|v| v.as_str());

  let mut results = Vec::new();
  for edge in &module.edges {
    context.check()?;
    if let Some(from_val) = from {
      if edge.from != from_val {
        continue;
      }
    }
    if let Some(to_val) = to {
      if edge.to != to_val {
        continue;
      }
    }
    results.push(to_json_value(
      &format!("edge {}->{}", edge.from, edge.to),
      edge,
    )?);
  }

  Ok(results)
}

/// list_edges: 모든 엣지 목록 반환
fn query_list_edges(
  module: &FxCoreModule,
  _args: &HashMap<String, Value>,
  context: &QueryContext,
) -> Result<Vec<Value>, String> {
  let mut results = Vec::new();
  for edge in &module.edges {
    context.check()?;
    results.push(to_json_value(
      &format!("edge {}->{}", edge.from, edge.to),
      edge,
    )?);
  }

  // Stable ordering: from, to로 정렬
  results.sort_by_key(|v| {
    let from = v
      .get("from")
      .and_then(|f| f.as_str())
      .unwrap_or("")
      .to_string();
    let to = v
      .get("to")
      .and_then(|t| t.as_str())
      .unwrap_or("")
      .to_string();
    (from, to)
  });

  Ok(results)
}

/// get_inputs: 모듈의 입력 목록 반환
fn query_get_inputs(
  module: &FxCoreModule,
  _args: &HashMap<String, Value>,
  context: &QueryContext,
) -> Result<Vec<Value>, String> {
  let mut results = Vec::new();
  for input in &module.inputs {
    context.check()?;
    results.push(to_json_value(&format!("input '{}'", input.name), input)?);
  }

  // Stable ordering: 이름으로 정렬
  results.sort_by_key(|v| {
    v.get("name")
      .and_then(|n| n.as_str())
      .unwrap_or("")
      .to_string()
  });

  Ok(results)
}

/// get_outputs: 특정 노드의 출력 포트 반환
fn query_get_outputs(
  module: &FxCoreModule,
  args: &HashMap<String, Value>,
  context: &QueryContext,
) -> Result<Vec<Value>, String> {
  let node_name = args
    .get("node")
    .and_then(|v| v.as_str())
    .ok_or_else(|| "Missing required parameter: node".to_string())?;

  context.check()?;
  // 노드 찾기
  // MEDIUM: 쿼리 에러 메시지 내부 상태 노출 수정 완료
  // 사용자 입력을 안전하게 이스케이프하여 내부 상태 노출 방지
  let node = module
    .nodes
    .iter()
    .find(|n| n.name == node_name)
    .ok_or_else(|| {
      // 특수 문자를 이스케이프하여 안전하게 처리
      let safe_name: String = node_name
        .chars()
        .flat_map(|c| match c {
          '\n' => "\\n".chars().collect::<Vec<_>>(),
          '\r' => "\\r".chars().collect::<Vec<_>>(),
          '\t' => "\\t".chars().collect::<Vec<_>>(),
          '"' => "\\\"".chars().collect::<Vec<_>>(),
          '\\' => "\\\\".chars().collect::<Vec<_>>(),
          _ => {
            // 제어 문자는 유니코드 이스케이프로 변환
            if c.is_control() {
              format!("\\u{:04x}", c as u32).chars().collect()
            } else {
              vec![c]
            }
          }
        })
        .collect();
      format!("Node not found: {}", safe_name)
    })?;

  context.check()?;
  // 노드가 사용하는 morphism 찾기
  let morphism = module
    .morphisms
    .iter()
    .find(|m| m.name == node.uses)
    .ok_or_else(|| {
      // 특수 문자를 이스케이프하여 안전하게 처리
      let safe_name: String = node
        .uses
        .chars()
        .flat_map(|c| match c {
          '\n' => "\\n".chars().collect::<Vec<_>>(),
          '\r' => "\\r".chars().collect::<Vec<_>>(),
          '\t' => "\\t".chars().collect::<Vec<_>>(),
          '"' => "\\\"".chars().collect::<Vec<_>>(),
          '\\' => "\\\\".chars().collect::<Vec<_>>(),
          _ => {
            if c.is_control() {
              format!("\\u{:04x}", c as u32).chars().collect()
            } else {
              vec![c]
            }
          }
        })
        .collect();
      format!("Morphism not found: {}", safe_name)
    })?;

  // 출력 포트 반환
  let mut results = Vec::new();
  for output in &morphism.outputs {
    context.check()?;
    results.push(to_json_value(
      &format!("output port '{}'", output.name),
      output,
    )?);
  }

  Ok(results)
}

#[cfg(test)]
mod tests {
  use super::*;
  use pnix_core::contracts::effect::Effect;
  use pnix_core::core::{FxCoreMeta, FxEdge, FxMorphism, FxNode};

  fn test_context() -> QueryContext {
    QueryContext::new(None)
  }

  fn create_test_module() -> FxCoreModule {
    FxCoreModule {
      meta: FxCoreMeta::default(),
      name: "test".to_string(),
      types: vec!["Int".to_string()],
      adt_types: Vec::new(),
      adttypes: Vec::new(),
      inputs: vec![],
      morphisms: vec![
        FxMorphism::simple(
          "add".to_string(),
          "Int".to_string(),
          "Int".to_string(),
          Effect::Pure,
        ),
        FxMorphism::simple(
          "mul".to_string(),
          "Int".to_string(),
          "Int".to_string(),
          Effect::Pure,
        ),
      ],
      nodes: vec![
        FxNode {
          name: "n1".to_string(),
          uses: "add".to_string(),
          meta: None,
          ..Default::default()
        },
        FxNode {
          name: "n2".to_string(),
          uses: "mul".to_string(),
          meta: None,
          ..Default::default()
        },
      ],
      edges: vec![FxEdge::simple("n1".to_string(), "n2".to_string())],
      scopes: Vec::new(),
    }
  }

  #[test]
  fn test_find_morphism() {
    let module = create_test_module();
    let mut args = HashMap::new();
    args.insert("name".to_string(), Value::String("add".to_string()));

    let result = query_find_morphism(&module, &args, &test_context()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].get("name").and_then(|n| n.as_str()), Some("add"));
  }

  #[test]
  fn test_list_morphisms() {
    let module = create_test_module();
    let args = HashMap::new();

    let result = query_list_morphisms(&module, &args, &test_context()).unwrap();
    assert_eq!(result.len(), 2);
    // Stable ordering 확인
    let names: Vec<&str> = result
      .iter()
      .map(|v| v.get("name").and_then(|n| n.as_str()).unwrap_or(""))
      .collect();
    assert_eq!(names, vec!["add", "mul"]);
  }

  #[test]
  fn test_find_node() {
    let module = create_test_module();
    let mut args = HashMap::new();
    args.insert("name".to_string(), Value::String("n1".to_string()));

    let result = query_find_node(&module, &args, &test_context()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].get("name").and_then(|n| n.as_str()), Some("n1"));
  }

  #[test]
  fn test_list_nodes() {
    let module = create_test_module();
    let args = HashMap::new();

    let result = query_list_nodes(&module, &args, &test_context()).unwrap();
    assert_eq!(result.len(), 2);
  }

  #[test]
  fn test_find_edge() {
    let module = create_test_module();
    let mut args = HashMap::new();
    args.insert("from".to_string(), Value::String("n1".to_string()));
    args.insert("to".to_string(), Value::String("n2".to_string()));

    let result = query_find_edge(&module, &args, &test_context()).unwrap();
    assert_eq!(result.len(), 1);
  }

  #[test]
  fn test_list_queries() {
    let queries = list_queries();
    assert!(queries.contains(&"find_morphism".to_string()));
    assert!(queries.contains(&"list_morphisms".to_string()));
    assert!(queries.contains(&"find_node".to_string()));
    assert!(queries.contains(&"list_nodes".to_string()));
  }
}
