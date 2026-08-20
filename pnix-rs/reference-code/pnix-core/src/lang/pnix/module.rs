//! Pnix Module Frontend (data-only)
//!
//! Goal: represent the line-based module DSL (types/inputs/externs/nodes/edges/scopes)
//! using the canonical Pnix surface syntax (`.sam/.px`) without evaluation.
//!
//! This module intentionally supports a **data-only subset** of Pnix:
//! - allowed: strings, ints, floats, bools, null, lists, attrsets (assign only)
//! - disallowed: vars/let/if/lambda/apply/select/binary ops, `inherit`
//!
//! If a value cannot be represented in this subset, we **fail loudly**.

use crate::ast::{
  AdtTypeDecl, AdtVariant, AstItem, AstModule, EdgeCondAst, EdgeSource, EdgeTarget, PortAst, SigAst,
};
use crate::diagnostics::Span;

use super::error::PnixError;
use super::syntax::{PnixAttrItem, PnixExpr};

const TOP_LEVEL_KEYS: &[&str] = &[
  "name",
  "types",
  "adt_types",
  "inputs",
  "externs",
  "nodes",
  "edges",
  "scopes",
];
const TOP_LEVEL_KEYS_WITH_IMPORTS: &[&str] = &[
  "imports",
  "name",
  "types",
  "adt_types",
  "inputs",
  "externs",
  "nodes",
  "edges",
  "scopes",
];

/// Pnix 모듈과 임포트: AST 모듈과 임포트 경로를 함께 포함하는 구조
pub struct PnixModuleWithImports {
  /// AST 모듈 (파싱된 모듈 AST)
  pub ast: AstModule,
  /// 임포트 경로 목록 (임포트한 모듈 경로 목록)
  pub imports: Vec<String>,
}

/// Pnix "module attrset"를 line-DSL AST 모듈로 파싱
///
/// 예상되는 최상위 형태:
/// ```text
/// {
///   name = "demo";
///   types = [ "Real" "Bool" ];
///   inputs = { x = "Real"; y = "Real"; };
///   externs = [ { name = "py.add"; input = "Real"; output = "Real"; } ];
///   nodes = [ { name = "N1"; uses = "py.add"; } ];
///   edges = [ { from = { input = "x"; }; to = { node = "N1"; port = "in"; }; } ];
///   scopes = [ { name = "global"; policy = "best_effort"; } ];
/// }
/// ```
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_module(
  source: &str,
  fallback_name: &str,
  file: Option<&str>,
) -> Result<AstModule, PnixError> {
  let expr = super::parse_expr(source)?;
  module_expr_to_ast(&expr, fallback_name, file)
}

/// Pnix 모듈을 임포트와 함께 파싱
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_module_with_imports(
  source: &str,
  fallback_name: &str,
  file: Option<&str>,
) -> Result<PnixModuleWithImports, PnixError> {
  let expr = super::parse_expr(source)?;
  module_expr_to_ast_with_imports(&expr, fallback_name, file)
}

/// Pnix 표현식을 AST 모듈로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn module_expr_to_ast(
  expr: &PnixExpr,
  fallback_name: &str,
  file: Option<&str>,
) -> Result<AstModule, PnixError> {
  let top = expect_attrset(expr, "module")?;
  let top_pairs = attrset_pairs(top, "module")?;

  check_unknown_keys(&top_pairs, TOP_LEVEL_KEYS)?;

  let file_for_span = file.unwrap_or(fallback_name);
  let span = Span::with_file(0, 0, file_for_span);

  let module_name = match get(&top_pairs, "name") {
    None => fallback_name.to_string(),
    Some(v) => expect_string(v, "module.name")?,
  };

  let items = parse_module_items(&top_pairs, &span)?;

  Ok(AstModule {
    name: module_name,
    items,
  })
}

/// Pnix 표현식을 AST 모듈과 임포트로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn module_expr_to_ast_with_imports(
  expr: &PnixExpr,
  fallback_name: &str,
  file: Option<&str>,
) -> Result<PnixModuleWithImports, PnixError> {
  let top = expect_attrset(expr, "module")?;
  let top_pairs = attrset_pairs(top, "module")?;

  check_unknown_keys(&top_pairs, TOP_LEVEL_KEYS_WITH_IMPORTS)?;

  let file_for_span = file.unwrap_or(fallback_name);
  let span = Span::with_file(0, 0, file_for_span);

  let module_name = match get(&top_pairs, "name") {
    None => fallback_name.to_string(),
    Some(v) => expect_string(v, "module.name")?,
  };

  let imports = match get(&top_pairs, "imports") {
    Some(v) => expect_string_list(v, "module.imports")?,
    None => Vec::new(),
  };

  let items = parse_module_items(&top_pairs, &span)?;

  Ok(PnixModuleWithImports {
    ast: AstModule {
      name: module_name,
      items,
    },
    imports,
  })
}

