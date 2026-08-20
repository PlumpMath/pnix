//! 최소 Clojure 리더 (표현식 부분집합만)
//!
//! 이 파서는 의도적으로 작게 설계되었으며 다음만 지원합니다:
//! - 숫자, 불리언, nil
//! - 심볼
//! - 리스트: (+ - * / mod if if-not let if-let if-some do when when-not when-let when-some unless cond case and or not nil? some? true? false? -> ->> some-> some->> cond-> cond->> as->
//!            < > <= >= = not= floor ceil abs sqrt sin cos tan exp ln pow str)
//! - 벡터/맵 리터럴
//!
//! FxCore로 lowering하기 위한 UnifiedExpr를 생성합니다.

use std::collections::{HashSet, VecDeque};

use crate::lang::clojure_error::ClojureError;
use crate::lang::pnix::UnifiedExpr;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
  LParen,
  RParen,
  LBracket,
  RBracket,
  LBrace,
  RBrace,
  Quote,
  SyntaxQuote,
  Unquote,
  UnquoteSplicing,
  Atom(String),
  Str(String),
}

/// Clojure 폼: Clojure 표현식의 기본 구조
#[derive(Debug, Clone, PartialEq)]
pub enum CljForm {
  /// 원자 (심볼, 숫자 등)
  Atom(
    /// 원자 값 (문자열)
    String,
  ),
  /// 문자열 리터럴
  Str(
    /// 문자열 값
    String,
  ),
  /// 리스트 (괄호로 둘러싸인 폼 목록)
  List(
    /// 폼 목록
    Vec<CljForm>,
  ),
  /// 벡터 (대괄호로 둘러싸인 폼 목록)
  Vector(
    /// 폼 목록
    Vec<CljForm>,
  ),
  /// 맵 (키-값 쌍 목록)
  Map(
    /// 키-값 쌍 목록
    Vec<(CljForm, CljForm)>,
  ),
  /// Quote ('form)
  Quote(
    /// 인용된 폼
    Box<CljForm>,
  ),
  /// Syntax quote (`form)
  SyntaxQuote(
    /// 문법 인용된 폼
    Box<CljForm>,
  ),
  /// Unquote (~form)
  Unquote(
    /// 언인용된 폼
    Box<CljForm>,
  ),
  /// Unquote splicing (~@form)
  UnquoteSplicing(
    /// 스플라이싱 언인용된 폼
    Box<CljForm>,
  ),
}

/// Clojure 표현식을 UnifiedExpr로 파싱
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_clj_expr(source: &str) -> Result<UnifiedExpr, ClojureError> {
  let mut forms = parse_clj_forms(source)?;
  if forms.is_empty() {
    return Err(ClojureError::Parse("empty input".to_string()));
  }
  if forms.len() > 1 {
    return Err(ClojureError::Parse(
      "unexpected tokens after expression".to_string(),
    ));
  }
  form_to_unified(&forms.remove(0))
}

/// Clojure 폼 목록을 파싱
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_clj_forms(source: &str) -> Result<Vec<CljForm>, ClojureError> {
  let tokens = tokenize(source)?;
  let mut queue: VecDeque<Token> = tokens.into();
  let mut forms = Vec::new();
  while !queue.is_empty() {
    forms.push(parse_form(&mut queue)?);
  }
  Ok(forms)
}

fn tokenize(source: &str) -> Result<Vec<Token>, ClojureError> {
  let mut tokens = Vec::new();
  let mut chars = source.chars().peekable();

  while let Some(ch) = chars.next() {
    match ch {
      '(' => tokens.push(Token::LParen),
      ')' => tokens.push(Token::RParen),
      '[' => tokens.push(Token::LBracket),
      ']' => tokens.push(Token::RBracket),
      '{' => tokens.push(Token::LBrace),
      '}' => tokens.push(Token::RBrace),
      '\'' => tokens.push(Token::Quote),
      '`' => tokens.push(Token::SyntaxQuote),
      '~' => {
        if let Some('@') = chars.peek().copied() {
          chars.next();
          tokens.push(Token::UnquoteSplicing);
        } else {
          tokens.push(Token::Unquote);
        }
      }
      '"' => tokens.push(Token::Str(parse_string(&mut chars)?)),
      ';' => {
        // comment to end of line
        while let Some(next) = chars.next() {
          if next == '\n' {
            break;
          }
        }
      }
      ch if ch.is_whitespace() || ch == ',' => {
        // skip whitespace and commas
      }
      _ => {
        let mut atom = String::new();
        atom.push(ch);
        while let Some(next) = chars.peek() {
          if next.is_whitespace()
            || matches!(next, '(' | ')' | '[' | ']' | '{' | '}' | '"' | ';' | ',')
          {
            break;
          }
          atom.push(*next);
          chars.next();
        }
        tokens.push(Token::Atom(atom));
      }
    }
  }

  Ok(tokens)
}

