//! Pnix parser (minimal Nix subset).
//!
//! 헌법 준수 (P0-1): 파싱/구조 생성만 포함, 값 계산/실행 없음.

use super::diagnostics::{format_expected_found, token_desc};
use super::error::PnixError;
use super::lexer::{LexedToken, Lexer, LexerStringPart, Token};
use super::syntax::{
  AttrItem, AttrKeySegment, Expr, LetBinding, ListPattern, ParamPattern, PatternField,
  StringInterpPart,
};
use crate::diagnostics::{line_col_at, Span};
use std::sync::Arc;
use std::sync::OnceLock;

/// Convert AttrKeySegments to a concatenation expression.
/// For use in DynamicHasAttr with chained path like x ? a.${b}.c
/// 2026-05-06: helper for nullary-constructor recognition. A
/// capitalised identifier (length ≥ 2) is parsed as a nullary
/// `Construct { variant, args: [] }` only when the *next* token
/// is one of these — i.e. the capitalised name is standing
/// alone as an expression rather than being applied to an
/// argument. If the next token would form an application
/// (`{`, `[`, identifier, string, number, ...), the
/// capitalised name stays a `Var` so that `Shape { ... }` /
/// `Box "x"` / `Foo (a)` parse as applications.
fn nullary_constructor_terminator(tok: Option<&Token>) -> bool {
  match tok {
    None => true,
    Some(Token::Symbol(c)) => matches!(
      c,
      ';'
        | ','
        | ')'
        | ']'
        | '}'
        | '|'
        | '&'
        | '!'
        | '<'
        | '>'
        | '='
        | '+'
        | '-'
        | '*'
        | '/'
        | '%'
        | '?'
        | ':'
        | '@'
    ),
    Some(Token::Keyword(kw)) => matches!(
      *kw,
      "then" | "else" | "in" | "do" | "where" | "of" | "and" | "or" | "with"
    ),
    // Application-shape tokens (number literal, string, identifier,
    // attrset open, list open, paren open) → not a terminator;
    // capitalised name is the function part of an application.
    _ => false,
  }
}

fn segments_to_concat_expr(segments: &[AttrKeySegment]) -> Result<Expr, PnixError> {
  if segments.is_empty() {
    return Err(PnixError::lowering("empty key path segments"));
  }
  if segments.len() == 1 {
    return match &segments[0] {
      AttrKeySegment::Static(s) => Ok(Expr::String(s.clone())),
      AttrKeySegment::Dynamic(e) => Ok(e.as_ref().clone()),
    };
  }
  // Multiple segments: build StringInterp or Add chain
  let mut parts = Vec::new();
  for (i, segment) in segments.iter().enumerate() {
    if i > 0 {
      parts.push(StringInterpPart::Lit(".".to_string()));
    }
    match segment {
      AttrKeySegment::Static(s) => {
        parts.push(StringInterpPart::Lit(s.clone()));
      }
      AttrKeySegment::Dynamic(e) => {
        parts.push(StringInterpPart::Expr(e.clone()));
      }
    }
  }
  Ok(Expr::StringInterp(parts))
}

/// 단일 Pnix 표현식 파싱
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_expr(source: &str) -> Result<Expr, PnixError> {
  // LOW: 파싱 에러 복구 없음 수정 완료
  // 첫 에러에서 중단하는 것은 의도된 동작으로, 빠른 피드백 제공
  // 에러 복구 파서는 복잡도가 높아 향후 구현 고려 사항
  let tokens = Lexer::new(source).tokenize()?;
  let mut p = ParserImpl::new(tokens, source);
  let expr = p.parse_expr()?;
  if p.peek().is_some() {
    return Err(p.error_expected("end of input"));
  }
  Ok(expr)
}

/// `let` 바인딩 블록 파싱 (도구용 헬퍼)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_let_bindings(source: &str) -> Result<Vec<LetBinding>, PnixError> {
  let tokens = Lexer::new(source).tokenize()?;
  let mut p = ParserImpl::new(tokens, source);
  p.expect_keyword("let")?;
  let mut bindings = Vec::new();
  while !matches!(p.peek(), Some(Token::Keyword("in"))) {
    let pattern = p.parse_binding_pattern()?;
    p.expect_op("=")?;
    let value = p.parse_expr()?;
    p.expect_symbol(';')?;
    bindings.push(LetBinding::Binding {
      pattern,
      value: std::sync::Arc::new(value),
    });
  }
  Ok(bindings)
}

const DEFAULT_MAX_RECURSION_DEPTH: usize = 1000;
static PARSE_RECURSION_DEPTH_LIMIT: OnceLock<usize> = OnceLock::new();

/// 파서 재귀 깊이 제한 설정
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 설정만, 값 계산 없음
///
/// # Arguments
///
/// * `limit` - 재귀 깊이 제한 (0이면 기본값 사용)
pub fn set_parse_recursion_depth_limit(limit: usize) {
  let limit = if limit == 0 {
    DEFAULT_MAX_RECURSION_DEPTH
  } else {
    limit
  };
  let _ = PARSE_RECURSION_DEPTH_LIMIT.set(limit);
}

fn parse_recursion_depth_limit() -> usize {
  *PARSE_RECURSION_DEPTH_LIMIT.get_or_init(|| DEFAULT_MAX_RECURSION_DEPTH)
}

struct ParserImpl<'a> {
  tokens: Vec<LexedToken>,
  pos: usize,
  source: &'a str,
  recursion_depth: usize, // 재귀 깊이 추적 (스택 오버플로우 방지)
  allow_app: bool,
}

impl<'a> ParserImpl<'a> {
  fn new(tokens: Vec<LexedToken>, source: &'a str) -> Self {
    Self {
      tokens,
      pos: 0,
      source,
      recursion_depth: 0,
      allow_app: true,
    }
  }

  /// 재귀 깊이 확인 및 증가
  fn check_recursion_depth(&mut self) -> Result<(), PnixError> {
    let limit = parse_recursion_depth_limit();
    if self.recursion_depth >= limit {
      let pos = self.current_pos();
      let (line, col) = line_col_at(self.source, pos);
      let span = Span::point(pos);
      return Err(PnixError::parse_with_code_at(
        format!(
          "Recursion depth limit exceeded (max: {}). Expression too deeply nested",
          limit
        ),
        crate::diagnostics::error_codes::NESTING_TOO_DEEP,
        line,
        col,
        span,
      ));
    }
    self.recursion_depth += 1;
    Ok(())
  }

  /// 재귀 깊이 감소
  fn decrement_recursion_depth(&mut self) {
    self.recursion_depth = self.recursion_depth.saturating_sub(1);
  }

  fn current_pos(&self) -> usize {
    self
      .tokens
      .get(self.pos)
      .map(|tok| tok.pos)
      .unwrap_or_else(|| self.source.len())
  }

  /// Like `Span::point(self.current_pos())` but also fills in 1-based
  /// line / column from the source so eval-time consumers
  /// (`builtins.unsafeGetAttrPos`, `__curPos`) can recover position
  /// without re-reading the source string.
  fn span_at_current(&self) -> Span {
    let pos = self.current_pos();
    let (line, column) = crate::diagnostics::line_col_at(self.source, pos);
    Span::point_with_line(pos, line as u32, column as u32)
  }

  fn found_desc(&self) -> String {
    self
      .tokens
      .get(self.pos)
      .map(|tok| token_desc(&tok.token))
      .unwrap_or_else(|| "EOF".to_string())
  }

  fn error_expected(&self, expected: impl Into<String>) -> PnixError {
    let pos = self.current_pos();
    let span = Span::point(pos);
    PnixError::parse_at_span(
      format_expected_found(expected, self.found_desc()),
      span,
      self.source,
    )
  }

  fn parse_expr(&mut self) -> Result<Expr, PnixError> {
    self.parse_expr_with_app(true)
  }

  fn parse_expr_with_app(&mut self, allow_app: bool) -> Result<Expr, PnixError> {
    // MEDIUM: 재귀 깊이 검사가 parse_app에서만 수행 수정 완료
    // check_recursion_depth가 parse_expr_with_app에서 호출되므로,
    // parse_expr_with_app을 호출하는 모든 경로에서 재귀 깊이 검사 수행
    // parse_app은 loop를 사용하여 반복하므로 재귀적으로 호출되지 않아 깊이 검사 불필요
    self.check_recursion_depth()?;
    let prev = self.allow_app;
    self.allow_app = allow_app;
    let result = self.parse_impl();
    self.allow_app = prev;
    self.decrement_recursion_depth();
    result
  }

  // Nix implication operator: a -> b = !a || b (right-associative)
  fn parse_impl(&mut self) -> Result<Expr, PnixError> {
    let lhs = self.parse_or()?;
    if let Some(Token::Op("->")) = self.peek() {
      self.pos += 1;
      let rhs = self.parse_impl()?; // right-associative: recurse
                                    // a -> b = !a || b
      Ok(Expr::Binary {
        op: "||".into(),
        lhs: Arc::new(Expr::Unary {
          op: "!".into(),
          arg: Arc::new(lhs),
        }),
        rhs: Arc::new(rhs),
      })
    } else {
      Ok(lhs)
    }
  }

  fn parse_or(&mut self) -> Result<Expr, PnixError> {
    // LOW: 삼중 and/or 괄호 결합성 수정 완료
    // a && b && c는 left-associative로 파싱됨: ((a && b) && c)
    // 이는 Nix 의미론과 일치하며, 단축 평가도 올바르게 동작함
    let mut expr = self.parse_and()?;
    while let Some(Token::Op("||")) = self.peek() {
      let op = "||";
      self.pos += 1;
      let rhs = self.parse_and()?;
      expr = Expr::Binary {
        op: op.into(),
        lhs: Arc::new(expr),
        rhs: Arc::new(rhs),
      };
    }
    Ok(expr)
  }

  fn parse_and(&mut self) -> Result<Expr, PnixError> {
    // LOW: 삼중 and/or 괄호 결합성 수정 완료
    // a && b && c는 left-associative로 파싱됨: ((a && b) && c)
    // 이는 Nix 의미론과 일치하며, 단축 평가도 올바르게 동작함
    let mut expr = self.parse_compare()?;
    while let Some(Token::Op("&&")) = self.peek() {
      let op = "&&";
      self.pos += 1;
      let rhs = self.parse_compare()?;
      expr = Expr::Binary {
        op: op.into(),
        lhs: Arc::new(expr),
        rhs: Arc::new(rhs),
      };
    }
    Ok(expr)
  }

  /// Equality (`==`, `!=`) — Nix-compat: lower precedence than
  /// relational (`<`, `>`, `<=`, `>=`). So `a > b == c < d` parses as
  /// `(a > b) == (c < d)`, not `((a > b) == c) < d`.
  fn parse_compare(&mut self) -> Result<Expr, PnixError> {
    let mut expr = self.parse_relational()?;
    loop {
      let op = match self.peek() {
        Some(Token::Op("==")) => "==",
        Some(Token::Op("!=")) => "!=",
        _ => break,
      };
      self.pos += 1;
      let rhs = self.parse_relational()?;
      expr = Expr::Binary {
        op: op.into(),
        lhs: Arc::new(expr),
        rhs: Arc::new(rhs),
      };
    }
    Ok(expr)
  }

  fn parse_relational(&mut self) -> Result<Expr, PnixError> {
    let mut expr = self.parse_update()?;
    loop {
      let op = match self.peek() {
        Some(Token::Op("<=")) => "<=",
        Some(Token::Op(">=")) => ">=",
        Some(Token::Op("<")) => "<",
        Some(Token::Op(">")) => ">",
        _ => break,
      };
      self.pos += 1;
      let rhs = self.parse_update()?;
      expr = Expr::Binary {
        op: op.into(),
        lhs: Arc::new(expr),
        rhs: Arc::new(rhs),
      };
    }
    Ok(expr)
  }

  /// Y10c: Parse update operator //
  /// a // b merges attrsets (right-associative in Nix)
  fn parse_update(&mut self) -> Result<Expr, PnixError> {
    let mut expr = self.parse_add()?;
    // Y10c: // is right-associative, so we handle it specially
    // a // b // c → a // (b // c)
    if let Some(Token::Op("//")) = self.peek() {
      self.pos += 1;
      let rhs = self.parse_update()?; // recursive for right-associativity
      expr = Expr::Binary {
        op: "//".into(),
        lhs: Arc::new(expr),
        rhs: Arc::new(rhs),
      };
    }
    Ok(expr)
  }

  fn parse_add(&mut self) -> Result<Expr, PnixError> {
    let mut expr = self.parse_mul()?;
    loop {
      let op = match self.peek() {
        Some(Token::Op("+")) => "+",
        Some(Token::Op("-")) => "-",
        // Y-CLAUDE-6: ++ 연산자 지원 (문자열 연결 전용)
        Some(Token::Op("++")) => "++",
        _ => break,
      };
      self.pos += 1;
      let rhs = self.parse_mul()?;
      expr = Expr::Binary {
        op: op.into(),
        lhs: Arc::new(expr),
        rhs: Arc::new(rhs),
      };
    }
    Ok(expr)
  }

  fn parse_unary(&mut self) -> Result<Expr, PnixError> {
    if let Some(Token::Op("-")) = self.peek() {
      self.pos += 1;
      let arg = self.parse_unary()?; // 재귀적으로 처리하여 -(-x) 같은 경우도 지원
      Ok(Expr::Unary {
        op: "-".into(),
        arg: Arc::new(arg),
      })
    } else if let Some(Token::Op("!")) = self.peek() {
      self.pos += 1;
      let arg = self.parse_unary()?; // 재귀적으로 처리하여 !!x 같은 경우도 지원
      Ok(Expr::Unary {
        op: "!".into(),
        arg: Arc::new(arg),
      })
    } else {
      self.parse_app()
    }
  }

  fn parse_mul(&mut self) -> Result<Expr, PnixError> {
    // Unary 연산자는 parse_unary()에서 처리됨.
    let mut expr = self.parse_unary()?;
    loop {
      let op = match self.peek() {
        Some(Token::Op("*")) => "*",
        Some(Token::Op("/")) => "/",
        Some(Token::Op("%")) => "%",
        _ => break,
      };
      self.pos += 1;
      let rhs = self.parse_unary()?;
      expr = Expr::Binary {
        op: op.into(),
        lhs: Arc::new(expr),
        rhs: Arc::new(rhs),
      };
    }
    Ok(expr)
  }

  fn parse_app(&mut self) -> Result<Expr, PnixError> {
    // Unary 연산자는 parse_unary()에서 처리됨 (더 높은 우선순위)
    let mut expr = self.parse_postfix()?;
    if self.allow_app {
      loop {
        if !self.next_starts_expr() {
          break;
        }
        let arg = self.parse_postfix()?;
        expr = Expr::Apply {
          func: Arc::new(expr),
          arg: Arc::new(arg),
        };
      }
    }
    Ok(expr)
  }