fn check_unknown_keys(
  top_pairs: &[(String, PnixExpr)],
  allowed_keys: &[&str],
) -> Result<(), PnixError> {
  // Unknown keys are almost always typos; fail loudly.
  for (k, _) in top_pairs {
    if k.starts_with('_') {
      continue;
    }
    if k == "meta" {
      continue;
    }
    if !allowed_keys.contains(&k.as_str()) {
      return Err(PnixError::UnsupportedSyntax {
        message: format!(
          "unknown top-level key `{}` (allowed: {})",
          k,
          allowed_keys.join(", ")
        ),
        span: None,
      });
    }
  }
  Ok(())
}

fn parse_module_items(
  top_pairs: &[(String, PnixExpr)],
  span: &Span,
) -> Result<Vec<AstItem>, PnixError> {
  let mut items: Vec<AstItem> = Vec::new();

  // types: [ "T" ... ]
  if let Some(v) = get(top_pairs, "types") {
    for ty in expect_string_list(v, "module.types")? {
      items.push(AstItem::TypeDecl {
        name: ty,
        span: span.clone(),
      });
    }
  }

  // adt_types: [ { name, params, variants } ... ]
  if let Some(v) = get(top_pairs, "adt_types") {
    items.extend(parse_adt_types(v, span)?);
  }

  // inputs: { x = "T"; ... } OR [ { name="x"; ty="T"; } ... ]
  if let Some(v) = get(top_pairs, "inputs") {
    items.extend(parse_inputs(v, span)?);
  }

  // externs: [ { ... } ... ]
  if let Some(v) = get(top_pairs, "externs") {
    items.extend(parse_externs(v, span)?);
  }

  // nodes: [ { ... } ... ]
  if let Some(v) = get(top_pairs, "nodes") {
    items.extend(parse_nodes(v, span)?);
  }

  // edges: [ { ... } ... ]
  if let Some(v) = get(top_pairs, "edges") {
    items.extend(parse_edges(v, span)?);
  }

  // scopes: [ { ... } ... ]
  if let Some(v) = get(top_pairs, "scopes") {
    items.extend(parse_scopes(v, span)?);
  }

  // imports: [ "path1", "path2", ... ] (Y07a: import 경로를 AST에 추가)
  if let Some(v) = get(top_pairs, "imports") {
    let import_paths = expect_string_list(v, "module.imports")?;
    for path in import_paths {
      items.push(AstItem::ImportDecl {
        path,
        span: span.clone(),
      });
    }
  }

  Ok(items)
}

fn parse_inputs(v: &PnixExpr, span: &Span) -> Result<Vec<AstItem>, PnixError> {
  match v {
    PnixExpr::AttrSet { items, .. } => {
      let pairs = attrset_pairs(items, "module.inputs")?;
      let mut out = Vec::new();
      for (name, ty_expr) in pairs {
        if name.starts_with('_') {
          continue;
        }
        let ty = expect_string(&ty_expr, &format!("module.inputs.{}", name))?;
        out.push(AstItem::InputDecl {
          name,
          ty,
          span: span.clone(),
        });
      }
      Ok(out)
    }
    PnixExpr::List(elements) => {
      let mut out = Vec::new();
      for (idx, el) in elements.iter().enumerate() {
        let el_items = expect_attrset(el, &format!("module.inputs[{}]", idx))?;
        let pairs = attrset_pairs(el_items, &format!("module.inputs[{}]", idx))?;
        ensure_only_keys(&pairs, &["name", "ty"], &format!("module.inputs[{}]", idx))?;
        let name = expect_string(
          get(&pairs, "name").ok_or_else(|| missing("module.inputs[].name"))?,
          &format!("module.inputs[{}].name", idx),
        )?;
        let ty = expect_string(
          get(&pairs, "ty").ok_or_else(|| missing("module.inputs[].ty"))?,
          &format!("module.inputs[{}].ty", idx),
        )?;
        out.push(AstItem::InputDecl {
          name,
          ty,
          span: span.clone(),
        });
      }
      Ok(out)
    }
    other => Err(PnixError::UnsupportedSyntax {
      message: format!(
        "module.inputs must be an attrset or list, got {}",
        kind(other)
      ),
      span: None,
    }),
  }
}