fn parse_string(
  chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<String, ClojureError> {
  let mut out = String::new();
  while let Some(ch) = chars.next() {
    match ch {
      '"' => return Ok(out),
      '\\' => {
        let escaped = chars.next().ok_or_else(|| {
          ClojureError::Parse("unterminated escape sequence in string".to_string())
        })?;
        out.push(escaped);
      }
      _ => out.push(ch),
    }
  }
  Err(ClojureError::Parse(
    "unterminated string literal".to_string(),
  ))
}

fn parse_form(tokens: &mut VecDeque<Token>) -> Result<CljForm, ClojureError> {
  match tokens.pop_front() {
    Some(Token::LParen) => parse_list(tokens, Token::RParen).map(CljForm::List),
    Some(Token::LBracket) => parse_list(tokens, Token::RBracket).map(CljForm::Vector),
    Some(Token::LBrace) => parse_map(tokens),
    Some(Token::Quote) => Ok(CljForm::Quote(Box::new(parse_form(tokens)?))),
    Some(Token::SyntaxQuote) => Ok(CljForm::SyntaxQuote(Box::new(parse_form(tokens)?))),
    Some(Token::Unquote) => Ok(CljForm::Unquote(Box::new(parse_form(tokens)?))),
    Some(Token::UnquoteSplicing) => Ok(CljForm::UnquoteSplicing(Box::new(parse_form(tokens)?))),
    Some(Token::Atom(value)) => Ok(CljForm::Atom(value)),
    Some(Token::Str(value)) => Ok(CljForm::Str(value)),
    Some(Token::RParen) | Some(Token::RBracket) | Some(Token::RBrace) => Err(ClojureError::Parse(
      "unexpected closing delimiter".to_string(),
    )),
    None => Err(ClojureError::Parse("unexpected end of input".to_string())),
  }
}

fn parse_list(tokens: &mut VecDeque<Token>, end: Token) -> Result<Vec<CljForm>, ClojureError> {
  let mut items = Vec::new();
  loop {
    match tokens.front() {
      Some(Token::RParen) if matches!(end, Token::RParen) => {
        tokens.pop_front();
        break;
      }
      Some(Token::RBracket) if matches!(end, Token::RBracket) => {
        tokens.pop_front();
        break;
      }
      None => return Err(ClojureError::Parse("unexpected end of list".to_string())),
      _ => items.push(parse_form(tokens)?),
    }
  }
  Ok(items)
}

fn parse_map(tokens: &mut VecDeque<Token>) -> Result<CljForm, ClojureError> {
  let mut items = Vec::new();
  loop {
    match tokens.front() {
      Some(Token::RBrace) => {
        tokens.pop_front();
        break;
      }
      None => return Err(ClojureError::Parse("unexpected end of map".to_string())),
      _ => items.push(parse_form(tokens)?),
    }
  }

  if items.len() % 2 != 0 {
    return Err(ClojureError::Parse(
      "map expects even number of forms".to_string(),
    ));
  }

  let mut pairs = Vec::new();
  for pair in items.chunks(2) {
    pairs.push((pair[0].clone(), pair[1].clone()));
  }
  Ok(CljForm::Map(pairs))
}

fn form_to_unified(form: &CljForm) -> Result<UnifiedExpr, ClojureError> {
  match form {
    CljForm::Atom(atom) => atom_to_unified(atom),
    CljForm::Str(value) => Ok(UnifiedExpr::String(value.clone())),
    CljForm::Vector(items) => parse_vector_literal(items),
    CljForm::Map(pairs) => parse_map_literal(pairs),
    CljForm::Quote(_) => Err(ClojureError::UnsupportedSyntax(
      "quote not supported here".to_string(),
    )),
    CljForm::SyntaxQuote(_) => Err(ClojureError::UnsupportedSyntax(
      "syntax-quote not supported here".to_string(),
    )),
    CljForm::Unquote(_) => Err(ClojureError::UnsupportedSyntax(
      "unquote not supported here".to_string(),
    )),
    CljForm::UnquoteSplicing(_) => Err(ClojureError::UnsupportedSyntax(
      "unquote-splicing not supported here".to_string(),
    )),
    CljForm::List(items) => list_to_unified(items),
  }
}

fn atom_to_unified(atom: &str) -> Result<UnifiedExpr, ClojureError> {
  if atom == "true" {
    return Ok(UnifiedExpr::Bool(true));
  }
  if atom == "false" {
    return Ok(UnifiedExpr::Bool(false));
  }
  if atom == "nil" {
    return Ok(UnifiedExpr::Null);
  }
  if atom == "param/system-time" || atom == "param.system_time" {
    return Ok(UnifiedExpr::ParamTime);
  }
  if atom == "param/delta-time" || atom == "param.delta_time" {
    return Ok(UnifiedExpr::ParamDeltaTime);
  }

  if let Ok(int_value) = atom.parse::<i64>() {
    return Ok(UnifiedExpr::Int(int_value));
  }
  if let Ok(float_value) = atom.parse::<f64>() {
    return Ok(UnifiedExpr::Float(float_value));
  }

  Ok(UnifiedExpr::Var(atom.to_string()))
}

fn list_to_unified(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  let (head, tail) = items
    .split_first()
    .ok_or_else(|| ClojureError::UnsupportedSyntax("empty list".to_string()))?;

  let head_sym_raw = match head {
    CljForm::Atom(sym) => sym.as_str(),
    _ => {
      return Err(ClojureError::UnsupportedSyntax(
        "list head must be a symbol".to_string(),
      ))
    }
  };
  let (head_sym, stdlib_ns) = normalize_stdlib_head_symbol(head_sym_raw);

  match head_sym {
    "+" => fold_add(tail),
    "*" => fold_mul(tail),
    "-" => fold_sub(tail),
    "/" => fold_div(tail),
    "mod" => fold_binary(tail, UnifiedExpr::Mod, 2),
    "<" => fold_order_compare(tail, UnifiedExpr::Lt, "<"),
    ">" => fold_order_compare(tail, UnifiedExpr::Gt, ">"),
    "<=" => fold_order_compare(tail, UnifiedExpr::Le, "<="),
    ">=" => fold_order_compare(tail, UnifiedExpr::Ge, ">="),
    "=" => fold_equality_compare(tail, UnifiedExpr::Eq, "="),
    "not=" => fold_not_equal(tail),
    "and" => parse_and_macro(tail),
    "or" => parse_or_macro(tail),
    "not" => parse_unary(tail, "not", UnifiedExpr::Not),
    "nil?" => parse_unary(tail, "nil?", |value| {
      UnifiedExpr::Eq(value, Box::new(UnifiedExpr::Null))
    }),
    "some?" => parse_unary(tail, "some?", |value| {
      UnifiedExpr::Ne(value, Box::new(UnifiedExpr::Null))
    }),
    "true?" => parse_unary(tail, "true?", |value| {
      UnifiedExpr::Eq(value, Box::new(UnifiedExpr::Bool(true)))
    }),
    "false?" => parse_unary(tail, "false?", |value| {
      UnifiedExpr::Eq(value, Box::new(UnifiedExpr::Bool(false)))
    }),
    "inc" => parse_apply_with_arity(tail, "inc", 1, Some(1)),
    "dec" => parse_apply_with_arity(tail, "dec", 1, Some(1)),
    "zero?" => parse_apply_with_arity(tail, "zero?", 1, Some(1)),
    "pos?" => parse_apply_with_arity(tail, "pos?", 1, Some(1)),
    "neg?" => parse_apply_with_arity(tail, "neg?", 1, Some(1)),
    "even?" => parse_apply_with_arity(tail, "even?", 1, Some(1)),
    "odd?" => parse_apply_with_arity(tail, "odd?", 1, Some(1)),
    "min" => parse_apply_with_arity(tail, "min", 1, None),
    "max" => parse_apply_with_arity(tail, "max", 1, None),
    "quot" => parse_apply_with_arity(tail, "quot", 2, Some(2)),
    "rem" => parse_apply_with_arity(tail, "rem", 2, Some(2)),
    "number?" => parse_apply_with_arity(tail, "number?", 1, Some(1)),
    "integer?" => parse_apply_with_arity(tail, "integer?", 1, Some(1)),
    "float?" => parse_apply_with_arity(tail, "float?", 1, Some(1)),
    "string?" => parse_apply_with_arity(tail, "string?", 1, Some(1)),
    "keyword?" => parse_apply_with_arity(tail, "keyword?", 1, Some(1)),
    "symbol?" => parse_apply_with_arity(tail, "symbol?", 1, Some(1)),
    "boolean?" => parse_apply_with_arity(tail, "boolean?", 1, Some(1)),
    "map?" => parse_apply_with_arity(tail, "map?", 1, Some(1)),
    "vector?" => parse_apply_with_arity(tail, "vector?", 1, Some(1)),
    "sequential?" => parse_apply_with_arity(tail, "sequential?", 1, Some(1)),
    "coll?" => parse_apply_with_arity(tail, "coll?", 1, Some(1)),
    "fn?" => parse_apply_with_arity(tail, "fn?", 1, Some(1)),
    "count" => parse_apply_with_arity(tail, "count", 1, Some(1)),
    "empty?" => parse_apply_with_arity(tail, "empty?", 1, Some(1)),
    "contains?" => parse_apply_with_arity(tail, "contains?", 2, Some(2)),
    "get" => parse_apply_with_arity(tail, "get", 2, Some(3)),
    "get-in" => parse_apply_with_arity(tail, "get-in", 2, Some(3)),
    "assoc" => parse_apply_with_arity(tail, "assoc", 3, None),
    "assoc-in" => parse_apply_with_arity(tail, "assoc-in", 3, Some(3)),
    "dissoc" => parse_apply_with_arity(tail, "dissoc", 2, None),
    "update" => parse_apply_with_arity(tail, "update", 3, None),
    "update-in" => parse_apply_with_arity(tail, "update-in", 3, None),
    "merge" => parse_apply_with_arity(tail, "merge", 0, None),
    "keys" => parse_apply_with_arity(tail, "keys", 1, Some(1)),
    "vals" => parse_apply_with_arity(tail, "vals", 1, Some(1)),
    "floor" => parse_unary(tail, "floor", UnifiedExpr::Floor),
    "ceil" => parse_unary(tail, "ceil", UnifiedExpr::Ceil),
    "abs" => parse_unary(tail, "abs", UnifiedExpr::Abs),
    "sqrt" => parse_unary(tail, "sqrt", UnifiedExpr::Sqrt),
    "sin" => parse_unary(tail, "sin", UnifiedExpr::Sin),
    "cos" => parse_unary(tail, "cos", UnifiedExpr::Cos),
    "tan" => parse_unary(tail, "tan", UnifiedExpr::Tan),
    "exp" => parse_unary(tail, "exp", UnifiedExpr::Exp),
    "log" => parse_unary(tail, "log", UnifiedExpr::Ln),
    "ln" => parse_unary(tail, "ln", UnifiedExpr::Ln),
    "log10" => parse_apply_with_arity(tail, "log10", 1, Some(1)),
    "asin" => parse_apply_with_arity(tail, "asin", 1, Some(1)),
    "acos" => parse_apply_with_arity(tail, "acos", 1, Some(1)),
    "atan" => parse_apply_with_arity(tail, "atan", 1, Some(1)),
    "to-radians" => parse_apply_with_arity(tail, "to-radians", 1, Some(1)),
    "to-degrees" => parse_apply_with_arity(tail, "to-degrees", 1, Some(1)),
    "pow" => parse_binary_exact(tail, "pow", UnifiedExpr::Pow),
    "str" => fold_concat(tail),
    "seq" => parse_apply_with_arity(tail, "seq", 1, Some(1)),
    "first" => parse_apply_with_arity(tail, "first", 1, Some(1)),
    "rest" => parse_apply_with_arity(tail, "rest", 1, Some(1)),
    "next" => parse_apply_with_arity(tail, "next", 1, Some(1)),
    "nth" => parse_apply_with_arity(tail, "nth", 2, Some(3)),
    "last" => parse_apply_with_arity(tail, "last", 1, Some(1)),
    "butlast" => parse_apply_with_arity(tail, "butlast", 1, Some(1)),
    "take" => parse_apply_with_arity(tail, "take", 2, Some(2)),
    "drop" => parse_apply_with_arity(tail, "drop", 2, Some(2)),
    "concat" => parse_apply_with_arity(tail, "concat", 0, None),
    "cons" => parse_apply_with_arity(tail, "cons", 2, Some(2)),
    "conj" => parse_apply_with_arity(tail, "conj", 1, None),
    "into" => parse_apply_with_arity(tail, "into", 2, Some(3)),
    "vec" => parse_apply_with_arity(tail, "vec", 1, Some(1)),
    "list" => parse_apply_with_arity(tail, "list", 0, None),
    "set" => parse_apply_with_arity(tail, "set", 1, Some(1)),
    "map" => parse_apply_with_arity(tail, "map", 2, None),
    "mapv" => parse_apply_with_arity(tail, "mapv", 2, None),
    "filter" => parse_apply_with_arity(tail, "filter", 2, Some(2)),
    "remove" => parse_apply_with_arity(tail, "remove", 2, Some(2)),
    "keep" => parse_apply_with_arity(tail, "keep", 2, Some(2)),
    "reduce" => parse_apply_with_arity(tail, "reduce", 2, Some(3)),
    "reduce-kv" => parse_apply_with_arity(tail, "reduce-kv", 3, Some(3)),
    "some" => parse_apply_with_arity(tail, "some", 2, Some(2)),
    "every?" => parse_apply_with_arity(tail, "every?", 2, Some(2)),
    "not-any?" => parse_apply_with_arity(tail, "not-any?", 2, Some(2)),
    "not-every?" => parse_apply_with_arity(tail, "not-every?", 2, Some(2)),
    "apply" => parse_apply_with_arity(tail, "apply", 2, None),
    "partial" => parse_apply_with_arity(tail, "partial", 2, None),
    "comp" => parse_apply_with_arity(tail, "comp", 0, None),
    "juxt" => parse_apply_with_arity(tail, "juxt", 1, None),
    "identity" => parse_apply_with_arity(tail, "identity", 1, Some(1)),
    "constantly" => parse_apply_with_arity(tail, "constantly", 1, Some(1)),
    "->" => parse_thread_macro(tail, false),
    "->>" => parse_thread_macro(tail, true),
    "some->" => parse_some_thread_macro(tail, false),
    "some->>" => parse_some_thread_macro(tail, true),
    "cond->" => parse_cond_thread_macro(tail, false),
    "cond->>" => parse_cond_thread_macro(tail, true),
    "as->" => parse_as_thread_macro(tail),
    "if" => parse_if(tail),
    "if-not" => parse_if_not(tail),
    "let" => parse_let(tail),
    "letfn" => parse_letfn(tail),
    "fn" => parse_fn(tail),
    "defn" => parse_defn(tail),
    "if-let" => parse_if_let(tail),
    "if-some" => parse_if_some(tail),
    "when" => parse_when(tail),
    "when-not" => parse_when_not(tail),
    "when-let" => parse_when_let(tail),
    "when-some" => parse_when_some(tail),
    "unless" => parse_unless(tail),
    "cond" => parse_cond(tail),
    "case" => parse_case(tail),
    "do" => parse_do(tail),
    _ => {
      if let Some(ns) = stdlib_ns {
        return Err(ClojureError::UnsupportedSyntax(format!(
          "blocked(reason_code=EVAL_TARGET_PNIX_UNSUPPORTED_MORPHISM): unsupported {ns} symbol `{head_sym_raw}`"
        )));
      }
      let args = tail
        .iter()
        .map(form_to_unified)
        .collect::<Result<Vec<_>, _>>()?;
      Ok(UnifiedExpr::Apply {
        func: head_sym.to_string(),
        args,
      })
    }
  }
}

fn normalize_stdlib_head_symbol(head_sym: &str) -> (&str, Option<&'static str>) {
  if let Some(sym) = head_sym.strip_prefix("clojure.core/") {
    return (sym, Some("clojure.core"));
  }
  if let Some(sym) = head_sym.strip_prefix("clojure.math/") {
    return (sym, Some("clojure.math"));
  }
  (head_sym, None)
}

fn parse_if(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.len() < 2 || items.len() > 3 {
    return Err(ClojureError::UnsupportedSyntax(
      "if expects 2 or 3 arguments".to_string(),
    ));
  }
  Ok(UnifiedExpr::If {
    cond: Box::new(form_to_unified(&items[0])?),
    then_: Box::new(form_to_unified(&items[1])?),
    else_: Box::new(if items.len() == 3 {
      form_to_unified(&items[2])?
    } else {
      UnifiedExpr::Null
    }),
  })
}

fn parse_when(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Err(ClojureError::UnsupportedSyntax(
      "when expects condition and body".to_string(),
    ));
  }
  let cond = form_to_unified(&items[0])?;
  let then_ = parse_do(&items[1..])?;
  Ok(UnifiedExpr::If {
    cond: Box::new(cond),
    then_: Box::new(then_),
    else_: Box::new(UnifiedExpr::Null),
  })
}