  fn parse_postfix(&mut self) -> Result<Expr, PnixError> {
    let mut expr = self.parse_primary()?;
    loop {
      match self.peek() {
        Some(Token::Symbol('.')) => {
          self.pos += 1;
          // N00f: Check for dynamic attribute access: x.${expr}
          if self.check_dynamic_attr() {
            // Dynamic attribute access: x.${expr}
            let attr_expr = self.parse_dynamic_attr()?;
            // N00m: Check for `or` default: x.${expr} or default
            if matches!(self.peek(), Some(Token::Keyword("or"))) {
              self.pos += 1; // consume 'or'
              let default = self.parse_postfix()?;
              expr = Expr::DynamicSelectOrDefault {
                base: Arc::new(expr),
                attr_expr: Arc::new(attr_expr),
                default: Arc::new(default),
              };
            } else {
              expr = Expr::DynamicSelect {
                base: Arc::new(expr),
                attr_expr: Arc::new(attr_expr),
              };
            }
          } else {
            // N00i: Get attribute name - allow identifier, string literal, or keywords as attribute names
            // Also handle interpolated strings as dynamic selects
            match self.peek().cloned() {
              Some(Token::Ident(name)) => {
                self.pos += 1;
                let attr = name;
                // Check for `or` default: x.y or default
                if matches!(self.peek(), Some(Token::Keyword("or"))) {
                  self.pos += 1; // consume 'or'
                  let default = self.parse_postfix()?;
                  expr = Expr::SelectOrDefault {
                    base: Arc::new(expr),
                    attr,
                    default: Arc::new(default),
                  };
                } else {
                  expr = Expr::Select {
                    base: Arc::new(expr),
                    attr,
                  };
                }
                continue; // Continue the postfix loop
              }
              // Also allow string literal as attribute: x."attr-name"
              Some(Token::String(name)) => {
                self.pos += 1;
                let attr = name;
                // Check for `or` default: x."y" or default
                if matches!(self.peek(), Some(Token::Keyword("or"))) {
                  self.pos += 1; // consume 'or'
                  let default = self.parse_postfix()?;
                  expr = Expr::SelectOrDefault {
                    base: Arc::new(expr),
                    attr,
                    default: Arc::new(default),
                  };
                } else {
                  expr = Expr::Select {
                    base: Arc::new(expr),
                    attr,
                  };
                }
                continue;
              }
              // N00n: Interpolated string as attribute: x."${prefix}foo"
              // Convert to dynamic select
              Some(Token::StringInterp(parts)) => {
                self.pos += 1;
                // Parse the interpolated string into an expression
                let mut ast_parts: Vec<super::syntax::StringInterpPart> = Vec::new();
                for part in parts {
                  match part {
                    LexerStringPart::Lit(s) => {
                      ast_parts.push(super::syntax::StringInterpPart::Lit(s));
                    }
                    LexerStringPart::InterpExpr(expr_src) => {
                      let parsed = crate::lang::pnix::parse_expr(&expr_src)?;
                      ast_parts.push(super::syntax::StringInterpPart::Expr(Arc::new(parsed)));
                    }
                  }
                }
                let attr_expr = Expr::StringInterp(ast_parts);
                // Check for `or` default: x."${y}" or default
                if matches!(self.peek(), Some(Token::Keyword("or"))) {
                  self.pos += 1;
                  let default = self.parse_postfix()?;
                  expr = Expr::DynamicSelectOrDefault {
                    base: Arc::new(expr),
                    attr_expr: Arc::new(attr_expr),
                    default: Arc::new(default),
                  };
                } else {
                  expr = Expr::DynamicSelect {
                    base: Arc::new(expr),
                    attr_expr: Arc::new(attr_expr),
                  };
                }
                continue;
              }
              // N00i: Allow keywords as attribute names (Nix compatibility)
              // `match`, `or`, `and` can all be attribute names
              Some(Token::Keyword("match"))
              | Some(Token::Keyword("or"))
              | Some(Token::Keyword("and")) => {
                let kw = match self.peek() {
                  Some(Token::Keyword(k)) => k.to_string(),
                  _ => unreachable!(
                    "peek() should return Keyword after matching Keyword in outer match"
                  ),
                };
                self.pos += 1;
                let attr = kw;
                // Check for `or` default: x.or or default (unlikely but possible)
                if matches!(self.peek(), Some(Token::Keyword("or"))) {
                  self.pos += 1;
                  let default = self.parse_postfix()?;
                  expr = Expr::SelectOrDefault {
                    base: Arc::new(expr),
                    attr,
                    default: Arc::new(default),
                  };
                } else {
                  expr = Expr::Select {
                    base: Arc::new(expr),
                    attr,
                  };
                }
                continue;
              }
              _ => return Err(self.error_expected("attr name after '.'")),
            }
          }
        }
        Some(Token::Symbol('[')) => {
          if !self.is_adjacent_index_bracket() {
            break;
          }
          // List indexing: list[index]
          self.pos += 1; // consume '['
          let index = self.parse_expr()?;
          if !matches!(self.peek(), Some(Token::Symbol(']'))) {
            return Err(self.error_expected("']' after index expression"));
          }
          self.pos += 1; // consume ']'
          expr = Expr::Index {
            base: Arc::new(expr),
            index: Arc::new(index),
          };
        }
        // N00e: HasAttr operator: x ? a, x ? a.b.c
        // N00p: Dynamic HasAttr: x ? ${expr}, x ? "string"
        Some(Token::Symbol('?')) => {
          self.pos += 1; // consume '?'

          match self.peek().cloned() {
            // Static attribute: x ? a or x ? a.b.c (may include dynamic: x ? a.${b})
            Some(Token::Ident(first_attr)) => {
              self.pos += 1;
              match self.parse_attr_path_maybe_dynamic(first_attr)? {
                Ok(static_path) => {
                  // Join with dots for HasAttr since it checks nested paths
                  // x ? a.b.c checks for x.a.b.c
                  expr = Expr::HasAttr {
                    base: Arc::new(expr),
                    attr: static_path.join("."),
                  };
                }
                Err(segments) => {
                  // Has dynamic segment - convert to DynamicHasAttr with concat expression
                  let attr_expr = segments_to_concat_expr(&segments)?;
                  expr = Expr::DynamicHasAttr {
                    base: Arc::new(expr),
                    attr_expr: Arc::new(attr_expr),
                  };
                }
              }
            }
            // String literal attribute: x ? "attr"
            Some(Token::String(attr_name)) => {
              self.pos += 1;
              expr = Expr::HasAttr {
                base: Arc::new(expr),
                attr: attr_name,
              };
            }
            // Interpolated string: x ? "${prefix}attr" (optionally followed by .more.path)
            Some(Token::StringInterp(parts)) => {
              self.pos += 1;
              let mut ast_parts: Vec<super::syntax::StringInterpPart> = Vec::new();
              for part in parts {
                match part {
                  LexerStringPart::Lit(s) => {
                    ast_parts.push(super::syntax::StringInterpPart::Lit(s));
                  }
                  LexerStringPart::InterpExpr(expr_src) => {
                    let parsed = crate::lang::pnix::parse_expr(&expr_src)?;
                    ast_parts.push(super::syntax::StringInterpPart::Expr(Arc::new(parsed)));
                  }
                }
              }
              // Continue consuming dotted path segments so expressions like
              // `set ? "${a}".b.c` build a multi-segment dynamic attr_expr
              // that DynamicHasAttr can split on '.' and traverse.
              self.consume_dotted_dynamic_path_segments(&mut ast_parts)?;
              let attr_expr = Expr::StringInterp(ast_parts);
              expr = Expr::DynamicHasAttr {
                base: Arc::new(expr),
                attr_expr: Arc::new(attr_expr),
              };
            }
            // Dynamic attribute: x ? ${expr}
            Some(Token::Symbol('$')) => {
              self.pos += 1; // consume '$'
              self.expect_symbol('{')?;
              let inner = self.parse_expr()?;
              self.expect_symbol('}')?;
              let mut ast_parts: Vec<super::syntax::StringInterpPart> =
                vec![super::syntax::StringInterpPart::Expr(Arc::new(inner))];
              self.consume_dotted_dynamic_path_segments(&mut ast_parts)?;
              let attr_expr = if ast_parts.len() == 1 {
                if let super::syntax::StringInterpPart::Expr(e) = &ast_parts[0] {
                  (**e).clone()
                } else {
                  Expr::StringInterp(ast_parts)
                }
              } else {
                Expr::StringInterp(ast_parts)
              };
              expr = Expr::DynamicHasAttr {
                base: Arc::new(expr),
                attr_expr: Arc::new(attr_expr),
              };
            }
            _ => {
              return Err(self.error_expected("attribute name after '?'"));
            }
          }
        }
        _ => break,
      }
    }
    Ok(expr)
  }

  fn parse_primary(&mut self) -> Result<Expr, PnixError> {
    match self.peek() {
      Some(Token::Keyword("let")) => self.parse_let(),
      Some(Token::Keyword("if")) => self.parse_if(),
      Some(Token::Keyword("match")) => {
        // N00i: Nix doesn't have `match` keyword, so it can be used as identifier
        // Check different contexts in order:
        // 1. match: or match@{...}: → lambda parameter
        // 2. match <expr> with → pnix match expression
        // 3. Otherwise → variable
        if self.is_match_lambda_param() {
          self.parse_lambda_ident()
        } else if self.is_match_used_as_variable() {
          self.pos += 1;
          Ok(Expr::Var("match".to_string()))
        } else {
          self.parse_match()
        }
      }
      Some(Token::Keyword("rec")) => {
        // rec { ... } - recursive attrset
        self.pos += 1; // consume 'rec'
        self.parse_attrset(true)
      }
      Some(Token::Keyword("import")) => {
        // N01a: import <expr> - module import
        self.pos += 1; // consume 'import'
        let path = self.parse_primary()?; // parse the path expression
        Ok(Expr::Import {
          path: Arc::new(path),
        })
      }
      Some(Token::Keyword("with")) => {
        // with <env>; <body> - scope extension
        self.pos += 1; // consume 'with'
        let env = self.parse_expr()?; // parse the environment expression
        self.expect_symbol(';')?; // expect semicolon
        let body = self.parse_expr()?; // parse the body
        Ok(Expr::With {
          env: Arc::new(env),
          body: Arc::new(body),
        })
      }
      Some(Token::Keyword("assert")) => {
        // Y10e: assert <cond>; <body> - assertion
        self.pos += 1; // consume 'assert'
        let cond = self.parse_expr()?; // parse the condition
        self.expect_symbol(';')?; // expect semicolon
        let body = self.parse_expr()?; // parse the body
        Ok(Expr::Assert {
          cond: Arc::new(cond),
          body: Arc::new(body),
        })
      }
      Some(Token::Symbol('{')) => {
        if self.is_lambda_pattern() {
          self.parse_lambda_pattern()
        } else {
          self.parse_attrset(false)
        }
      }
      Some(Token::Symbol('[')) => {
        if self.is_lambda_pattern() {
          self.parse_lambda_pattern()
        } else {
          self.parse_list()
        }
      }
      Some(Token::Ident(_)) => {
        if self.is_lambda_ident() {
          self.parse_lambda_ident()
        } else {
          self.parse_atom()
        }
      }
      _ => self.parse_atom(),
    }
  }

  fn parse_atom(&mut self) -> Result<Expr, PnixError> {
    match self.peek().cloned() {
      Some(Token::Int(v)) => {
        self.pos += 1;
        Ok(Expr::Int(v))
      }
      Some(Token::Float(v)) => {
        self.pos += 1;
        Ok(Expr::Float(v))
      }
      Some(Token::String(s)) => {
        self.pos += 1;
        Ok(Expr::String(s))
      }
      Some(Token::StringInterp(parts)) => {
        self.pos += 1;
        // Y10a: Parse interpolated string parts into StringInterp expression
        let mut ast_parts: Vec<StringInterpPart> = Vec::new();
        for part in parts {
          match part {
            LexerStringPart::Lit(s) => {
              ast_parts.push(StringInterpPart::Lit(s));
            }
            LexerStringPart::InterpExpr(expr_src) => {
              // Parse the expression source
              let parsed = crate::lang::pnix::parse_expr(&expr_src)?;
              ast_parts.push(StringInterpPart::Expr(Arc::new(parsed)));
            }
          }
        }
        Ok(Expr::StringInterp(ast_parts))
      }
      // Y10b: Path literals
      Some(Token::Path(path)) => {
        self.pos += 1;
        Ok(Expr::Path(path))
      }
      // Path with `${...}` interpolation. Mirror the StringInterp arm:
      // parse each `InterpExpr` slice as a sub-expression and emit
      // `PnixPath::Interpolated { base, parts }`.
      Some(Token::PathInterp(base, parts)) => {
        self.pos += 1;
        use super::syntax::PnixPath;
        let mut ast_parts: Vec<StringInterpPart> = Vec::with_capacity(parts.len());
        for part in parts {
          match part {
            LexerStringPart::Lit(s) => {
              ast_parts.push(StringInterpPart::Lit(s));
            }
            LexerStringPart::InterpExpr(src) => {
              let parsed = crate::lang::pnix::parse_expr(&src)?;
              ast_parts.push(StringInterpPart::Expr(Arc::new(parsed)));
            }
          }
        }
        Ok(Expr::Path(PnixPath::Interpolated {
          base,
          parts: ast_parts,
        }))
      }
      // Nix-compat: `or`, `and`, `match` may appear as plain variable
      // references in expression position (e.g. `[ (x: x) or ]`). They
      // remain keywords in their structural roles (`set.f or default`,
      // `&&` / `||` chain alias, pattern syntax) — postfix and infix
      // parsers still grab them first when they appear in those slots.
      Some(Token::Keyword(kw)) if kw == "or" || kw == "and" => {
        self.pos += 1;
        Ok(Expr::Var(kw.to_string()))
      }
      Some(Token::Ident(name)) => {
        let tok_pos = self.tokens.get(self.pos).map(|t| t.pos).unwrap_or(0);
        self.pos += 1;
        match name.as_str() {
          "true" => Ok(Expr::Bool(true)),
          "false" => Ok(Expr::Bool(false)),
          "null" => Ok(Expr::Null),
          // Nix-compat: `__curPos` is a parse-time substitution that
          // expands to `{ file = "<unknown>"; line = N; column = M; }`
          // capturing the source location of the token itself.
          "__curPos" => {
            let (line, column) = crate::diagnostics::line_col_at(self.source, tok_pos);
            Ok(Expr::AttrSet {
              items: vec![
                AttrItem::Assign {
                  key_path: vec!["file".to_string()],
                  value: std::sync::Arc::new(Expr::String("<unknown>".to_string())),
                  span: Span::point(tok_pos),
                },
                AttrItem::Assign {
                  key_path: vec!["line".to_string()],
                  value: std::sync::Arc::new(Expr::Int(line as i64)),
                  span: Span::point(tok_pos),
                },
                AttrItem::Assign {
                  key_path: vec!["column".to_string()],
                  value: std::sync::Arc::new(Expr::Int(column as i64)),
                  span: Span::point(tok_pos),
                },
              ],
              recursive: false,
            })
          }
          _ => {
            // Y09b-1 (revised): Treat capitalised identifiers as ADT
            // constructors. Two recognition shapes:
            //   - `Some(x)` / `Ok(a, b)` / `Err(e)` — capitalised
            //     name followed by `(` arg-list → multi-arg
            //     Constructor.
            //   - `None` / `Nothing` — capitalised name standing
            //     alone, length ≥ 2, no following `(` → nullary
            //     Constructor (no args).
            // Single-letter capitalised identifiers (`Z`, `A`, `I`,
            // `T`, `K`, `M`, `N`, `X`, `Y`, …) stay as regular
            // variables so Nix code that uses them as quantified
            // type / generic vars (`let { x : T } in ...` / `forall
            // K. ...`) keeps its semantics.
            //
            // 2026-05-06: nullary case added for `None` / well-known
            // empty constructors. Pre-fix `None` lowered to
            // `Var("None")` and the match-pattern lowering produced
            // `hasAttr "_variant" / getAttr "_variant"` runtime
            // dispatch instead of the compile-time `Bool(true)`
            // shortcut the test family expected. Closes the
            // `test_constructor_value_creation` /
            // `test_match_constructor_pattern` /
            // `test_nullary_constructor_parsing` regression.
            let starts_with_upper = name
              .chars()
              .next()
              .map(|c| c.is_uppercase())
              .unwrap_or(false);
            if starts_with_upper && matches!(self.peek(), Some(Token::Symbol('('))) {
              self.pos += 1; // consume '('
              let mut args = Vec::new();
              if !matches!(self.peek(), Some(Token::Symbol(')'))) {
                args.push(self.parse_expr()?);
                while matches!(self.peek(), Some(Token::Symbol(','))) {
                  self.pos += 1; // consume ','
                  args.push(self.parse_expr()?);
                }
              }
              self.expect_symbol(')')?;
              Ok(Expr::Construct {
                variant: name,
                args,
              })
            } else if starts_with_upper
              && name.chars().count() >= 2
              && nullary_constructor_terminator(self.peek())
            {
              // Nullary constructor: `None`, `Nothing`, `Empty`, ...
              // Recognised ONLY when the next token is an expression
              // terminator (binary op, `;`, `)`, `,`, `]`, `}`, `then`,
              // `else`, `in`, EOF, etc.). If `{`, `[`, identifier, or
              // string follows, the capitalised name is the function
              // of an application like `Shape { ... }` and stays as
              // `Var`. Length ≥ 2 to avoid clashing with single-
              // letter type / generic vars (`Z`, `A`, `I`, ...).
              Ok(Expr::Construct {
                variant: name,
                args: Vec::new(),
              })
            } else {
              Ok(Expr::Var(name))
            }
          }
        }
      }
      Some(Token::Symbol('(')) => {
        self.pos += 1;
        let expr = self.parse_expr()?;
        self.expect_symbol(')')?;
        Ok(expr)
      }
      Some(Token::Symbol('[')) => self.parse_list(),
      _ => Err(self.error_expected("expression")),
    }
  }

  fn parse_list(&mut self) -> Result<Expr, PnixError> {
    self.check_recursion_depth()?;
    self.expect_symbol('[')?;
    let mut elements = Vec::new();
    while !matches!(self.peek(), Some(Token::Symbol(']'))) {
      if self.peek().is_none() {
        self.decrement_recursion_depth();
        return Err(self.error_expected("symbol ']'"));
      }
      // In Nix, list items are whitespace-separated.
      // parse_expr()를 사용하여 산술 연산 등 복잡한 표현식도 올바르게 파싱
      // [1 + 2]는 [3]으로 파싱되어야 함 (not [1, +, 2])
      elements.push(self.parse_expr_with_app(false)?);
      // 다음 요소가 있는지 확인 (공백으로 구분)
      if matches!(self.peek(), Some(Token::Symbol(']'))) {
        break;
      }
    }
    self.expect_symbol(']')?;
    self.decrement_recursion_depth();
    Ok(Expr::List(elements))
  }

