//! FxCore graph code generation
//!
//! FxCore graph → 실행 가능한 코드 생성
//! 텍스트 생성만 (헌법 C1 준수, P0-1 준수)

use crate::codegen::target::{CodeGenError, CodeGenResult, GeneratedCode, TargetLanguage};
use crate::core::FxCoreModule;
use crate::utils::escape_json_for_string;
use std::collections::{BTreeSet, HashMap};

/// FxCore 그래프에서 코드 생성
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 컴파일러/링커 호출 금지
pub fn generate(fx: &FxCoreModule, lang: TargetLanguage) -> CodeGenResult<GeneratedCode> {
  match lang {
    TargetLanguage::JavaScript => gen_javascript(fx),
    TargetLanguage::TypeScript => gen_typescript(fx),
    TargetLanguage::Python => gen_python(fx),
    TargetLanguage::Clojure => gen_clojure(fx),
    TargetLanguage::Nix => gen_nix(fx),
  }
}

/// JavaScript code generation
fn gen_javascript(fx: &FxCoreModule) -> CodeGenResult<GeneratedCode> {
  let mut code = String::new();
  let order = topo_order_checked(fx)?;

  // Header
  code.push_str(&format!("// Generated from FxCore graph: {}\n", fx.name));
  code.push_str("// DO NOT EDIT - This file is auto-generated\n\n");

  // Main execution function
  code.push_str(&format!(
    "async function execute_{name}(call, inputs) {{\n",
    name = sanitize_name(&fx.name)
  ));

  code.push_str("  const outputs = {};\n\n");

  // Generate node calls in topological order
  for node_name in &order {
    if let Some(node) = fx.nodes.iter().find(|n| &n.name == node_name) {
      let input_expr = build_input_expr(fx, node_name);
      let name_lit = json_string_literal(&node.name);
      let uses_lit = json_string_literal(&node.uses);
      code.push_str(&format!(
        "  outputs[{name}] = await call({uses}, {input});\n",
        name = name_lit,
        uses = uses_lit,
        input = input_expr
      ));
    }
  }

  code.push_str("\n  return outputs;\n");
  code.push_str("}\n\n");

  // Export
  code.push_str(&format!(
    "module.exports = {{ execute_{} }};\n",
    sanitize_name(&fx.name)
  ));

  Ok(GeneratedCode::new(code, TargetLanguage::JavaScript))
}

/// TypeScript code generation
fn gen_typescript(fx: &FxCoreModule) -> CodeGenResult<GeneratedCode> {
  let mut code = String::new();
  let order = topo_order_checked(fx)?;

  // Header
  code.push_str(&format!("// Generated from FxCore graph: {}\n", fx.name));
  code.push_str("// DO NOT EDIT - This file is auto-generated\n\n");

  // Type for morphism caller
  code.push_str("type MorphismCaller = (name: string, input: unknown) => Promise<unknown>;\n\n");

  // Main execution function
  code.push_str(&format!(
        "export async function execute_{name}(\n  call: MorphismCaller,\n  inputs: Record<string, unknown>\n): Promise<Record<string, unknown>> {{\n",
        name = sanitize_name(&fx.name)
    ));

  code.push_str("  const outputs: Record<string, unknown> = {};\n\n");

  // Generate node calls in topological order
  for node_name in &order {
    if let Some(node) = fx.nodes.iter().find(|n| &n.name == node_name) {
      let input_expr = build_input_expr(fx, node_name);
      let name_lit = json_string_literal(&node.name);
      let uses_lit = json_string_literal(&node.uses);
      code.push_str(&format!(
        "  outputs[{name}] = await call({uses}, {input});\n",
        name = name_lit,
        uses = uses_lit,
        input = input_expr
      ));
    }
  }

  code.push_str("\n  return outputs;\n");
  code.push_str("}\n");

  Ok(GeneratedCode::new(code, TargetLanguage::TypeScript))
}

