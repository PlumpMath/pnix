//! AST: Language-specific Abstract Syntax Tree

mod types;
pub use types::*;

use crate::diagnostics::{Diagnostics, Span};

/// 소스 텍스트를 AST로 파싱
///
/// Stage-2 문법:
/// ```text
/// type <TypeName>
/// input <Name> : <Type>                                      # Stage-2 입력
/// extern <backend>.<name> : <In> -> <Out>                    # Stage-1 단순
/// extern <backend>.<name> : (p: T, ...) -> (p: T, ...)       # Stage-2 포트
/// node <NodeName> uses <backend>.<name>
/// edge <From> -> <To>                                        # Stage-1 단순
/// edge <From>.<port> -> <To>.<port>                          # Stage-2 포트
/// edge input.<Name> -> <Node>.<port>                         # Stage-2 입력 연결
/// ```
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_module(text: &str, name: &str, diags: &mut Diagnostics) -> Result<AstModule, String> {
  let mut items = Vec::new();

  for (lineno, raw) in text.lines().enumerate() {
    let line = raw.trim();

    // 빈 줄 / 주석(# 또는 //) 무시
    if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
      continue;
    }

    // type <TypeName>
    if let Some(rest) = line.strip_prefix("type ") {
      let t = rest.trim();
      if t.is_empty() || !is_type_name(t) {
        diags.push(
          format!("invalid type declaration: {}", raw),
          Some(span_for_line(text, lineno)),
        );
        continue;
      }
      items.push(AstItem::TypeDecl {
        name: t.to_string(),
        span: span_for_line(text, lineno),
      });
      continue;
    }

    // input <Name> : <Type>
    if let Some(rest) = line.strip_prefix("input ") {
      if let Some(item) = parse_input(rest, text, lineno, diags) {
        items.push(item);
      }
      continue;
    }

    // extern <name> : <sig>
    if let Some(rest) = line.strip_prefix("extern ") {
      if let Some(item) = parse_extern(rest, text, lineno, diags) {
        items.push(item);
      }
      continue;
    }

    // node <NodeName> uses <ExternName> [gate] [optional] [scope S] [cost C] [priority P]
    if let Some(rest) = line.strip_prefix("node ") {
      let parts: Vec<&str> = rest.split_whitespace().collect();
      if parts.len() < 3 || parts[1] != "uses" {
        diags.push(
          format!(
            "invalid node syntax (expected `node <Name> uses <Extern> [modifiers]`): {}",
            raw
          ),
          Some(span_for_line(text, lineno)),
        );
        continue;
      }

      let node_name = parts[0].trim();
      let uses = parts[2].trim();

      if !is_ident_like(node_name) {
        diags.push(
          format!("invalid node name `{}`", node_name),
          Some(span_for_line(text, lineno)),
        );
        continue;
      }
      if !is_ident_like(uses) {
        diags.push(
          format!("invalid uses target `{}`", uses),
          Some(span_for_line(text, lineno)),
        );
        continue;
      }

      // Parse modifiers
      let mut kind: Option<String> = None;
      let mut optional = false;
      let mut scope: Option<String> = None;
      let mut cost: Option<String> = None;
      let mut priority: Option<i32> = None;

      let mut i = 3;
      while i < parts.len() {
        match parts[i] {
          "gate" => {
            kind = Some("gate".into());
            i += 1;
          }
          "optional" => {
            optional = true;
            i += 1;
          }
          "scope" => {
            if i + 1 >= parts.len() {
              diags.push("missing scope name", Some(span_for_line(text, lineno)));
              break;
            }
            scope = Some(parts[i + 1].to_string());
            i += 2;
          }
          "cost" => {
            if i + 1 >= parts.len() {
              diags.push("missing cost value", Some(span_for_line(text, lineno)));
              break;
            }
            cost = Some(parts[i + 1].to_string());
            i += 2;
          }
          "priority" => {
            if i + 1 >= parts.len() {
              diags.push("missing priority value", Some(span_for_line(text, lineno)));
              break;
            }
            priority = parts[i + 1].parse::<i32>().ok();
            i += 2;
          }
          other => {
            diags.push(
              format!("unknown node modifier `{}`", other),
              Some(span_for_line(text, lineno)),
            );
            break;
          }
        }
      }

      items.push(AstItem::NodeDecl {
        name: node_name.to_string(),
        uses: uses.to_string(),
        kind,
        optional,
        scope,
        cost,
        priority,
        span: span_for_line(text, lineno),
      });
      continue;
    }

    // test <Name> = <Expr> (Y11a)
    if let Some(rest) = line.strip_prefix("test ") {
      let parts: Vec<&str> = rest.splitn(2, '=').collect();
      if parts.len() != 2 {
        diags.push(
          format!(
            "invalid test syntax (expected `test <Name> = <Expr>`): {}",
            raw
          ),
          Some(span_for_line(text, lineno)),
        );
        continue;
      }

      let test_name = parts[0].trim();
      let test_expr = parts[1].trim();

      if test_name.is_empty() || !is_ident_like(test_name) {
        diags.push(
          format!("invalid test name `{}`", test_name),
          Some(span_for_line(text, lineno)),
        );
        continue;
      }

      if test_expr.is_empty() {
        diags.push(
          format!("invalid test expression: {}", rest),
          Some(span_for_line(text, lineno)),
        );
        continue;
      }

      items.push(AstItem::TestDecl {
        name: test_name.to_string(),
        expr: test_expr.to_string(),
        span: span_for_line(text, lineno),
      });
      continue;
    }

    // @test node <Name> uses <Extern> (Y11a: 어노테이션 형태)
    if let Some(rest) = line.strip_prefix("@test ") {
      // node 선언과 동일하게 파싱하되, test 플래그 추가
      if let Some(node_rest) = rest.strip_prefix("node ") {
        let parts: Vec<&str> = node_rest.split_whitespace().collect();
        if parts.len() < 3 || parts[1] != "uses" {
          diags.push(
            format!(
              "invalid @test node syntax (expected `@test node <Name> uses <Extern>`): {}",
              raw
            ),
            Some(span_for_line(text, lineno)),
          );
          continue;
        }

        let node_name = parts[0].trim();
        let uses = parts[2].trim();

        if !is_ident_like(node_name) || !is_ident_like(uses) {
          diags.push(
            format!("invalid @test node name or uses target: {}", raw),
            Some(span_for_line(text, lineno)),
          );
          continue;
        }

        // NodeDecl에 test 플래그 추가 (기존 NodeDecl 재사용)
        items.push(AstItem::NodeDecl {
          name: node_name.to_string(),
          uses: uses.to_string(),
          kind: Some("test".into()), // test 플래그로 사용
          optional: false,
          scope: None,
          cost: None,
          priority: None,
          span: span_for_line(text, lineno),
        });
        continue;
      }

      // @test <Expr> 형태도 지원
      items.push(AstItem::TestDecl {
        name: format!("test_{}", items.len()), // 자동 생성 이름
        expr: rest.trim().to_string(),
        span: span_for_line(text, lineno),
      });
      continue;
    }

    // scope <Name> policy <Policy>
    if let Some(rest) = line.strip_prefix("scope ") {
      let parts: Vec<&str> = rest.split_whitespace().collect();
      if parts.len() < 3 || parts[1] != "policy" {
        diags.push(
          format!(
            "invalid scope syntax (expected `scope <Name> policy <Policy>`): {}",
            raw
          ),
          Some(span_for_line(text, lineno)),
        );
        continue;
      }

      let scope_name = parts[0].trim();
      let policy = parts[2].trim();

      if !is_ident_simple(scope_name) {
        diags.push(
          format!("invalid scope name `{}`", scope_name),
          Some(span_for_line(text, lineno)),
        );
        continue;
      }

      items.push(AstItem::ScopeDecl {
        name: scope_name.to_string(),
        policy: policy.to_string(),
        span: span_for_line(text, lineno),
      });
      continue;
    }

    // edge <From>[.port] -> <To>[.port]
    if let Some(rest) = line.strip_prefix("edge ") {
      if let Some(item) = parse_edge(rest, text, lineno, diags) {
        items.push(item);
      }
      continue;
    }

    diags.push(
      format!("unknown top-level form: {}", raw),
      Some(span_for_line(text, lineno)),
    );
  }

  Ok(AstModule {
    name: name.into(),
    items,
  })
}