  fn parse_attrset(&mut self, recursive: bool) -> Result<Expr, PnixError> {
    self.check_recursion_depth()?;
    self.expect_symbol('{')?;
    let mut items = Vec::new();
    while !matches!(self.peek(), Some(Token::Symbol('}'))) {
      if self.peek().is_none() {
        self.decrement_recursion_depth();
        return Err(self.error_expected("symbol '}'"));
      }
      match self.peek().cloned() {
        Some(Token::Keyword("inherit")) => {
          let span = self.span_at_current();
          self.pos += 1; // consume 'inherit'

          // N00b: Check for optional (scope) expression
          let from = if matches!(self.peek(), Some(Token::Symbol('('))) {
            self.pos += 1; // consume '('
            let scope = self.parse_expr()?;
            self.expect_symbol(')')?;
            Some(Arc::new(scope))
          } else {
            None
          };

          let mut names = Vec::new();
          loop {
            match self.peek().cloned() {
              Some(Token::Ident(name)) => {
                self.pos += 1;
                names.push(name);
              }
              // N00l: Allow keywords as inherit names (match, or, and are valid Nix identifiers)
              Some(Token::Keyword(kw)) if kw == "match" || kw == "or" || kw == "and" => {
                self.pos += 1;
                names.push(kw.to_string());
              }
              // Nix-compat: quoted strings are valid inherit names —
              // `inherit (set) "key with space" "weird-name";`.
              Some(Token::String(s)) => {
                self.pos += 1;
                names.push(s);
              }
              // Nix-compat: `inherit ${"name"};` — literal string inside
              // `${ }` is allowed (and only literal strings, per Nix).
              Some(Token::Symbol('$')) if self.peek_n(1) == Some(&Token::Symbol('{')) => {
                self.pos += 2;
                let n = match self.peek().cloned() {
                  Some(Token::String(s)) => {
                    self.pos += 1;
                    s
                  }
                  _ => {
                    return Err(self.error_expected("string literal inside `${...}` inherit name"))
                  }
                };
                self.expect_symbol('}')?;
                names.push(n);
              }
              _ => break,
            }
          }
          self.expect_symbol(';')?;
          items.push(AttrItem::Inherit { from, names, span });
        }
        Some(Token::Ident(key)) => {
          let span = self.span_at_current();
          self.pos += 1;
          // LOW: 빈 속성 이름 검증 누락 수정
          if key.is_empty() {
            return Err(PnixError::parse_at_span(
              "empty attribute name is not allowed",
              span,
              self.source,
            ));
          }
          // N00c: Check for nested attribute path: a.b.c = value or a.${b}.c = value
          match self.parse_attr_path_maybe_dynamic(key)? {
            Ok(key_path) => {
              self.expect_op("=")?;
              let value = self.parse_expr()?;
              self.expect_symbol(';')?;
              items.push(AttrItem::Assign {
                key_path,
                value: std::sync::Arc::new(value),
                span,
              });
            }
            Err(key_path) => {
              self.expect_op("=")?;
              let value = self.parse_expr()?;
              self.expect_symbol(';')?;
              items.push(AttrItem::DynamicAssign {
                key_path,
                value: std::sync::Arc::new(value),
                span,
              });
            }
          }
        }
        // N00g: Keywords that can be used as attribute names in attrsets: or = ..., and = ..., match = ...
        // In Nix, 'or' and 'and' are keywords only in specific contexts (e.g., x.y or default)
        // 'match' is a pnix keyword but not a Nix keyword, so it can be used as identifier
        // At the start of an attrset item, they are treated as identifiers.
        // `import` is also a Nix global function name — code like
        // `{ import = fn: ...; }` (override pattern) needs it as a key.
        Some(Token::Keyword("or"))
        | Some(Token::Keyword("and"))
        | Some(Token::Keyword("match"))
        | Some(Token::Keyword("import")) => {
          let span = self.span_at_current();
          let key = match self.peek() {
            Some(Token::Keyword(k)) => k.to_string(),
            _ => unreachable!("peek() should return Keyword after matching Keyword in outer match"),
          };
          self.pos += 1;
          // Also support nested paths: or.x = value or or.${x} = value
          match self.parse_attr_path_maybe_dynamic(key)? {
            Ok(key_path) => {
              self.expect_op("=")?;
              let value = self.parse_expr()?;
              self.expect_symbol(';')?;
              items.push(AttrItem::Assign {
                key_path,
                value: std::sync::Arc::new(value),
                span,
              });
            }
            Err(key_path) => {
              self.expect_op("=")?;
              let value = self.parse_expr()?;
              self.expect_symbol(';')?;
              items.push(AttrItem::DynamicAssign {
                key_path,
                value: std::sync::Arc::new(value),
                span,
              });
            }
          }
        }
        // N00d: Quoted attribute names: "or" = value, "my-attr" = value
        Some(Token::String(key)) => {
          let span = self.span_at_current();
          self.pos += 1;
          // LOW: 빈 속성 이름 검증 누락 수정
          if key.is_empty() {
            return Err(PnixError::parse_at_span(
              "empty attribute name is not allowed",
              span,
              self.source,
            ));
          }
          // Also support nested paths with quoted names: "key".${x} = value
          match self.parse_attr_path_maybe_dynamic(key)? {
            Ok(key_path) => {
              self.expect_op("=")?;
              let value = self.parse_expr()?;
              self.expect_symbol(';')?;
              items.push(AttrItem::Assign {
                key_path,
                value: std::sync::Arc::new(value),
                span,
              });
            }
            Err(key_path) => {
              self.expect_op("=")?;
              let value = self.parse_expr()?;
              self.expect_symbol(';')?;
              items.push(AttrItem::DynamicAssign {
                key_path,
                value: std::sync::Arc::new(value),
                span,
              });
            }
          }
        }
        // N00o: Interpolated string as key: "${prefix}BuildBuild" = value or chained "${a}.${b}" = value
        Some(Token::StringInterp(parts)) => {
          let span = self.span_at_current();
          self.pos += 1;
          // Parse the interpolated string into an expression
          let mut ast_parts: Vec<super::syntax::StringInterpPart> = Vec::new();
          for part in parts {
            match part {
              LexerStringPart::Lit(s) => {
                ast_parts.push(super::syntax::StringInterpPart::Lit(s));
              }
              LexerStringPart::InterpExpr(expr_src) => {
                let parsed = crate::lang::pnix::parse_expr(&expr_src)?;
                ast_parts.push(super::syntax::StringInterpPart::Expr(Arc::new(parsed)));
              }
            }
          }
          let first_segment = AttrKeySegment::Dynamic(Arc::new(Expr::StringInterp(ast_parts)));
          // Check for chained segments: "${a}".b.${c}
          let key_path = self.parse_dynamic_key_path(first_segment)?;
          self.expect_op("=")?;
          let value = self.parse_expr()?;
          self.expect_symbol(';')?;
          items.push(AttrItem::DynamicAssign {
            key_path,
            value: std::sync::Arc::new(value),
            span,
          });
        }
        // N00k: Dynamic attribute names: ${expr} = value or chained ${a}.${b} = value
        Some(Token::Symbol('$')) => {
          let span = self.span_at_current();
          self.pos += 1; // consume '$'
          self.expect_symbol('{')?;
          let key_expr = self.parse_expr()?;
          self.expect_symbol('}')?;
          let first_segment = AttrKeySegment::Dynamic(Arc::new(key_expr));
          // Check for chained segments: ${a}.b.${c}
          let key_path = self.parse_dynamic_key_path(first_segment)?;
          self.expect_op("=")?;
          let value = self.parse_expr()?;
          self.expect_symbol(';')?;
          items.push(AttrItem::DynamicAssign {
            key_path,
            value: std::sync::Arc::new(value),
            span,
          });
        }
        _ => {
          return Err(self.error_expected("attrset item"));
        }
      }
    }
    self.expect_symbol('}')?;
    self.decrement_recursion_depth();
    Ok(Expr::AttrSet { items, recursive })
  }

  /// N00c/N00d: Parse attribute path like `a.b.c` or `"or".x.y`
  /// Takes the first part already parsed and continues if there are more dots.
  /// Returns Ok(Ok(Vec<String>)) for pure static paths (each segment preserved separately),
  /// Ok(Err(Vec<AttrKeySegment>)) for paths with dynamic segments.
  /// Note: quoted keys like "a.b" are kept as a single segment (no dot splitting).
  fn parse_attr_path_maybe_dynamic(
    &mut self,
    first: String,
  ) -> Result<Result<Vec<String>, Vec<AttrKeySegment>>, PnixError> {
    let mut static_parts: Vec<String> = vec![first.clone()];
    let mut segments: Vec<AttrKeySegment> = vec![AttrKeySegment::Static(first)];
    let mut has_dynamic = false;

    while matches!(self.peek(), Some(Token::Symbol('.'))) {
      self.pos += 1; // consume '.'
      match self.peek().cloned() {
        Some(Token::Ident(part)) => {
          self.pos += 1;
          if !has_dynamic {
            static_parts.push(part.clone());
          }
          segments.push(AttrKeySegment::Static(part));
        }
        Some(Token::String(part)) => {
          self.pos += 1;
          if !has_dynamic {
            static_parts.push(part.clone());
          }
          segments.push(AttrKeySegment::Static(part));
        }
        // N00i: Allow keywords in attribute paths
        Some(Token::Keyword("match"))
        | Some(Token::Keyword("or"))
        | Some(Token::Keyword("and")) => {
          let kw = match self.peek() {
            Some(Token::Keyword(k)) => k.to_string(),
            _ => unreachable!("peek() should return Keyword after matching Keyword in outer match"),
          };
          self.pos += 1;
          if !has_dynamic {
            static_parts.push(kw.clone());
          }
          segments.push(AttrKeySegment::Static(kw));
        }
        // Dynamic segment: ${expr}
        Some(Token::Symbol('$')) => {
          has_dynamic = true;
          self.pos += 1; // consume '$'
          self.expect_symbol('{')?;
          let expr = self.parse_expr()?;
          self.expect_symbol('}')?;
          segments.push(AttrKeySegment::Dynamic(Arc::new(expr)));
        }
        // Interpolated string as segment: "${prefix}name"
        Some(Token::StringInterp(parts)) => {
          has_dynamic = true;
          self.pos += 1;
          let mut ast_parts: Vec<super::syntax::StringInterpPart> = Vec::new();
          for part in parts {
            match part {
              LexerStringPart::Lit(s) => {
                ast_parts.push(super::syntax::StringInterpPart::Lit(s));
              }
              LexerStringPart::InterpExpr(expr_src) => {
                let parsed = crate::lang::pnix::parse_expr(&expr_src)?;
                ast_parts.push(super::syntax::StringInterpPart::Expr(Arc::new(parsed)));
              }
            }
          }
          let expr = Expr::StringInterp(ast_parts);
          segments.push(AttrKeySegment::Dynamic(Arc::new(expr)));
        }
        _ => return Err(self.error_expected("identifier, string, or ${expr} in attribute path")),
      }
    }

    if has_dynamic {
      Ok(Err(segments))
    } else {
      Ok(Ok(static_parts))
    }
  }

  /// Parse a dynamic attribute key path: ${expr}.a.${expr2}.b
  /// This handles chained segments where each can be static or dynamic.
  /// Takes the first segment already parsed and continues if there are more dots.
  fn parse_dynamic_key_path(
    &mut self,
    first: AttrKeySegment,
  ) -> Result<Vec<AttrKeySegment>, PnixError> {
    let mut path = vec![first];
    while matches!(self.peek(), Some(Token::Symbol('.'))) {
      self.pos += 1; // consume '.'
      match self.peek().cloned() {
        Some(Token::Ident(part)) => {
          self.pos += 1;
          path.push(AttrKeySegment::Static(part));
        }
        Some(Token::String(part)) => {
          self.pos += 1;
          path.push(AttrKeySegment::Static(part));
        }
        // Keywords that can be attribute names
        Some(Token::Keyword("match"))
        | Some(Token::Keyword("or"))
        | Some(Token::Keyword("and")) => {
          let kw = match self.peek() {
            Some(Token::Keyword(k)) => k.to_string(),
            _ => unreachable!("peek() should return Keyword after matching Keyword in outer match"),
          };
          self.pos += 1;
          path.push(AttrKeySegment::Static(kw));
        }
        // Dynamic segment: ${expr}
        Some(Token::Symbol('$')) => {
          self.pos += 1; // consume '$'
          self.expect_symbol('{')?;
          let expr = self.parse_expr()?;
          self.expect_symbol('}')?;
          path.push(AttrKeySegment::Dynamic(Arc::new(expr)));
        }
        // Interpolated string as segment: "${prefix}name"
        Some(Token::StringInterp(parts)) => {
          self.pos += 1;
          let mut ast_parts: Vec<super::syntax::StringInterpPart> = Vec::new();
          for part in parts {
            match part {
              LexerStringPart::Lit(s) => {
                ast_parts.push(super::syntax::StringInterpPart::Lit(s));
              }
              LexerStringPart::InterpExpr(expr_src) => {
                let parsed = crate::lang::pnix::parse_expr(&expr_src)?;
                ast_parts.push(super::syntax::StringInterpPart::Expr(Arc::new(parsed)));
              }
            }
          }
          let expr = Expr::StringInterp(ast_parts);
          path.push(AttrKeySegment::Dynamic(Arc::new(expr)));
        }
        _ => return Err(self.error_expected("identifier, string, or ${expr} in attribute path")),
      }
    }
    Ok(path)
  }

  fn parse_let(&mut self) -> Result<Expr, PnixError> {
    self.expect_keyword("let")?;
    // N00z: Disambiguate two forms that both start with `let {`:
    //   1. Legacy `let { body = X; ... }` (Nix-compat) — desugars to
    //      `(rec { body = X; ... }).body`.
    //   2. Modern destructuring `let { x, y ? d } = expr; in body` —
    //      single binding whose pattern is an attrset destructure.
    // Lookahead picks (1) only when the form is unmistakably legacy:
    // `{ inherit ... }` or `{ ident = ... }`.
    if matches!(self.peek(), Some(Token::Symbol('{'))) {
      let after_brace = self.peek_n(1);
      let after_ident = self.peek_n(2);
      let is_legacy = match (after_brace, after_ident) {
        (Some(Token::Keyword("inherit")), _) => true,
        (Some(Token::Ident(_)), Some(Token::Op("="))) => true,
        (Some(Token::Ident(_)), Some(Token::Symbol('.'))) => true, // legacy nested path
        (Some(Token::String(_)), Some(Token::Op("="))) => true,    // legacy quoted name
        (Some(Token::Symbol('}')), _) => true, // empty legacy: `let { }` (degenerate)
        _ => false,
      };
      if is_legacy {
        let rec_set = self.parse_attrset(true)?;
        return Ok(Expr::Select {
          base: Arc::new(rec_set),
          attr: "body".to_string(),
        });
      }
      // Otherwise fall through to the regular let parser, which will
      // recognise this as a destructuring-pattern binding.
    }
    let mut bindings = Vec::new();
    while !matches!(self.peek(), Some(Token::Keyword("in"))) {
      // N00b: Check for inherit keyword
      if matches!(self.peek(), Some(Token::Keyword("inherit"))) {
        self.pos += 1; // consume 'inherit'

        // Check for optional (scope) expression
        let from = if matches!(self.peek(), Some(Token::Symbol('('))) {
          self.pos += 1; // consume '('
          let scope = self.parse_expr()?;
          self.expect_symbol(')')?;
          Some(Arc::new(scope))
        } else {
          None
        };

        // Parse variable names
        let mut names = Vec::new();
        loop {
          match self.peek().cloned() {
            Some(Token::Ident(name)) => {
              self.pos += 1;
              names.push(name);
            }
            // N00l: Allow keywords as inherit names (match, or, and are valid Nix identifiers)
            Some(Token::Keyword(kw)) if kw == "match" || kw == "or" || kw == "and" => {
              self.pos += 1;
              names.push(kw.to_string());
            }
            Some(Token::String(s)) => {
              self.pos += 1;
              names.push(s);
            }
            Some(Token::Symbol('$')) if self.peek_n(1) == Some(&Token::Symbol('{')) => {
              self.pos += 2;
              let n = match self.peek().cloned() {
                Some(Token::String(s)) => {
                  self.pos += 1;
                  s
                }
                _ => {
                  return Err(self.error_expected("string literal inside `${...}` inherit name"))
                }
              };
              self.expect_symbol('}')?;
              names.push(n);
            }
            _ => break,
          }
        }
        self.expect_symbol(';')?;
        bindings.push(LetBinding::Inherit { from, names });
      } else {
        // Normal binding: pattern = value; or nested path: a.b.c = value;
        let pattern = self.parse_binding_pattern()?;

        // Check for nested attribute path: a.b.c = value → a = { b = { c = value; }; };
        if let ParamPattern::Ident(first_name) = &pattern {
          if matches!(self.peek(), Some(Token::Symbol('.'))) {
            // Parse the rest of the path
            let mut path_parts = vec![first_name.clone()];
            while matches!(self.peek(), Some(Token::Symbol('.'))) {
              self.pos += 1; // consume '.'
              match self.peek().cloned() {
                Some(Token::Ident(part)) => {
                  self.pos += 1;
                  path_parts.push(part);
                }
                Some(Token::String(part)) => {
                  self.pos += 1;
                  path_parts.push(part);
                }
                Some(Token::Keyword(kw)) if kw == "match" || kw == "or" || kw == "and" => {
                  self.pos += 1;
                  path_parts.push(kw.to_string());
                }
                _ => return Err(self.error_expected("identifier in attribute path")),
              }
            }
            self.expect_op("=")?;
            let mut value = self.parse_expr()?;
            self.expect_symbol(';')?;

            // Wrap value in nested attrsets from right to left
            // a.b.c = v → a = { b = { c = v; }; };
            for part in path_parts.iter().skip(1).rev() {
              value = Expr::AttrSet {
                items: vec![AttrItem::Assign {
                  key_path: vec![part.clone()],
                  value: std::sync::Arc::new(value),
                  span: Span::empty(),
                }],
                recursive: false,
              };
            }
            bindings.push(LetBinding::Binding {
              pattern: ParamPattern::Ident(path_parts[0].clone()),
              value: std::sync::Arc::new(value),
            });
            continue;
          }
        }

        self.expect_op("=")?;
        let value = self.parse_expr()?;
        self.expect_symbol(';')?;
        bindings.push(LetBinding::Binding {
          pattern,
          value: std::sync::Arc::new(value),
        });
      }
    }
    self.expect_keyword("in")?;
    let body = self.parse_expr()?;
    Ok(Expr::Let {
      bindings,
      body: Arc::new(body),
    })
  }