fn parse_externs(v: &PnixExpr, span: &Span) -> Result<Vec<AstItem>, PnixError> {
  let elements = expect_list(v, "module.externs")?;
  let mut out = Vec::new();
  for (idx, el) in elements.iter().enumerate() {
    let el_items = expect_attrset(el, &format!("module.externs[{}]", idx))?;
    let pairs = attrset_pairs(el_items, &format!("module.externs[{}]", idx))?;
    ensure_only_keys(
      &pairs,
      &[
        "name", "sig", "input", "output", "inputs", "outputs", "meta",
      ],
      &format!("module.externs[{}]", idx),
    )?;

    let name = expect_string(
      get(&pairs, "name").ok_or_else(|| missing("module.externs[].name"))?,
      &format!("module.externs[{}].name", idx),
    )?;

    let sig = if let Some(sig_expr) = get(&pairs, "sig") {
      parse_sig(sig_expr, &format!("module.externs[{}].sig", idx))?
    } else if get(&pairs, "inputs").is_some() || get(&pairs, "outputs").is_some() {
      parse_sig_from_ports(&pairs, &format!("module.externs[{}]", idx))?
    } else {
      let input = expect_string(
        get(&pairs, "input").ok_or_else(|| missing("module.externs[].input"))?,
        &format!("module.externs[{}].input", idx),
      )?;
      let output = expect_string(
        get(&pairs, "output").ok_or_else(|| missing("module.externs[].output"))?,
        &format!("module.externs[{}].output", idx),
      )?;
      SigAst::simple(input, output)
    };

    out.push(AstItem::ExternDecl {
      name,
      sig,
      span: span.clone(),
    });
  }
  Ok(out)
}

fn parse_sig(sig_expr: &PnixExpr, ctx: &str) -> Result<SigAst, PnixError> {
  let items = expect_attrset(sig_expr, ctx)?;
  let pairs = attrset_pairs(items, ctx)?;
  ensure_only_keys(&pairs, &["input", "output", "inputs", "outputs"], ctx)?;

  if get(&pairs, "inputs").is_some() || get(&pairs, "outputs").is_some() {
    parse_sig_from_ports(&pairs, ctx)
  } else {
    let input = expect_string(
      get(&pairs, "input").ok_or_else(|| missing(&format!("{ctx}.input")))?,
      &format!("{ctx}.input"),
    )?;
    let output = expect_string(
      get(&pairs, "output").ok_or_else(|| missing(&format!("{ctx}.output")))?,
      &format!("{ctx}.output"),
    )?;
    Ok(SigAst::simple(input, output))
  }
}

fn parse_sig_from_ports(pairs: &[(String, PnixExpr)], ctx: &str) -> Result<SigAst, PnixError> {
  let inputs_expr = get(pairs, "inputs").ok_or_else(|| missing(&format!("{ctx}.inputs")))?;
  let outputs_expr = get(pairs, "outputs").ok_or_else(|| missing(&format!("{ctx}.outputs")))?;
  let inputs = parse_ports(inputs_expr, &format!("{ctx}.inputs"))?;
  let outputs = parse_ports(outputs_expr, &format!("{ctx}.outputs"))?;
  Ok(SigAst::ported(inputs, outputs))
}

fn parse_ports(v: &PnixExpr, ctx: &str) -> Result<Vec<PortAst>, PnixError> {
  let elements = expect_list(v, ctx)?;
  let mut out = Vec::new();
  for (idx, el) in elements.iter().enumerate() {
    let el_items = expect_attrset(el, &format!("{ctx}[{idx}]"))?;
    let pairs = attrset_pairs(el_items, &format!("{ctx}[{idx}]"))?;
    ensure_only_keys(&pairs, &["name", "ty"], &format!("{ctx}[{idx}]"))?;
    let name = expect_string(
      get(&pairs, "name").ok_or_else(|| missing(&format!("{ctx}[{idx}].name")))?,
      &format!("{ctx}[{idx}].name"),
    )?;
    let ty = expect_string(
      get(&pairs, "ty").ok_or_else(|| missing(&format!("{ctx}[{idx}].ty")))?,
      &format!("{ctx}[{idx}].ty"),
    )?;
    out.push(PortAst { name, ty });
  }
  Ok(out)
}

fn parse_nodes(v: &PnixExpr, span: &Span) -> Result<Vec<AstItem>, PnixError> {
  let elements = expect_list(v, "module.nodes")?;
  let mut out = Vec::new();
  for (idx, el) in elements.iter().enumerate() {
    let el_items = expect_attrset(el, &format!("module.nodes[{}]", idx))?;
    let pairs = attrset_pairs(el_items, &format!("module.nodes[{}]", idx))?;
    ensure_only_keys(
      &pairs,
      &[
        "name", "uses", "kind", "gate", "optional", "scope", "cost", "priority", "meta",
      ],
      &format!("module.nodes[{}]", idx),
    )?;

    let name = expect_string(
      get(&pairs, "name").ok_or_else(|| missing("module.nodes[].name"))?,
      &format!("module.nodes[{}].name", idx),
    )?;
    let uses = expect_string(
      get(&pairs, "uses").ok_or_else(|| missing("module.nodes[].uses"))?,
      &format!("module.nodes[{}].uses", idx),
    )?;

    let kind = if let Some(v) = get(&pairs, "kind") {
      Some(expect_string(v, &format!("module.nodes[{}].kind", idx))?)
    } else if get_bool(&pairs, "gate")?.unwrap_or(false) {
      Some("gate".into())
    } else {
      None
    };

    let optional = get_bool(&pairs, "optional")?.unwrap_or(false);
    let scope = get_opt_string(&pairs, "scope", &format!("module.nodes[{}].scope", idx))?;
    let cost = get_opt_string(&pairs, "cost", &format!("module.nodes[{}].cost", idx))?;
    let priority = get_opt_int(
      &pairs,
      "priority",
      &format!("module.nodes[{}].priority", idx),
    )?;

    out.push(AstItem::NodeDecl {
      name,
      uses,
      kind,
      optional,
      scope,
      cost,
      priority,
      span: span.clone(),
    });
  }
  Ok(out)
}