/// Parse input declaration: <Name> : <Type>
fn parse_input(rest: &str, text: &str, lineno: usize, diags: &mut Diagnostics) -> Option<AstItem> {
  let parts: Vec<&str> = rest.splitn(2, ':').collect();
  if parts.len() != 2 {
    diags.push(
      format!(
        "invalid input syntax (expected `input <Name> : <Type>`): {}",
        rest
      ),
      Some(span_for_line(text, lineno)),
    );
    return None;
  }

  let input_name = parts[0].trim();
  let input_type = parts[1].trim();

  if input_name.is_empty() || !is_ident_simple(input_name) {
    diags.push(
      format!("invalid input name `{}`", input_name),
      Some(span_for_line(text, lineno)),
    );
    return None;
  }

  if input_type.is_empty() {
    diags.push(
      format!("invalid input type: {}", rest),
      Some(span_for_line(text, lineno)),
    );
    return None;
  }

  Some(AstItem::InputDecl {
    name: input_name.to_string(),
    ty: input_type.to_string(),
    span: span_for_line(text, lineno),
  })
}

/// Parse extern declaration
fn parse_extern(rest: &str, text: &str, lineno: usize, diags: &mut Diagnostics) -> Option<AstItem> {
  let parts: Vec<&str> = rest.splitn(2, ':').collect();
  if parts.len() != 2 {
    diags.push(
      format!(
        "invalid extern syntax (expected `extern <name> : <sig>`): {}",
        rest
      ),
      Some(span_for_line(text, lineno)),
    );
    return None;
  }

  let ext_name = parts[0].trim();
  let sig_raw = parts[1].trim();

  if ext_name.is_empty() || !is_ident_like(ext_name) {
    diags.push(
      format!("invalid extern name `{}`", ext_name),
      Some(span_for_line(text, lineno)),
    );
    return None;
  }

  // Check if ported syntax: (port: Type, ...) -> (port: Type, ...)
  let sig = if sig_raw.starts_with('(') {
    parse_ported_sig(sig_raw, text, lineno, diags)?
  } else {
    // Stage-1 simple syntax: Type -> Type
    let sig_parts: Vec<&str> = sig_raw.split("->").collect();
    if sig_parts.len() != 2 {
      diags.push(
        format!("invalid signature (expected `<In> -> <Out>`): {}", sig_raw),
        Some(span_for_line(text, lineno)),
      );
      return None;
    }

    let input = sig_parts[0].trim();
    let output = sig_parts[1].trim();

    if input.is_empty() || output.is_empty() {
      diags.push(
        format!("invalid signature types: {}", sig_raw),
        Some(span_for_line(text, lineno)),
      );
      return None;
    }

    SigAst::simple(input.to_string(), output.to_string())
  };

  Some(AstItem::ExternDecl {
    name: ext_name.to_string(),
    sig,
    span: span_for_line(text, lineno),
  })
}

