use anyhow::{anyhow, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum PxValue {
  String(String),
  List(Vec<PxValue>),
  AttrSet(BTreeMap<String, PxValue>),
}

impl PxValue {
  pub fn as_str(&self) -> Option<&str> {
    match self {
      Self::String(s) => Some(s.as_str()),
      _ => None,
    }
  }

  pub fn as_string_list(&self) -> Vec<String> {
    match self {
      Self::List(items) => items
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect(),
      _ => vec![],
    }
  }

  pub fn as_attrset(&self) -> Option<&BTreeMap<String, PxValue>> {
    match self {
      Self::AttrSet(map) => Some(map),
      _ => None,
    }
  }

  pub fn get(&self, key: &str) -> Option<&PxValue> {
    self.as_attrset()?.get(key)
  }

  pub fn get_str(&self, key: &str) -> String {
    self
      .get(key)
      .and_then(|v| v.as_str())
      .unwrap_or("")
      .to_string()
  }

  pub fn get_str_list(&self, key: &str) -> Vec<String> {
    self
      .get(key)
      .map(|v| v.as_string_list())
      .unwrap_or_default()
  }
}

pub fn parse_px(input: &str) -> Result<PxValue> {
  let mut cursor = SourceCursor::new(input);
  skip_whitespace_and_comments(&mut cursor);
  parse_value(&mut cursor)
}

pub fn parse_px_file(path: &Path) -> Result<PxValue> {
  let content =
    std::fs::read_to_string(path).with_context(|| format!("read .px file {}", path.display()))?;
  parse_px(&content).with_context(|| format!("parse .px file {}", path.display()))
}

/// `parse_px_file` 의 pnix-eval fallback variant.
///
/// 2026-05-17 C6 convergence note: this fallback is a
/// read-model/config loader for query-runtime documents. It must not be
/// cited as canonical mirror primitive proof; semantic decisions should
/// move to registered mirror lenses driven by pnixc-meta.
///
/// 헌법 §20 정합 path: *.px* 가 attrset literal 이 아니라 generic constructor +
/// `import + ++` 같은 *.px* expression 으로 표현되면 literal-only `parse_px`
/// 가 받지 못한다. 그 경우 `pnix_eval::eval_pnix_file` (pnix language
/// evaluator) 로 *.px* expression 을 evaluate 한 후 결과 `pnix_eval::Value`
/// 를 `PxValue` 로 lower 한다.
///
/// 호출 순서:
/// 1. `parse_px` — literal-only (작고 fast). 통과하면 그 결과 사용.
/// 2. `pnix_eval::eval_pnix_file` — full *.px* expression. literal parse 가
///    fail 한 경우 fallback. 결과를 `pnix_eval_value_to_px_value` 로 lower.
///
/// 두 path 모두 fail 하면 literal parser error 를 우선 report (디버깅이 더
/// 직관적; expression error 는 안쪽 details).
pub fn parse_px_file_with_pnix_eval_fallback(path: &Path) -> Result<PxValue> {
  let content =
    std::fs::read_to_string(path).with_context(|| format!("read .px file {}", path.display()))?;
  match parse_px(&content) {
    Ok(value) => Ok(value),
    Err(literal_err) => {
      let evaluated = pnix_eval::eval_pnix_file(path).map_err(|nix_err| {
        anyhow!(
          "literal parse failed and nix-eval fallback also failed; \
           literal: {}; nix-eval: {}",
          literal_err,
          nix_err
        )
      })?;
      pnix_eval_value_to_px_value(&evaluated).with_context(|| {
        format!(
          "lower nix-eval result to PxValue for {}; literal parse error: {}",
          path.display(),
          literal_err
        )
      })
    }
  }
}

/// `pnix_eval::Value` → `PxValue` lowering.
///
/// PxValue 는 String / List / AttrSet 3 variant 만. 숫자 / bool / null 은 모두
/// String 으로 (PxValue 의 carrier 가 본래 그렇다 — `parse_px` 가 `"true"` /
/// `"0.95"` 같은 form 만 받음). Lambda / Thunk / Path / StringContext 등 nix
/// 전용 variant 는 lower 시 reject (PxValue 표현 불가).
pub fn pnix_eval_value_to_px_value(value: &pnix_eval::Value) -> Result<PxValue> {
  use pnix_eval::Value;
  match value {
    Value::String(s) => Ok(PxValue::String(s.clone())),
    Value::StringContext { text, .. } => Ok(PxValue::String(text.clone())),
    Value::Int(i) => Ok(PxValue::String(i.to_string())),
    Value::Float(f) => Ok(PxValue::String(f.to_string())),
    Value::Bool(b) => Ok(PxValue::String(b.to_string())),
    Value::Null => Ok(PxValue::String(String::new())),
    Value::Path(p) => Ok(PxValue::String(p.to_string_lossy().into_owned())),
    Value::List(items) => {
      let lowered: Result<Vec<_>> = items.iter().map(pnix_eval_value_to_px_value).collect();
      Ok(PxValue::List(lowered?))
    }
    Value::AttrSet(map) => {
      let mut out = BTreeMap::new();
      for (k, v) in map.iter() {
        out.insert(k.clone(), pnix_eval_value_to_px_value(v)?);
      }
      Ok(PxValue::AttrSet(out))
    }
    _ => Err(anyhow!(
      "cannot lower nix-eval Value variant to PxValue (lambda/thunk/etc. unsupported in PxValue carrier)"
    )),
  }
}

/// Character cursor that tracks 1-based line and column positions so that parse
/// errors can point `.px` authors at the exact source location.
struct SourceCursor<'a> {
  chars: std::str::Chars<'a>,
  peeked: Option<char>,
  line: usize,
  col: usize,
}