fn parse_edges(v: &PnixExpr, span: &Span) -> Result<Vec<AstItem>, PnixError> {
  let elements = expect_list(v, "module.edges")?;
  let mut out = Vec::new();
  for (idx, el) in elements.iter().enumerate() {
    let el_items = expect_attrset(el, &format!("module.edges[{}]", idx))?;
    let pairs = attrset_pairs(el_items, &format!("module.edges[{}]", idx))?;
    ensure_only_keys(
      &pairs,
      &["from", "to", "cond", "meta"],
      &format!("module.edges[{}]", idx),
    )?;

    let from_expr = get(&pairs, "from").ok_or_else(|| missing("module.edges[].from"))?;
    let to_expr = get(&pairs, "to").ok_or_else(|| missing("module.edges[].to"))?;

    let from = parse_edge_source(from_expr, &format!("module.edges[{}].from", idx))?;
    let to = parse_edge_target(to_expr, &format!("module.edges[{}].to", idx))?;
    let cond = if let Some(cond_expr) = get(&pairs, "cond") {
      Some(parse_edge_cond(
        cond_expr,
        &format!("module.edges[{}].cond", idx),
      )?)
    } else {
      None
    };

    out.push(AstItem::EdgeDecl {
      from,
      to,
      cond,
      span: span.clone(),
    });
  }
  Ok(out)
}

fn parse_edge_source(v: &PnixExpr, ctx: &str) -> Result<EdgeSource, PnixError> {
  // Support both shorthand string and structured attrset formats:
  // - Shorthand: "input.x", "add", "add.out"
  // - Structured: { input = "x"; }, { node = "add"; port = "out"; }

  // Try string shorthand first
  if let Ok(s) = try_string(v) {
    return parse_edge_source_shorthand(&s, ctx);
  }

  // Fall back to structured attrset format
  let items = expect_attrset(v, ctx)?;
  let pairs = attrset_pairs(items, ctx)?;
  ensure_only_keys(&pairs, &["input", "node", "port"], ctx)?;

  let input = get_opt_string(&pairs, "input", &format!("{ctx}.input"))?;
  let node = get_opt_string(&pairs, "node", &format!("{ctx}.node"))?;
  let port = get_opt_string(&pairs, "port", &format!("{ctx}.port"))?;

  match (input, node) {
    (Some(name), None) => {
      if port.is_some() {
        return Err(PnixError::UnsupportedSyntax {
          message: format!("{ctx}: input source must not specify port"),
          span: None,
        });
      }
      Ok(EdgeSource::Input { name })
    }
    (None, Some(node)) => Ok(EdgeSource::Node { node, port }),
    (Some(_), Some(_)) => Err(PnixError::UnsupportedSyntax {
      message: format!("{ctx}: expected exactly one of `input` or `node`"),
      span: None,
    }),
    (None, None) => Err(PnixError::UnsupportedSyntax {
      message: format!("{ctx}: expected `input` or `node`"),
      span: None,
    }),
  }
}

/// Parse shorthand edge source: "input.x" -> Input, "add" or "add.out" -> Node
fn parse_edge_source_shorthand(s: &str, ctx: &str) -> Result<EdgeSource, PnixError> {
  if let Some(rest) = s.strip_prefix("input.") {
    // "input.x" -> EdgeSource::Input { name: "x" }
    if rest.is_empty() {
      return Err(PnixError::UnsupportedSyntax {
        message: format!("{ctx}: invalid input reference, expected 'input.<name>'"),
        span: None,
      });
    }
    Ok(EdgeSource::Input {
      name: rest.to_string(),
    })
  } else if s.contains('.') {
    // "node.port" or "node.name.port" -> EdgeSource::Node { node, port }
    // Y07e: Use rfind to split on the last '.' to handle node names with dots
    if let Some(dot_pos) = s.rfind('.') {
      let node = s[..dot_pos].to_string();
      let port = s[dot_pos + 1..].to_string();
      Ok(EdgeSource::Node {
        node,
        port: Some(port),
      })
    } else {
      // Should not happen if contains('.') is true, but handle gracefully
      Ok(EdgeSource::Node {
        node: s.to_string(),
        port: None,
      })
    }
  } else {
    // "node" -> EdgeSource::Node { node, port: None }
    Ok(EdgeSource::Node {
      node: s.to_string(),
      port: None,
    })
  }
}