/// Parse ported signature: (port: Type, ...) -> (port: Type, ...)
fn parse_ported_sig(
  sig_raw: &str,
  text: &str,
  lineno: usize,
  diags: &mut Diagnostics,
) -> Option<SigAst> {
  let arrow_parts: Vec<&str> = sig_raw.split("->").collect();
  if arrow_parts.len() != 2 {
    diags.push(
      format!(
        "invalid ported signature (expected `(...) -> (...)`): {}",
        sig_raw
      ),
      Some(span_for_line(text, lineno)),
    );
    return None;
  }

  let inputs = parse_port_list(arrow_parts[0].trim(), text, lineno, diags)?;
  let outputs = parse_port_list(arrow_parts[1].trim(), text, lineno, diags)?;

  Some(SigAst::ported(inputs, outputs))
}

/// Parse port list: (port: Type, port2: Type2)
fn parse_port_list(
  s: &str,
  text: &str,
  lineno: usize,
  diags: &mut Diagnostics,
) -> Option<Vec<PortAst>> {
  let s = s.trim();
  if !s.starts_with('(') || !s.ends_with(')') {
    diags.push(
      format!("port list must be enclosed in parentheses: {}", s),
      Some(span_for_line(text, lineno)),
    );
    return None;
  }

  let inner = &s[1..s.len() - 1];
  let mut ports = Vec::new();

  for part in inner.split(',') {
    let part = part.trim();
    if part.is_empty() {
      continue;
    }

    let colon_parts: Vec<&str> = part.splitn(2, ':').collect();
    if colon_parts.len() != 2 {
      diags.push(
        format!("invalid port declaration (expected `name: Type`): {}", part),
        Some(span_for_line(text, lineno)),
      );
      return None;
    }

    let port_name = colon_parts[0].trim();
    let port_type = colon_parts[1].trim();

    if port_name.is_empty() || port_type.is_empty() {
      diags.push(
        format!("empty port name or type: {}", part),
        Some(span_for_line(text, lineno)),
      );
      return None;
    }

    ports.push(PortAst {
      name: port_name.to_string(),
      ty: port_type.to_string(),
    });
  }

  Some(ports)
}