impl<'a> SourceCursor<'a> {
  fn new(input: &'a str) -> Self {
    Self {
      chars: input.chars(),
      peeked: None,
      line: 1,
      col: 1,
    }
  }

  fn peek(&mut self) -> Option<char> {
    if self.peeked.is_none() {
      self.peeked = self.chars.next();
    }
    self.peeked
  }

  fn next(&mut self) -> Option<char> {
    let c = match self.peeked.take() {
      Some(c) => Some(c),
      None => self.chars.next(),
    };
    if let Some(ch) = c {
      if ch == '\n' {
        self.line += 1;
        self.col = 1;
      } else {
        self.col += 1;
      }
    }
    c
  }

  fn position(&self) -> SourcePosition {
    SourcePosition {
      line: self.line,
      col: self.col,
    }
  }
}

#[derive(Copy, Clone, Debug)]
struct SourcePosition {
  line: usize,
  col: usize,
}

impl std::fmt::Display for SourcePosition {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "at line {}, col {}", self.line, self.col)
  }
}

fn skip_whitespace_and_comments(cursor: &mut SourceCursor<'_>) {
  loop {
    match cursor.peek() {
      Some(c) if c.is_whitespace() => {
        cursor.next();
      }
      Some('#') => {
        while let Some(c) = cursor.next() {
          if c == '\n' {
            break;
          }
        }
      }
      _ => break,
    }
  }
}

fn parse_value(cursor: &mut SourceCursor<'_>) -> Result<PxValue> {
  skip_whitespace_and_comments(cursor);
  match cursor.peek() {
    Some('{') => parse_attrset(cursor),
    Some('[') => parse_list(cursor),
    Some('"') => parse_string(cursor),
    Some(_) => parse_atom(cursor),
    None => Err(anyhow!(
      "unexpected end of .px input ({})",
      cursor.position()
    )),
  }
}

fn parse_string(cursor: &mut SourceCursor<'_>) -> Result<PxValue> {
  let start = cursor.position();
  expect_char(cursor, '"')?;
  let mut s = String::new();
  loop {
    match cursor.next() {
      Some('"') => return Ok(PxValue::String(s)),
      Some('\\') => match cursor.next() {
        Some('n') => s.push('\n'),
        Some('t') => s.push('\t'),
        Some('"') => s.push('"'),
        Some('\\') => s.push('\\'),
        Some(c) => {
          s.push('\\');
          s.push(c);
        }
        None => {
          return Err(anyhow!(
            "unterminated escape in .px string (string started {})",
            start
          ))
        }
      },
      Some(c) => s.push(c),
      None => {
        return Err(anyhow!(
          "unterminated .px string (string started {})",
          start
        ))
      }
    }
  }
}

fn parse_list(cursor: &mut SourceCursor<'_>) -> Result<PxValue> {
  let start = cursor.position();
  expect_char(cursor, '[')?;
  let mut items = Vec::new();
  loop {
    skip_whitespace_and_comments(cursor);
    match cursor.peek() {
      Some(']') => {
        cursor.next();
        return Ok(PxValue::List(items));
      }
      None => {
        return Err(anyhow!("unterminated .px list (list started {})", start));
      }
      _ => {}
    }
    items.push(parse_value(cursor)?);
  }
}