fn parse_edge_target(v: &PnixExpr, ctx: &str) -> Result<EdgeTarget, PnixError> {
  // Support both shorthand string and structured attrset formats:
  // - Shorthand: "add", "add.in"
  // - Structured: { node = "add"; port = "in"; }

  // Try string shorthand first
  if let Ok(s) = try_string(v) {
    return parse_edge_target_shorthand(&s);
  }

  // Fall back to structured attrset format
  let items = expect_attrset(v, ctx)?;
  let pairs = attrset_pairs(items, ctx)?;
  ensure_only_keys(&pairs, &["node", "port"], ctx)?;
  let node = expect_string(
    get(&pairs, "node").ok_or_else(|| missing(&format!("{ctx}.node")))?,
    &format!("{ctx}.node"),
  )?;
  let port = get_opt_string(&pairs, "port", &format!("{ctx}.port"))?;
  Ok(EdgeTarget { node, port })
}

/// Parse shorthand edge target: "add" or "add.in" or "node.name.port" -> EdgeTarget
fn parse_edge_target_shorthand(s: &str) -> Result<EdgeTarget, PnixError> {
  if s.contains('.') {
    // Y07e: Use rfind to split on the last '.' to handle node names with dots
    // "node.port" -> EdgeTarget { node: "node", port: Some("port") }
    // "node.name.port" -> EdgeTarget { node: "node.name", port: Some("port") }
    if let Some(dot_pos) = s.rfind('.') {
      let node = s[..dot_pos].to_string();
      let port = s[dot_pos + 1..].to_string();
      Ok(EdgeTarget {
        node,
        port: Some(port),
      })
    } else {
      // Should not happen if contains('.') is true, but handle gracefully
      Ok(EdgeTarget {
        node: s.to_string(),
        port: None,
      })
    }
  } else {
    // "node" -> EdgeTarget { node, port: None }
    Ok(EdgeTarget {
      node: s.to_string(),
      port: None,
    })
  }
}

fn parse_edge_cond(v: &PnixExpr, ctx: &str) -> Result<EdgeCondAst, PnixError> {
  let items = expect_attrset(v, ctx)?;

  // Collect values for each key (allowing duplicates for when/unless)
  let mut whens: Vec<String> = Vec::new();
  let mut unlesses: Vec<String> = Vec::new();
  let mut onfails: Vec<String> = Vec::new();
  let mut all_whens: Vec<String> = Vec::new();
  let mut all_unlesses: Vec<String> = Vec::new();

  for it in items {
    match it {
      PnixAttrItem::Assign {
        key_path, value, ..
      } => {
        // For edge conditions, we expect simple single-key paths
        let key = key_path.join(".");
        ensure_data_only(value, &format!("{ctx}.{key}"))?;
        match key.as_str() {
          "when" => whens.push(expect_string(value, &format!("{ctx}.when"))?),
          "unless" => unlesses.push(expect_string(value, &format!("{ctx}.unless"))?),
          "onfail" => onfails.push(expect_string(value, &format!("{ctx}.onfail"))?),
          "all_when" => {
            all_whens.extend(expect_string_list(value, &format!("{ctx}.all_when"))?);
          }
          "all_unless" => {
            all_unlesses.extend(expect_string_list(value, &format!("{ctx}.all_unless"))?);
          }
          _ => {
            return Err(PnixError::UnsupportedSyntax {
              message: format!(
                "{ctx}: unknown key `{key}`, expected one of when/unless/onfail/all_when/all_unless"
              ),
              span: None,
            });
          }
        }
      }
      PnixAttrItem::Inherit { .. } => {
        return Err(PnixError::UnsupportedSyntax {
          message: format!("{ctx}: `inherit` is not allowed in edge condition"),
          span: None,
        });
      }
      PnixAttrItem::DynamicAssign { .. } => {
        return Err(PnixError::UnsupportedSyntax {
          message: format!("{ctx}: dynamic attribute keys are not allowed in edge condition"),
          span: None,
        });
      }
    }
  }

  // Multiple when = AllWhen
  if whens.len() > 1 {
    if !unlesses.is_empty()
      || !onfails.is_empty()
      || !all_whens.is_empty()
      || !all_unlesses.is_empty()
    {
      return Err(PnixError::UnsupportedSyntax {
        message: format!("{ctx}: multiple when keys cannot be combined with other conditions"),
        span: None,
      });
    }
    return Ok(EdgeCondAst::AllWhen(whens));
  }

  // Multiple unless = AllUnless
  if unlesses.len() > 1 {
    if !whens.is_empty() || !onfails.is_empty() || !all_whens.is_empty() || !all_unlesses.is_empty()
    {
      return Err(PnixError::UnsupportedSyntax {
        message: format!("{ctx}: multiple unless keys cannot be combined with other conditions"),
        span: None,
      });
    }
    return Ok(EdgeCondAst::AllUnless(unlesses));
  }

  // Explicit all_when list
  if !all_whens.is_empty() {
    if !whens.is_empty() || !unlesses.is_empty() || !onfails.is_empty() || !all_unlesses.is_empty()
    {
      return Err(PnixError::UnsupportedSyntax {
        message: format!("{ctx}: all_when cannot be combined with other conditions"),
        span: None,
      });
    }
    return Ok(EdgeCondAst::AllWhen(all_whens));
  }

  // Explicit all_unless list
  if !all_unlesses.is_empty() {
    if !whens.is_empty() || !unlesses.is_empty() || !onfails.is_empty() {
      return Err(PnixError::UnsupportedSyntax {
        message: format!("{ctx}: all_unless cannot be combined with other conditions"),
        span: None,
      });
    }
    return Ok(EdgeCondAst::AllUnless(all_unlesses));
  }

  // Compound: when + unless (both present, single each)
  if whens.len() == 1 && unlesses.len() == 1 {
    if !onfails.is_empty() {
      return Err(PnixError::UnsupportedSyntax {
        message: format!("{ctx}: when+unless cannot be combined with onfail"),
        span: None,
      });
    }
    return Ok(EdgeCondAst::WhenUnless {
      when: whens.remove(0),
      unless: unlesses.remove(0),
    });
  }

  // Simple cases: exactly one of when/unless/onfail
  if whens.len() == 1 && unlesses.is_empty() && onfails.is_empty() {
    return Ok(EdgeCondAst::When(whens.remove(0)));
  }
  if unlesses.len() == 1 && whens.is_empty() && onfails.is_empty() {
    return Ok(EdgeCondAst::Unless(unlesses.remove(0)));
  }
  if onfails.len() == 1 && whens.is_empty() && unlesses.is_empty() {
    return Ok(EdgeCondAst::OnFail(onfails.remove(0)));
  }

  // Multiple onfails is an error
  if onfails.len() > 1 {
    return Err(PnixError::UnsupportedSyntax {
      message: format!("{ctx}: multiple onfail keys are not allowed"),
      span: None,
    });
  }

  // Empty or invalid combination
  if whens.is_empty() && unlesses.is_empty() && onfails.is_empty() {
    return Err(PnixError::UnsupportedSyntax {
      message: format!("{ctx}: expected at least one of when/unless/onfail"),
      span: None,
    });
  }

  Err(PnixError::UnsupportedSyntax {
    message: format!("{ctx}: invalid condition combination"),
    span: None,
  })
}