/// Python code generation
fn gen_python(fx: &FxCoreModule) -> CodeGenResult<GeneratedCode> {
  let mut code = String::new();
  let order = topo_order_checked(fx)?;

  // Header
  code.push_str(&format!("# Generated from FxCore graph: {}\n", fx.name));
  code.push_str("# DO NOT EDIT - This file is auto-generated\n\n");
  code.push_str("from typing import Any, Callable, Dict, Awaitable\n\n");

  // Main execution function
  code.push_str(&format!(
        "async def execute_{name}(\n    call: Callable[[str, Any], Awaitable[Any]],\n    inputs: Dict[str, Any]\n) -> Dict[str, Any]:\n",
        name = sanitize_name(&fx.name)
    ));

  code.push_str("    outputs: Dict[str, Any] = {}\n\n");

  // Generate node calls
  for node_name in &order {
    if let Some(node) = fx.nodes.iter().find(|n| &n.name == node_name) {
      let input_expr = build_input_expr_py(fx, node_name);
      let name_lit = json_string_literal(&node.name);
      let uses_lit = json_string_literal(&node.uses);
      code.push_str(&format!(
        "    outputs[{name}] = await call({uses}, {input})\n",
        name = name_lit,
        uses = uses_lit,
        input = input_expr
      ));
    }
  }

  code.push_str("\n    return outputs\n");

  Ok(GeneratedCode::new(code, TargetLanguage::Python))
}

/// Clojure code generation
fn gen_clojure(fx: &FxCoreModule) -> CodeGenResult<GeneratedCode> {
  let mut code = String::new();
  let order = topo_order_checked(fx)?;
  let name_map = build_name_map_for_lang(fx, TargetLanguage::Clojure);

  // Header
  code.push_str(&format!(";; Generated from FxCore graph: {}\n", fx.name));
  code.push_str(";; DO NOT EDIT - This file is auto-generated\n\n");

  code.push_str("(ns generated.graph)\n\n");

  // Main execution function
  code.push_str(&format!(
    "(defn execute-{name}\n  [call inputs]\n  (let [\n",
    name = sanitize_name_clj(&fx.name)
  ));

  // Generate node bindings
  for node_name in &order {
    if let Some(node) = fx.nodes.iter().find(|n| &n.name == node_name) {
      let sanitized = name_map
        .get(node_name)
        .map(|s| s.as_str())
        .unwrap_or(node_name.as_str());
      let input_expr = build_input_expr_clj(fx, node_name, &name_map);
      let uses_lit = json_string_literal(&node.uses);
      code.push_str(&format!(
        "        {name} (call {uses} {input})\n",
        name = sanitized,
        uses = uses_lit,
        input = input_expr
      ));
    }
  }

  code.push_str("       ]\n    {");

  // Build result map
  for (i, node_name) in order.iter().enumerate() {
    let sanitized = name_map
      .get(node_name)
      .map(|s| s.as_str())
      .unwrap_or(node_name.as_str());
    if i > 0 {
      code.push(' ');
    }
    code.push_str(&format!(":{name} {name}", name = sanitized));
  }
  code.push_str("}))\n");

  Ok(GeneratedCode::new(code, TargetLanguage::Clojure))
}

/// Nix code generation
fn gen_nix(fx: &FxCoreModule) -> CodeGenResult<GeneratedCode> {
  let mut code = String::new();
  let order = topo_order_checked(fx)?;
  let name_map = build_name_map_for_lang(fx, TargetLanguage::Nix);

  // Header
  code.push_str(&format!("# Generated from FxCore graph: {}\n", fx.name));
  code.push_str("# DO NOT EDIT - This file is auto-generated\n\n");

  // Main function
  code.push_str("{ call, inputs }:\n\nlet\n");

  // Generate node bindings
  for node_name in &order {
    if let Some(node) = fx.nodes.iter().find(|n| &n.name == node_name) {
      let sanitized = name_map
        .get(&node.name)
        .map(|s| s.as_str())
        .unwrap_or(node.name.as_str());
      let input_expr = build_input_expr_nix(fx, node_name, &name_map);
      let uses_lit = nix_string_literal(&node.uses);
      code.push_str(&format!(
        "  {name} = call {uses} {input};\n",
        name = sanitized,
        uses = uses_lit,
        input = input_expr
      ));
    }
  }

  code.push_str("in {\n");

  // Build result attrset
  for node_name in &order {
    let sanitized = name_map
      .get(node_name)
      .map(|s| s.as_str())
      .unwrap_or(node_name.as_str());
    code.push_str(&format!("  {name} = {name};\n", name = sanitized));
  }
  code.push_str("}\n");

  Ok(GeneratedCode::new(code, TargetLanguage::Nix))
}