fn parse_if_not(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.len() < 2 || items.len() > 3 {
    return Err(ClojureError::UnsupportedSyntax(
      "if-not expects 2 or 3 arguments".to_string(),
    ));
  }
  Ok(UnifiedExpr::If {
    cond: Box::new(form_to_unified(&items[0])?),
    then_: Box::new(if items.len() == 3 {
      form_to_unified(&items[2])?
    } else {
      UnifiedExpr::Null
    }),
    else_: Box::new(form_to_unified(&items[1])?),
  })
}

fn parse_if_let(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.len() < 2 || items.len() > 3 {
    return Err(ClojureError::UnsupportedSyntax(
      "if-let expects binding, then, and optional else".to_string(),
    ));
  }

  let (binding_name, binding_value) = parse_single_binding(&items[0], "if-let")?;
  let temp_name = fresh_internal_binding("__pnix_if_let_tmp", items);
  let then_expr = UnifiedExpr::Let {
    name: binding_name,
    value: Box::new(UnifiedExpr::Var(temp_name.clone())),
    body: Box::new(form_to_unified(&items[1])?),
  };
  let else_expr = if items.len() == 3 {
    form_to_unified(&items[2])?
  } else {
    UnifiedExpr::Null
  };

  Ok(UnifiedExpr::Let {
    name: temp_name.clone(),
    value: Box::new(binding_value),
    body: Box::new(UnifiedExpr::If {
      cond: Box::new(truthy_expr_for_var(&temp_name)),
      then_: Box::new(then_expr),
      else_: Box::new(else_expr),
    }),
  })
}

fn parse_if_some(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.len() < 2 || items.len() > 3 {
    return Err(ClojureError::UnsupportedSyntax(
      "if-some expects binding, then, and optional else".to_string(),
    ));
  }

  let (binding_name, binding_value) = parse_single_binding(&items[0], "if-some")?;
  let temp_name = fresh_internal_binding("__pnix_if_some_tmp", items);
  let then_expr = UnifiedExpr::Let {
    name: binding_name,
    value: Box::new(UnifiedExpr::Var(temp_name.clone())),
    body: Box::new(form_to_unified(&items[1])?),
  };
  let else_expr = if items.len() == 3 {
    form_to_unified(&items[2])?
  } else {
    UnifiedExpr::Null
  };

  Ok(UnifiedExpr::Let {
    name: temp_name.clone(),
    value: Box::new(binding_value),
    body: Box::new(UnifiedExpr::If {
      cond: Box::new(UnifiedExpr::Ne(
        Box::new(UnifiedExpr::Var(temp_name.clone())),
        Box::new(UnifiedExpr::Null),
      )),
      then_: Box::new(then_expr),
      else_: Box::new(else_expr),
    }),
  })
}

fn parse_when_let(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Err(ClojureError::UnsupportedSyntax(
      "when-let expects binding vector".to_string(),
    ));
  }

  let (binding_name, binding_value) = parse_single_binding(&items[0], "when-let")?;
  let temp_name = fresh_internal_binding("__pnix_when_let_tmp", items);
  let body_expr = if items.len() == 1 {
    UnifiedExpr::Null
  } else {
    parse_do(&items[1..])?
  };

  Ok(UnifiedExpr::Let {
    name: temp_name.clone(),
    value: Box::new(binding_value),
    body: Box::new(UnifiedExpr::If {
      cond: Box::new(truthy_expr_for_var(&temp_name)),
      then_: Box::new(UnifiedExpr::Let {
        name: binding_name,
        value: Box::new(UnifiedExpr::Var(temp_name)),
        body: Box::new(body_expr),
      }),
      else_: Box::new(UnifiedExpr::Null),
    }),
  })
}

fn parse_when_some(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Err(ClojureError::UnsupportedSyntax(
      "when-some expects binding vector".to_string(),
    ));
  }

  let (binding_name, binding_value) = parse_single_binding(&items[0], "when-some")?;
  let temp_name = fresh_internal_binding("__pnix_when_some_tmp", items);
  let body_expr = if items.len() == 1 {
    UnifiedExpr::Null
  } else {
    parse_do(&items[1..])?
  };

  Ok(UnifiedExpr::Let {
    name: temp_name.clone(),
    value: Box::new(binding_value),
    body: Box::new(UnifiedExpr::If {
      cond: Box::new(UnifiedExpr::Ne(
        Box::new(UnifiedExpr::Var(temp_name.clone())),
        Box::new(UnifiedExpr::Null),
      )),
      then_: Box::new(UnifiedExpr::Let {
        name: binding_name,
        value: Box::new(UnifiedExpr::Var(temp_name)),
        body: Box::new(body_expr),
      }),
      else_: Box::new(UnifiedExpr::Null),
    }),
  })
}

fn parse_unless(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Err(ClojureError::UnsupportedSyntax(
      "unless expects condition and body".to_string(),
    ));
  }
  let cond = form_to_unified(&items[0])?;
  let else_ = parse_do(&items[1..])?;
  Ok(UnifiedExpr::If {
    cond: Box::new(cond),
    then_: Box::new(UnifiedExpr::Null),
    else_: Box::new(else_),
  })
}

fn parse_when_not(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Err(ClojureError::UnsupportedSyntax(
      "when-not expects condition and body".to_string(),
    ));
  }
  let cond = form_to_unified(&items[0])?;
  let else_ = parse_do(&items[1..])?;
  Ok(UnifiedExpr::If {
    cond: Box::new(cond),
    then_: Box::new(UnifiedExpr::Null),
    else_: Box::new(else_),
  })
}

fn parse_cond(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Ok(UnifiedExpr::Null);
  }
  if items.len() % 2 != 0 {
    return Err(ClojureError::UnsupportedSyntax(
      "cond expects even number of forms".to_string(),
    ));
  }

  let clauses = items.chunks(2).collect::<Vec<_>>();
  let mut expr = UnifiedExpr::Null;

  for (idx, clause) in clauses.iter().enumerate().rev() {
    let test = &clause[0];
    let body = form_to_unified(&clause[1])?;
    match test {
      CljForm::Atom(sym) if sym == ":else" => {
        if idx != clauses.len() - 1 {
          return Err(ClojureError::UnsupportedSyntax(
            "cond :else clause must be last".to_string(),
          ));
        }
        expr = body;
      }
      _ => {
        expr = UnifiedExpr::If {
          cond: Box::new(form_to_unified(test)?),
          then_: Box::new(body),
          else_: Box::new(expr),
        };
      }
    }
  }

  Ok(expr)
}

fn parse_case(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.len() < 4 {
    return Err(ClojureError::UnsupportedSyntax(
      "case expects expression, one or more test/result pairs, and default".to_string(),
    ));
  }
  if items.len() % 2 != 0 {
    return Err(ClojureError::UnsupportedSyntax(
      "case expects expression, test/result pairs, and default".to_string(),
    ));
  }

  let temp_name = fresh_internal_binding("__pnix_case_tmp", items);
  let value_expr = form_to_unified(&items[0])?;
  let clauses = &items[1..items.len() - 1];
  let mut out = form_to_unified(items.last().unwrap())?;

  for clause in clauses.chunks(2).rev() {
    let test_expr = form_to_unified(&clause[0])?;
    let result_expr = form_to_unified(&clause[1])?;
    out = UnifiedExpr::If {
      cond: Box::new(UnifiedExpr::Eq(
        Box::new(UnifiedExpr::Var(temp_name.clone())),
        Box::new(test_expr),
      )),
      then_: Box::new(result_expr),
      else_: Box::new(out),
    };
  }

  Ok(UnifiedExpr::Let {
    name: temp_name,
    value: Box::new(value_expr),
    body: Box::new(out),
  })
}

fn parse_single_binding(
  binding_form: &CljForm,
  form_name: &str,
) -> Result<(String, UnifiedExpr), ClojureError> {
  let values = match binding_form {
    CljForm::Vector(values) => values,
    _ => {
      return Err(ClojureError::UnsupportedSyntax(format!(
        "{} expects [symbol expr] binding",
        form_name
      )))
    }
  };
  if values.len() != 2 {
    return Err(ClojureError::UnsupportedSyntax(format!(
      "{} binding vector must contain exactly 2 forms",
      form_name
    )));
  }

  let name = match &values[0] {
    CljForm::Atom(sym) => sym.clone(),
    _ => {
      return Err(ClojureError::UnsupportedSyntax(format!(
        "{} binding name must be symbol",
        form_name
      )))
    }
  };
  Ok((name, form_to_unified(&values[1])?))
}

fn truthy_expr_for_var(name: &str) -> UnifiedExpr {
  let var = UnifiedExpr::Var(name.to_string());
  let not_nil = UnifiedExpr::Ne(Box::new(var.clone()), Box::new(UnifiedExpr::Null));
  let not_false = UnifiedExpr::Ne(Box::new(var), Box::new(UnifiedExpr::Bool(false)));
  UnifiedExpr::And(Box::new(not_nil), Box::new(not_false))
}

fn fresh_internal_binding(prefix: &str, forms: &[CljForm]) -> String {
  let mut used = HashSet::new();
  for form in forms {
    collect_symbols(form, &mut used);
  }
  if !used.contains(prefix) {
    return prefix.to_string();
  }

  let mut idx = 1usize;
  loop {
    let candidate = format!("{}_{}", prefix, idx);
    if !used.contains(candidate.as_str()) {
      return candidate;
    }
    idx += 1;
  }
}

fn collect_symbols(form: &CljForm, out: &mut HashSet<String>) {
  match form {
    CljForm::Atom(sym) => {
      out.insert(sym.clone());
    }
    CljForm::Str(_) => {}
    CljForm::List(items) | CljForm::Vector(items) => {
      for item in items {
        collect_symbols(item, out);
      }
    }
    CljForm::Map(pairs) => {
      for (k, v) in pairs {
        collect_symbols(k, out);
        collect_symbols(v, out);
      }
    }
    CljForm::Quote(inner)
    | CljForm::SyntaxQuote(inner)
    | CljForm::Unquote(inner)
    | CljForm::UnquoteSplicing(inner) => collect_symbols(inner, out),
  }
}

fn bind_pattern(
  pattern: &CljForm,
  value: UnifiedExpr,
  body: UnifiedExpr,
  form_name: &str,
) -> Result<UnifiedExpr, ClojureError> {
  match pattern {
    CljForm::Atom(sym) => Ok(UnifiedExpr::Let {
      name: sym.clone(),
      value: Box::new(value),
      body: Box::new(body),
    }),
    CljForm::Vector(items) => bind_vector_pattern(items, value, body, form_name),
    CljForm::Map(pairs) => bind_map_pattern(pairs, value, body, form_name),
    _ => Err(ClojureError::UnsupportedSyntax(format!(
      "{} binding pattern must be symbol/vector/map",
      form_name
    ))),
  }
}

fn is_valid_symbol_name(name: &str) -> bool {
  if name.is_empty() || name.starts_with(':') || name == "nil" || name == "true" || name == "false"
  {
    return false;
  }
  if name.parse::<i64>().is_ok() || name.parse::<f64>().is_ok() {
    return false;
  }
  true
}