fn parse_scopes(v: &PnixExpr, span: &Span) -> Result<Vec<AstItem>, PnixError> {
  let elements = expect_list(v, "module.scopes")?;
  let mut out = Vec::new();
  for (idx, el) in elements.iter().enumerate() {
    let el_items = expect_attrset(el, &format!("module.scopes[{}]", idx))?;
    let pairs = attrset_pairs(el_items, &format!("module.scopes[{}]", idx))?;
    ensure_only_keys(
      &pairs,
      &["name", "policy", "meta"],
      &format!("module.scopes[{}]", idx),
    )?;

    let name = expect_string(
      get(&pairs, "name").ok_or_else(|| missing("module.scopes[].name"))?,
      &format!("module.scopes[{}].name", idx),
    )?;
    let policy = expect_string(
      get(&pairs, "policy").ok_or_else(|| missing("module.scopes[].policy"))?,
      &format!("module.scopes[{}].policy", idx),
    )?;
    out.push(AstItem::ScopeDecl {
      name,
      policy,
      span: span.clone(),
    });
  }
  Ok(out)
}

/// Parse adt_types: [ { name, params, variants: [ { name, fields } ... ] } ... ]
fn parse_adt_types(v: &PnixExpr, span: &Span) -> Result<Vec<AstItem>, PnixError> {
  let elements = expect_list(v, "module.adt_types")?;
  let mut out = Vec::new();
  for (idx, el) in elements.iter().enumerate() {
    let el_items = expect_attrset(el, &format!("module.adt_types[{}]", idx))?;
    let pairs = attrset_pairs(el_items, &format!("module.adt_types[{}]", idx))?;
    ensure_only_keys(
      &pairs,
      &["name", "params", "variants", "meta"],
      &format!("module.adt_types[{}]", idx),
    )?;

    let name = expect_string(
      get(&pairs, "name").ok_or_else(|| missing("module.adt_types[].name"))?,
      &format!("module.adt_types[{}].name", idx),
    )?;

    // params is optional, defaults to empty
    let params = match get(&pairs, "params") {
      Some(v) => expect_string_list(v, &format!("module.adt_types[{}].params", idx))?,
      None => vec![],
    };

    // variants is required
    let variants_expr =
      get(&pairs, "variants").ok_or_else(|| missing("module.adt_types[].variants"))?;
    let variants = parse_adt_variants(
      variants_expr,
      &format!("module.adt_types[{}].variants", idx),
    )?;

    out.push(AstItem::AdtTypeDecl(AdtTypeDecl {
      name,
      params,
      variants,
      span: span.clone(),
    }));
  }
  Ok(out)
}