/// Parse edge declaration: From[.port] -> To[.port] [when G] [unless G] [onfail N]
fn parse_edge(rest: &str, text: &str, lineno: usize, diags: &mut Diagnostics) -> Option<AstItem> {
  let parts: Vec<&str> = rest.split("->").collect();
  if parts.len() != 2 {
    diags.push(
      format!(
        "invalid edge syntax (expected `edge <A> -> <B> [cond]`): {}",
        rest
      ),
      Some(span_for_line(text, lineno)),
    );
    return None;
  }

  let from_str = parts[0].trim();
  let to_and_cond = parts[1].trim();

  // Split target and condition
  let to_parts: Vec<&str> = to_and_cond.split_whitespace().collect();
  let to_str = to_parts.first().copied().unwrap_or("");

  // Parse source (from): input.X or node.port
  let from = parse_edge_source(from_str, text, lineno, diags)?;

  // Parse target (to): always node.port
  let to = parse_edge_target(to_str, text, lineno, diags)?;

  // Parse condition (when/unless/onfail)
  let cond = if to_parts.len() >= 3 {
    let cond_type = to_parts[1];
    let cond_ref = to_parts[2];
    match cond_type {
      "when" => Some(EdgeCondAst::When(cond_ref.to_string())),
      "unless" => Some(EdgeCondAst::Unless(cond_ref.to_string())),
      "onfail" => Some(EdgeCondAst::OnFail(cond_ref.to_string())),
      _ => {
        diags.push(
          format!(
            "unknown edge condition `{}` (expected when/unless/onfail)",
            cond_type
          ),
          Some(span_for_line(text, lineno)),
        );
        None
      }
    }
  } else {
    None
  };

  Some(AstItem::EdgeDecl {
    from,
    to,
    cond,
    span: span_for_line(text, lineno),
  })
}

/// Parse edge source: "input.X" or "node" or "node.port"
fn parse_edge_source(
  s: &str,
  text: &str,
  lineno: usize,
  diags: &mut Diagnostics,
) -> Option<EdgeSource> {
  // Check for input.X pattern
  if let Some(input_name) = s.strip_prefix("input.") {
    if input_name.is_empty() || !is_ident_simple(input_name) {
      diags.push(
        format!("invalid input reference `{}`", s),
        Some(span_for_line(text, lineno)),
      );
      return None;
    }
    return Some(EdgeSource::Input {
      name: input_name.to_string(),
    });
  }

  // Otherwise it's node.port
  let (node, port) = if let Some(dot_pos) = s.find('.') {
    (s[..dot_pos].to_string(), Some(s[dot_pos + 1..].to_string()))
  } else {
    (s.to_string(), None)
  };

  if !is_ident_simple(&node) {
    diags.push(
      format!("invalid node name `{}`", node),
      Some(span_for_line(text, lineno)),
    );
    return None;
  }

  if let Some(ref p) = port {
    if !is_ident_simple(p) {
      diags.push(
        format!("invalid port name `{}`", p),
        Some(span_for_line(text, lineno)),
      );
      return None;
    }
  }

  Some(EdgeSource::Node { node, port })
}

/// Parse edge target: "node" or "node.port"
fn parse_edge_target(
  s: &str,
  text: &str,
  lineno: usize,
  diags: &mut Diagnostics,
) -> Option<EdgeTarget> {
  let (node, port) = if let Some(dot_pos) = s.find('.') {
    (s[..dot_pos].to_string(), Some(s[dot_pos + 1..].to_string()))
  } else {
    (s.to_string(), None)
  };

  if !is_ident_simple(&node) {
    diags.push(
      format!("invalid target node name `{}`", node),
      Some(span_for_line(text, lineno)),
    );
    return None;
  }

  if let Some(ref p) = port {
    if !is_ident_simple(p) {
      diags.push(
        format!("invalid target port name `{}`", p),
        Some(span_for_line(text, lineno)),
      );
      return None;
    }
  }

  Some(EdgeTarget { node, port })
}

fn is_ident_like(s: &str) -> bool {
  s.chars()
    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn is_ident_simple(s: &str) -> bool {
  !s.is_empty()
    && s
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn is_type_name(s: &str) -> bool {
  let mut it = s.chars();
  match it.next() {
    Some(c) if c.is_ascii_uppercase() => it.all(|c| c.is_ascii_alphanumeric() || c == '_'),
    _ => false,
  }
}

fn span_for_line(text: &str, target_line: usize) -> Span {
  let mut start = 0usize;
  for (i, line) in text.lines().enumerate() {
    if i == target_line {
      let end = start + line.len();
      return Span::new(start, end);
    }
    start += line.len() + 1;
  }
  Span::default()
}