fn bind_vector_pattern(
  items: &[CljForm],
  value: UnifiedExpr,
  body: UnifiedExpr,
  form_name: &str,
) -> Result<UnifiedExpr, ClojureError> {
  let mut positional = Vec::new();
  let mut rest_pattern: Option<CljForm> = None;

  let mut idx = 0usize;
  while idx < items.len() {
    if matches!(&items[idx], CljForm::Atom(sym) if sym == "&") {
      if idx + 1 >= items.len() || idx + 2 != items.len() {
        return Err(ClojureError::UnsupportedSyntax(format!(
          "{} vector destructuring expects '&' followed by one trailing binding",
          form_name
        )));
      }
      rest_pattern = Some(items[idx + 1].clone());
      break;
    }
    positional.push(items[idx].clone());
    idx += 1;
  }

  let tmp_name = fresh_internal_binding("__pnix_bind_vec_tmp", &[CljForm::Vector(items.to_vec())]);
  let mut out = body;

  if let Some(rest) = rest_pattern {
    let rest_value = UnifiedExpr::Apply {
      func: "drop".to_string(),
      args: vec![
        UnifiedExpr::Int(positional.len() as i64),
        UnifiedExpr::Var(tmp_name.clone()),
      ],
    };
    out = bind_pattern(&rest, rest_value, out, form_name)?;
  }

  for (i, pattern) in positional.iter().enumerate().rev() {
    let nth_value = UnifiedExpr::Apply {
      func: "nth".to_string(),
      args: vec![
        UnifiedExpr::Var(tmp_name.clone()),
        UnifiedExpr::Int(i as i64),
      ],
    };
    out = bind_pattern(pattern, nth_value, out, form_name)?;
  }

  Ok(UnifiedExpr::Let {
    name: tmp_name,
    value: Box::new(value),
    body: Box::new(out),
  })
}

fn bind_map_pattern(
  pairs: &[(CljForm, CljForm)],
  value: UnifiedExpr,
  body: UnifiedExpr,
  form_name: &str,
) -> Result<UnifiedExpr, ClojureError> {
  let mut key_names = Vec::new();
  let mut as_pattern: Option<CljForm> = None;

  for (k, v) in pairs {
    match k {
      CljForm::Atom(tag) if tag == ":keys" => {
        let items = match v {
          CljForm::Vector(items) => items,
          _ => {
            return Err(ClojureError::UnsupportedSyntax(format!(
              "{} map destructuring :keys expects vector of symbols",
              form_name
            )))
          }
        };
        for item in items {
          let CljForm::Atom(name) = item else {
            return Err(ClojureError::UnsupportedSyntax(format!(
              "{} map destructuring :keys expects symbols",
              form_name
            )));
          };
          key_names.push(name.clone());
        }
      }
      CljForm::Atom(tag) if tag == ":as" => {
        as_pattern = Some(v.clone());
      }
      _ => {
        return Err(ClojureError::UnsupportedSyntax(format!(
          "{} map destructuring supports only :keys and :as",
          form_name
        )))
      }
    }
  }

  let tmp_name = fresh_internal_binding("__pnix_bind_map_tmp", &[CljForm::Map(pairs.to_vec())]);
  let mut out = body;

  if let Some(as_binding) = as_pattern {
    out = bind_pattern(
      &as_binding,
      UnifiedExpr::Var(tmp_name.clone()),
      out,
      form_name,
    )?;
  }

  for key_name in key_names.into_iter().rev() {
    out = UnifiedExpr::Let {
      name: key_name.clone(),
      value: Box::new(UnifiedExpr::Apply {
        func: "getAttr".to_string(),
        args: vec![
          UnifiedExpr::String(key_name),
          UnifiedExpr::Var(tmp_name.clone()),
        ],
      }),
      body: Box::new(out),
    };
  }

  Ok(UnifiedExpr::Let {
    name: tmp_name,
    value: Box::new(value),
    body: Box::new(out),
  })
}

fn parse_let(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Err(ClojureError::UnsupportedSyntax(
      "let expects bindings vector and optional body".to_string(),
    ));
  }

  let bindings = match &items[0] {
    CljForm::Vector(values) => values,
    _ => {
      return Err(ClojureError::UnsupportedSyntax(
        "let expects vector bindings".to_string(),
      ))
    }
  };

  if bindings.len() % 2 != 0 {
    return Err(ClojureError::UnsupportedSyntax(
      "let bindings must be even".to_string(),
    ));
  }

  let mut body = parse_do(&items[1..])?;
  let mut pairs = bindings.chunks(2).collect::<Vec<_>>();
  pairs.reverse();

  for pair in pairs {
    let value = form_to_unified(&pair[1])?;
    body = bind_pattern(&pair[0], value, body, "let")?;
  }

  Ok(body)
}

fn parse_fn(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.len() < 2 {
    return Err(ClojureError::UnsupportedSyntax(
      "fn expects parameter vector and body".to_string(),
    ));
  }
  if matches!(items[0], CljForm::List(_)) {
    return Err(ClojureError::UnsupportedSyntax(
      "fn multi-arity signatures are not supported yet".to_string(),
    ));
  }

  let params = match &items[0] {
    CljForm::Vector(values) => values,
    _ => {
      return Err(ClojureError::UnsupportedSyntax(
        "fn expects parameter vector".to_string(),
      ))
    }
  };

  let mut normalized_params = Vec::new();
  let mut idx = 0usize;
  while idx < params.len() {
    if matches!(&params[idx], CljForm::Atom(sym) if sym == "&") {
      if idx + 1 >= params.len() || idx + 2 != params.len() {
        return Err(ClojureError::UnsupportedSyntax(
          "fn parameter vector expects '&' followed by one trailing binding".to_string(),
        ));
      }
      normalized_params.push(params[idx + 1].clone());
      break;
    }
    normalized_params.push(params[idx].clone());
    idx += 1;
  }

  if normalized_params.is_empty() {
    return Err(ClojureError::UnsupportedSyntax(
      "fn parameter vector cannot be empty".to_string(),
    ));
  }

  let mut out = parse_do(&items[1..])?;
  for (idx, pattern) in normalized_params.into_iter().enumerate().rev() {
    match pattern {
      CljForm::Atom(name) => {
        out = UnifiedExpr::Lambda {
          param: name,
          body: Box::new(out),
        };
      }
      other => {
        let prefix = format!("__pnix_fn_arg_{}", idx);
        let tmp_name = fresh_internal_binding(&prefix, &[other.clone()]);
        let bound = bind_pattern(&other, UnifiedExpr::Var(tmp_name.clone()), out, "fn")?;
        out = UnifiedExpr::Lambda {
          param: tmp_name,
          body: Box::new(bound),
        };
      }
    }
  }
  Ok(out)
}

fn parse_defn(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.len() < 3 {
    return Err(ClojureError::UnsupportedSyntax(
      "defn expects name, parameter vector, and body".to_string(),
    ));
  }

  let name = match &items[0] {
    CljForm::Atom(name) if is_valid_symbol_name(name) => name.clone(),
    _ => {
      return Err(ClojureError::UnsupportedSyntax(
        "defn name must be symbol".to_string(),
      ))
    }
  };

  let mut idx = 1usize;
  if matches!(items.get(idx), Some(CljForm::Str(_))) {
    idx += 1;
  }
  if matches!(items.get(idx), Some(CljForm::Map(_))) {
    idx += 1;
  }
  if idx >= items.len() {
    return Err(ClojureError::UnsupportedSyntax(
      "defn expects parameter vector and body".to_string(),
    ));
  }

  let fn_expr = parse_fn(&items[idx..])?;
  Ok(UnifiedExpr::Let {
    name: name.clone(),
    value: Box::new(fn_expr),
    body: Box::new(UnifiedExpr::Var(name)),
  })
}

fn parse_letfn(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.len() < 2 {
    return Err(ClojureError::UnsupportedSyntax(
      "letfn expects bindings vector and body".to_string(),
    ));
  }

  let bindings = match &items[0] {
    CljForm::Vector(values) => values,
    _ => {
      return Err(ClojureError::UnsupportedSyntax(
        "letfn expects vector bindings".to_string(),
      ))
    }
  };

  let mut out = parse_do(&items[1..])?;
  for binding in bindings.iter().rev() {
    let spec = match binding {
      CljForm::List(spec) => spec,
      _ => {
        return Err(ClojureError::UnsupportedSyntax(
          "letfn binding must be list form".to_string(),
        ))
      }
    };
    if spec.len() < 3 {
      return Err(ClojureError::UnsupportedSyntax(
        "letfn binding expects (name [params] body...)".to_string(),
      ));
    }
    let name = match &spec[0] {
      CljForm::Atom(sym) if is_valid_symbol_name(sym) => sym.clone(),
      _ => {
        return Err(ClojureError::UnsupportedSyntax(
          "letfn binding name must be symbol".to_string(),
        ))
      }
    };
    let fn_expr = parse_fn(&spec[1..])?;
    out = UnifiedExpr::Let {
      name,
      value: Box::new(fn_expr),
      body: Box::new(out),
    };
  }

  Ok(out)
}

fn parse_do(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Ok(UnifiedExpr::Null);
  }
  let mut last = UnifiedExpr::Null;
  for item in items {
    last = form_to_unified(item)?;
  }
  Ok(last)
}

fn parse_thread_macro(items: &[CljForm], thread_last: bool) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Err(ClojureError::UnsupportedSyntax(format!(
      "{} expects at least 1 argument",
      if thread_last { "->>" } else { "->" }
    )));
  }

  let expanded = expand_thread_macro(items, thread_last)?;
  form_to_unified(&expanded)
}

fn parse_some_thread_macro(
  items: &[CljForm],
  thread_last: bool,
) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Err(ClojureError::UnsupportedSyntax(format!(
      "{} expects at least 1 argument",
      if thread_last { "some->>" } else { "some->" }
    )));
  }

  let mut acc_expr = form_to_unified(&items[0])?;
  for (idx, step) in items[1..].iter().enumerate() {
    let prefix = format!("__pnix_some_thread_tmp_{}", idx);
    let tmp_name = fresh_internal_binding(&prefix, items);
    let tmp_var_expr = UnifiedExpr::Var(tmp_name.clone());
    let step_form = apply_thread_step_form(CljForm::Atom(tmp_name.clone()), step, thread_last)?;
    let step_expr = form_to_unified(&step_form)?;
    acc_expr = UnifiedExpr::Let {
      name: tmp_name.clone(),
      value: Box::new(acc_expr),
      body: Box::new(UnifiedExpr::If {
        // some->/some->> short-circuit only on nil, not on false.
        cond: Box::new(UnifiedExpr::Ne(
          Box::new(tmp_var_expr.clone()),
          Box::new(UnifiedExpr::Null),
        )),
        then_: Box::new(step_expr),
        else_: Box::new(UnifiedExpr::Null),
      }),
    };
  }
  Ok(acc_expr)
}