/// Parse variants: [ { name, fields } ... ]
fn parse_adt_variants(v: &PnixExpr, ctx: &str) -> Result<Vec<AdtVariant>, PnixError> {
  let elements = expect_list(v, ctx)?;
  let mut out = Vec::new();
  for (idx, el) in elements.iter().enumerate() {
    let el_items = expect_attrset(el, &format!("{ctx}[{idx}]"))?;
    let pairs = attrset_pairs(el_items, &format!("{ctx}[{idx}]"))?;
    ensure_only_keys(
      &pairs,
      &["name", "fields", "meta"],
      &format!("{ctx}[{idx}]"),
    )?;

    let name = expect_string(
      get(&pairs, "name").ok_or_else(|| missing(&format!("{ctx}[{idx}].name")))?,
      &format!("{ctx}[{idx}].name"),
    )?;

    // fields is optional, defaults to empty
    let fields = match get(&pairs, "fields") {
      Some(v) => expect_string_list(v, &format!("{ctx}[{idx}].fields"))?,
      None => vec![],
    };

    out.push(AdtVariant { name, fields });
  }
  Ok(out)
}

fn attrset_pairs(items: &[PnixAttrItem], ctx: &str) -> Result<Vec<(String, PnixExpr)>, PnixError> {
  let mut out: Vec<(String, PnixExpr)> = Vec::new();
  for it in items {
    match it {
      PnixAttrItem::Assign {
        key_path, value, ..
      } => {
        let key = key_path.join(".");
        if out.iter().any(|(k, _)| k == &key) {
          return Err(PnixError::UnsupportedSyntax {
            message: format!("{ctx}: duplicate key `{}`", key),
            span: None,
          });
        }
        ensure_data_only(value, &format!("{ctx}.{}", key))?;
        out.push((key, (**value).clone()));
      }
      PnixAttrItem::Inherit { .. } => {
        return Err(PnixError::UnsupportedSyntax {
          message: format!("{ctx}: `inherit` is not allowed in module attrsets"),
          span: None,
        });
      }
      PnixAttrItem::DynamicAssign { .. } => {
        return Err(PnixError::UnsupportedSyntax {
          message: format!("{ctx}: dynamic attribute keys are not allowed in module attrsets"),
          span: None,
        });
      }
    }
  }
  Ok(out)
}

fn ensure_only_keys(
  pairs: &[(String, PnixExpr)],
  allowed: &[&str],
  ctx: &str,
) -> Result<(), PnixError> {
  for (k, _) in pairs {
    if k.starts_with('_') {
      continue;
    }
    if k == "meta" {
      continue;
    }
    if !allowed.contains(&k.as_str()) {
      return Err(PnixError::UnsupportedSyntax {
        message: format!(
          "{ctx}: unknown key `{}` (allowed: {})",
          k,
          allowed.join(", ")
        ),
        span: None,
      });
    }
  }
  Ok(())
}

fn ensure_data_only(v: &PnixExpr, ctx: &str) -> Result<(), PnixError> {
  match v {
    PnixExpr::Int(_)
    | PnixExpr::Float(_)
    | PnixExpr::Bool(_)
    | PnixExpr::Null
    | PnixExpr::String(_)
    | PnixExpr::Path(_) => Ok(()),
    PnixExpr::List(xs) => {
      for (i, x) in xs.iter().enumerate() {
        ensure_data_only(x, &format!("{ctx}[{i}]"))?;
      }
      Ok(())
    }
    PnixExpr::AttrSet { items, .. } => {
      for it in items {
        match it {
          PnixAttrItem::Assign {
            key_path, value, ..
          } => {
            let key = key_path.join(".");
            ensure_data_only(value, &format!("{ctx}.{}", key))?;
          }
          PnixAttrItem::Inherit { .. } => {
            return Err(PnixError::UnsupportedSyntax {
              message: format!("{ctx}: `inherit` is not allowed in module attrsets"),
              span: None,
            });
          }
          PnixAttrItem::DynamicAssign { .. } => {
            return Err(PnixError::UnsupportedSyntax {
              message: format!("{ctx}: dynamic attribute keys are not allowed in module attrsets"),
              span: None,
            });
          }
        }
      }
      Ok(())
    }
    PnixExpr::Match { .. } => Err(PnixError::UnsupportedSyntax {
      message: format!("{ctx}: match expressions are not supported in data-only context"),
      span: None,
    }),
    other => Err(PnixError::UnsupportedSyntax {
      message: format!(
        "{ctx}: non-data Pnix expression is not allowed in modules (got {})",
        kind(other)
      ),
      span: None,
    }),
  }
}