/// Topological order of nodes based on edges
fn topo_order(fx: &FxCoreModule) -> Vec<String> {
  let mut indeg: HashMap<String, usize> = fx.nodes.iter().map(|n| (n.name.clone(), 0)).collect();
  let mut adj: HashMap<String, Vec<String>> =
    fx.nodes.iter().map(|n| (n.name.clone(), vec![])).collect();

  for e in &fx.edges {
    if e.from == "input" {
      continue;
    }
    if let Some(targets) = adj.get_mut(&e.from) {
      targets.push(e.to.clone());
    }
    if let Some(deg) = indeg.get_mut(&e.to) {
      *deg += 1;
    }
  }

  for targets in adj.values_mut() {
    targets.sort();
  }

  let mut ready: BTreeSet<String> = indeg
    .iter()
    .filter_map(|(n, &d)| if d == 0 { Some(n.clone()) } else { None })
    .collect();
  let mut order = Vec::new();

  // 결정론 보장: BTreeSet의 pop_first()를 사용하여 항상 가장 작은 요소부터 처리
  while let Some(n) = ready.pop_first() {
    order.push(n.clone());
    if let Some(targets) = adj.get(&n) {
      for m in targets {
        if let Some(d) = indeg.get_mut(m) {
          *d = d.saturating_sub(1);
          if *d == 0 {
            ready.insert(m.clone());
          }
        }
      }
    }
  }

  order
}

fn topo_order_checked(fx: &FxCoreModule) -> CodeGenResult<Vec<String>> {
  let order = topo_order(fx);
  if order.len() != fx.nodes.len() {
    let mut missing: Vec<String> = fx
      .nodes
      .iter()
      .filter(|n| !order.iter().any(|name| name == &n.name))
      .map(|n| n.name.clone())
      .collect();
    missing.sort();
    return Err(CodeGenError::Internal(format!(
      "cyclic graph detected: {:?}",
      missing
    )));
  }
  Ok(order)
}

/// Build input expression for a node (TypeScript)
fn build_input_expr(fx: &FxCoreModule, node_name: &str) -> String {
  let mut sources: Vec<String> = Vec::new();

  for e in &fx.edges {
    if e.to == node_name {
      if e.from == "input" {
        // External input
        if let Some(input_name) = e.from_input.as_deref().or(e.to_port.as_deref()) {
          sources.push(format!("inputs[{}]", json_string_literal(input_name)));
        } else {
          sources.push("inputs".into());
        }
      } else {
        // Node output
        if let Some(ref port) = e.from_port {
          sources.push(format!(
            "outputs[{}]?.[{}]",
            json_string_literal(&e.from),
            json_string_literal(port)
          ));
        } else {
          sources.push(format!("outputs[{}]", json_string_literal(&e.from)));
        }
      }
    }
  }

  // 결정론 보장: sources를 정렬하여 항상 동일한 순서로 생성
  sources.sort();

  if sources.is_empty() {
    "{}".into()
  } else if sources.len() == 1 {
    sources.remove(0)
  } else {
    format!("{{ ...{} }}", sources.join(", ..."))
  }
}

/// Build input expression for Python
fn build_input_expr_py(fx: &FxCoreModule, node_name: &str) -> String {
  let mut sources: Vec<String> = Vec::new();

  for e in &fx.edges {
    if e.to == node_name {
      if e.from == "input" {
        if let Some(input_name) = e.from_input.as_deref().or(e.to_port.as_deref()) {
          sources.push(format!("inputs.get({})", json_string_literal(input_name)));
        } else {
          sources.push("inputs".into());
        }
      } else if let Some(ref port) = e.from_port {
        sources.push(format!(
          "outputs.get({}, {{}}).get({})",
          json_string_literal(&e.from),
          json_string_literal(port)
        ));
      } else {
        sources.push(format!("outputs.get({})", json_string_literal(&e.from)));
      }
    }
  }

  // 결정론 보장: sources를 정렬하여 항상 동일한 순서로 생성
  sources.sort();

  if sources.is_empty() {
    "{}".into()
  } else if sources.len() == 1 {
    sources.remove(0)
  } else {
    format!("{{{}}}", sources.join(", **"))
  }
}