fn parse_cond_thread_macro(
  items: &[CljForm],
  thread_last: bool,
) -> Result<UnifiedExpr, ClojureError> {
  let op_name = if thread_last { "cond->>" } else { "cond->" };
  if items.is_empty() {
    return Err(ClojureError::UnsupportedSyntax(format!(
      "{} expects at least 1 argument",
      op_name
    )));
  }
  if (items.len() - 1) % 2 != 0 {
    return Err(ClojureError::UnsupportedSyntax(format!(
      "{} expects pairs of test and threading step",
      op_name
    )));
  }

  let mut acc_expr = form_to_unified(&items[0])?;
  for (idx, pair) in items[1..].chunks(2).enumerate() {
    let test_expr = form_to_unified(&pair[0])?;
    let prefix = format!("__pnix_cond_thread_tmp_{}", idx);
    let tmp_name = fresh_internal_binding(&prefix, items);
    let step_form = apply_thread_step_form(CljForm::Atom(tmp_name.clone()), &pair[1], thread_last)?;
    let step_expr = form_to_unified(&step_form)?;
    acc_expr = UnifiedExpr::Let {
      name: tmp_name.clone(),
      value: Box::new(acc_expr),
      body: Box::new(UnifiedExpr::If {
        cond: Box::new(test_expr),
        then_: Box::new(step_expr),
        else_: Box::new(UnifiedExpr::Var(tmp_name)),
      }),
    };
  }
  Ok(acc_expr)
}

fn parse_as_thread_macro(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.len() < 2 {
    return Err(ClojureError::UnsupportedSyntax(
      "as-> expects initial value, binding symbol, and optional forms".to_string(),
    ));
  }
  let binding_name = match &items[1] {
    CljForm::Atom(sym) => sym.clone(),
    _ => {
      return Err(ClojureError::UnsupportedSyntax(
        "as-> binding name must be symbol".to_string(),
      ))
    }
  };

  let mut acc_expr = form_to_unified(&items[0])?;
  if items.len() == 2 {
    return Ok(acc_expr);
  }

  for step in &items[2..] {
    acc_expr = UnifiedExpr::Let {
      name: binding_name.clone(),
      value: Box::new(acc_expr),
      body: Box::new(form_to_unified(step)?),
    };
  }
  Ok(acc_expr)
}

fn parse_and_macro(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Ok(UnifiedExpr::Bool(true));
  }
  if items.len() == 1 {
    return form_to_unified(&items[0]);
  }

  let mut acc_expr = form_to_unified(items.last().unwrap())?;
  for (idx, item) in items[..items.len() - 1].iter().enumerate().rev() {
    let prefix = format!("__pnix_and_tmp_{}", idx);
    let tmp_name = fresh_internal_binding(&prefix, items);
    let tmp_var = UnifiedExpr::Var(tmp_name.clone());
    acc_expr = UnifiedExpr::Let {
      name: tmp_name.clone(),
      value: Box::new(form_to_unified(item)?),
      body: Box::new(UnifiedExpr::If {
        cond: Box::new(truthy_expr_for_var(&tmp_name)),
        then_: Box::new(acc_expr),
        else_: Box::new(tmp_var),
      }),
    };
  }
  Ok(acc_expr)
}

fn parse_or_macro(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Ok(UnifiedExpr::Null);
  }
  if items.len() == 1 {
    return form_to_unified(&items[0]);
  }

  let mut acc_expr = form_to_unified(items.last().unwrap())?;
  for (idx, item) in items[..items.len() - 1].iter().enumerate().rev() {
    let prefix = format!("__pnix_or_tmp_{}", idx);
    let tmp_name = fresh_internal_binding(&prefix, items);
    let tmp_var = UnifiedExpr::Var(tmp_name.clone());
    acc_expr = UnifiedExpr::Let {
      name: tmp_name.clone(),
      value: Box::new(form_to_unified(item)?),
      body: Box::new(UnifiedExpr::If {
        cond: Box::new(truthy_expr_for_var(&tmp_name)),
        then_: Box::new(tmp_var),
        else_: Box::new(acc_expr),
      }),
    };
  }
  Ok(acc_expr)
}

fn expand_thread_macro(items: &[CljForm], thread_last: bool) -> Result<CljForm, ClojureError> {
  let mut acc = items[0].clone();
  for step in &items[1..] {
    acc = apply_thread_step_form(acc, step, thread_last)?;
  }
  Ok(acc)
}

fn apply_thread_step_form(
  acc: CljForm,
  step: &CljForm,
  thread_last: bool,
) -> Result<CljForm, ClojureError> {
  match step {
    CljForm::Atom(sym) => Ok(CljForm::List(vec![CljForm::Atom(sym.clone()), acc])),
    CljForm::List(forms) => {
      if forms.is_empty() {
        return Err(ClojureError::UnsupportedSyntax(
          "threading step cannot be empty list".to_string(),
        ));
      }
      let mut rewritten = forms.clone();
      if thread_last {
        rewritten.push(acc);
      } else {
        rewritten.insert(1, acc);
      }
      Ok(CljForm::List(rewritten))
    }
    _ => Err(ClojureError::UnsupportedSyntax(
      "threading step must be symbol or list".to_string(),
    )),
  }
}

fn parse_unary<F>(items: &[CljForm], name: &str, ctor: F) -> Result<UnifiedExpr, ClojureError>
where
  F: Fn(Box<UnifiedExpr>) -> UnifiedExpr,
{
  if items.len() != 1 {
    return Err(ClojureError::UnsupportedSyntax(format!(
      "{} expects exactly 1 argument",
      name
    )));
  }
  Ok(ctor(Box::new(form_to_unified(&items[0])?)))
}

fn parse_binary_exact<F>(
  items: &[CljForm],
  name: &str,
  ctor: F,
) -> Result<UnifiedExpr, ClojureError>
where
  F: Fn(Box<UnifiedExpr>, Box<UnifiedExpr>) -> UnifiedExpr,
{
  if items.len() != 2 {
    return Err(ClojureError::UnsupportedSyntax(format!(
      "{} expects exactly 2 arguments",
      name
    )));
  }
  Ok(ctor(
    Box::new(form_to_unified(&items[0])?),
    Box::new(form_to_unified(&items[1])?),
  ))
}

fn parse_apply_with_arity(
  items: &[CljForm],
  name: &str,
  min_args: usize,
  max_args: Option<usize>,
) -> Result<UnifiedExpr, ClojureError> {
  if items.len() < min_args {
    return Err(ClojureError::UnsupportedSyntax(match max_args {
      Some(max) if min_args == max => format!("{name} expects exactly {min_args} arguments"),
      Some(max) => format!("{name} expects between {min_args} and {max} arguments"),
      None => format!("{name} expects at least {min_args} arguments"),
    }));
  }
  if let Some(max_args) = max_args {
    if items.len() > max_args {
      return Err(ClojureError::UnsupportedSyntax(if min_args == max_args {
        format!("{name} expects exactly {min_args} arguments")
      } else {
        format!("{name} expects between {min_args} and {max_args} arguments")
      }));
    }
  }

  let args = items
    .iter()
    .map(form_to_unified)
    .collect::<Result<Vec<_>, _>>()?;
  Ok(UnifiedExpr::Apply {
    func: name.to_string(),
    args,
  })
}

fn fold_add(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Ok(UnifiedExpr::Int(0));
  }
  if items.len() == 1 {
    return form_to_unified(&items[0]);
  }

  let mut iter = items.iter();
  let first = form_to_unified(iter.next().unwrap())?;
  let mut expr = first;
  for item in iter {
    let rhs = form_to_unified(item)?;
    expr = UnifiedExpr::Add(Box::new(expr), Box::new(rhs));
  }
  Ok(expr)
}

fn fold_mul(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Ok(UnifiedExpr::Int(1));
  }
  if items.len() == 1 {
    return form_to_unified(&items[0]);
  }

  let mut iter = items.iter();
  let first = form_to_unified(iter.next().unwrap())?;
  let mut expr = first;
  for item in iter {
    let rhs = form_to_unified(item)?;
    expr = UnifiedExpr::Mul(Box::new(expr), Box::new(rhs));
  }
  Ok(expr)
}

fn fold_binary<F>(items: &[CljForm], op: F, min_args: usize) -> Result<UnifiedExpr, ClojureError>
where
  F: Fn(Box<UnifiedExpr>, Box<UnifiedExpr>) -> UnifiedExpr,
{
  if items.len() < min_args {
    return Err(ClojureError::UnsupportedSyntax(format!(
      "operator expects at least {} arguments",
      min_args
    )));
  }

  let mut iter = items.iter();
  let first = form_to_unified(iter.next().unwrap())?;
  let mut expr = first;

  for item in iter {
    let rhs = form_to_unified(item)?;
    expr = op(Box::new(expr), Box::new(rhs));
  }

  Ok(expr)
}

fn fold_concat(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.is_empty() {
    return Ok(UnifiedExpr::String(String::new()));
  }
  if items.len() == 1 {
    return form_to_unified(&items[0]);
  }
  fold_binary(items, UnifiedExpr::Concat, 2)
}

fn fold_order_compare<F>(
  items: &[CljForm],
  cmp: F,
  op_name: &str,
) -> Result<UnifiedExpr, ClojureError>
where
  F: Fn(Box<UnifiedExpr>, Box<UnifiedExpr>) -> UnifiedExpr,
{
  if items.len() < 2 {
    return Err(ClojureError::UnsupportedSyntax(format!(
      "{} expects at least 2 arguments",
      op_name
    )));
  }

  let values = items
    .iter()
    .map(form_to_unified)
    .collect::<Result<Vec<_>, _>>()?;

  let mut checks = values
    .windows(2)
    .map(|pair| cmp(Box::new(pair[0].clone()), Box::new(pair[1].clone())))
    .collect::<Vec<_>>();

  let first = checks
    .drain(0..1)
    .next()
    .ok_or_else(|| ClojureError::UnsupportedSyntax("invalid comparison chain".to_string()))?;
  Ok(checks.into_iter().fold(first, |acc, next| {
    UnifiedExpr::And(Box::new(acc), Box::new(next))
  }))
}

fn fold_equality_compare<F>(
  items: &[CljForm],
  eq_op: F,
  op_name: &str,
) -> Result<UnifiedExpr, ClojureError>
where
  F: Fn(Box<UnifiedExpr>, Box<UnifiedExpr>) -> UnifiedExpr,
{
  fold_order_compare(items, eq_op, op_name)
}

fn fold_not_equal(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  if items.len() < 2 {
    return Err(ClojureError::UnsupportedSyntax(
      "not= expects at least 2 arguments".to_string(),
    ));
  }

  let values = items
    .iter()
    .map(form_to_unified)
    .collect::<Result<Vec<_>, _>>()?;

  let mut checks = Vec::new();
  for i in 0..values.len() {
    for j in (i + 1)..values.len() {
      checks.push(UnifiedExpr::Ne(
        Box::new(values[i].clone()),
        Box::new(values[j].clone()),
      ));
    }
  }

  let first = checks
    .drain(0..1)
    .next()
    .ok_or_else(|| ClojureError::UnsupportedSyntax("invalid not= chain".to_string()))?;
  Ok(checks.into_iter().fold(first, |acc, next| {
    UnifiedExpr::And(Box::new(acc), Box::new(next))
  }))
}