fn parse_attrset(cursor: &mut SourceCursor<'_>) -> Result<PxValue> {
  let start = cursor.position();
  expect_char(cursor, '{')?;
  let mut map = BTreeMap::new();
  loop {
    skip_whitespace_and_comments(cursor);
    match cursor.peek() {
      Some('}') => {
        cursor.next();
        return Ok(PxValue::AttrSet(map));
      }
      None => {
        return Err(anyhow!(
          "unterminated .px attrset (attrset started {})",
          start
        ));
      }
      _ => {}
    }
    let key_start = cursor.position();
    let key = parse_identifier(cursor)?;
    skip_whitespace_and_comments(cursor);
    expect_char(cursor, '=')?;
    skip_whitespace_and_comments(cursor);
    let mut value = parse_value(cursor)?;
    // Lambda tolerance: `ident : body` pattern 이 PxValue attrset 안에 등장하면
    // parse_atom 이 `ident:` 까지 읽고 멈추는데, 그 뒤 whitespace + body 가
    // 붙는다. 이 경우 balanced brace/bracket 로 body 를 opaque raw source 로
    // 이어 붙여 `PxValue::String("<ident>: <body source>")` 로 반환한다. PxValue
    // 기반 reducer 는 이 key 를 의미적으로 해석하지 않고 (lambda 는 pnix-eval
    // owner), parse_all_data_px_files 류 walk 검사는 parse 성공만 확인한다.
    if let PxValue::String(atom) = &value {
      if atom.ends_with(':') {
        skip_whitespace_and_comments(cursor);
        let next_is_terminator = matches!(cursor.peek(), Some(';') | Some('}') | Some(']') | None);
        if !next_is_terminator {
          let body = consume_opaque_expression(cursor);
          value = PxValue::String(format!("{} {}", atom, body));
        }
      }
    }
    skip_whitespace_and_comments(cursor);
    if cursor.peek() == Some(';') {
      cursor.next();
    }
    if map.contains_key(&key) {
      return Err(anyhow!(
        "duplicate attrset key '{}' in .px ({})",
        key,
        key_start
      ));
    }
    map.insert(key, value);
  }
}

/// Attrset 안에서 lambda body 같이 PxValue 가 의미적으로 다루지 않는 raw
/// expression 을 balanced 로 소비한다. `;` 또는 닫는 `}` 를 만나면 (depth 0
/// 기준) 멈춘다. string 리터럴 안의 이스케이프된 `"` 는 건너뛴다.
fn consume_opaque_expression(cursor: &mut SourceCursor<'_>) -> String {
  let mut out = String::new();
  let mut brace = 0i32;
  let mut bracket = 0i32;
  let mut paren = 0i32;
  loop {
    match cursor.peek() {
      None => break,
      Some('"') => {
        out.push('"');
        cursor.next();
        loop {
          match cursor.next() {
            None => return out,
            Some('\\') => {
              out.push('\\');
              if let Some(esc) = cursor.next() {
                out.push(esc);
              }
            }
            Some('"') => {
              out.push('"');
              break;
            }
            Some(ch) => out.push(ch),
          }
        }
      }
      Some('{') => {
        brace += 1;
        out.push('{');
        cursor.next();
      }
      Some('}') => {
        if brace == 0 && paren == 0 {
          return out;
        }
        brace -= 1;
        out.push('}');
        cursor.next();
      }
      Some('[') => {
        bracket += 1;
        out.push('[');
        cursor.next();
      }
      Some(']') => {
        if bracket == 0 && paren == 0 {
          return out;
        }
        bracket -= 1;
        out.push(']');
        cursor.next();
      }
      Some('(') => {
        paren += 1;
        out.push('(');
        cursor.next();
      }
      Some(')') => {
        if paren == 0 {
          return out;
        }
        paren -= 1;
        out.push(')');
        cursor.next();
      }
      Some(';') if brace == 0 && bracket == 0 && paren == 0 => return out,
      Some(c) => {
        out.push(c);
        cursor.next();
      }
    }
  }
  out
}

fn parse_atom(cursor: &mut SourceCursor<'_>) -> Result<PxValue> {
  let start = cursor.position();
  let mut atom = String::new();
  while let Some(c) = cursor.peek() {
    if c.is_whitespace() || matches!(c, ';' | '}' | ']' | '#') {
      break;
    }
    atom.push(c);
    cursor.next();
  }
  if atom.is_empty() {
    Err(anyhow!("expected atom in .px ({})", start))
  } else {
    Ok(PxValue::String(atom))
  }
}

fn parse_identifier(cursor: &mut SourceCursor<'_>) -> Result<String> {
  let start = cursor.position();
  let mut id = String::new();
  while let Some(c) = cursor.peek() {
    if c.is_alphanumeric() || c == '_' || c == '-' {
      id.push(c);
      cursor.next();
    } else {
      break;
    }
  }
  if id.is_empty() {
    Err(anyhow!("expected identifier in .px ({})", start))
  } else {
    Ok(id)
  }
}

fn expect_char(cursor: &mut SourceCursor<'_>, expected: char) -> Result<()> {
  skip_whitespace_and_comments(cursor);
  let pos = cursor.position();
  match cursor.next() {
    Some(c) if c == expected => Ok(()),
    Some(c) => Err(anyhow!(
      "expected '{}' but got '{}' in .px ({})",
      expected,
      c,
      pos
    )),
    None => Err(anyhow!(
      "expected '{}' but got EOF in .px ({})",
      expected,
      pos
    )),
  }
}