/// Build input expression for Clojure
fn build_input_expr_clj(
  fx: &FxCoreModule,
  node_name: &str,
  name_map: &HashMap<String, String>,
) -> String {
  let mut sources: Vec<String> = Vec::new();

  for e in &fx.edges {
    if e.to == node_name {
      if e.from == "input" {
        if let Some(input_name) = e.from_input.as_deref().or(e.to_port.as_deref()) {
          sources.push(format!(
            "(get inputs (keyword {}))",
            json_string_literal(input_name)
          ));
        } else {
          sources.push("inputs".into());
        }
      } else {
        let from = name_map
          .get(&e.from)
          .map(|s| s.as_str())
          .unwrap_or(e.from.as_str());
        if let Some(ref port) = e.from_port {
          sources.push(format!(
            "(get {} (keyword {}))",
            from,
            json_string_literal(port)
          ));
        } else {
          sources.push(from.to_string());
        }
      }
    }
  }

  // 결정론 보장: sources를 정렬하여 항상 동일한 순서로 생성
  sources.sort();

  if sources.is_empty() {
    "{}".into()
  } else if sources.len() == 1 {
    sources.remove(0)
  } else {
    format!("(merge {})", sources.join(" "))
  }
}

/// Build input expression for Nix
fn build_input_expr_nix(
  fx: &FxCoreModule,
  node_name: &str,
  name_map: &HashMap<String, String>,
) -> String {
  let mut sources: Vec<String> = Vec::new();

  for e in &fx.edges {
    if e.to == node_name {
      if e.from == "input" {
        if let Some(input_name) = e.from_input.as_deref().or(e.to_port.as_deref()) {
          sources.push(format!("inputs.{}", nix_string_literal(input_name)));
        } else {
          sources.push("inputs".into());
        }
      } else {
        let from = name_map
          .get(&e.from)
          .map(|s| s.as_str())
          .unwrap_or(e.from.as_str());
        if let Some(ref port) = e.from_port {
          sources.push(format!("{}.{}", from, nix_string_literal(port)));
        } else {
          sources.push(from.to_string());
        }
      }
    }
  }

  // 결정론 보장: sources를 정렬하여 항상 동일한 순서로 생성
  sources.sort();

  if sources.is_empty() {
    "{}".into()
  } else if sources.len() == 1 {
    sources.remove(0)
  } else {
    format!("({} // {})", sources[0], sources[1..].join(" // "))
  }
}

fn json_string_literal(value: &str) -> String {
  format!("\"{}\"", escape_json_for_string(value))
}

fn nix_string_literal(value: &str) -> String {
  format!("\"{}\"", escape_nix_string(value))
}

fn escape_nix_string(value: &str) -> String {
  let mut escaped = String::with_capacity(value.len() + value.len() / 10);
  let mut chars = value.chars().peekable();
  while let Some(ch) = chars.next() {
    match ch {
      '\\' => escaped.push_str("\\\\"),
      '"' => escaped.push_str("\\\""),
      '\n' => escaped.push_str("\\n"),
      '\r' => escaped.push_str("\\r"),
      '\t' => escaped.push_str("\\t"),
      '$' if matches!(chars.peek(), Some('{')) => escaped.push_str("\\$"),
      _ => escaped.push(ch),
    }
  }
  escaped
}

/// Sanitize name for code generation with language-specific rules and collision handling
fn sanitize_name(name: &str) -> String {
  sanitize_name_for_lang(TargetLanguage::JavaScript, name, &[])
}

/// Sanitize name for Clojure (kebab-case)
fn sanitize_name_clj(name: &str) -> String {
  sanitize_name_for_lang(TargetLanguage::Clojure, name, &[])
}

fn build_name_map_for_lang(fx: &FxCoreModule, lang: TargetLanguage) -> HashMap<String, String> {
  let mut names: Vec<&str> = fx.nodes.iter().map(|n| n.name.as_str()).collect();
  names.sort();

  let mut used_names = Vec::new();
  let mut map = HashMap::new();

  for name in names {
    let sanitized = sanitize_name_for_lang(lang, name, &used_names);
    used_names.push(sanitized.clone());
    map.insert(name.to_string(), sanitized);
  }

  map
}