fn fold_sub(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  match items.len() {
    0 => Err(ClojureError::UnsupportedSyntax(
      "- expects at least one argument".to_string(),
    )),
    1 => Ok(UnifiedExpr::Neg(Box::new(form_to_unified(&items[0])?))),
    _ => {
      let mut iter = items.iter();
      let first = form_to_unified(iter.next().unwrap())?;
      let mut expr = first;
      for item in iter {
        let rhs = form_to_unified(item)?;
        expr = UnifiedExpr::Sub(Box::new(expr), Box::new(rhs));
      }
      Ok(expr)
    }
  }
}

fn fold_div(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  match items.len() {
    0 => Err(ClojureError::UnsupportedSyntax(
      "/ expects at least one argument".to_string(),
    )),
    1 => Ok(UnifiedExpr::Div(
      Box::new(UnifiedExpr::Int(1)),
      Box::new(form_to_unified(&items[0])?),
    )),
    _ => {
      let mut iter = items.iter();
      let first = form_to_unified(iter.next().unwrap())?;
      let mut expr = first;
      for item in iter {
        let rhs = form_to_unified(item)?;
        expr = UnifiedExpr::Div(Box::new(expr), Box::new(rhs));
      }
      Ok(expr)
    }
  }
}

fn map_key_to_attr_name(key: &CljForm) -> Result<String, ClojureError> {
  match key {
    CljForm::Atom(atom) => Ok(atom.strip_prefix(':').unwrap_or(atom.as_str()).to_string()),
    CljForm::Str(value) => Ok(value.clone()),
    CljForm::Quote(inner) => map_key_to_attr_name(inner),
    _ => Err(ClojureError::UnsupportedSyntax(
      "map key must be atom/string/keyword".to_string(),
    )),
  }
}

fn parse_vector_literal(items: &[CljForm]) -> Result<UnifiedExpr, ClojureError> {
  let values = items
    .iter()
    .map(form_to_unified)
    .collect::<Result<Vec<_>, _>>()?;
  Ok(UnifiedExpr::List(values))
}