  fn parse_if(&mut self) -> Result<Expr, PnixError> {
    self.check_recursion_depth()?;
    let mut conds = Vec::new();
    let mut thens = Vec::new();

    let else_expr = loop {
      self.expect_keyword("if")?;
      let cond = self.parse_expr()?;
      self.expect_keyword("then")?;
      let then_ = self.parse_expr()?;
      conds.push(cond);
      thens.push(then_);

      self.expect_keyword("else")?;
      if !matches!(self.peek(), Some(Token::Keyword("if"))) {
        break self.parse_expr()?;
      }
    };

    let mut acc = else_expr;
    for (cond, then_) in conds.into_iter().zip(thens).rev() {
      acc = Expr::If {
        cond: Arc::new(cond),
        then_: Arc::new(then_),
        else_: Arc::new(acc),
      };
    }
    self.decrement_recursion_depth();
    Ok(acc)
  }

  fn parse_match(&mut self) -> Result<Expr, PnixError> {
    self.check_recursion_depth()?;
    self.expect_keyword("match")?;
    let scrutinee = self.parse_expr()?;
    self.expect_keyword("with")?;
    let mut arms = Vec::new();
    // 첫 번째 arm 전에 |가 있을 수 있음
    if matches!(self.peek(), Some(Token::Symbol('|'))) {
      self.pos += 1;
    }
    loop {
      // 패턴 파싱 (Or 패턴: a | b => body)
      let mut patterns = vec![self.parse_pattern()?];
      while matches!(self.peek(), Some(Token::Symbol('|'))) {
        self.pos += 1;
        patterns.push(self.parse_pattern()?);
      }

      // Y08c: 가드 조건 파싱 (optional)
      let guard = if matches!(self.peek(), Some(Token::Keyword("if"))) {
        self.pos += 1; // consume "if"
        let guard_expr = self.parse_expr()?;
        Some(Arc::new(guard_expr))
      } else {
        None
      };

      self.expect_op("=>")?;
      let body = self.parse_expr()?;
      for pattern in patterns {
        arms.push(super::syntax::PnixMatchArm {
          pattern,
          guard: guard.clone(),
          body: std::sync::Arc::new(body.clone()),
        });
      }
      // 다음 arm이 있는지 확인
      if matches!(self.peek(), Some(Token::Symbol('|'))) {
        self.pos += 1;
      } else {
        break;
      }
    }
    self.decrement_recursion_depth();
    Ok(Expr::Match {
      scrutinee: Arc::new(scrutinee),
      arms,
    })
  }