/// Sanitize name for a specific language with collision detection
///
/// # Arguments
/// * `lang` - Target language
/// * `name` - Original name to sanitize
/// * `used_names` - Set of already used names to avoid collisions
///
/// # Returns
/// Sanitized name, with suffix added if collision detected
fn sanitize_name_for_lang(lang: TargetLanguage, name: &str, used_names: &[String]) -> String {
  let sanitized = match lang {
    TargetLanguage::JavaScript | TargetLanguage::TypeScript => sanitize_js_ts_identifier(name),
    TargetLanguage::Python => sanitize_python_identifier(name),
    TargetLanguage::Clojure => sanitize_clojure_identifier(name),
    TargetLanguage::Nix => sanitize_nix_identifier(name),
  };

  // Check for collisions and add suffix if needed
  let mut final_name = sanitized.clone();
  let mut suffix = 1;
  while used_names.contains(&final_name) {
    final_name = format!("{}_{}", sanitized, suffix);
    suffix += 1;
  }

  final_name
}

/// Sanitize identifier for JavaScript/TypeScript
/// Rules: Start with letter/$, then alphanumeric/_/$
fn sanitize_js_ts_identifier(name: &str) -> String {
  let mut result = String::with_capacity(name.len());
  let chars: Vec<char> = name.chars().collect();

  for (i, ch) in chars.iter().enumerate() {
    if i == 0 {
      // First char: letter, $, or _
      if ch.is_alphabetic() || *ch == '$' || *ch == '_' {
        result.push(*ch);
      } else if ch.is_ascii_digit() {
        result.push('_');
        result.push(*ch);
      } else {
        result.push('_');
      }
    } else {
      // Subsequent chars: alphanumeric, $, _
      if ch.is_alphanumeric() || *ch == '$' || *ch == '_' {
        result.push(*ch);
      } else {
        result.push('_');
      }
    }
  }

  // Ensure not empty and not a keyword
  if result.is_empty() {
    result.push('_');
  }

  // Check for JS/TS keywords
  if is_js_ts_keyword(&result) {
    result.push('_');
  }

  result
}

/// Sanitize identifier for Python
/// Rules: Start with letter/_, then alphanumeric/_
fn sanitize_python_identifier(name: &str) -> String {
  let mut result = String::with_capacity(name.len());
  let chars: Vec<char> = name.chars().collect();

  for (i, ch) in chars.iter().enumerate() {
    if i == 0 {
      // First char: letter or _
      if ch.is_alphabetic() || *ch == '_' {
        result.push(*ch);
      } else if ch.is_ascii_digit() {
        result.push('_');
        result.push(*ch);
      } else {
        result.push('_');
      }
    } else {
      // Subsequent chars: alphanumeric or _
      if ch.is_alphanumeric() || *ch == '_' {
        result.push(*ch);
      } else {
        result.push('_');
      }
    }
  }

  // Ensure not empty
  if result.is_empty() {
    result.push('_');
  }

  // Check for Python keywords
  if is_python_keyword(&result) {
    result.push('_');
  }

  result
}

/// Sanitize identifier for Clojure (kebab-case)
/// Rules: alphanumeric, -, ?, !, *, +, =, <, >, &
fn sanitize_clojure_identifier(name: &str) -> String {
  let mut result = String::with_capacity(name.len());

  for ch in name.chars() {
    if ch.is_alphanumeric()
      || matches!(
        ch,
        '-' | '?' | '!' | '*' | '+' | '=' | '<' | '>' | '&' | '_'
      )
    {
      if ch == '_' {
        result.push('-');
      } else {
        result.push(ch);
      }
    } else {
      result.push('-');
    }
  }

  // Ensure not empty
  if result.is_empty() {
    result.push('-');
  }

  // Remove leading/trailing dashes
  result = result.trim_matches('-').to_string();
  if result.is_empty() {
    result.push('-');
  }

  result
}

/// Sanitize identifier for Nix
/// Rules: alphanumeric, _, -, '
fn sanitize_nix_identifier(name: &str) -> String {
  let mut result = String::with_capacity(name.len());

  for ch in name.chars() {
    if ch.is_alphanumeric() || matches!(ch, '_' | '-' | '\'') {
      result.push(ch);
    } else {
      result.push('_');
    }
  }

  // Ensure not empty
  if result.is_empty() {
    result.push('_');
  }

  // Check for Nix keywords
  if is_nix_keyword(&result) {
    result.push('_');
  }

  result
}