fn parse_map_literal(pairs: &[(CljForm, CljForm)]) -> Result<UnifiedExpr, ClojureError> {
  let mut items = Vec::with_capacity(pairs.len());
  for (k, v) in pairs {
    items.push((map_key_to_attr_name(k)?, form_to_unified(v)?));
  }
  Ok(UnifiedExpr::AttrSet(items))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_simple_add() {
    let expr = parse_clj_expr("(+ 1 2)").unwrap();
    assert!(matches!(expr, UnifiedExpr::Add(..)));
  }

  #[test]
  fn parse_param_time() {
    let expr = parse_clj_expr("param/system-time").unwrap();
    assert!(matches!(expr, UnifiedExpr::ParamTime));
  }

  #[test]
  fn parse_let() {
    let expr = parse_clj_expr("(let [x 1] (+ x 2))").unwrap();
    assert!(matches!(expr, UnifiedExpr::Let { .. }));
  }

  #[test]
  fn parse_when_unless_cond() {
    let when_expr = parse_clj_expr("(when true 42)").unwrap();
    assert!(matches!(when_expr, UnifiedExpr::If { .. }));

    let when_not_expr = parse_clj_expr("(when-not false 9)").unwrap();
    assert!(matches!(when_not_expr, UnifiedExpr::If { .. }));

    let unless_expr = parse_clj_expr("(unless false 7)").unwrap();
    assert!(matches!(unless_expr, UnifiedExpr::If { .. }));

    let cond_expr = parse_clj_expr("(cond false 1 :else 2)").unwrap();
    assert!(matches!(cond_expr, UnifiedExpr::If { .. }));

    let case_expr = parse_clj_expr("(case 2 1 \"one\" 2 \"two\" \"other\")").unwrap();
    assert!(matches!(case_expr, UnifiedExpr::Let { .. }));
  }

  #[test]
  fn parse_optional_body_and_else_forms() {
    let if_expr = parse_clj_expr("(if true 1)").unwrap();
    let UnifiedExpr::If { else_, .. } = if_expr else {
      panic!("expected if");
    };
    assert!(matches!(*else_, UnifiedExpr::Null));

    let if_not_expr = parse_clj_expr("(if-not false 1)").unwrap();
    let UnifiedExpr::If { then_, else_, .. } = if_not_expr else {
      panic!("expected if-not");
    };
    assert!(matches!(*then_, UnifiedExpr::Null));
    assert!(matches!(*else_, UnifiedExpr::Int(1)));

    let when_expr = parse_clj_expr("(when true)").unwrap();
    let UnifiedExpr::If { then_, else_, .. } = when_expr else {
      panic!("expected when->if");
    };
    assert!(matches!(*then_, UnifiedExpr::Null));
    assert!(matches!(*else_, UnifiedExpr::Null));

    let when_not_expr = parse_clj_expr("(when-not false)").unwrap();
    let UnifiedExpr::If { then_, else_, .. } = when_not_expr else {
      panic!("expected when-not->if");
    };
    assert!(matches!(*then_, UnifiedExpr::Null));
    assert!(matches!(*else_, UnifiedExpr::Null));

    let unless_expr = parse_clj_expr("(unless false)").unwrap();
    let UnifiedExpr::If { then_, else_, .. } = unless_expr else {
      panic!("expected unless->if");
    };
    assert!(matches!(*then_, UnifiedExpr::Null));
    assert!(matches!(*else_, UnifiedExpr::Null));

    let let_expr = parse_clj_expr("(let [x 1])").unwrap();
    let UnifiedExpr::Let { body, .. } = let_expr else {
      panic!("expected let");
    };
    assert!(matches!(*body, UnifiedExpr::Null));

    let do_expr = parse_clj_expr("(do)").unwrap();
    assert!(matches!(do_expr, UnifiedExpr::Null));
  }

  #[test]
  fn parse_if_let_and_when_let() {
    let if_let_expr = parse_clj_expr("(if-let [x 5] (+ x 1) 0)").unwrap();
    let UnifiedExpr::Let { body, .. } = if_let_expr else {
      panic!("expected outer let for if-let");
    };
    let UnifiedExpr::If { then_, else_, .. } = *body else {
      panic!("expected if body for if-let");
    };
    assert!(matches!(*then_, UnifiedExpr::Let { name, .. } if name == "x"));
    assert!(matches!(*else_, UnifiedExpr::Int(0)));

    let when_let_expr = parse_clj_expr("(when-let [x 7] (+ x 2))").unwrap();
    let UnifiedExpr::Let { body, .. } = when_let_expr else {
      panic!("expected outer let for when-let");
    };
    let UnifiedExpr::If { then_, else_, .. } = *body else {
      panic!("expected if body for when-let");
    };
    assert!(matches!(*then_, UnifiedExpr::Let { name, .. } if name == "x"));
    assert!(matches!(*else_, UnifiedExpr::Null));
  }

  #[test]
  fn parse_if_some_and_when_some() {
    let if_some_expr = parse_clj_expr("(if-some [x false] (+ 1 2) 0)").unwrap();
    let UnifiedExpr::Let { body, .. } = if_some_expr else {
      panic!("expected outer let for if-some");
    };
    let UnifiedExpr::If { cond, then_, else_ } = *body else {
      panic!("expected if body for if-some");
    };
    let UnifiedExpr::Ne(lhs, rhs) = *cond else {
      panic!("expected nil-check condition");
    };
    assert!(matches!(*lhs, UnifiedExpr::Var(_)));
    assert!(matches!(*rhs, UnifiedExpr::Null));
    assert!(matches!(*then_, UnifiedExpr::Let { name, .. } if name == "x"));
    assert!(matches!(*else_, UnifiedExpr::Int(0)));

    let when_some_expr = parse_clj_expr("(when-some [x 7] (+ x 2))").unwrap();
    let UnifiedExpr::Let { body, .. } = when_some_expr else {
      panic!("expected outer let for when-some");
    };
    let UnifiedExpr::If { cond, then_, else_ } = *body else {
      panic!("expected if body for when-some");
    };
    let UnifiedExpr::Ne(_, rhs) = *cond else {
      panic!("expected nil-check condition");
    };
    assert!(matches!(*rhs, UnifiedExpr::Null));
    assert!(matches!(*then_, UnifiedExpr::Let { name, .. } if name == "x"));
    assert!(matches!(*else_, UnifiedExpr::Null));
  }

  #[test]
  fn parse_if_let_when_let_validation_errors() {
    let err = parse_clj_expr("(if-let [x] x 0)").unwrap_err();
    assert!(format!("{err}").contains("if-let binding vector must contain exactly 2 forms"));

    let err = parse_clj_expr("(when-let [x] x)").unwrap_err();
    assert!(format!("{err}").contains("when-let binding vector must contain exactly 2 forms"));

    let err = parse_clj_expr("(if-let [x 1] x 0 1)").unwrap_err();
    assert!(format!("{err}").contains("if-let expects binding, then, and optional else"));

    let err = parse_clj_expr("(if-some [x] x 0)").unwrap_err();
    assert!(format!("{err}").contains("if-some binding vector must contain exactly 2 forms"));

    let err = parse_clj_expr("(when-some [x] x)").unwrap_err();
    assert!(format!("{err}").contains("when-some binding vector must contain exactly 2 forms"));

    let err = parse_clj_expr("(if-some [x 1] x 0 1)").unwrap_err();
    assert!(format!("{err}").contains("if-some expects binding, then, and optional else"));

    let err = parse_clj_expr("(case 1 2)").unwrap_err();
    assert!(format!("{err}")
      .contains("case expects expression, one or more test/result pairs, and default"));

    let err = parse_clj_expr("(case 1 1 \"one\" 2 \"two\")").unwrap_err();
    assert!(format!("{err}").contains("case expects expression, test/result pairs, and default"));

    let err = parse_clj_expr("(if-not true 1 2 3)").unwrap_err();
    assert!(format!("{err}").contains("if-not expects 2 or 3 arguments"));

    let err = parse_clj_expr("(when-not)").unwrap_err();
    assert!(format!("{err}").contains("when-not expects condition and body"));
  }

  #[test]
  fn parse_thread_first_and_last() {
    let thread_first = parse_clj_expr("(-> 5 (+ 1) (* 2))").unwrap();
    assert!(matches!(thread_first, UnifiedExpr::Mul(..)));

    let thread_last = parse_clj_expr("(->> 5 (+ 1) (* 2))").unwrap();
    assert!(matches!(thread_last, UnifiedExpr::Mul(..)));
  }

  #[test]
  fn parse_some_thread_first_and_last() {
    let some_thread_first = parse_clj_expr("(some-> 5 (+ 1) (* 2))").unwrap();
    assert!(matches!(some_thread_first, UnifiedExpr::Let { .. }));

    let some_thread_last = parse_clj_expr("(some->> 5 (+ 1) (* 2))").unwrap();
    assert!(matches!(some_thread_last, UnifiedExpr::Let { .. }));

    // some-> must short-circuit only on nil; false should flow to next step.
    let false_chain = parse_clj_expr("(some-> false not)").unwrap();
    let UnifiedExpr::Let { name, body, .. } = false_chain else {
      panic!("expected some-> lowering to let");
    };
    let UnifiedExpr::If { cond, then_, else_ } = *body else {
      panic!("expected some-> lowering body to if");
    };
    let UnifiedExpr::Ne(lhs, rhs) = *cond else {
      panic!("expected nil-check condition");
    };
    assert!(matches!(*lhs, UnifiedExpr::Var(v) if v == name));
    assert!(matches!(*rhs, UnifiedExpr::Null));
    assert!(matches!(*then_, UnifiedExpr::Not(..)));
    assert!(matches!(*else_, UnifiedExpr::Null));
  }

  #[test]
  fn parse_cond_thread_first_and_last() {
    let cond_thread_first = parse_clj_expr("(cond-> 5 true (- 1) false (* 100))").unwrap();
    assert!(matches!(cond_thread_first, UnifiedExpr::Let { .. }));

    let cond_thread_last = parse_clj_expr("(cond->> 5 true (- 1) false (* 100))").unwrap();
    assert!(matches!(cond_thread_last, UnifiedExpr::Let { .. }));

    let UnifiedExpr::Let { body, .. } = cond_thread_first else {
      panic!("expected cond-> lowering to let");
    };
    let UnifiedExpr::If { else_, .. } = *body else {
      panic!("expected cond-> lowering body to if");
    };
    assert!(matches!(*else_, UnifiedExpr::Var(_)));
  }

  #[test]
  fn parse_as_thread_macro() {
    let as_thread = parse_clj_expr("(as-> 5 x (+ x 1) (* x 2))").unwrap();
    assert!(matches!(as_thread, UnifiedExpr::Let { name, .. } if name == "x"));

    let as_thread_no_steps = parse_clj_expr("(as-> 5 x)").unwrap();
    assert!(matches!(as_thread_no_steps, UnifiedExpr::Int(5)));
  }

  #[test]
  fn parse_thread_macro_validation_errors() {
    let err = parse_clj_expr("(->)").unwrap_err();
    assert!(format!("{err}").contains("-> expects at least 1 argument"));

    let err = parse_clj_expr("(-> 1 ())").unwrap_err();
    assert!(format!("{err}").contains("threading step cannot be empty list"));

    let err = parse_clj_expr("(-> 1 [inc])").unwrap_err();
    assert!(format!("{err}").contains("threading step must be symbol or list"));
  }

  #[test]
  fn parse_some_thread_macro_validation_errors() {
    let err = parse_clj_expr("(some->)").unwrap_err();
    assert!(format!("{err}").contains("some-> expects at least 1 argument"));

    let err = parse_clj_expr("(some-> 1 ())").unwrap_err();
    assert!(format!("{err}").contains("threading step cannot be empty list"));

    let err = parse_clj_expr("(some-> 1 [inc])").unwrap_err();
    assert!(format!("{err}").contains("threading step must be symbol or list"));
  }

  #[test]
  fn parse_cond_thread_macro_validation_errors() {
    let err = parse_clj_expr("(cond->)").unwrap_err();
    assert!(format!("{err}").contains("cond-> expects at least 1 argument"));

    let err = parse_clj_expr("(cond-> 1 true)").unwrap_err();
    assert!(format!("{err}").contains("cond-> expects pairs of test and threading step"));

    let err = parse_clj_expr("(cond-> 1 true ())").unwrap_err();
    assert!(format!("{err}").contains("threading step cannot be empty list"));

    let err = parse_clj_expr("(cond-> 1 true [inc])").unwrap_err();
    assert!(format!("{err}").contains("threading step must be symbol or list"));
  }

  #[test]
  fn parse_as_thread_macro_validation_errors() {
    let err = parse_clj_expr("(as->)").unwrap_err();
    assert!(
      format!("{err}").contains("as-> expects initial value, binding symbol, and optional forms")
    );

    let err = parse_clj_expr("(as-> 1)").unwrap_err();
    assert!(
      format!("{err}").contains("as-> expects initial value, binding symbol, and optional forms")
    );

    let err = parse_clj_expr("(as-> 1 [x] (+ x 1))").unwrap_err();
    assert!(format!("{err}").contains("as-> binding name must be symbol"));
  }

  #[test]
  fn parse_logic_and_comparison_forms() {
    let lt_chain = parse_clj_expr("(< 1 2 3)").unwrap();
    assert!(matches!(lt_chain, UnifiedExpr::And(..)));

    let and_expr = parse_clj_expr("(and true false)").unwrap();
    assert!(matches!(and_expr, UnifiedExpr::Let { .. }));

    let and_empty = parse_clj_expr("(and)").unwrap();
    assert!(matches!(and_empty, UnifiedExpr::Bool(true)));

    let or_expr = parse_clj_expr("(or false 42)").unwrap();
    assert!(matches!(or_expr, UnifiedExpr::Let { .. }));

    let or_empty = parse_clj_expr("(or)").unwrap();
    assert!(matches!(or_empty, UnifiedExpr::Null));

    let not_expr = parse_clj_expr("(not true)").unwrap();
    assert!(matches!(not_expr, UnifiedExpr::Not(..)));

    let nil_pred = parse_clj_expr("(nil? nil)").unwrap();
    assert!(matches!(nil_pred, UnifiedExpr::Eq(..)));

    let some_pred = parse_clj_expr("(some? false)").unwrap();
    assert!(matches!(some_pred, UnifiedExpr::Ne(..)));

    let true_pred = parse_clj_expr("(true? true)").unwrap();
    assert!(matches!(true_pred, UnifiedExpr::Eq(..)));

    let false_pred = parse_clj_expr("(false? false)").unwrap();
    assert!(matches!(false_pred, UnifiedExpr::Eq(..)));

    let neq_expr = parse_clj_expr("(not= 1 2 3)").unwrap();
    assert!(matches!(neq_expr, UnifiedExpr::And(..)));
  }

  #[test]
  fn parse_math_and_string_forms() {
    let sqrt_expr = parse_clj_expr("(sqrt 9)").unwrap();
    assert!(matches!(sqrt_expr, UnifiedExpr::Sqrt(..)));

    let log_expr = parse_clj_expr("(log 1)").unwrap();
    assert!(matches!(log_expr, UnifiedExpr::Ln(..)));

    let pow_expr = parse_clj_expr("(pow 2 3)").unwrap();
    assert!(matches!(pow_expr, UnifiedExpr::Pow(..)));

    let str_expr = parse_clj_expr("(str \"a\" \"b\" \"c\")").unwrap();
    assert!(matches!(str_expr, UnifiedExpr::Concat(..)));
  }

  #[test]
  fn parse_namespaced_core_and_math_forms() {
    let add_expr = parse_clj_expr("(clojure.core/+ 1 2)").unwrap();
    assert!(matches!(add_expr, UnifiedExpr::Add(..)));

    let inc_expr = parse_clj_expr("(clojure.core/inc 1)").unwrap();
    let UnifiedExpr::Apply { func, args } = inc_expr else {
      panic!("expected Apply for namespaced clojure.core/inc");
    };
    assert_eq!(func, "inc");
    assert_eq!(args.len(), 1);

    let str_expr = parse_clj_expr("(clojure.core/str \"a\" \"b\")").unwrap();
    assert!(matches!(str_expr, UnifiedExpr::Concat(..)));

    let log_expr = parse_clj_expr("(clojure.math/log 1)").unwrap();
    assert!(matches!(log_expr, UnifiedExpr::Ln(..)));

    let log10_expr = parse_clj_expr("(clojure.math/log10 1000)").unwrap();
    let UnifiedExpr::Apply { func, args } = log10_expr else {
      panic!("expected Apply for namespaced clojure.math/log10");
    };
    assert_eq!(func, "log10");
    assert_eq!(args.len(), 1);

    let pow_expr = parse_clj_expr("(clojure.math/pow 2 3)").unwrap();
    assert!(matches!(pow_expr, UnifiedExpr::Pow(..)));
  }

  #[test]
  fn parse_namespaced_core_math_unknown_symbol_is_fail_closed() {
    let core_err = parse_clj_expr("(clojure.core/does-not-exist 1)").unwrap_err();
    let core_msg = format!("{core_err}");
    assert!(core_msg.contains("EVAL_TARGET_PNIX_UNSUPPORTED_MORPHISM"));
    assert!(core_msg.contains("clojure.core/does-not-exist"));

    let math_err = parse_clj_expr("(clojure.math/tau 1)").unwrap_err();
    let math_msg = format!("{math_err}");
    assert!(math_msg.contains("EVAL_TARGET_PNIX_UNSUPPORTED_MORPHISM"));
    assert!(math_msg.contains("clojure.math/tau"));
  }

  #[test]
  fn parse_non_stdlib_namespaced_symbol_keeps_apply_shape() {
    let expr = parse_clj_expr("(user.math/pow 2 3)").unwrap();
    let UnifiedExpr::Apply { func, args } = expr else {
      panic!("expected Apply for namespaced non-stdlib symbol");
    };
    assert_eq!(func, "user.math/pow");
    assert_eq!(args.len(), 2);
  }

  #[test]
  fn parse_vector_and_map_literals() {
    let vec_expr = parse_clj_expr("[1 2 3]").unwrap();
    assert!(matches!(vec_expr, UnifiedExpr::List(v) if v.len() == 3));

    let map_expr = parse_clj_expr("{:x 1 \"y\" 2}").unwrap();
    assert!(matches!(map_expr, UnifiedExpr::AttrSet(items) if items.len() == 2));
  }

  #[test]
  fn parse_sequence_basic_forms() {
    let cases = [
      ("(seq [1 2])", "seq", 1usize),
      ("(first [1 2])", "first", 1usize),
      ("(rest [1 2])", "rest", 1usize),
      ("(next [1 2])", "next", 1usize),
      ("(nth [1 2 3] 1)", "nth", 2usize),
      ("(nth [1 2 3] 7 :nf)", "nth", 3usize),
      ("(last [1 2])", "last", 1usize),
      ("(butlast [1 2])", "butlast", 1usize),
      ("(take 2 [1 2 3])", "take", 2usize),
      ("(drop 1 [1 2 3])", "drop", 2usize),
      ("(concat)", "concat", 0usize),
      ("(concat [1] [2] [3])", "concat", 3usize),
      ("(cons 0 [1 2])", "cons", 2usize),
      ("(conj [1 2] 3 4)", "conj", 3usize),
      ("(into [] [1 2])", "into", 2usize),
      ("(into [] [1 2] [3])", "into", 3usize),
      ("(vec (list 1 2))", "vec", 1usize),
      ("(list)", "list", 0usize),
      ("(list 1 2 3)", "list", 3usize),
      ("(set [1 2 2])", "set", 1usize),
    ];

    for (source, expected_func, expected_arity) in cases {
      let expr = parse_clj_expr(source).unwrap();
      let UnifiedExpr::Apply { func, args } = expr else {
        panic!("expected Apply for {}", source);
      };
      assert_eq!(func, expected_func, "source={}", source);
      assert_eq!(args.len(), expected_arity, "source={}", source);
    }
  }

  #[test]
  fn parse_sequence_basic_arity_errors() {
    let cases = [
      ("(seq)", "seq expects exactly 1 arguments"),
      ("(first [1] [2])", "first expects exactly 1 arguments"),
      ("(nth [1 2])", "nth expects between 2 and 3 arguments"),
      (
        "(nth [1 2] 0 :x :y)",
        "nth expects between 2 and 3 arguments",
      ),
      ("(take 1)", "take expects exactly 2 arguments"),
      ("(drop 1)", "drop expects exactly 2 arguments"),
      ("(cons 1)", "cons expects exactly 2 arguments"),
      ("(conj)", "conj expects at least 1 arguments"),
      ("(into [])", "into expects between 2 and 3 arguments"),
      (
        "(into [] [] [] [])",
        "into expects between 2 and 3 arguments",
      ),
      ("(vec)", "vec expects exactly 1 arguments"),
      ("(set 1 2)", "set expects exactly 1 arguments"),
    ];

    for (source, expected) in cases {
      let err = parse_clj_expr(source).unwrap_err();
      assert!(
        format!("{err}").contains(expected),
        "source={} err={}",
        source,
        err
      );
    }
  }

  #[test]
  fn parse_sequence_higher_order_forms() {
    let cases = [
      ("(map inc [1 2 3])", "map", 2usize),
      ("(map + [1 2] [3 4])", "map", 3usize),
      ("(mapv inc [1 2 3])", "mapv", 2usize),
      ("(filter odd? [1 2 3])", "filter", 2usize),
      ("(remove odd? [1 2 3])", "remove", 2usize),
      ("(keep identity [1 nil 3])", "keep", 2usize),
      ("(reduce + [1 2 3])", "reduce", 2usize),
      ("(reduce + 0 [1 2 3])", "reduce", 3usize),
      (
        "(reduce-kv (fn [acc k v] (+ acc v)) 0 {:a 1})",
        "reduce-kv",
        3usize,
      ),
      ("(some odd? [1 2 3])", "some", 2usize),
      ("(every? odd? [1 3 5])", "every?", 2usize),
      ("(not-any? odd? [2 4 6])", "not-any?", 2usize),
      ("(not-every? odd? [1 2 3])", "not-every?", 2usize),
    ];

    for (source, expected_func, expected_arity) in cases {
      let expr = parse_clj_expr(source).unwrap();
      let UnifiedExpr::Apply { func, args } = expr else {
        panic!("expected Apply for {}", source);
      };
      assert_eq!(func, expected_func, "source={}", source);
      assert_eq!(args.len(), expected_arity, "source={}", source);
    }
  }

  #[test]
  fn parse_sequence_higher_order_arity_errors() {
    let cases = [
      ("(map inc)", "map expects at least 2 arguments"),
      ("(mapv inc)", "mapv expects at least 2 arguments"),
      ("(filter odd?)", "filter expects exactly 2 arguments"),
      ("(remove odd?)", "remove expects exactly 2 arguments"),
      ("(keep identity)", "keep expects exactly 2 arguments"),
      ("(reduce +)", "reduce expects between 2 and 3 arguments"),
      (
        "(reduce + 0 [1 2] [3 4])",
        "reduce expects between 2 and 3 arguments",
      ),
      ("(reduce-kv + 0)", "reduce-kv expects exactly 3 arguments"),
      ("(some odd?)", "some expects exactly 2 arguments"),
      ("(every? odd?)", "every? expects exactly 2 arguments"),
      ("(not-any? odd?)", "not-any? expects exactly 2 arguments"),
      (
        "(not-every? odd?)",
        "not-every? expects exactly 2 arguments",
      ),
    ];

    for (source, expected) in cases {
      let err = parse_clj_expr(source).unwrap_err();
      assert!(
        format!("{err}").contains(expected),
        "source={} err={}",
        source,
        err
      );
    }
  }

  fn contains_apply_func(expr: &UnifiedExpr, name: &str) -> bool {
    match expr {
      UnifiedExpr::Apply { func, args } => {
        func == name || args.iter().any(|arg| contains_apply_func(arg, name))
      }
      UnifiedExpr::Let { value, body, .. } => {
        contains_apply_func(value, name) || contains_apply_func(body, name)
      }
      UnifiedExpr::Lambda { body, .. } => contains_apply_func(body, name),
      UnifiedExpr::If { cond, then_, else_ } => {
        contains_apply_func(cond, name)
          || contains_apply_func(then_, name)
          || contains_apply_func(else_, name)
      }
      UnifiedExpr::Add(lhs, rhs)
      | UnifiedExpr::Sub(lhs, rhs)
      | UnifiedExpr::Mul(lhs, rhs)
      | UnifiedExpr::Div(lhs, rhs)
      | UnifiedExpr::Mod(lhs, rhs)
      | UnifiedExpr::Concat(lhs, rhs)
      | UnifiedExpr::Pow(lhs, rhs)
      | UnifiedExpr::Lt(lhs, rhs)
      | UnifiedExpr::Gt(lhs, rhs)
      | UnifiedExpr::Le(lhs, rhs)
      | UnifiedExpr::Ge(lhs, rhs)
      | UnifiedExpr::Eq(lhs, rhs)
      | UnifiedExpr::Ne(lhs, rhs)
      | UnifiedExpr::And(lhs, rhs)
      | UnifiedExpr::Or(lhs, rhs) => {
        contains_apply_func(lhs, name) || contains_apply_func(rhs, name)
      }
      UnifiedExpr::Neg(arg)
      | UnifiedExpr::Not(arg)
      | UnifiedExpr::Floor(arg)
      | UnifiedExpr::Ceil(arg)
      | UnifiedExpr::Abs(arg)
      | UnifiedExpr::Sqrt(arg)
      | UnifiedExpr::Sin(arg)
      | UnifiedExpr::Cos(arg)
      | UnifiedExpr::Tan(arg)
      | UnifiedExpr::Exp(arg)
      | UnifiedExpr::Ln(arg)
      | UnifiedExpr::Fx(arg) => contains_apply_func(arg, name),
      UnifiedExpr::List(items) | UnifiedExpr::Derived { args: items, .. } => {
        items.iter().any(|item| contains_apply_func(item, name))
      }
      UnifiedExpr::AttrSet(items) => items
        .iter()
        .any(|(_, value)| contains_apply_func(value, name)),
      UnifiedExpr::Merge(lhs, rhs) => {
        contains_apply_func(lhs, name) || contains_apply_func(rhs, name)
      }
      UnifiedExpr::Construct { args, .. } => args.iter().any(|arg| contains_apply_func(arg, name)),
      UnifiedExpr::Int(_)
      | UnifiedExpr::Float(_)
      | UnifiedExpr::Bool(_)
      | UnifiedExpr::String(_)
      | UnifiedExpr::Var(_)
      | UnifiedExpr::ParamTime
      | UnifiedExpr::ParamDeltaTime
      | UnifiedExpr::ParamSignal(_)
      | UnifiedExpr::SignalVar(_)
      | UnifiedExpr::Null
      | UnifiedExpr::Throw(_)
      | UnifiedExpr::Interop { .. } => false,
    }
  }

  #[test]
  fn parse_fn_defn_letfn_forms() {
    let fn_expr = parse_clj_expr("(fn [x y] (+ x y))").unwrap();
    assert!(matches!(fn_expr, UnifiedExpr::Lambda { .. }));

    let defn_expr = parse_clj_expr("(defn add1 [x] (+ x 1))").unwrap();
    let UnifiedExpr::Let { name, value, body } = defn_expr else {
      panic!("expected defn to lower as let");
    };
    assert_eq!(name, "add1");
    assert!(matches!(*value, UnifiedExpr::Lambda { .. }));
    assert!(matches!(*body, UnifiedExpr::Var(v) if v == "add1"));

    let letfn_expr = parse_clj_expr("(letfn [(inc1 [x] (+ x 1))] (inc1 2))").unwrap();
    assert!(matches!(letfn_expr, UnifiedExpr::Let { name, .. } if name == "inc1"));
  }

  #[test]
  fn parse_fn_and_let_destructuring_forms() {
    let let_vec = parse_clj_expr("(let [[x y & rest] [1 2 3 4]] (+ x y))").unwrap();
    assert!(contains_apply_func(&let_vec, "nth"));
    assert!(contains_apply_func(&let_vec, "drop"));

    let let_map = parse_clj_expr("(let [{:keys [a b] :as m} {:a 1 :b 2}] (+ a b))").unwrap();
    assert!(contains_apply_func(&let_map, "getAttr"));

    let fn_map = parse_clj_expr("(fn [{:keys [x]}] x)").unwrap();
    assert!(contains_apply_func(&fn_map, "getAttr"));
  }

  #[test]
  fn parse_function_binding_helper_forms() {
    let cases = [
      ("(apply + [1 2 3])", "apply", 2usize),
      ("(partial + 1 2)", "partial", 3usize),
      ("(comp inc str)", "comp", 2usize),
      ("(comp)", "comp", 0usize),
      ("(juxt inc dec)", "juxt", 2usize),
      ("(identity 1)", "identity", 1usize),
      ("(constantly 7)", "constantly", 1usize),
    ];

    for (source, expected_func, expected_arity) in cases {
      let expr = parse_clj_expr(source).unwrap();
      let UnifiedExpr::Apply { func, args } = expr else {
        panic!("expected Apply for {}", source);
      };
      assert_eq!(func, expected_func, "source={}", source);
      assert_eq!(args.len(), expected_arity, "source={}", source);
    }
  }

  #[test]
  fn parse_function_binding_validation_errors() {
    let cases = [
      ("(fn)", "fn expects parameter vector and body"),
      ("(fn [] 1)", "fn parameter vector cannot be empty"),
      (
        "(fn [x &] x)",
        "fn parameter vector expects '&' followed by one trailing binding",
      ),
      ("(defn 1 [x] x)", "defn name must be symbol"),
      ("(letfn [x] x)", "letfn binding must be list form"),
      (
        "(let [{:foo 1} {:foo 1}] :ok)",
        "let map destructuring supports only :keys and :as",
      ),
      ("(apply +)", "apply expects at least 2 arguments"),
      ("(partial +)", "partial expects at least 2 arguments"),
      ("(juxt)", "juxt expects at least 1 arguments"),
      ("(identity)", "identity expects exactly 1 arguments"),
      ("(constantly 1 2)", "constantly expects exactly 1 arguments"),
    ];

    for (source, expected) in cases {
      let err = parse_clj_expr(source).unwrap_err();
      assert!(
        format!("{err}").contains(expected),
        "source={} err={}",
        source,
        err
      );
    }
  }
}