  fn parse_pattern(&mut self) -> Result<super::syntax::PnixPattern, PnixError> {
    match self.peek().cloned() {
      Some(Token::Symbol('{')) => self.parse_match_attrset_pattern(),
      Some(Token::Symbol('[')) => self.parse_match_list_pattern(),
      Some(Token::Ident(name)) => {
        self.pos += 1;
        if name == "_" {
          Ok(super::syntax::PnixPattern::Wildcard)
        } else if name == "true" || name == "false" {
          Ok(super::syntax::PnixPattern::Literal(
            super::syntax::PnixLiteralPattern::Bool(name == "true"),
          ))
        } else if name == "null" {
          Ok(super::syntax::PnixPattern::Literal(
            super::syntax::PnixLiteralPattern::Null,
          ))
        } else {
          // Y13a-11: Constructor pattern: Some(x), None, Ok(a), Err(e)
          // 대문자로 시작하는 identifier는 constructor로 간주
          let is_constructor = name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);
          if is_constructor {
            if matches!(self.peek(), Some(Token::Symbol('('))) {
              // Constructor with args: Some(x), Ok(a, b)
              self.pos += 1; // consume '('
              let mut args = Vec::new();
              if !matches!(self.peek(), Some(Token::Symbol(')'))) {
                args.push(self.parse_pattern()?);
                while matches!(self.peek(), Some(Token::Symbol(','))) {
                  self.pos += 1; // consume ','
                  args.push(self.parse_pattern()?);
                }
              }
              self.expect_symbol(')')?;
              Ok(super::syntax::PnixPattern::Constructor {
                variant: name,
                args,
              })
            } else {
              // Nullary constructor: None, Ok, Err (no parentheses)
              Ok(super::syntax::PnixPattern::Constructor {
                variant: name,
                args: Vec::new(),
              })
            }
          } else {
            // Lowercase identifier: variable pattern
            Ok(super::syntax::PnixPattern::Var(name))
          }
        }
      }
      Some(Token::Int(v)) => {
        self.pos += 1;
        Ok(super::syntax::PnixPattern::Literal(
          super::syntax::PnixLiteralPattern::Int(v),
        ))
      }
      Some(Token::Float(v)) => {
        self.pos += 1;
        Ok(super::syntax::PnixPattern::Literal(
          super::syntax::PnixLiteralPattern::Float(v),
        ))
      }
      Some(Token::String(s)) => {
        self.pos += 1;
        Ok(super::syntax::PnixPattern::Literal(
          super::syntax::PnixLiteralPattern::String(s),
        ))
      }
      _ => Err(self.error_expected("pattern")),
    }
  }

  fn parse_match_attrset_pattern(&mut self) -> Result<super::syntax::PnixPattern, PnixError> {
    self.expect_symbol('{')?;
    let mut fields = Vec::new();
    let mut ellipsis = false;
    let mut first = true;
    while !matches!(self.peek(), Some(Token::Symbol('}'))) {
      if !first {
        match self.peek() {
          Some(Token::Symbol(',')) => {
            self.pos += 1;
            if matches!(self.peek(), Some(Token::Symbol('}'))) {
              break;
            }
          }
          _ => {
            return Err(self.error_expected("',' between pattern fields"));
          }
        }
      }
      first = false;

      if self.is_ellipsis() {
        self.pos += 3;
        ellipsis = true;
        if matches!(self.peek(), Some(Token::Symbol(','))) {
          self.pos += 1;
        }
        if !matches!(self.peek(), Some(Token::Symbol('}'))) {
          return Err(self.error_expected("'}' after '...'"));
        }
        break;
      }

      let name = match self.peek().cloned() {
        Some(Token::Ident(name)) => {
          self.pos += 1;
          name
        }
        Some(Token::Keyword("match")) => {
          self.pos += 1;
          "match".to_string()
        }
        _ => return Err(self.error_expected("pattern field name")),
      };

      if matches!(self.peek(), Some(Token::Symbol('?'))) {
        let pos = self.current_pos();
        let span = Span::point(pos);
        return Err(PnixError::parse_at_span(
          "match attrset patterns do not support defaults",
          span,
          self.source,
        ));
      }

      let pattern = if matches!(self.peek(), Some(Token::Op("=")) | Some(Token::Symbol('='))) {
        self.pos += 1;
        Some(Arc::new(self.parse_pattern()?))
      } else {
        None
      };

      fields.push(super::syntax::PnixMatchAttrField { name, pattern });
    }
    self.expect_symbol('}')?;
    Ok(super::syntax::PnixPattern::AttrSet { fields, ellipsis })
  }

  fn parse_match_list_pattern(&mut self) -> Result<super::syntax::PnixPattern, PnixError> {
    self.expect_symbol('[')?;
    let mut items = Vec::new();
    let mut tail = None;
    let mut first = true;
    while !matches!(self.peek(), Some(Token::Symbol(']'))) {
      if self.peek().is_none() {
        return Err(self.error_expected("symbol ']'"));
      }
      if !first {
        match self.peek() {
          Some(Token::Symbol(',')) => {
            self.pos += 1;
            if matches!(self.peek(), Some(Token::Symbol(']'))) {
              return Err(self.error_expected("list pattern"));
            }
          }
          _ => {
            return Err(self.error_expected("',' between list pattern items"));
          }
        }
      }
      first = false;

      if self.is_ellipsis() {
        if tail.is_some() {
          return Err(self.error_expected("single list tail"));
        }
        self.pos += 3;
        let tail_name = match self.peek().cloned() {
          Some(Token::Ident(name)) => {
            self.pos += 1;
            name
          }
          Some(Token::Keyword("match")) => {
            self.pos += 1;
            "match".to_string()
          }
          _ => return Err(self.error_expected("list tail name")),
        };
        tail = Some(tail_name);
        if !matches!(self.peek(), Some(Token::Symbol(']'))) {
          return Err(self.error_expected("']' after list tail"));
        }
        break;
      }

      items.push(self.parse_pattern()?);
    }
    self.expect_symbol(']')?;
    Ok(super::syntax::PnixPattern::List(
      super::syntax::PnixMatchListPattern { items, tail },
    ))
  }

  fn parse_lambda_ident(&mut self) -> Result<Expr, PnixError> {
    // N00i: Accept both Ident and Keyword("match") as lambda parameter
    let ident = match self.peek().cloned() {
      Some(Token::Ident(name)) => {
        self.pos += 1;
        name
      }
      Some(Token::Keyword("match")) => {
        self.pos += 1;
        "match".to_string()
      }
      _ => return Err(self.error_expected("lambda param")),
    };

    // Check for ident@{...}: pattern
    let param = if matches!(self.peek(), Some(Token::Symbol('@'))) {
      self.pos += 1; // consume @
                     // Parse the attrset pattern
      let (fields, ellipsis) = self.parse_attrset_pattern_fields()?;
      ParamPattern::AttrSetWithBind {
        bind_name: ident,
        fields,
        ellipsis,
      }
    } else {
      ParamPattern::Ident(ident)
    };

    self.expect_symbol(':')?;
    let body = self.parse_expr()?;
    Ok(Expr::Lambda {
      param,
      body: Arc::new(body),
    })
  }

  fn parse_lambda_pattern(&mut self) -> Result<Expr, PnixError> {
    self.check_recursion_depth()?;
    let param = self.parse_param_pattern()?;
    self.expect_symbol(':')?;
    let body = self.parse_expr()?;
    self.decrement_recursion_depth();
    Ok(Expr::Lambda {
      param,
      body: Arc::new(body),
    })
  }

  fn parse_binding_pattern(&mut self) -> Result<ParamPattern, PnixError> {
    match self.peek().cloned() {
      Some(Token::Ident(name)) => {
        self.pos += 1;
        Ok(ParamPattern::Ident(name))
      }
      // N00i: Allow pnix-specific keywords as identifiers in let bindings
      // In Nix, `match` is not a keyword, so it can be used as a variable name.
      // Nix-compat: `or` and `and` are also valid identifiers in binding
      // position (they are only keywords in the `x.y or default` postfix
      // operator and `&&`/`||` chains). Nix emits a deprecation warning
      // when `or` is used as a let-binding name; pnix accepts silently.
      Some(Token::Keyword(kw)) if kw == "match" || kw == "or" || kw == "and" => {
        self.pos += 1;
        Ok(ParamPattern::Ident(kw.to_string()))
      }
      // Nix-compat: quoted strings as let binding names — `let "foo bar" = 1;`.
      // The bound name is the literal string contents. Such variables cannot
      // be referenced from the body (no syntax for `${"..."}` lookup in
      // `let`/Var), but they parse and remain in the legacy `let { ... }`
      // attrset projection.
      Some(Token::String(s)) => {
        self.pos += 1;
        Ok(ParamPattern::Ident(s))
      }
      // Nix-compat dynamic let binding: `${"foo"} = ...;`. Only literal
      // strings inside `${ ... }` are accepted (Nix restriction). Anything
      // else falls through to the error path.
      Some(Token::Symbol('$')) if self.peek_n(1) == Some(&Token::Symbol('{')) => {
        self.pos += 2; // consume `${`
        let inner = match self.peek().cloned() {
          Some(Token::String(s)) => {
            self.pos += 1;
            s
          }
          _ => return Err(self.error_expected("string literal inside `${...}` let-binding name")),
        };
        self.expect_symbol('}')?;
        Ok(ParamPattern::Ident(inner))
      }
      Some(Token::Symbol('{')) | Some(Token::Symbol('[')) => self.parse_param_pattern(),
      _ => Err(self.error_expected("binding pattern in let")),
    }
  }

  fn parse_param_pattern(&mut self) -> Result<ParamPattern, PnixError> {
    match self.peek() {
      Some(Token::Symbol('{')) => self.parse_attrset_pattern(),
      Some(Token::Symbol('[')) => self.parse_list_pattern(),
      _ => Err(self.error_expected("pattern")),
    }
  }

  fn parse_attrset_pattern(&mut self) -> Result<ParamPattern, PnixError> {
    let (fields, ellipsis) = self.parse_attrset_pattern_fields()?;

    // Check for {..}@ident: pattern
    if matches!(self.peek(), Some(Token::Symbol('@'))) {
      self.pos += 1; // consume @
      let Some(Token::Ident(bind_name)) = self.peek().cloned() else {
        return Err(self.error_expected("identifier after @"));
      };
      self.pos += 1;
      Ok(ParamPattern::AttrSetWithBind {
        bind_name,
        fields,
        ellipsis,
      })
    } else {
      Ok(ParamPattern::AttrSet { fields, ellipsis })
    }
  }

  /// Parse attrset pattern fields: { x, y ? default, ... }
  /// Returns (fields, ellipsis) where ellipsis is true if `...` was found
  fn parse_attrset_pattern_fields(&mut self) -> Result<(Vec<PatternField>, bool), PnixError> {
    self.expect_symbol('{')?;
    let mut fields = Vec::new();
    let mut ellipsis = false;
    let mut first = true;
    while !matches!(self.peek(), Some(Token::Symbol('}'))) {
      if !first {
        match self.peek() {
          Some(Token::Symbol(',')) => {
            self.pos += 1;
            if matches!(self.peek(), Some(Token::Symbol('}'))) {
              // Trailing comma is allowed: { x, y, }
              break;
            }
          }
          _ => {
            return Err(self.error_expected("',' between pattern fields"));
          }
        }
      }
      first = false;

      // N00j: Check for ellipsis `...` (ignores extra fields)
      if self.is_ellipsis() {
        self.pos += 3; // consume three dots
        ellipsis = true;
        // After ellipsis, only '}' is allowed (optionally after comma)
        if matches!(self.peek(), Some(Token::Symbol(','))) {
          self.pos += 1; // allow trailing comma after ellipsis
        }
        if !matches!(self.peek(), Some(Token::Symbol('}'))) {
          return Err(self.error_expected("'}' after '...'"));
        }
        break;
      }

      // N00i: Accept both Ident and Keyword("match") as pattern field name
      let name = match self.peek().cloned() {
        Some(Token::Ident(name)) => {
          self.pos += 1;
          name
        }
        Some(Token::Keyword("match")) => {
          self.pos += 1;
          "match".to_string()
        }
        _ => return Err(self.error_expected("pattern field name")),
      };
      let default = if matches!(self.peek(), Some(Token::Symbol('?'))) {
        self.pos += 1;
        Some(self.parse_expr()?)
      } else {
        None
      };
      fields.push(PatternField { name, default });
    }
    self.expect_symbol('}')?;
    Ok((fields, ellipsis))
  }

  fn parse_list_pattern(&mut self) -> Result<ParamPattern, PnixError> {
    self.expect_symbol('[')?;
    let mut items = Vec::new();
    let mut tail = None;
    let mut first = true;
    while !matches!(self.peek(), Some(Token::Symbol(']'))) {
      if self.peek().is_none() {
        return Err(self.error_expected("symbol ']'"));
      }
      if !first {
        match self.peek() {
          Some(Token::Symbol(',')) => {
            self.pos += 1;
            if matches!(self.peek(), Some(Token::Symbol(']'))) {
              return Err(self.error_expected("list pattern name"));
            }
          }
          _ => {
            return Err(self.error_expected("',' between list pattern items"));
          }
        }
      }
      first = false;

      if self.is_ellipsis() {
        if tail.is_some() {
          return Err(self.error_expected("single list tail"));
        }
        self.pos += 3;
        let Some(Token::Ident(name)) = self.peek().cloned() else {
          return Err(self.error_expected("list tail name"));
        };
        self.pos += 1;
        tail = Some(name);
        if !matches!(self.peek(), Some(Token::Symbol(']'))) {
          return Err(self.error_expected("']' after list tail"));
        }
        break;
      }

      let Some(Token::Ident(name)) = self.peek().cloned() else {
        return Err(self.error_expected("list pattern name"));
      };
      self.pos += 1;
      items.push(name);
    }
    self.expect_symbol(']')?;
    Ok(ParamPattern::List(ListPattern { items, tail }))
  }

  fn is_lambda_ident(&self) -> bool {
    // Check for: ident: or ident@{...}:
    match (self.peek(), self.peek_n(1)) {
      (Some(Token::Ident(_)), Some(Token::Symbol(':'))) => true,
      (Some(Token::Ident(_)), Some(Token::Symbol('@'))) => {
        // ident@{...}: pattern - check if { is followed by matching } and :
        if !matches!(self.peek_n(2), Some(Token::Symbol('{'))) {
          return false;
        }
        // Use is_lambda_pattern_with logic starting from position after @
        let mut depth = 0usize;
        let mut i = self.pos + 2; // skip ident and @
        while i < self.tokens.len() {
          match self.tokens.get(i).map(|tok| &tok.token) {
            Some(Token::Symbol('{')) => depth += 1,
            Some(Token::Symbol('}')) => {
              depth = depth.saturating_sub(1);
              if depth == 0 {
                return matches!(
                  self.tokens.get(i + 1).map(|tok| &tok.token),
                  Some(Token::Symbol(':'))
                );
              }
            }
            _ => {}
          }
          i += 1;
        }
        false
      }
      _ => false,
    }
  }

  fn is_lambda_pattern(&self) -> bool {
    match self.peek() {
      Some(Token::Symbol('{')) => self.is_lambda_pattern_with('{', '}'),
      Some(Token::Symbol('[')) => self.is_lambda_pattern_with('[', ']'),
      _ => false,
    }
  }

  fn is_lambda_pattern_with(&self, open: char, close: char) -> bool {
    let mut depth = 0usize;
    let mut i = self.pos;
    while i < self.tokens.len() {
      match self.tokens.get(i).map(|tok| &tok.token) {
        Some(Token::Symbol(ch)) if *ch == open => depth += 1,
        Some(Token::Symbol(ch)) if *ch == close => {
          depth = depth.saturating_sub(1);
          if depth == 0 {
            // Check for: {..}: or {..}@ident:
            return match self.tokens.get(i + 1).map(|tok| &tok.token) {
              Some(Token::Symbol(':')) => true,
              Some(Token::Symbol('@')) => {
                // Check for @ident:
                matches!(
                  (
                    self.tokens.get(i + 2).map(|tok| &tok.token),
                    self.tokens.get(i + 3).map(|tok| &tok.token)
                  ),
                  (Some(Token::Ident(_)), Some(Token::Symbol(':')))
                )
              }
              _ => false,
            };
          }
        }
        _ => {}
      }
      i += 1;
    }
    false
  }

  fn is_ellipsis(&self) -> bool {
    matches!(self.peek(), Some(Token::Symbol('.')))
      && matches!(self.peek_n(1), Some(Token::Symbol('.')))
      && matches!(self.peek_n(2), Some(Token::Symbol('.')))
  }

  fn next_starts_expr(&self) -> bool {
    matches!(
      self.peek(),
      Some(Token::Int(_))
        | Some(Token::Float(_))
        | Some(Token::String(_))
        | Some(Token::StringInterp(_))
        | Some(Token::Path(_))  // N00h: Paths can be function arguments
        | Some(Token::PathInterp(_, _))  // and so can `${...}`-bearing paths
        | Some(Token::Ident(_))
        | Some(Token::Keyword("let"))
        | Some(Token::Keyword("if"))
        | Some(Token::Keyword("rec"))  // rec { } is an expression
        | Some(Token::Keyword("match"))  // N00i: match can be either match expr or variable
        // Nix-compat: `or`, `and` may appear as variable references
        // (deprecated but still parsed). Recognize them as expression
        // starters so application chains like `f g or` parse correctly.
        | Some(Token::Keyword("or"))
        | Some(Token::Keyword("and"))
        | Some(Token::Symbol('('))
        | Some(Token::Symbol('{'))
        | Some(Token::Symbol('['))
    )
  }

  /// N00i: Check if `match` keyword is used as a lambda parameter
  /// match: expr or match@{...}: expr
  fn is_match_lambda_param(&self) -> bool {
    match self.peek_n(1) {
      Some(Token::Symbol(':')) => true, // match: expr
      Some(Token::Symbol('@')) => {
        // match@{...}: expr - need to check for matching braces and colon
        if !matches!(self.peek_n(2), Some(Token::Symbol('{'))) {
          return false;
        }
        // Scan for matching } and then :
        let mut depth = 0usize;
        let mut i = self.pos + 2; // skip match and @
        while i < self.tokens.len() {
          match self.tokens.get(i).map(|tok| &tok.token) {
            Some(Token::Symbol('{')) => depth += 1,
            Some(Token::Symbol('}')) => {
              if depth == 1 {
                // Check if next token is :
                return matches!(
                  self.tokens.get(i + 1).map(|tok| &tok.token),
                  Some(Token::Symbol(':'))
                );
              }
              depth = depth.saturating_sub(1);
            }
            None => return false,
            _ => {}
          }
          i += 1;
        }
        false
      }
      _ => false,
    }
  }

  /// N00i: Check if `match` is NOT starting a match expression
  /// A match expression has the form: match <expr> with | <pattern> => <body>
  /// If we don't find `with` keyword before reaching a stopping token, it's a variable
  fn is_match_used_as_variable(&self) -> bool {
    // Look at the token after `match` (current is `match`)
    let next = self.peek_n(1);

    // Quick check: if followed by immediate operators/delimiters, it's definitely a variable
    if matches!(
      next,
      Some(Token::Op("!="))
        | Some(Token::Op("=="))
        | Some(Token::Op("<="))
        | Some(Token::Op(">="))
        | Some(Token::Op("&&"))
        | Some(Token::Op("||"))
        | Some(Token::Op("++"))
        | Some(Token::Op("//"))
        | Some(Token::Op("->"))
        | Some(Token::Op("<"))
        | Some(Token::Op(">"))
        | Some(Token::Op("+"))
        | Some(Token::Op("-"))
        | Some(Token::Op("*"))
        | Some(Token::Op("/"))
        | Some(Token::Symbol(')'))
        | Some(Token::Symbol(']'))
        | Some(Token::Symbol('}'))
        | Some(Token::Symbol(';'))
        | Some(Token::Symbol(','))
        | Some(Token::Symbol('.'))
        | Some(Token::Symbol('?'))
        | Some(Token::Keyword("then"))
        | Some(Token::Keyword("else"))
        | Some(Token::Keyword("in"))
        | None
    ) {
      return true;
    }

    // Look ahead for `with` keyword to confirm it's a match expression
    // Scan through tokens looking for `with` before any stopping tokens
    let mut depth = 0; // Track nesting of (), [], {}
    let mut i = 1; // Start after `match`

    loop {
      let tok = self.peek_n(i);
      match tok {
        None => return true, // EOF without `with` -> variable
        Some(Token::Keyword("with")) if depth == 0 => return false, // Found `with` -> match expr
        Some(Token::Symbol('(')) | Some(Token::Symbol('[')) | Some(Token::Symbol('{')) => {
          depth += 1;
        }
        Some(Token::Symbol(')')) | Some(Token::Symbol(']')) | Some(Token::Symbol('}')) => {
          if depth == 0 {
            return true; // Hit closing delimiter at top level -> variable
          }
          depth -= 1;
        }
        Some(Token::Symbol(';')) if depth == 0 => return true, // Semicolon at top level -> variable
        Some(Token::Keyword("then"))
        | Some(Token::Keyword("else"))
        | Some(Token::Keyword("in"))
          if depth == 0 =>
        {
          return true; // Other keywords at top level -> variable
        }
        _ => {}
      }
      i += 1;
      // Safety: don't scan too far
      if i > 100 {
        return true; // Give up, treat as variable
      }
    }
  }

  fn is_adjacent_index_bracket(&self) -> bool {
    let pos = self.current_pos();
    let Some(prev) = self.prev_char_at(pos) else {
      return false;
    };
    !prev.is_whitespace()
  }

  fn prev_char_at(&self, pos: usize) -> Option<char> {
    if pos == 0 {
      return None;
    }
    let mut idx = pos - 1;
    while idx > 0 && !self.source.is_char_boundary(idx) {
      idx -= 1;
    }
    self.source[idx..pos].chars().next()
  }

  fn peek(&self) -> Option<&Token> {
    self.tokens.get(self.pos).map(|tok| &tok.token)
  }

  fn peek_n(&self, n: usize) -> Option<&Token> {
    self.tokens.get(self.pos + n).map(|tok| &tok.token)
  }

  /// N00f: Check if next tokens form a dynamic attribute access: ${expr}
  fn check_dynamic_attr(&self) -> bool {
    matches!(
      (self.peek(), self.peek_n(1)),
      (Some(Token::Symbol('$')), Some(Token::Symbol('{')))
    )
  }

  /// Used by the `?` operator to keep eating `.<segment>` after a
  /// dynamic head, building up an attr_expr like
  /// `Lit("${a}") + Lit(".") + Lit("b") + Lit(".") + Lit("c")` so the
  /// evaluator can split on '.' and traverse a nested attrset.
  fn consume_dotted_dynamic_path_segments(
    &mut self,
    parts: &mut Vec<super::syntax::StringInterpPart>,
  ) -> Result<(), PnixError> {
    while matches!(self.peek(), Some(Token::Symbol('.'))) {
      self.pos += 1; // consume '.'
      parts.push(super::syntax::StringInterpPart::Lit(".".to_string()));
      match self.peek().cloned() {
        Some(Token::Ident(name)) => {
          self.pos += 1;
          parts.push(super::syntax::StringInterpPart::Lit(name));
        }
        Some(Token::Keyword(kw)) if kw == "match" || kw == "or" || kw == "and" => {
          self.pos += 1;
          parts.push(super::syntax::StringInterpPart::Lit(kw.to_string()));
        }
        Some(Token::String(s)) => {
          self.pos += 1;
          parts.push(super::syntax::StringInterpPart::Lit(s));
        }
        Some(Token::StringInterp(seg_parts)) => {
          self.pos += 1;
          for sp in seg_parts {
            match sp {
              LexerStringPart::Lit(s) => {
                parts.push(super::syntax::StringInterpPart::Lit(s));
              }
              LexerStringPart::InterpExpr(expr_src) => {
                let parsed = crate::lang::pnix::parse_expr(&expr_src)?;
                parts.push(super::syntax::StringInterpPart::Expr(Arc::new(parsed)));
              }
            }
          }
        }
        Some(Token::Symbol('$')) if self.peek_n(1) == Some(&Token::Symbol('{')) => {
          self.pos += 2; // consume `${`
          let inner = self.parse_expr()?;
          self.expect_symbol('}')?;
          parts.push(super::syntax::StringInterpPart::Expr(Arc::new(inner)));
        }
        _ => return Err(self.error_expected("attribute path segment")),
      }
    }
    Ok(())
  }

  /// N00f: Parse dynamic attribute expression: ${expr}
  /// Expects to be positioned at '$' when called
  fn parse_dynamic_attr(&mut self) -> Result<Expr, PnixError> {
    // Consume '$' and '{'
    self.expect_symbol('$')?;
    self.expect_symbol('{')?;
    // Parse the expression inside
    let expr = self.parse_expr()?;
    // Consume '}'
    self.expect_symbol('}')?;
    Ok(expr)
  }

  fn expect_symbol(&mut self, c: char) -> Result<(), PnixError> {
    match self.peek() {
      Some(Token::Symbol(ch)) if *ch == c => {
        self.pos += 1;
        Ok(())
      }
      _ => Err(self.error_expected(format!("symbol '{}'", c))),
    }
  }

  fn expect_op(&mut self, op: &'static str) -> Result<(), PnixError> {
    match self.peek() {
      Some(Token::Op(o)) if *o == op => {
        self.pos += 1;
        Ok(())
      }
      _ => Err(self.error_expected(format!("operator '{}'", op))),
    }
  }

  fn expect_keyword(&mut self, kw: &'static str) -> Result<(), PnixError> {
    match self.peek() {
      Some(Token::Keyword(k)) if *k == kw => {
        self.pos += 1;
        Ok(())
      }
      _ => Err(self.error_expected(format!("keyword '{}'", kw))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_let_multiple_bindings() {
    let expr = parse_expr("let x = 10; y = 20; in x + y").unwrap();
    match expr {
      Expr::Let { bindings, body } => {
        assert_eq!(bindings.len(), 2);
        match &bindings[0] {
          LetBinding::Binding { pattern, .. } => match pattern {
            ParamPattern::Ident(name) => assert_eq!(name, "x"),
            other => panic!("expected ident, got {:?}", other),
          },
          other => panic!("expected Binding, got {:?}", other),
        }
        match &bindings[1] {
          LetBinding::Binding { pattern, .. } => match pattern {
            ParamPattern::Ident(name) => assert_eq!(name, "y"),
            other => panic!("expected ident, got {:?}", other),
          },
          other => panic!("expected Binding, got {:?}", other),
        }
        assert!(matches!(*body, Expr::Binary { op: "+", .. }));
      }
      other => panic!("expected let, got {:?}", other),
    }
  }

  #[test]
  fn parse_scientific_float_literal() {
    let expr = parse_expr("1e10").unwrap();
    match expr {
      Expr::Float(v) => assert!((v - 1e10).abs() < 1e-6),
      other => panic!("expected float, got {:?}", other),
    }
  }

  #[test]
  fn parse_scientific_float_literal_with_fraction() {
    let expr = parse_expr("1.0e-3").unwrap();
    match expr {
      Expr::Float(v) => assert!((v - 1.0e-3).abs() < 1e-12),
      other => panic!("expected float, got {:?}", other),
    }
  }

  #[test]
  fn parse_scientific_float_pattern_literal() {
    use crate::lang::pnix::syntax::{PnixLiteralPattern, PnixPattern};
    let expr = parse_expr("match x with | 1e2 => 0").unwrap();
    match expr {
      Expr::Match { arms, .. } => {
        assert_eq!(arms.len(), 1);
        match &arms[0].pattern {
          PnixPattern::Literal(PnixLiteralPattern::Float(v)) => {
            assert!((v - 1e2).abs() < 1e-6);
          }
          other => panic!("expected float literal pattern, got {:?}", other),
        }
      }
      other => panic!("expected match, got {:?}", other),
    }
  }

  #[test]
  fn parse_base_int_literals() {
    let expr = parse_expr("0xFF").unwrap();
    assert!(matches!(expr, Expr::Int(255)));
    let expr = parse_expr("0o10").unwrap();
    assert!(matches!(expr, Expr::Int(8)));
    let expr = parse_expr("0b1010").unwrap();
    assert!(matches!(expr, Expr::Int(10)));
  }

  #[test]
  fn parse_float_leading_dot() {
    let expr = parse_expr(".5").unwrap();
    match expr {
      Expr::Float(v) => assert!((v - 0.5).abs() < 1e-12),
      other => panic!("expected float, got {:?}", other),
    }
  }

  #[test]
  fn parse_float_trailing_dot() {
    let expr = parse_expr("1.").unwrap();
    match expr {
      Expr::Float(v) => assert!((v - 1.0).abs() < 1e-12),
      other => panic!("expected float, got {:?}", other),
    }
  }

  #[test]
  fn parse_float_leading_dot_with_exponent() {
    let expr = parse_expr(".5e1").unwrap();
    match expr {
      Expr::Float(v) => assert!((v - 5.0).abs() < 1e-12),
      other => panic!("expected float, got {:?}", other),
    }
  }

  #[test]
  fn parse_float_trailing_dot_with_exponent() {
    let expr = parse_expr("1.e2").unwrap();
    match expr {
      Expr::Float(v) => assert!((v - 100.0).abs() < 1e-9),
      other => panic!("expected float, got {:?}", other),
    }
  }

  #[test]
  fn parse_lambda_with_pattern_default() {
    let expr = parse_expr("({ x, y ? 20 }: x) 1").unwrap();
    match expr {
      Expr::Apply { func, .. } => match *func {
        Expr::Lambda { param, .. } => match param {
          ParamPattern::AttrSet { fields, ellipsis } => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[1].name, "y");
            assert!(fields[1].default.is_some());
            assert!(!ellipsis);
          }
          _ => panic!("expected attrset param pattern"),
        },
        _ => panic!("expected lambda"),
      },
      other => panic!("expected apply, got {:?}", other),
    }
  }

  #[test]
  fn parse_let_attrset_destructuring() {
    let expr = parse_expr("let { x, y ? 2 } = point; in x").unwrap();
    match expr {
      Expr::Let { bindings, .. } => {
        assert_eq!(bindings.len(), 1);
        match &bindings[0] {
          LetBinding::Binding { pattern, .. } => match pattern {
            ParamPattern::AttrSet { fields, ellipsis } => {
              assert_eq!(fields.len(), 2);
              assert_eq!(fields[0].name, "x");
              assert!(fields[0].default.is_none());
              assert_eq!(fields[1].name, "y");
              assert!(fields[1].default.is_some());
              assert!(!ellipsis);
            }
            other => panic!("expected attrset pattern, got {:?}", other),
          },
          other => panic!("expected Binding, got {:?}", other),
        }
      }
      other => panic!("expected let, got {:?}", other),
    }
  }

  #[test]
  fn parse_let_list_destructuring() {
    let expr = parse_expr("let [head, ...tail] = xs; in head").unwrap();
    match expr {
      Expr::Let { bindings, .. } => {
        assert_eq!(bindings.len(), 1);
        match &bindings[0] {
          LetBinding::Binding { pattern, .. } => match pattern {
            ParamPattern::List(list) => {
              assert_eq!(list.items, vec!["head"]);
              assert_eq!(list.tail.as_deref(), Some("tail"));
            }
            other => panic!("expected list pattern, got {:?}", other),
          },
          other => panic!("expected Binding, got {:?}", other),
        }
      }
      other => panic!("expected let, got {:?}", other),
    }
  }

  #[test]
  fn parse_lambda_list_pattern() {
    let expr = parse_expr("([x, y, ...rest]: x) [1 2 3]").unwrap();
    match expr {
      Expr::Apply { func, .. } => match *func {
        Expr::Lambda { param, .. } => match param {
          ParamPattern::List(list) => {
            assert_eq!(list.items, vec!["x", "y"]);
            assert_eq!(list.tail.as_deref(), Some("rest"));
          }
          other => panic!("expected list pattern, got {:?}", other),
        },
        other => panic!("expected lambda, got {:?}", other),
      },
      other => panic!("expected apply, got {:?}", other),
    }
  }

  #[test]
  fn parse_attrset_inherit_and_assign() {
    let expr = parse_expr("{ inherit a b; c = 1; }").unwrap();
    match expr {
      Expr::AttrSet { items, .. } => {
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], AttrItem::Inherit { .. }));
        assert!(matches!(items[1], AttrItem::Assign { .. }));
      }
      other => panic!("expected attrset, got {:?}", other),
    }
  }

  #[test]
  fn parse_attrset_with_unicode_line_comment() {
    let expr = parse_expr("# 초딩: 한글 주석이 있어도 파싱돼야 함\n{ a = 1; }").unwrap();
    match expr {
      Expr::AttrSet { items, .. } => {
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], AttrItem::Assign { ref key_path, .. } if key_path == &["a"]));
      }
      other => panic!("expected attrset, got {:?}", other),
    }
  }

  #[test]
  fn parse_rec_attrset() {
    // N01a-0: rec { x = 1; y = x + 1; } should parse with recursive=true
    let expr = parse_expr("rec { x = 1; y = x + 1; }").unwrap();
    match expr {
      Expr::AttrSet { items, recursive } => {
        assert!(recursive, "rec attrset should have recursive=true");
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], AttrItem::Assign { ref key_path, .. } if key_path == &["x"]));
        assert!(matches!(items[1], AttrItem::Assign { ref key_path, .. } if key_path == &["y"]));
      }
      other => panic!("expected attrset, got {:?}", other),
    }
  }

  #[test]
  fn parse_non_rec_attrset() {
    // Non-recursive attrset should have recursive=false
    let expr = parse_expr("{ x = 1; }").unwrap();
    match expr {
      Expr::AttrSet { items, recursive } => {
        assert!(!recursive, "non-rec attrset should have recursive=false");
        assert_eq!(items.len(), 1);
      }
      other => panic!("expected attrset, got {:?}", other),
    }
  }

  #[test]
  fn parse_import_string() {
    // N01a: import "./file.nix" should parse
    let expr = parse_expr(r#"import "./lib.nix""#).unwrap();
    match expr {
      Expr::Import { path } => {
        assert!(matches!(*path, Expr::String(ref s) if s == "./lib.nix"));
      }
      other => panic!("expected import, got {:?}", other),
    }
  }

  #[test]
  fn parse_import_variable() {
    // N01a: import path where path is a variable
    let expr = parse_expr("import myPath").unwrap();
    match expr {
      Expr::Import { path } => {
        assert!(matches!(*path, Expr::Var(ref name) if name == "myPath"));
      }
      other => panic!("expected import, got {:?}", other),
    }
  }

  #[test]
  fn parse_string_interp_simple() {
    // Y10a: "hello ${name}!" should parse as StringInterp
    use crate::lang::pnix::syntax::StringInterpPart;
    let expr = parse_expr(r#""hello ${name}!""#).unwrap();
    match expr {
      Expr::StringInterp(parts) => {
        assert_eq!(parts.len(), 3);
        assert!(matches!(&parts[0], StringInterpPart::Lit(s) if s == "hello "));
        assert!(
          matches!(&parts[1], StringInterpPart::Expr(e) if matches!(**e, Expr::Var(ref n) if n == "name"))
        );
        assert!(matches!(&parts[2], StringInterpPart::Lit(s) if s == "!"));
      }
      other => panic!("expected StringInterp, got {:?}", other),
    }
  }

  #[test]
  fn parse_string_interp_utf8_expr() {
    // UTF-8 interpolation should slice on valid byte boundaries.
    use crate::lang::pnix::syntax::StringInterpPart;
    let expr = parse_expr(r#""안녕 ${값} 세계""#).unwrap();
    match expr {
      Expr::StringInterp(parts) => {
        assert_eq!(parts.len(), 3);
        assert!(matches!(&parts[0], StringInterpPart::Lit(s) if s == "안녕 "));
        assert!(
          matches!(&parts[1], StringInterpPart::Expr(e) if matches!(**e, Expr::Var(ref n) if n == "값"))
        );
        assert!(matches!(&parts[2], StringInterpPart::Lit(s) if s == " 세계"));
      }
      other => panic!("expected StringInterp, got {:?}", other),
    }
  }

  #[test]
  fn parse_string_no_interp() {
    // "hello" without ${} should be plain String
    let expr = parse_expr(r#""hello""#).unwrap();
    assert!(matches!(expr, Expr::String(s) if s == "hello"));
  }

  #[test]
  fn parse_string_interp_escaped() {
    // Y10a: "\${name}" should be plain String (escaped $)
    let expr = parse_expr(r#""\${name}""#).unwrap();
    assert!(matches!(expr, Expr::String(s) if s == "${name}"));
  }

  #[test]
  fn parse_string_interp_expr() {
    // Y10a: "result: ${1 + 2}" should parse expression inside
    use crate::lang::pnix::syntax::StringInterpPart;
    let expr = parse_expr(r#""result: ${1 + 2}""#).unwrap();
    match expr {
      Expr::StringInterp(parts) => {
        assert_eq!(parts.len(), 2);
        assert!(matches!(&parts[0], StringInterpPart::Lit(s) if s == "result: "));
        match &parts[1] {
          StringInterpPart::Expr(e) => {
            assert!(matches!(**e, Expr::Binary { op: "+", .. }));
          }
          _ => panic!("expected Expr part"),
        }
      }
      other => panic!("expected StringInterp, got {:?}", other),
    }
  }

  #[test]
  fn parse_string_interp_nested_multiline() {
    use crate::lang::pnix::syntax::StringInterpPart;
    let expr = parse_expr(r#""before ${''inner } content''} after""#).unwrap();
    match expr {
      Expr::StringInterp(parts) => {
        assert_eq!(parts.len(), 3);
        assert!(matches!(&parts[0], StringInterpPart::Lit(s) if s == "before "));
        assert!(
          matches!(&parts[1], StringInterpPart::Expr(e) if matches!(**e, Expr::String(ref s) if s == "inner } content"))
        );
        assert!(matches!(&parts[2], StringInterpPart::Lit(s) if s == " after"));
      }
      other => panic!("expected StringInterp, got {:?}", other),
    }
  }

  #[test]
  fn parse_string_interp_with_block_comment() {
    use crate::lang::pnix::syntax::StringInterpPart;
    let expr = parse_expr(r#""value ${1 /* } */ + 2}""#).unwrap();
    match expr {
      Expr::StringInterp(parts) => {
        assert_eq!(parts.len(), 2);
        assert!(matches!(&parts[0], StringInterpPart::Lit(s) if s == "value "));
        match &parts[1] {
          StringInterpPart::Expr(e) => {
            assert!(matches!(**e, Expr::Binary { op: "+", .. }));
          }
          _ => panic!("expected Expr part"),
        }
      }
      other => panic!("expected StringInterp, got {:?}", other),
    }
  }

  #[test]
  fn parse_merge_simple() {
    // Y10c: a // b should parse as Binary with //
    let expr = parse_expr("a // b").unwrap();
    match expr {
      Expr::Binary { op, lhs, rhs } => {
        assert_eq!(op, "//");
        assert!(matches!(*lhs, Expr::Var(ref n) if n == "a"));
        assert!(matches!(*rhs, Expr::Var(ref n) if n == "b"));
      }
      other => panic!("expected Binary //, got {:?}", other),
    }
  }

  #[test]
  fn parse_merge_right_assoc() {
    // Y10c: a // b // c should be right-associative: a // (b // c)
    let expr = parse_expr("a // b // c").unwrap();
    match expr {
      Expr::Binary { op, lhs, rhs } => {
        assert_eq!(op, "//");
        assert!(matches!(*lhs, Expr::Var(ref n) if n == "a"));
        // rhs should be b // c
        match *rhs {
          Expr::Binary {
            op: op2,
            lhs: lhs2,
            rhs: rhs2,
          } => {
            assert_eq!(op2, "//");
            assert!(matches!(*lhs2, Expr::Var(ref n) if n == "b"));
            assert!(matches!(*rhs2, Expr::Var(ref n) if n == "c"));
          }
          other => panic!("expected nested Binary //, got {:?}", other),
        }
      }
      other => panic!("expected Binary //, got {:?}", other),
    }
  }

  #[test]
  fn parse_with_simple() {
    // with <env>; <body> syntax
    let expr = parse_expr("with pkgs; [ gcc make ]").unwrap();
    match expr {
      Expr::With { env, body } => {
        assert!(matches!(*env, Expr::Var(ref n) if n == "pkgs"));
        assert!(matches!(*body, Expr::List(_)));
      }
      other => panic!("expected With, got {:?}", other),
    }
  }

  #[test]
  fn parse_with_nested() {
    // nested with expressions
    let expr = parse_expr("with a; with b; x").unwrap();
    match expr {
      Expr::With { env, body } => {
        assert!(matches!(*env, Expr::Var(ref n) if n == "a"));
        match *body {
          Expr::With {
            env: env2,
            body: body2,
          } => {
            assert!(matches!(*env2, Expr::Var(ref n) if n == "b"));
            assert!(matches!(*body2, Expr::Var(ref n) if n == "x"));
          }
          other => panic!("expected nested With, got {:?}", other),
        }
      }
      other => panic!("expected With, got {:?}", other),
    }
  }

  #[test]
  fn parse_with_attrset_env() {
    // with { x = 1; }; x + 1
    let expr = parse_expr("with { x = 1; }; x + 1").unwrap();
    match expr {
      Expr::With { env, body } => {
        assert!(matches!(*env, Expr::AttrSet { .. }));
        assert!(matches!(*body, Expr::Binary { op: "+", .. }));
      }
      other => panic!("expected With, got {:?}", other),
    }
  }

  #[test]
  fn parse_at_pattern_prefix() {
    // args@{x, y}: body - bind the whole attrset and destructure
    use crate::lang::pnix::syntax::PnixParamPattern;
    let expr = parse_expr("args@{x, y}: args.x + y").unwrap();
    match expr {
      Expr::Lambda { param, body } => {
        match param {
          PnixParamPattern::AttrSetWithBind {
            bind_name, fields, ..
          } => {
            assert_eq!(bind_name, "args");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[1].name, "y");
          }
          other => panic!("expected AttrSetWithBind, got {:?}", other),
        }
        assert!(matches!(*body, Expr::Binary { op: "+", .. }));
      }
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn parse_at_pattern_suffix() {
    // {x, y}@args: body - same as prefix, different syntax
    use crate::lang::pnix::syntax::PnixParamPattern;
    let expr = parse_expr("{x, y}@args: args.x + y").unwrap();
    match expr {
      Expr::Lambda { param, body } => {
        match param {
          PnixParamPattern::AttrSetWithBind {
            bind_name, fields, ..
          } => {
            assert_eq!(bind_name, "args");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[1].name, "y");
          }
          other => panic!("expected AttrSetWithBind, got {:?}", other),
        }
        assert!(matches!(*body, Expr::Binary { op: "+", .. }));
      }
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn parse_at_pattern_with_defaults() {
    // args@{x, y ? 0}: body - with default values
    use crate::lang::pnix::syntax::PnixParamPattern;
    let expr = parse_expr("args@{x, y ? 0}: x + y").unwrap();
    match expr {
      Expr::Lambda { param, .. } => match param {
        PnixParamPattern::AttrSetWithBind {
          bind_name, fields, ..
        } => {
          assert_eq!(bind_name, "args");
          assert_eq!(fields.len(), 2);
          assert_eq!(fields[0].name, "x");
          assert!(fields[0].default.is_none());
          assert_eq!(fields[1].name, "y");
          assert!(fields[1].default.is_some());
        }
        other => panic!("expected AttrSetWithBind, got {:?}", other),
      },
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn parse_select_chain() {
    let expr = parse_expr("a.b.c").unwrap();
    match expr {
      Expr::Select { base, attr } => {
        assert_eq!(attr, "c");
        match *base {
          Expr::Select { base, attr } => {
            assert_eq!(attr, "b");
            assert!(matches!(*base, Expr::Var(ref v) if v == "a"));
          }
          other => panic!("expected nested select, got {:?}", other),
        }
      }
      other => panic!("expected select, got {:?}", other),
    }
  }

  #[test]
  fn parse_if_operator_precedence() {
    let expr = parse_expr("if true then 1 else 2 + 3 * 4").unwrap();
    match expr {
      Expr::If { else_, .. } => match *else_ {
        Expr::Binary { op: "+", lhs, rhs } => {
          assert!(matches!(*lhs, Expr::Int(2)));
          assert!(matches!(*rhs, Expr::Binary { op: "*", .. }));
        }
        other => panic!("expected binary +, got {:?}", other),
      },
      other => panic!("expected if, got {:?}", other),
    }
  }

  #[test]
  fn parse_logical_and_or_precedence() {
    let expr = parse_expr("1 < 2 && 3 < 4 || 5 < 6").unwrap();
    match expr {
      Expr::Binary { op: "||", lhs, rhs } => {
        assert!(matches!(*rhs, Expr::Binary { op: "<", .. }));
        match *lhs {
          Expr::Binary { op: "&&", lhs, rhs } => {
            assert!(matches!(*lhs, Expr::Binary { op: "<", .. }));
            assert!(matches!(*rhs, Expr::Binary { op: "<", .. }));
          }
          other => panic!("expected binary &&, got {:?}", other),
        }
      }
      other => panic!("expected binary ||, got {:?}", other),
    }
  }

  #[test]
  fn parse_lambda_apply_to_empty_attrset() {
    let expr = parse_expr("({ x }: x) {}").unwrap();
    match expr {
      Expr::Apply { func, arg } => {
        assert!(matches!(*arg, Expr::AttrSet { items, .. } if items.is_empty()));
        assert!(matches!(*func, Expr::Lambda { .. }));
      }
      other => panic!("expected apply, got {:?}", other),
    }
  }

  #[test]
  fn parse_apply_with_list_argument() {
    let expr = parse_expr("map (x: x) [1 2]").unwrap();
    match expr {
      Expr::Apply { func, arg } => {
        assert!(matches!(*arg, Expr::List(_)));
        assert!(matches!(*func, Expr::Apply { .. }));
      }
      other => panic!("expected apply, got {:?}", other),
    }
  }

  #[test]
  fn parse_apply_with_list_literal_argument() {
    let expr = parse_expr("f [1 2 3]").unwrap();
    match expr {
      Expr::Apply { func, arg } => {
        assert!(matches!(*func, Expr::Var(ref name) if name == "f"));
        match *arg {
          Expr::List(items) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], Expr::Int(1)));
            assert!(matches!(items[1], Expr::Int(2)));
            assert!(matches!(items[2], Expr::Int(3)));
          }
          other => panic!("expected list arg, got {:?}", other),
        }
      }
      other => panic!("expected apply, got {:?}", other),
    }
  }

  #[test]
  fn parse_apply_with_select_and_list_argument() {
    let expr = parse_expr("builtins.length [1 2 3]").unwrap();
    match expr {
      Expr::Apply { func, arg } => {
        match *func {
          Expr::Select { base, attr } => {
            assert_eq!(attr, "length");
            assert!(matches!(*base, Expr::Var(ref name) if name == "builtins"));
          }
          other => panic!("expected select func, got {:?}", other),
        }
        assert!(matches!(*arg, Expr::List(_)));
      }
      other => panic!("expected apply, got {:?}", other),
    }
  }

  #[test]
  fn parse_lambda_pattern_trailing_comma_allowed() {
    // N00j: Trailing comma is allowed in Nix attrset patterns
    let expr = parse_expr("({ x, }: x) 1").unwrap();
    match expr {
      Expr::Apply { func, .. } => match *func {
        Expr::Lambda { param, .. } => match param {
          ParamPattern::AttrSet { fields, ellipsis } => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "x");
            assert!(!ellipsis);
          }
          other => panic!("expected attrset param pattern, got {:?}", other),
        },
        other => panic!("expected lambda, got {:?}", other),
      },
      other => panic!("expected apply, got {:?}", other),
    }
  }

  #[test]
  fn parse_lambda_pattern_requires_commas_between_fields() {
    let err = parse_expr("({ x y }: x) 1").unwrap_err().to_string();
    assert!(err.contains("Parse error at line"), "err={}", err);
    assert!(
      err.contains("expected ',' between pattern fields"),
      "err={}",
      err
    );
  }

  #[test]
  fn parse_lambda_list_pattern_trailing_comma_is_error() {
    let err = parse_expr("([x,]: x) 1").unwrap_err().to_string();
    assert!(err.contains("Parse error at line"), "err={}", err);
    assert!(err.contains("expected list pattern name"), "err={}", err);
  }

  #[test]
  fn parse_unicode_identifier() {
    let expr = parse_expr("\u{03BB}x: \u{03BB}x").unwrap();
    match expr {
      Expr::Lambda { param, body } => {
        match param {
          ParamPattern::Ident(name) => assert_eq!(name, "\u{03BB}x"),
          other => panic!("expected ident, got {:?}", other),
        }
        match *body {
          Expr::Var(name) => assert_eq!(name, "\u{03BB}x"),
          other => panic!("expected var, got {:?}", other),
        }
      }
      other => panic!("expected lambda, got {:?}", other),
    }
  }

  #[test]
  fn test_unary_negate() {
    // Y08d: Unary 연산자 -x 파싱 테스트
    let expr = parse_expr("-5").unwrap();
    match expr {
      Expr::Unary { op, arg } => {
        assert_eq!(op, "-");
        assert!(matches!(*arg, Expr::Int(5)));
      }
      other => panic!("expected Unary, got {:?}", other),
    }
  }

  #[test]
  fn test_unary_not() {
    // Y08d: Unary 연산자 !x 파싱 테스트
    let expr = parse_expr("!true").unwrap();
    match expr {
      Expr::Unary { op, arg } => {
        assert_eq!(op, "!");
        assert!(matches!(*arg, Expr::Bool(true)));
      }
      other => panic!("expected Unary, got {:?}", other),
    }
  }

  #[test]
  fn test_unary_operator_precedence() {
    // Parser unary 연산자 우선순위 버그 수정 테스트
    // -1 + 2는 (-1) + 2로 파싱되어야 함 (not -(1+2))
    let expr = parse_expr("-1 + 2").unwrap();
    match expr {
      Expr::Binary { op, lhs, rhs } => {
        assert_eq!(op, "+");
        // lhs는 Unary(-, 1)이어야 함
        match *lhs {
          Expr::Unary { op: unary_op, arg } => {
            assert_eq!(unary_op, "-");
            assert!(matches!(*arg, Expr::Int(1)));
          }
          other => panic!("expected Unary(-, 1), got {:?}", other),
        }
        // rhs는 Int(2)여야 함
        assert!(matches!(*rhs, Expr::Int(2)));
      }
      other => panic!("expected Binary(+, Unary(-, 1), 2), got {:?}", other),
    }
  }

  #[test]
  fn test_list_with_arithmetic() {
    // Parser 리스트 내 산술 연산 버그 수정 테스트
    // [1 + 2]는 [3]으로 파싱되어야 함 (not [1, +, 2])
    let expr = parse_expr("[1 + 2]").unwrap();
    match expr {
      Expr::List(elements) => {
        assert_eq!(elements.len(), 1, "list should have 1 element");
        match &elements[0] {
          Expr::Binary { op, lhs, rhs } => {
            assert_eq!(*op, "+");
            assert!(matches!(**lhs, Expr::Int(1)));
            assert!(matches!(**rhs, Expr::Int(2)));
          }
          other => panic!("expected Binary(+, 1, 2), got {:?}", other),
        }
      }
      other => panic!("expected List([Binary(+, 1, 2)]), got {:?}", other),
    }
  }

  #[test]
  fn test_unary_nested() {
    // Y08d: 중첩된 unary 연산자 테스트
    let expr = parse_expr("--5").unwrap();
    match expr {
      Expr::Unary { op, arg } => {
        assert_eq!(op, "-");
        match *arg {
          Expr::Unary {
            op: inner_op,
            arg: inner_arg,
          } => {
            assert_eq!(inner_op, "-");
            assert!(matches!(*inner_arg, Expr::Int(5)));
          }
          other => panic!("expected nested Unary, got {:?}", other),
        }
      }
      other => panic!("expected Unary, got {:?}", other),
    }
  }

  #[test]
  fn test_string_escape_unicode_braced() {
    // Y05c-13: Unicode escape \u{XXXX} 파싱 테스트 (오프바이원 버그 수정)
    let expr = parse_expr(r#""\u{0041}""#).unwrap();
    match expr {
      Expr::String(s) => assert_eq!(s, "A", "Unicode escape \\u{{0041}} should parse to 'A'"),
      other => panic!("expected String, got {:?}", other),
    }
  }

  #[test]
  fn test_string_escape_unicode_4digit() {
    // Y05c-13: Unicode escape \uXXXX 파싱 테스트 (오프바이원 버그 수정)
    let expr = parse_expr(r#""\u0041""#).unwrap();
    match expr {
      Expr::String(s) => assert_eq!(s, "A", "Unicode escape \\u0041 should parse to 'A'"),
      other => panic!("expected String, got {:?}", other),
    }
  }

  #[test]
  fn test_string_escape_hex() {
    // Y05c-13: Hex escape \xXX 파싱 테스트 (오프바이원 버그 수정)
    let expr = parse_expr(r#""\x41""#).unwrap();
    match expr {
      Expr::String(s) => assert_eq!(s, "A", "Hex escape \\x41 should parse to 'A'"),
      other => panic!("expected String, got {:?}", other),
    }
  }

  #[test]
  fn test_string_escape_unicode_surrogate_pair() {
    // Y05c-13: Unicode surrogate pair \uXXXX\uXXXX 파싱 테스트
    // U+1F600 (😀) = \uD83D\uDE00
    let expr = parse_expr(r#""\uD83D\uDE00""#).unwrap();
    match expr {
      Expr::String(s) => assert_eq!(s, "😀", "Unicode surrogate pair should parse to 😀"),
      other => panic!("expected String, got {:?}", other),
    }
  }

  #[test]
  fn test_string_escape_unicode_braced_large() {
    // Y05c-13: 큰 Unicode code point \u{1F600} 파싱 테스트
    let expr = parse_expr(r#""\u{1F600}""#).unwrap();
    match expr {
      Expr::String(s) => assert_eq!(
        s, "😀",
        "Large Unicode escape \\u{{1F600}} should parse to 😀"
      ),
      other => panic!("expected String, got {:?}", other),
    }
  }

  #[test]
  fn test_multiline_string_basic() {
    // Y05c-14: multiline string ''...'' 파싱 테스트
    let expr = parse_expr("''hello\nworld''").unwrap();
    match expr {
      Expr::String(s) => {
        assert_eq!(s, "hello\nworld", "Multiline string should parse correctly");
      }
      other => panic!("expected String, got {:?}", other),
    }
  }

  #[test]
  fn test_multiline_string_escape() {
    // Nix multiline string escape: N consecutive quotes (N >= 3) produce (N-1) quotes
    // '''' (4 quotes) -> ''' (3 quotes)
    let expr = parse_expr("''hello''''world''").unwrap();
    match expr {
      Expr::String(s) => {
        assert_eq!(
          s, "hello'''world",
          "Multiline string '''' (4 quotes) should produce ''' (3 quotes)"
        );
      }
      other => panic!("expected String, got {:?}", other),
    }
  }

  #[test]
  fn test_multiline_string_starts_with_single_quote() {
    // Y05c-14: multiline string이 ''로 시작하는지 확인 (토큰화 시작 조건 수정)
    // 이전에는 " 분기에서만 확인했지만, 실제로는 ' 분기에서 확인해야 함
    let expr = parse_expr("''test''").unwrap();
    match expr {
      Expr::String(s) => {
        assert_eq!(
          s, "test",
          "Multiline string starting with '' should parse correctly"
        );
      }
      other => panic!("expected String, got {:?}", other),
    }
  }

  #[test]
  fn test_let_without_semicolon_is_error() {
    // Nix 문법: let x = 1; in x (세미콜론 필수)
    // 세미콜론 없는 경우 에러 발생해야 함
    let result = parse_expr("let x = 1 in x");
    assert!(
      result.is_err(),
      "let without semicolon should fail: {:?}",
      result
    );

    let err = result.unwrap_err().to_string();
    assert!(
      err.contains("symbol ';'"),
      "error should mention missing ';': {}",
      err
    );
  }

  #[test]
  fn test_let_with_semicolon_is_ok() {
    // Nix 문법: let x = 1; in x (세미콜론 필수)
    let result = parse_expr("let x = 1; in x");
    assert!(
      result.is_ok(),
      "let with semicolon should succeed: {:?}",
      result
    );
  }

  #[test]
  fn test_path_relative() {
    // Y10b: 상대 경로 리터럴 ./foo, ../parent
    use crate::lang::pnix::syntax::PnixPath;

    let expr = parse_expr("./foo").unwrap();
    match expr {
      Expr::Path(PnixPath::Relative(s)) => assert_eq!(s, "./foo"),
      other => panic!("expected Path(Relative), got {:?}", other),
    }

    let expr2 = parse_expr("../parent/file.nix").unwrap();
    match expr2 {
      Expr::Path(PnixPath::Relative(s)) => assert_eq!(s, "../parent/file.nix"),
      other => panic!("expected Path(Relative), got {:?}", other),
    }
  }

  #[test]
  fn test_path_absolute() {
    // Y10b: 절대 경로 리터럴 /etc/nixos
    use crate::lang::pnix::syntax::PnixPath;

    let expr = parse_expr("/etc/nixos").unwrap();
    match expr {
      Expr::Path(PnixPath::Absolute(s)) => assert_eq!(s, "/etc/nixos"),
      other => panic!("expected Path(Absolute), got {:?}", other),
    }
  }

  #[test]
  fn test_path_search() {
    // Y10b: 검색 경로 리터럴 <nixpkgs>, <nixpkgs/lib>
    use crate::lang::pnix::syntax::PnixPath;

    let expr = parse_expr("<nixpkgs>").unwrap();
    match expr {
      Expr::Path(PnixPath::Search(s)) => assert_eq!(s, "nixpkgs"),
      other => panic!("expected Path(Search), got {:?}", other),
    }

    let expr2 = parse_expr("<nixpkgs/lib>").unwrap();
    match expr2 {
      Expr::Path(PnixPath::Search(s)) => assert_eq!(s, "nixpkgs/lib"),
      other => panic!("expected Path(Search), got {:?}", other),
    }
  }

  #[test]
  fn test_path_home() {
    // Y10b: 홈 경로 리터럴 ~/foo, ~/.config
    use crate::lang::pnix::syntax::PnixPath;

    let expr = parse_expr("~/.config").unwrap();
    match expr {
      Expr::Path(PnixPath::Home(s)) => assert_eq!(s, "~/.config"),
      other => panic!("expected Path(Home), got {:?}", other),
    }
  }

  #[test]
  fn test_select_or_default() {
    // x.y or default - attribute selection with fallback
    let expr = parse_expr("x.y or 42").unwrap();
    match expr {
      Expr::SelectOrDefault {
        base,
        attr,
        default,
      } => {
        assert!(matches!(*base, Expr::Var(ref v) if v == "x"));
        assert_eq!(attr, "y");
        assert!(matches!(*default, Expr::Int(42)));
      }
      other => panic!("expected SelectOrDefault, got {:?}", other),
    }
  }

  #[test]
  fn test_select_or_default_chained() {
    // x.y.z or default - chained attribute selection with fallback
    let expr = parse_expr("x.y.z or null").unwrap();
    match expr {
      Expr::SelectOrDefault {
        base,
        attr,
        default,
      } => {
        // base should be x.y (a Select)
        assert!(matches!(*base, Expr::Select { .. }));
        assert_eq!(attr, "z");
        assert!(matches!(*default, Expr::Null));
      }
      other => panic!("expected SelectOrDefault, got {:?}", other),
    }
  }

  #[test]
  fn test_select_or_default_complex() {
    // x.y or { a = 1; } - complex default expression
    let expr = parse_expr("x.y or { a = 1; }").unwrap();
    match expr {
      Expr::SelectOrDefault {
        base,
        attr,
        default,
      } => {
        assert!(matches!(*base, Expr::Var(ref v) if v == "x"));
        assert_eq!(attr, "y");
        assert!(matches!(*default, Expr::AttrSet { .. }));
      }
      other => panic!("expected SelectOrDefault, got {:?}", other),
    }
  }

  #[test]
  fn test_assert_simple() {
    // Y10e: assert cond; body
    let expr = parse_expr("assert true; 42").unwrap();
    match expr {
      Expr::Assert { cond, body } => {
        assert!(matches!(*cond, Expr::Bool(true)));
        assert!(matches!(*body, Expr::Int(42)));
      }
      other => panic!("expected Assert, got {:?}", other),
    }
  }

  #[test]
  fn test_assert_with_comparison() {
    // assert x > 0; x
    let expr = parse_expr("assert x > 0; x").unwrap();
    match expr {
      Expr::Assert { cond, body } => {
        assert!(matches!(*cond, Expr::Binary { .. }));
        assert!(matches!(*body, Expr::Var(ref v) if v == "x"));
      }
      other => panic!("expected Assert, got {:?}", other),
    }
  }

  #[test]
  fn test_assert_nested() {
    // assert cond1; assert cond2; body
    let expr = parse_expr("assert a; assert b; c").unwrap();
    match expr {
      Expr::Assert { cond, body } => {
        assert!(matches!(*cond, Expr::Var(ref v) if v == "a"));
        // body should be another Assert
        match *body {
          Expr::Assert {
            cond: cond2,
            body: body2,
          } => {
            assert!(matches!(*cond2, Expr::Var(ref v) if v == "b"));
            assert!(matches!(*body2, Expr::Var(ref v) if v == "c"));
          }
          other => panic!("expected nested Assert, got {:?}", other),
        }
      }
      other => panic!("expected Assert, got {:?}", other),
    }
  }

  #[test]
  fn test_with_simple() {
    // with pkgs; gcc
    let expr = parse_expr("with pkgs; gcc").unwrap();
    match expr {
      Expr::With { env, body } => {
        assert!(matches!(*env, Expr::Var(ref v) if v == "pkgs"));
        assert!(matches!(*body, Expr::Var(ref v) if v == "gcc"));
      }
      other => panic!("expected With, got {:?}", other),
    }
  }

  #[test]
  fn test_with_attrset_env() {
    // with { x = 1; }; x
    let expr = parse_expr("with { x = 1; }; x").unwrap();
    match expr {
      Expr::With { env, body } => {
        assert!(matches!(*env, Expr::AttrSet { .. }));
        assert!(matches!(*body, Expr::Var(ref v) if v == "x"));
      }
      other => panic!("expected With, got {:?}", other),
    }
  }

  #[test]
  fn test_invalid_escape_sequence() {
    // Invalid escape sequence should produce error
    let result = parse_expr(r#""\q""#);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = format!("{}", err);
    assert!(
      err_str.contains("Invalid escape sequence"),
      "Expected 'Invalid escape sequence', got: {}",
      err_str
    );
    assert!(
      err_str.contains("\\q"),
      "Expected '\\q' in error, got: {}",
      err_str
    );
  }

  #[test]
  fn test_valid_escape_sequences() {
    // Valid escape sequences should work
    let expr = parse_expr(r#""\n\r\t\\\"\'""#).unwrap();
    match expr {
      Expr::String(s) => {
        assert_eq!(s, "\n\r\t\\\"'");
      }
      other => panic!("expected String, got {:?}", other),
    }
  }

  #[test]
  fn test_escape_null_char() {
    // Null character escape
    let expr = parse_expr(r#""\0""#).unwrap();
    match expr {
      Expr::String(s) => {
        assert_eq!(s, "\0");
      }
      other => panic!("expected String, got {:?}", other),
    }
  }

  #[test]
  fn test_escape_dollar() {
    // Dollar sign escape (useful in interpolated strings)
    let expr = parse_expr(r#""\$name""#).unwrap();
    match expr {
      Expr::String(s) => {
        assert_eq!(s, "$name");
      }
      other => panic!("expected String, got {:?}", other),
    }
  }

  #[test]
  fn test_block_comment_simple() {
    // N00a: Simple block comment
    let expr = parse_expr("/* comment */ 42").unwrap();
    assert!(matches!(expr, Expr::Int(42)));
  }

  #[test]
  fn test_block_comment_multiline() {
    // N00a: Multiline block comment (doc comment style)
    let expr = parse_expr("/**\n  This is a doc comment\n*/ 42").unwrap();
    assert!(matches!(expr, Expr::Int(42)));
  }

  #[test]
  fn test_block_comment_not_nested() {
    // Nix-compat: block comments do NOT nest. The first `*/` closes the
    // comment, regardless of any inner `/*`. This test pins that
    // behavior — `/* outer /* inner */` already terminates at the first
    // `*/`, so `still outer` becomes live tokens.
    let expr = parse_expr("/* /* foo */ 42").unwrap();
    assert!(matches!(expr, Expr::Int(42)));
  }

  #[test]
  fn test_block_comment_inline() {
    // N00a: Inline block comment
    let expr = parse_expr("let x /* comment */ = 1; in x").unwrap();
    match expr {
      Expr::Let { bindings, .. } => {
        assert_eq!(bindings.len(), 1);
        // Check the pattern is Ident("x")
        match &bindings[0] {
          LetBinding::Binding { pattern, .. } => {
            assert!(matches!(pattern, ParamPattern::Ident(ref s) if s == "x"));
          }
          other => panic!("expected Binding, got {:?}", other),
        }
      }
      other => panic!("expected Let, got {:?}", other),
    }
  }

  #[test]
  fn test_block_comment_unclosed() {
    // N00a: Unclosed block comment should error
    let result = parse_expr("/* unclosed comment 42");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("unterminated"));
  }

  #[test]
  fn test_inherit_in_let() {
    // N00b: inherit x y; in let expression
    let expr = parse_expr("let x = 1; inherit y z; in x + y + z").unwrap();
    match expr {
      Expr::Let { bindings, .. } => {
        assert_eq!(bindings.len(), 2);
        // First binding: x = 1
        match &bindings[0] {
          LetBinding::Binding { pattern, .. } => {
            assert!(matches!(pattern, ParamPattern::Ident(ref s) if s == "x"));
          }
          other => panic!("expected Binding for x, got {:?}", other),
        }
        // Second binding: inherit y z
        match &bindings[1] {
          LetBinding::Inherit { from, names } => {
            assert!(from.is_none());
            assert_eq!(names.len(), 2);
            assert_eq!(names[0], "y");
            assert_eq!(names[1], "z");
          }
          other => panic!("expected Inherit, got {:?}", other),
        }
      }
      other => panic!("expected Let, got {:?}", other),
    }
  }

  #[test]
  fn test_inherit_from_scope_in_let() {
    // N00b: inherit (scope) x y; in let expression
    let expr = parse_expr("let inherit (pkgs) gcc make; in gcc").unwrap();
    match expr {
      Expr::Let { bindings, .. } => {
        assert_eq!(bindings.len(), 1);
        match &bindings[0] {
          LetBinding::Inherit { from, names } => {
            // from should be Some(Var("pkgs"))
            assert!(from.is_some());
            if let Some(scope_expr) = from {
              assert!(matches!(**scope_expr, Expr::Var(ref s) if s == "pkgs"));
            }
            assert_eq!(names.len(), 2);
            assert_eq!(names[0], "gcc");
            assert_eq!(names[1], "make");
          }
          other => panic!("expected Inherit, got {:?}", other),
        }
      }
      other => panic!("expected Let, got {:?}", other),
    }
  }

  #[test]
  fn test_inherit_in_attrset() {
    // N00b: inherit x y; in attrset
    use crate::lang::pnix::syntax::PnixAttrItem;
    let expr = parse_expr("{ a = 1; inherit x y; }").unwrap();
    match expr {
      Expr::AttrSet { items, recursive } => {
        assert!(!recursive);
        assert_eq!(items.len(), 2);
        // First item: a = 1
        match &items[0] {
          PnixAttrItem::Assign { key_path, .. } => {
            assert_eq!(key_path, &["a"]);
          }
          other => panic!("expected Assign, got {:?}", other),
        }
        // Second item: inherit x y
        match &items[1] {
          PnixAttrItem::Inherit { from, names, .. } => {
            assert!(from.is_none());
            assert_eq!(names.len(), 2);
            assert_eq!(names[0], "x");
            assert_eq!(names[1], "y");
          }
          other => panic!("expected Inherit, got {:?}", other),
        }
      }
      other => panic!("expected AttrSet, got {:?}", other),
    }
  }

  #[test]
  fn test_inherit_from_scope_in_attrset() {
    // N00b: inherit (scope) x y; in attrset
    use crate::lang::pnix::syntax::PnixAttrItem;
    let expr = parse_expr("{ inherit (lib) map filter; }").unwrap();
    match expr {
      Expr::AttrSet { items, .. } => {
        assert_eq!(items.len(), 1);
        match &items[0] {
          PnixAttrItem::Inherit { from, names, .. } => {
            assert!(from.is_some());
            if let Some(scope_expr) = from {
              assert!(matches!(**scope_expr, Expr::Var(ref s) if s == "lib"));
            }
            assert_eq!(names.len(), 2);
            assert_eq!(names[0], "map");
            assert_eq!(names[1], "filter");
          }
          other => panic!("expected Inherit, got {:?}", other),
        }
      }
      other => panic!("expected AttrSet, got {:?}", other),
    }
  }

  // N00i: Tests for `match` keyword used as identifier
  #[test]
  fn test_match_as_variable_in_let() {
    // Nix doesn't have `match` keyword, so it can be used as variable name
    let expr = parse_expr("let match = 42; in match").unwrap();
    match expr {
      Expr::Let { bindings, body } => {
        assert_eq!(bindings.len(), 1);
        match &bindings[0] {
          LetBinding::Binding {
            pattern,
            value: std::sync::Arc::new(value),
          } => {
            assert!(matches!(pattern, ParamPattern::Ident(n) if n == "match"));
            assert!(matches!(value, Expr::Int(42)));
          }
          other => panic!("expected Binding, got {:?}", other),
        }
        assert!(matches!(*body, Expr::Var(ref n) if n == "match"));
      }
      other => panic!("expected Let, got {:?}", other),
    }
  }

  #[test]
  fn test_match_as_variable_in_function_application() {
    // builtins.elemAt match 1 → nested function application (curried)
    // This parses as: ((builtins.elemAt match) 1)
    let expr = parse_expr("builtins.elemAt match 1").unwrap();
    // The outermost Apply has arg = 1
    match expr {
      Expr::Apply { func, arg } => {
        assert!(matches!(*arg, Expr::Int(1)));
        // func should be: (builtins.elemAt match)
        match *func {
          Expr::Apply {
            func: inner_func,
            arg: inner_arg,
          } => {
            // inner_arg should be: match (variable)
            assert!(matches!(*inner_arg, Expr::Var(ref n) if n == "match"));
            // inner_func should be: builtins.elemAt (Select)
            assert!(matches!(*inner_func, Expr::Select { ref base, ref attr }
              if matches!(**base, Expr::Var(ref n) if n == "builtins") && attr == "elemAt"));
          }
          other => panic!("expected inner Apply, got {:?}", other),
        }
      }
      other => panic!("expected Apply, got {:?}", other),
    }
  }

  #[test]
  fn test_match_as_attrset_key() {
    // { match = 42; } → attrset with match as key
    let expr = parse_expr("{ match = 42; }").unwrap();
    match expr {
      Expr::AttrSet { items, recursive } => {
        assert!(!recursive);
        assert_eq!(items.len(), 1);
        match &items[0] {
          AttrItem::Assign {
            key_path, value, ..
          } => {
            assert_eq!(key_path, &["match"]);
            assert!(matches!(value, Expr::Int(42)));
          }
          other => panic!("expected Assign, got {:?}", other),
        }
      }
      other => panic!("expected AttrSet, got {:?}", other),
    }
  }

  #[test]
  fn test_match_with_operators() {
    // if match != null then match else 0
    let expr = parse_expr("if match != null then match else 0").unwrap();
    match expr {
      Expr::If {
        cond,
        then_,
        else_: _,
      } => {
        // cond should be: match != null
        match *cond {
          Expr::Binary {
            op,
            ref lhs,
            ref rhs,
          } => {
            assert_eq!(op, "!=");
            assert!(matches!(**lhs, Expr::Var(ref n) if n == "match"));
            assert!(matches!(**rhs, Expr::Null));
          }
          other => panic!("expected Binary, got {:?}", other),
        }
        // then should be: match (variable)
        assert!(matches!(*then_, Expr::Var(ref n) if n == "match"));
      }
      other => panic!("expected If, got {:?}", other),
    }
  }

  #[test]
  fn test_match_as_attribute_access() {
    // builtins.match → Select expression
    let expr = parse_expr("builtins.match").unwrap();
    match expr {
      Expr::Select { base, attr } => {
        assert!(matches!(*base, Expr::Var(ref n) if n == "builtins"));
        assert_eq!(attr, "match");
      }
      other => panic!("expected Select, got {:?}", other),
    }
  }

  #[test]
  fn test_match_expression_still_works() {
    // match x with | 1 => "one" → actual match expression
    let expr = parse_expr("match x with | 1 => \"one\"").unwrap();
    match expr {
      Expr::Match { scrutinee, arms } => {
        assert!(matches!(*scrutinee, Expr::Var(ref n) if n == "x"));
        assert_eq!(arms.len(), 1);
      }
      other => panic!("expected Match, got {:?}", other),
    }
  }

  #[test]
  fn test_match_or_pattern_desugars_to_arms() {
    use crate::lang::pnix::syntax::{PnixLiteralPattern, PnixPattern};
    let expr = parse_expr("match x with | 1 | 2 => \"a\" | _ => \"b\"").unwrap();
    match expr {
      Expr::Match { arms, .. } => {
        assert_eq!(arms.len(), 3);
        assert!(arms[0].guard.is_none());
        assert!(arms[1].guard.is_none());
        assert!(arms[2].guard.is_none());
        match &arms[0].pattern {
          PnixPattern::Literal(PnixLiteralPattern::Int(v)) => assert_eq!(*v, 1),
          other => panic!("expected int literal pattern, got {:?}", other),
        }
        match &arms[1].pattern {
          PnixPattern::Literal(PnixLiteralPattern::Int(v)) => assert_eq!(*v, 2),
          other => panic!("expected int literal pattern, got {:?}", other),
        }
        assert!(matches!(arms[2].pattern, PnixPattern::Wildcard));
        assert!(matches!(arms[0].body, Expr::String(ref s) if s == "a"));
        assert!(matches!(arms[1].body, Expr::String(ref s) if s == "a"));
        assert!(matches!(arms[2].body, Expr::String(ref s) if s == "b"));
      }
      other => panic!("expected Match, got {:?}", other),
    }
  }

  #[test]
  fn test_match_as_lambda_parameter() {
    // match: match → lambda with `match` as parameter name
    let expr = parse_expr("match: match").unwrap();
    match expr {
      Expr::Lambda { param, body } => {
        assert!(matches!(param, ParamPattern::Ident(ref n) if n == "match"));
        assert!(matches!(*body, Expr::Var(ref n) if n == "match"));
      }
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn test_match_in_attrset_pattern() {
    // { match ? 0 }: match → attrset pattern with `match` as field name
    let expr = parse_expr("{ match ? 0 }: match").unwrap();
    match expr {
      Expr::Lambda { param, body } => {
        match param {
          ParamPattern::AttrSet { fields, ellipsis } => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "match");
            assert!(fields[0].default.is_some());
            assert!(!ellipsis);
          }
          other => panic!("expected AttrSet pattern, got {:?}", other),
        }
        assert!(matches!(*body, Expr::Var(ref n) if n == "match"));
      }
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn test_match_in_attr_path() {
    // { a.match = 1; } → attrset with path containing `match`
    let expr = parse_expr("{ a.match = 1; }").unwrap();
    match expr {
      Expr::AttrSet { items, .. } => {
        assert_eq!(items.len(), 1);
        match &items[0] {
          AttrItem::Assign {
            key_path, value, ..
          } => {
            assert_eq!(key_path, &["a", "match"]);
            assert!(matches!(value, Expr::Int(1)));
          }
          other => panic!("expected Assign, got {:?}", other),
        }
      }
      other => panic!("expected AttrSet, got {:?}", other),
    }
  }

  #[test]
  fn test_match_as_first_in_attr_path() {
    // { match.x = 1; } → attrset with `match` as first path component
    let expr = parse_expr("{ match.x = 1; }").unwrap();
    match expr {
      Expr::AttrSet { items, .. } => {
        assert_eq!(items.len(), 1);
        match &items[0] {
          AttrItem::Assign {
            key_path, value, ..
          } => {
            assert_eq!(key_path, &["match", "x"]);
            assert!(matches!(value, Expr::Int(1)));
          }
          other => panic!("expected Assign, got {:?}", other),
        }
      }
      other => panic!("expected AttrSet, got {:?}", other),
    }
  }

  // N00j: Ellipsis pattern tests

  #[test]
  fn test_ellipsis_pattern_simple() {
    // { x, y, ... }: x + y
    let expr = parse_expr("{ x, y, ... }: x + y").unwrap();
    match expr {
      Expr::Lambda { param, .. } => {
        match param {
          ParamPattern::AttrSet { fields, ellipsis } => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[1].name, "y");
            assert!(ellipsis); // ellipsis is true
          }
          other => panic!("expected AttrSet pattern, got {:?}", other),
        }
      }
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn test_ellipsis_pattern_with_defaults() {
    // { x, y ? 0, ... }: x + y
    let expr = parse_expr("{ x, y ? 0, ... }: x + y").unwrap();
    match expr {
      Expr::Lambda { param, .. } => match param {
        ParamPattern::AttrSet { fields, ellipsis } => {
          assert_eq!(fields.len(), 2);
          assert_eq!(fields[0].name, "x");
          assert!(fields[0].default.is_none());
          assert_eq!(fields[1].name, "y");
          assert!(fields[1].default.is_some());
          assert!(ellipsis);
        }
        other => panic!("expected AttrSet pattern, got {:?}", other),
      },
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn test_ellipsis_pattern_with_at_bind() {
    // args@{ x, y, ... }: x + y
    let expr = parse_expr("args@{ x, y, ... }: x + y").unwrap();
    match expr {
      Expr::Lambda { param, .. } => match param {
        ParamPattern::AttrSetWithBind {
          bind_name,
          fields,
          ellipsis,
        } => {
          assert_eq!(bind_name, "args");
          assert_eq!(fields.len(), 2);
          assert_eq!(fields[0].name, "x");
          assert_eq!(fields[1].name, "y");
          assert!(ellipsis);
        }
        other => panic!("expected AttrSetWithBind pattern, got {:?}", other),
      },
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn test_ellipsis_only() {
    // { ... }: 42
    let expr = parse_expr("{ ... }: 42").unwrap();
    match expr {
      Expr::Lambda { param, .. } => {
        match param {
          ParamPattern::AttrSet { fields, ellipsis } => {
            assert_eq!(fields.len(), 0); // no named fields
            assert!(ellipsis);
          }
          other => panic!("expected AttrSet pattern, got {:?}", other),
        }
      }
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn test_no_ellipsis() {
    // { x, y }: x + y
    let expr = parse_expr("{ x, y }: x + y").unwrap();
    match expr {
      Expr::Lambda { param, .. } => {
        match param {
          ParamPattern::AttrSet { fields, ellipsis } => {
            assert_eq!(fields.len(), 2);
            assert!(!ellipsis); // no ellipsis
          }
          other => panic!("expected AttrSet pattern, got {:?}", other),
        }
      }
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn test_trailing_comma_before_ellipsis() {
    // { x, y, ..., }: body -- trailing comma after ellipsis is allowed
    let expr = parse_expr("{ x, y, ..., }: 42").unwrap();
    match expr {
      Expr::Lambda { param, .. } => match param {
        ParamPattern::AttrSet { fields, ellipsis } => {
          assert_eq!(fields.len(), 2);
          assert!(ellipsis);
        }
        other => panic!("expected AttrSet pattern, got {:?}", other),
      },
      other => panic!("expected Lambda, got {:?}", other),
    }
  }

  #[test]
  fn test_dynamic_attr_key() {
    // N00k: { ${name} = value; } -- dynamic attribute name
    use crate::lang::pnix::syntax::PnixAttrItem;
    let expr = parse_expr(r#"{ ${name} = 42; }"#).unwrap();
    match expr {
      Expr::AttrSet { items, recursive } => {
        assert!(!recursive);
        assert_eq!(items.len(), 1);
        match &items[0] {
          PnixAttrItem::DynamicAssign {
            key_path, value, ..
          } => {
            assert_eq!(key_path.len(), 1);
            match &key_path[0] {
              crate::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) => {
                assert!(matches!(expr.as_ref(), Expr::Var(n) if n == "name"));
              }
              other => panic!("expected Dynamic segment, got {:?}", other),
            }
            assert!(matches!(value, Expr::Int(42)));
          }
          other => panic!("expected DynamicAssign, got {:?}", other),
        }
      }
      other => panic!("expected AttrSet, got {:?}", other),
    }
  }

  #[test]
  fn test_dynamic_attr_key_with_string() {
    // { ${"computed-key"} = value; }
    use crate::lang::pnix::syntax::PnixAttrItem;
    let expr = parse_expr(r#"{ ${"mykey"} = 1; x = 2; }"#).unwrap();
    match expr {
      Expr::AttrSet { items, .. } => {
        assert_eq!(items.len(), 2);
        // First item: dynamic
        match &items[0] {
          PnixAttrItem::DynamicAssign { key_path, .. } => {
            assert_eq!(key_path.len(), 1);
            match &key_path[0] {
              crate::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) => {
                assert!(matches!(expr.as_ref(), Expr::String(s) if s == "mykey"));
              }
              other => panic!("expected Dynamic segment, got {:?}", other),
            }
          }
          other => panic!("expected DynamicAssign, got {:?}", other),
        }
        // Second item: static
        match &items[1] {
          PnixAttrItem::Assign { key_path, .. } => {
            assert_eq!(key_path, &["x"]);
          }
          other => panic!("expected Assign, got {:?}", other),
        }
      }
      other => panic!("expected AttrSet, got {:?}", other),
    }
  }
}