/// Check if string is a JavaScript/TypeScript keyword
fn is_js_ts_keyword(s: &str) -> bool {
  matches!(
    s,
    "break"
      | "case"
      | "catch"
      | "class"
      | "const"
      | "continue"
      | "debugger"
      | "default"
      | "delete"
      | "do"
      | "else"
      | "export"
      | "extends"
      | "finally"
      | "for"
      | "function"
      | "if"
      | "import"
      | "in"
      | "instanceof"
      | "new"
      | "return"
      | "super"
      | "switch"
      | "this"
      | "throw"
      | "try"
      | "typeof"
      | "var"
      | "void"
      | "while"
      | "with"
      | "yield"
      | "let"
      | "static"
      | "enum"
      | "implements"
      | "interface"
      | "package"
      | "private"
      | "protected"
      | "public"
      | "abstract"
      | "as"
      | "assert"
      | "async"
      | "await"
      | "from"
      | "of"
      | "true"
      | "false"
      | "null"
      | "undefined"
  )
}

/// Check if string is a Python keyword
fn is_python_keyword(s: &str) -> bool {
  matches!(
    s,
    "and"
      | "as"
      | "assert"
      | "break"
      | "class"
      | "continue"
      | "def"
      | "del"
      | "elif"
      | "else"
      | "except"
      | "exec"
      | "finally"
      | "for"
      | "from"
      | "global"
      | "if"
      | "import"
      | "in"
      | "is"
      | "lambda"
      | "not"
      | "or"
      | "pass"
      | "print"
      | "raise"
      | "return"
      | "try"
      | "while"
      | "with"
      | "yield"
      | "False"
      | "None"
      | "True"
  )
}