fn expect_attrset<'a>(v: &'a PnixExpr, ctx: &str) -> Result<&'a Vec<PnixAttrItem>, PnixError> {
  match v {
    PnixExpr::AttrSet { items, .. } => Ok(items),
    other => Err(PnixError::UnsupportedSyntax {
      message: format!("{ctx}: expected attrset, got {}", kind(other)),
      span: None,
    }),
  }
}

fn expect_list<'a>(v: &'a PnixExpr, ctx: &str) -> Result<&'a [PnixExpr], PnixError> {
  match v {
    PnixExpr::List(xs) => Ok(xs),
    other => Err(PnixError::UnsupportedSyntax {
      message: format!("{ctx}: expected list, got {}", kind(other)),
      span: None,
    }),
  }
}

fn expect_string(v: &PnixExpr, ctx: &str) -> Result<String, PnixError> {
  match v {
    PnixExpr::String(s) => Ok(s.clone()),
    other => Err(PnixError::UnsupportedSyntax {
      message: format!("{ctx}: expected string, got {}", kind(other)),
      span: None,
    }),
  }
}

/// Try to extract a string; returns Err if not a string (for fallback parsing)
fn try_string(v: &PnixExpr) -> Result<String, ()> {
  match v {
    PnixExpr::String(s) => Ok(s.clone()),
    _ => Err(()),
  }
}

fn expect_string_list(v: &PnixExpr, ctx: &str) -> Result<Vec<String>, PnixError> {
  let xs = expect_list(v, ctx)?;
  let mut out = Vec::new();
  for (i, el) in xs.iter().enumerate() {
    out.push(expect_string(el, &format!("{ctx}[{i}]"))?);
  }
  Ok(out)
}

fn get<'a>(pairs: &'a [(String, PnixExpr)], key: &str) -> Option<&'a PnixExpr> {
  pairs.iter().find_map(|(k, v)| (k == key).then_some(v))
}

fn get_opt_string(
  pairs: &[(String, PnixExpr)],
  key: &str,
  ctx: &str,
) -> Result<Option<String>, PnixError> {
  match get(pairs, key) {
    None => Ok(None),
    Some(v) => Ok(Some(expect_string(v, ctx)?)),
  }
}

fn get_opt_int(
  pairs: &[(String, PnixExpr)],
  key: &str,
  ctx: &str,
) -> Result<Option<i32>, PnixError> {
  match get(pairs, key) {
    None => Ok(None),
    Some(PnixExpr::Int(v)) => {
      i32::try_from(*v)
        .map(Some)
        .map_err(|_| PnixError::UnsupportedSyntax {
          message: format!("{ctx}: priority out of i32 range"),
          span: None,
        })
    }
    Some(other) => Err(PnixError::UnsupportedSyntax {
      message: format!("{ctx}: expected int, got {}", kind(other)),
      span: None,
    }),
  }
}

fn get_bool(pairs: &[(String, PnixExpr)], key: &str) -> Result<Option<bool>, PnixError> {
  match get(pairs, key) {
    None => Ok(None),
    Some(PnixExpr::Bool(v)) => Ok(Some(*v)),
    Some(other) => Err(PnixError::UnsupportedSyntax {
      message: format!("expected bool for `{key}`, got {}", kind(other)),
      span: None,
    }),
  }
}

fn missing(ctx: &str) -> PnixError {
  PnixError::UnsupportedSyntax {
    message: format!("missing required field: {}", ctx),
    span: None,
  }
}

fn kind(v: &PnixExpr) -> &'static str {
  match v {
    PnixExpr::Int(_) => "int",
    PnixExpr::Float(_) => "float",
    PnixExpr::Bool(_) => "bool",
    PnixExpr::Null => "null",
    PnixExpr::String(_) => "string",
    PnixExpr::StringInterp(_) => "stringInterp",
    PnixExpr::Var(_) => "var",
    PnixExpr::Let { .. } => "let",
    PnixExpr::If { .. } => "if",
    PnixExpr::Lambda { .. } => "lambda",
    PnixExpr::Apply { .. } => "apply",
    PnixExpr::AttrSet { .. } => "attrset",
    PnixExpr::List(_) => "list",
    PnixExpr::Select { .. } => "select",
    PnixExpr::SelectOrDefault { .. } => "selectOrDefault",
    PnixExpr::Binary { .. } => "binary",
    PnixExpr::Unary { .. } => "unary",
    PnixExpr::Construct { .. } => "construct",
    PnixExpr::Match { .. } => "match",
    PnixExpr::Import { .. } => "import",
    PnixExpr::With { .. } => "with",
    PnixExpr::Path(_) => "path",
    PnixExpr::Assert { .. } => "assert",
    PnixExpr::Index { .. } => "index",
    PnixExpr::HasAttr { .. } => "hasAttr",
    PnixExpr::DynamicHasAttr { .. } => "dynamicHasAttr",
    PnixExpr::DynamicSelect { .. } => "dynamicSelect",
    PnixExpr::DynamicSelectOrDefault { .. } => "dynamicSelectOrDefault",
  }
}
