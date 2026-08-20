//! UI JSON: PNIX 표현식을 UI 스펙 JSON으로 변환

use serde_json::Value;

use crate::lang::pnix::syntax::{PnixAttrItem, PnixExpr};

/// 종류 필드 이름
const FIELD_KIND: &str = "kind";

/// Pnix 표현식을 UI 스펙 JSON으로 변환
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn pnix_expr_to_json(expr: &PnixExpr) -> Result<Value, String> {
  match expr {
    PnixExpr::Int(value) => Ok(Value::Number((*value).into())),
    PnixExpr::Float(value) => serde_json::Number::from_f64(*value)
      .map(Value::Number)
      .ok_or_else(|| "invalid float literal".to_string()),
    PnixExpr::Bool(value) => Ok(Value::Bool(*value)),
    PnixExpr::Null => Ok(Value::Null),
    PnixExpr::String(value) => Ok(Value::String(value.clone())),
    PnixExpr::AttrSet { items, recursive } => {
      if *recursive {
        return Err("recursive attrset not supported".to_string());
      }
      let mut map = serde_json::Map::new();
      for item in items {
        match item {
          PnixAttrItem::Assign {
            key_path, value, ..
          } => {
            let value = pnix_expr_to_json(value)?;
            insert_json_path(&mut map, key_path, value)?;
          }
          PnixAttrItem::DynamicAssign { .. } => {
            return Err("dynamic attrset keys not supported".to_string());
          }
          PnixAttrItem::Inherit { .. } => {
            return Err("inherit attrset items not supported".to_string());
          }
        }
      }
      Ok(Value::Object(map))
    }
    PnixExpr::List(items) => {
      let mut list = Vec::with_capacity(items.len());
      for item in items {
        list.push(pnix_expr_to_json(item)?);
      }
      Ok(Value::Array(list))
    }
    PnixExpr::Apply { func, arg } => {
      let mut json = pnix_expr_to_json(arg)?;
      if let Some(name) = constructor_name(func) {
        if let Some(kind) = constructor_kind(name) {
          if let Value::Object(map) = &mut json {
            map
              .entry(FIELD_KIND.to_string())
              .or_insert_with(|| Value::String(kind.to_string()));
          } else {
            return Err(format!("ui constructor '{}' requires attrset", name));
          }
        }
      }
      Ok(json)
    }
    _ => Err("unsupported pnix expression for ui spec".to_string()),
  }
}

/// Pnix 리스트 구분자 정규화: 세미콜론을 쉼표로 변환
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 변환만, 파일 I/O 없음
pub fn normalize_pnix_list_separators(expr: &str) -> String {
  #[derive(Clone, Copy)]
  enum StrMode {
    None,
    Double,
    Multi,
  }

  if !expr.contains(';') || !expr.contains('[') {
    return expr.to_string();
  }
  // M2-FREECAT-030: UTF-8 인코딩 문제 해결
  // bytes[idx] as char는 UTF-8 바이트를 Latin-1로 잘못 해석하므로
  // 문자 단위로 처리하도록 변경
  let mut chars = expr.chars().peekable();
  let mut out = String::with_capacity(expr.len());
  let mut depth_paren = 0usize;
  let mut depth_brace = 0usize;
  let mut depth_bracket = 0usize;
  let mut list_stack: Vec<(usize, usize)> = Vec::new();
  let mut mode = StrMode::None;

  while let Some(ch) = chars.next() {
    match mode {
      StrMode::Double => {
        out.push(ch);
        if ch == '\\' {
          if let Some(escaped) = chars.next() {
            out.push(escaped);
            continue;
          }
        } else if ch == '"' {
          mode = StrMode::None;
        }
        continue;
      }
      StrMode::Multi => {
        if ch == '\'' {
          if let Some(&next) = chars.peek() {
            if next == '\'' {
              out.push('\'');
              out.push('\'');
              chars.next(); // consume second '
              mode = StrMode::None;
              continue;
            }
          }
        }
        out.push(ch);
        continue;
      }
      StrMode::None => {}
    }
    match ch {
      '#' => {
        out.push(ch);
        while let Some(&next_ch) = chars.peek() {
          if next_ch == '\n' {
            break;
          }
          out.push(chars.next().unwrap());
        }
        continue;
      }
      '"' => {
        mode = StrMode::Double;
        out.push(ch);
        continue;
      }
      '\'' => {
        if let Some(&next) = chars.peek() {
          if next == '\'' {
            mode = StrMode::Multi;
            out.push('\'');
            out.push('\'');
            chars.next(); // consume second '
            continue;
          }
        }
      }
      '{' => {
        depth_brace += 1;
      }
      '}' => {
        depth_brace = depth_brace.saturating_sub(1);
      }
      '[' => {
        list_stack.push((depth_brace, depth_paren));
        depth_bracket += 1;
      }
      ']' => {
        depth_bracket = depth_bracket.saturating_sub(1);
        list_stack.pop();
      }
      '(' => {
        depth_paren += 1;
      }
      ')' => {
        depth_paren = depth_paren.saturating_sub(1);
      }
      ';' => {
        if depth_bracket > 0 {
          if let Some((list_brace, list_paren)) = list_stack.last() {
            if depth_brace == *list_brace && depth_paren == *list_paren {
              continue; // Skip this semicolon
            }
          }
        }
      }
      _ => {}
    }
    out.push(ch);
  }
  out
}

fn constructor_name(expr: &PnixExpr) -> Option<&str> {
  match expr {
    PnixExpr::Var(name) => Some(name.as_str()),
    PnixExpr::Select { attr, .. } => Some(attr.as_str()),
    _ => None,
  }
}

fn constructor_kind(name: &str) -> Option<&'static str> {
  match name.to_ascii_lowercase().as_str() {
    "scene" => Some("scene"),
    "layer" => Some("layer"),
    "rect" => Some("rect"),
    "text" => Some("text"),
    "image" => Some("image"),
    "path" => Some("path"),
    "group" => Some("group"),
    "geometry" => Some("geometry"),
    "anim" => Some("anim"),
    "camera" => Some("camera"),
    "light" => Some("light"),
    _ => None,
  }
}

fn insert_json_path(
  map: &mut serde_json::Map<String, Value>,
  path: &[String],
  value: Value,
) -> Result<(), String> {
  if path.is_empty() {
    return Err("empty attrset key path".to_string());
  }
  let mut cursor = map;
  let last_index = path.len() - 1;
  for (index, key) in path.iter().enumerate() {
    if index == last_index {
      if cursor.contains_key(key) {
        return Err(format!("duplicate key '{}'", key));
      }
      cursor.insert(key.clone(), value);
      return Ok(());
    }
    let entry = cursor
      .entry(key.clone())
      .or_insert_with(|| Value::Object(serde_json::Map::new()));
    match entry {
      Value::Object(nested) => {
        cursor = nested;
      }
      _ => {
        return Err(format!("attrset key path conflict at '{}'", key));
      }
    }
  }
  Ok(())
}