/// Check if string is a Nix keyword
fn is_nix_keyword(s: &str) -> bool {
  matches!(
    s,
    "if"
      | "then"
      | "else"
      | "assert"
      | "with"
      | "let"
      | "in"
      | "rec"
      | "inherit"
      | "or"
      | "import"
      | "true"
      | "false"
      | "null"
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::{CostHint, ExecutionContract, FxEdge, FxInput, FxNode, NodeKind};

  fn make_simple_graph() -> FxCoreModule {
    FxCoreModule {
      meta: Default::default(),
      name: "test-graph".into(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![
        FxNode {
          name: "n1".into(),
          uses: "py.normalize".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
        FxNode {
          name: "n2".into(),
          uses: "deno.render".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
      ],
      edges: vec![FxEdge::simple("n1".into(), "n2".into())],
      scopes: vec![],
    }
  }

  fn make_injection_graph() -> FxCoreModule {
    FxCoreModule {
      meta: Default::default(),
      name: "inject-graph".into(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![FxInput {
        name: "in\"put".into(),
        ty: "Int".into(),
      }],
      morphisms: vec![],
      nodes: vec![
        FxNode {
          name: "node\"1".into(),
          uses: "py.${evil}\"call".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
        FxNode {
          name: "n2".into(),
          uses: "deno.render".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
      ],
      edges: vec![
        FxEdge::from_input("in\"put".into(), "node\"1".into(), None),
        FxEdge::ported("node\"1".into(), Some("po\"rt".into()), "n2".into(), None),
      ],
      scopes: vec![],
    }
  }

  #[test]
  fn test_gen_javascript() {
    let fx = make_simple_graph();
    let result = gen_javascript(&fx).unwrap();
    assert!(result.code.contains("execute_test_graph"));
    assert!(result.code.contains("py.normalize"));
    assert!(result.code.contains("module.exports"));
    assert_eq!(result.language, TargetLanguage::JavaScript);
  }

  #[test]
  fn test_gen_typescript() {
    let fx = make_simple_graph();
    let result = gen_typescript(&fx).unwrap();
    assert!(result.code.contains("execute_test_graph"));
    assert!(result.code.contains("py.normalize"));
    assert!(result.code.contains("deno.render"));
    assert_eq!(result.language, TargetLanguage::TypeScript);
  }

  #[test]
  fn test_gen_python() {
    let fx = make_simple_graph();
    let result = gen_python(&fx).unwrap();
    assert!(result.code.contains("async def execute_test_graph"));
    assert!(result.code.contains("py.normalize"));
    assert_eq!(result.language, TargetLanguage::Python);
  }

  #[test]
  fn test_gen_clojure() {
    let fx = make_simple_graph();
    let result = gen_clojure(&fx).unwrap();
    assert!(result.code.contains("execute-test-graph"));
    assert!(result.code.contains("py.normalize"));
    assert_eq!(result.language, TargetLanguage::Clojure);
  }

  #[test]
  fn test_gen_nix() {
    let fx = make_simple_graph();
    let result = gen_nix(&fx).unwrap();
    assert!(result.code.contains("n1 = call"));
    assert!(result.code.contains("py.normalize"));
    assert_eq!(result.language, TargetLanguage::Nix);
  }

  #[test]
  fn test_gen_nix_unique_names() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "collision".into(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![
        FxNode {
          name: "a.b".into(),
          uses: "py.normalize".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
        FxNode {
          name: "a_b".into(),
          uses: "py.normalize".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
      ],
      edges: vec![],
      scopes: vec![],
    };

    let result = gen_nix(&fx).unwrap();
    assert!(result.code.contains("a_b = call"));
    assert!(result.code.contains("a_b_1 = call"));
  }

  #[test]
  fn test_topo_order() {
    let fx = make_simple_graph();
    let order = topo_order(&fx);
    let n1_pos = order.iter().position(|x| x == "n1").unwrap();
    let n2_pos = order.iter().position(|x| x == "n2").unwrap();
    assert!(n1_pos < n2_pos, "n1 should come before n2");
  }

  #[test]
  fn test_sanitize_name_collision() {
    // Test collision detection: "a-b" and "a_b" should both become "a_b" but with suffix
    let used = vec!["a_b".to_string()];
    let result = sanitize_name_for_lang(TargetLanguage::JavaScript, "a-b", &used);
    assert_eq!(result, "a_b_1");

    let used2 = vec!["a_b".to_string(), "a_b_1".to_string()];
    let result2 = sanitize_name_for_lang(TargetLanguage::JavaScript, "a-b", &used2);
    assert_eq!(result2, "a_b_2");
  }

  #[test]
  fn test_gen_clojure_unique_names() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "collision".into(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![
        FxNode {
          name: "a-b".into(),
          uses: "py.normalize".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
        FxNode {
          name: "a_b".into(),
          uses: "py.normalize".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
      ],
      edges: vec![],
      scopes: vec![],
    };

    let result = gen_clojure(&fx).unwrap();
    assert!(result.code.contains("a-b (call"));
    assert!(result.code.contains("a-b_1 (call"));
  }

  #[test]
  fn test_sanitize_name_js_ts() {
    // Test JS/TS identifier rules
    assert_eq!(sanitize_js_ts_identifier("test-name"), "test_name");
    assert_eq!(sanitize_js_ts_identifier("123invalid"), "_123invalid");
    assert_eq!(sanitize_js_ts_identifier("$valid"), "$valid");
    assert_eq!(sanitize_js_ts_identifier("if"), "if_"); // keyword
  }

  #[test]
  fn test_sanitize_name_python() {
    // Test Python identifier rules
    assert_eq!(sanitize_python_identifier("test-name"), "test_name");
    assert_eq!(sanitize_python_identifier("123invalid"), "_123invalid");
    assert_eq!(sanitize_python_identifier("def"), "def_"); // keyword
  }

  #[test]
  fn test_sanitize_name_clojure() {
    // Test Clojure identifier rules
    assert_eq!(sanitize_clojure_identifier("test_name"), "test-name");
    assert_eq!(sanitize_clojure_identifier("test.name"), "test-name");
    assert_eq!(sanitize_clojure_identifier("valid?"), "valid?");
  }

  #[test]
  fn test_sanitize_name_nix() {
    // Test Nix identifier rules
    assert_eq!(sanitize_nix_identifier("test-name"), "test-name");
    assert_eq!(sanitize_nix_identifier("test_name"), "test_name");
    assert_eq!(sanitize_nix_identifier("let"), "let_"); // keyword
  }

  #[test]
  fn test_topo_order_deterministic_for_independent_nodes() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "independent".into(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![
        FxNode {
          name: "b".into(),
          uses: "py.normalize".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
        FxNode {
          name: "a".into(),
          uses: "py.normalize".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
      ],
      edges: vec![],
      scopes: vec![],
    };

    let order = topo_order(&fx);
    assert_eq!(order, vec!["a", "b"]);
  }

  #[test]
  fn test_topo_order_ignores_input_edges() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "input-edges".into(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![FxInput {
        name: "x".into(),
        ty: "Int".into(),
      }],
      morphisms: vec![],
      nodes: vec![FxNode {
        name: "n1".into(),
        uses: "py.normalize".into(),
        kind: NodeKind::Normal,
        optional: false,
        scope: "global".into(),
        cost: CostHint::Medium,
        priority: 0,
        contract: ExecutionContract::default(),

        meta: None,
      }],
      edges: vec![FxEdge::from_input("x".into(), "n1".into(), None)],
      scopes: vec![],
    };

    let order = topo_order(&fx);
    assert_eq!(order, vec!["n1"]);
  }

  #[test]
  fn test_generate_all_implemented() {
    let fx = make_simple_graph();
    for lang in TargetLanguage::all_implemented() {
      let result = generate(&fx, *lang);
      assert!(result.is_ok(), "Failed for {:?}", lang);
    }
  }

  #[test]
  fn test_generate_cycle_error() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "cycle".into(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![
        FxNode {
          name: "a".into(),
          uses: "py.normalize".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
        FxNode {
          name: "b".into(),
          uses: "py.normalize".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
      ],
      edges: vec![
        FxEdge::simple("a".into(), "b".into()),
        FxEdge::simple("b".into(), "a".into()),
      ],
      scopes: vec![],
    };

    let result = gen_javascript(&fx);
    assert!(matches!(result, Err(CodeGenError::Internal(msg)) if msg.contains("cyclic graph")));
  }

  #[test]
  fn test_codegen_escapes_string_literals() {
    let fx = make_injection_graph();
    let name_lit = json_string_literal("node\"1");
    let uses_lit = json_string_literal("py.${evil}\"call");
    let input_lit = json_string_literal("in\"put");
    let port_lit = json_string_literal("po\"rt");

    let js = gen_javascript(&fx).unwrap();
    assert!(js
      .code
      .contains(&format!("outputs[{name_lit}] = await call({uses_lit}")));
    assert!(js.code.contains(&format!("inputs[{input_lit}]")));
    assert!(js.code.contains(&format!("?.[{port_lit}]")));

    let ts = gen_typescript(&fx).unwrap();
    assert!(ts
      .code
      .contains(&format!("outputs[{name_lit}] = await call({uses_lit}")));

    let py = gen_python(&fx).unwrap();
    assert!(py
      .code
      .contains(&format!("outputs[{name_lit}] = await call({uses_lit}")));
    assert!(py.code.contains(&format!("inputs.get({input_lit})")));

    let clj = gen_clojure(&fx).unwrap();
    let clj_names = build_name_map_for_lang(&fx, TargetLanguage::Clojure);
    let clj_node = clj_names.get("node\"1").unwrap();
    assert!(clj.code.contains(&format!("(call {uses_lit}")));
    assert!(clj
      .code
      .contains(&format!("(get inputs (keyword {input_lit}))")));
    assert!(clj
      .code
      .contains(&format!("(get {clj_node} (keyword {port_lit}))")));

    let nix = gen_nix(&fx).unwrap();
    let nix_names = build_name_map_for_lang(&fx, TargetLanguage::Nix);
    let nix_node = nix_names.get("node\"1").unwrap();
    let nix_uses = nix_string_literal("py.${evil}\"call");
    let nix_input = nix_string_literal("in\"put");
    let nix_port = nix_string_literal("po\"rt");
    assert!(nix.code.contains(&format!("call {nix_uses}")));
    assert!(nix.code.contains(&format!("inputs.{nix_input}")));
    assert!(nix.code.contains(&format!("{nix_node}.{nix_port}")));
    assert!(nix.code.contains("\\${"));
  }

  #[test]
  fn test_input_edge_uses_from_input_name() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "input-map".into(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![FxInput {
        name: "external_input".into(),
        ty: "Int".into(),
      }],
      morphisms: vec![],
      nodes: vec![FxNode {
        name: "n1".into(),
        uses: "py.normalize".into(),
        kind: NodeKind::Normal,
        optional: false,
        scope: "global".into(),
        cost: CostHint::Medium,
        priority: 0,
        contract: ExecutionContract::default(),

        meta: None,
      }],
      edges: vec![FxEdge::from_input(
        "external_input".into(),
        "n1".into(),
        Some("port".into()),
      )],
      scopes: vec![],
    };

    let result = gen_javascript(&fx).unwrap();
    assert!(result.code.contains("inputs[\"external_input\"]"));
    assert!(!result.code.contains("inputs[\"port\"]"));
  }
}
