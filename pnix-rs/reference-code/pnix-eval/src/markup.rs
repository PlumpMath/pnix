use crate::Value;
use anyhow::{anyhow, Result};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn xml_parse(input: &str) -> Result<Value> {
  let json = pnix_xml_core::xml_json_from_xml_str(input)
    .map_err(|err| anyhow!("builtins.xmlParse: {}", err))?;
  Ok(json_to_value(&json))
}

pub fn xml_emit(value: &Value) -> Result<String> {
  let mut out = String::new();
  match value {
    Value::AttrSet(map) => {
      if markup_kind(map)? == "document" {
        xml_emit_document(map, &mut out)?;
      } else {
        xml_emit_node(value, &mut out)?;
      }
    }
    Value::List(items) => {
      for item in items.iter() {
        xml_emit_node(item, &mut out)?;
      }
    }
    other => {
      return Err(anyhow!(
        "xml.emit: expected document or node, got {}",
        value_type_name(other)
      ))
    }
  }
  Ok(out)
}

pub fn html_parse(input: &str) -> Result<Value> {
  // In-house tolerant HTML parser (replaces the scraper/html5ever dependency
  // chain, ~42 crates — dependency-liberation campaign). It builds the same
  // document/element/text/comment node shape the emitter round-trips, with one
  // intentional simplification versus html5ever: no implied `<html><head><body>`
  // wrapping (the parsed tree mirrors the source nesting directly). Void and
  // raw-text (`script`/`style`) elements, comments, doctype, and HTML entity
  // decoding in text/attribute values are handled; element/attr names are
  // lowercased and attrs sorted, matching the prior behavior.
  let mut p = HtmlParser {
    chars: input.chars().collect(),
    pos: 0,
  };
  let mut doctype = None;
  let children = p.parse_nodes(None, &mut doctype);
  Ok(html_document_node(children, doctype))
}

struct HtmlParser {
  chars: Vec<char>,
  pos: usize,
}

impl HtmlParser {
  fn eof(&self) -> bool {
    self.pos >= self.chars.len()
  }
  fn peek(&self) -> Option<char> {
    self.chars.get(self.pos).copied()
  }
  fn peek_at(&self, n: usize) -> Option<char> {
    self.chars.get(self.pos + n).copied()
  }
  fn starts_with(&self, s: &str) -> bool {
    s.chars().enumerate().all(|(i, c)| self.peek_at(i) == Some(c))
  }

  /// Parse a run of sibling nodes. `parent` is the enclosing element's tag
  /// (None at document level). Returns on EOF or a close tag: a close tag
  /// matching `parent` is consumed and ends the run; a non-matching close
  /// tag is left unconsumed so an ancestor can close (tolerant auto-close);
  /// a stray close tag at document level is skipped.
  fn parse_nodes(&mut self, parent: Option<&str>, doctype: &mut Option<String>) -> Vec<Value> {
    let mut out = Vec::new();
    loop {
      if self.eof() {
        break;
      }
      if self.starts_with("<!--") {
        out.push(self.parse_comment());
      } else if self.starts_with("<!") {
        self.parse_declaration(doctype);
      } else if self.starts_with("</") {
        match self.peek_close_tag_name() {
          Some(name) => match parent {
            Some(p) if p == name => {
              self.consume_close_tag();
              break;
            }
            Some(_) => break, // ancestor's close — leave for the ancestor
            None => self.consume_close_tag(), // stray close at top — skip
          },
          None => self.consume_close_tag(),
        }
      } else if self.peek() == Some('<')
        && self.peek_at(1).map(is_tag_name_start).unwrap_or(false)
      {
        if let Some(el) = self.parse_element(doctype) {
          out.push(el);
        }
      } else {
        let text = self.parse_text();
        if !text.is_empty() {
          out.push(html_text_node(text));
        }
      }
    }
    out
  }

  fn parse_comment(&mut self) -> Value {
    self.pos += 4; // <!--
    let start = self.pos;
    while !self.eof() && !self.starts_with("-->") {
      self.pos += 1;
    }
    let text: String = self.chars[start..self.pos].iter().collect();
    if self.starts_with("-->") {
      self.pos += 3;
    }
    html_comment_node(text)
  }

  /// `<!DOCTYPE html>` (capture the name) or any other `<!...>` declaration
  /// (skipped). First doctype name wins, matching the prior behavior.
  fn parse_declaration(&mut self, doctype: &mut Option<String>) {
    self.pos += 2; // <!
    let start = self.pos;
    while !self.eof() && self.peek() != Some('>') {
      self.pos += 1;
    }
    let body: String = self.chars[start..self.pos].iter().collect();
    if self.peek() == Some('>') {
      self.pos += 1;
    }
    let mut words = body.split_whitespace();
    if let Some(kw) = words.next() {
      if kw.eq_ignore_ascii_case("doctype") {
        if let Some(name) = words.next() {
          if doctype.is_none() && !name.is_empty() {
            *doctype = Some(name.to_ascii_lowercase());
          }
        }
      }
    }
  }

  /// Peek the name of the close tag at the cursor without consuming.
  fn peek_close_tag_name(&self) -> Option<String> {
    let mut i = self.pos + 2; // skip </
    let mut name = String::new();
    while let Some(&c) = self.chars.get(i) {
      if is_tag_name_char(c) {
        name.push(c.to_ascii_lowercase());
        i += 1;
      } else {
        break;
      }
    }
    if name.is_empty() {
      None
    } else {
      Some(name)
    }
  }

  fn consume_close_tag(&mut self) {
    while !self.eof() && self.peek() != Some('>') {
      self.pos += 1;
    }
    if self.peek() == Some('>') {
      self.pos += 1;
    }
  }

  fn parse_element(&mut self, doctype: &mut Option<String>) -> Option<Value> {
    self.pos += 1; // <
    let mut name = String::new();
    while let Some(c) = self.peek() {
      if is_tag_name_char(c) {
        name.push(c.to_ascii_lowercase());
        self.pos += 1;
      } else {
        break;
      }
    }
    if name.is_empty() {
      return None;
    }
    let mut attrs = self.parse_attributes();
    attrs.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));

    // Consume '>' or '/>'.
    let mut self_closing = false;
    if self.peek() == Some('/') {
      self_closing = true;
      self.pos += 1;
    }
    if self.peek() == Some('>') {
      self.pos += 1;
    }

    let children = if self_closing || is_html_void_element(&name) {
      Vec::new()
    } else if is_html_raw_text_element(&name) {
      self.parse_raw_text(&name)
    } else {
      self.parse_nodes(Some(&name), doctype)
    };
    Some(html_element_node(name, attrs, children))
  }

  fn parse_attributes(&mut self) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    loop {
      self.skip_ws();
      match self.peek() {
        None | Some('>') => break,
        Some('/') if self.peek_at(1) == Some('>') => break,
        Some('/') => {
          self.pos += 1;
          continue;
        }
        _ => {}
      }
      let mut key = String::new();
      while let Some(c) = self.peek() {
        if c == '=' || c == '>' || c == '/' || c.is_whitespace() {
          break;
        }
        key.push(c.to_ascii_lowercase());
        self.pos += 1;
      }
      if key.is_empty() {
        // Defensive: avoid an infinite loop on an unexpected char.
        self.pos += 1;
        continue;
      }
      self.skip_ws();
      let value = if self.peek() == Some('=') {
        self.pos += 1;
        self.skip_ws();
        self.parse_attr_value()
      } else {
        String::new()
      };
      attrs.push((key.clone(), html_normalize_attr_value(&key, &value)));
    }
    attrs
  }

  fn parse_attr_value(&mut self) -> String {
    match self.peek() {
      Some(q @ ('"' | '\'')) => {
        self.pos += 1;
        let start = self.pos;
        while let Some(c) = self.peek() {
          if c == q {
            break;
          }
          self.pos += 1;
        }
        let raw: String = self.chars[start..self.pos].iter().collect();
        if self.peek() == Some(q) {
          self.pos += 1;
        }
        decode_entities(&raw)
      }
      _ => {
        let start = self.pos;
        while let Some(c) = self.peek() {
          if c == '>' || c == '/' || c.is_whitespace() {
            break;
          }
          self.pos += 1;
        }
        let raw: String = self.chars[start..self.pos].iter().collect();
        decode_entities(&raw)
      }
    }
  }

  /// Raw-text element content: everything up to the matching `</name>`,
  /// verbatim (no tag or entity processing), as a single text node.
  fn parse_raw_text(&mut self, name: &str) -> Vec<Value> {
    let start = self.pos;
    loop {
      if self.eof() {
        break;
      }
      if self.starts_with("</") {
        if let Some(close) = self.peek_close_tag_name() {
          if close == name {
            break;
          }
        }
      }
      self.pos += 1;
    }
    let text: String = self.chars[start..self.pos].iter().collect();
    if self.starts_with("</") {
      self.consume_close_tag();
    }
    if text.is_empty() {
      Vec::new()
    } else {
      vec![html_text_node(text)]
    }
  }

  /// Text up to the next markup start (`</`, `<!`, or `<` + tag-name char).
  /// A bare `<` not starting markup is kept as literal text. Entities decoded.
  fn parse_text(&mut self) -> String {
    let start = self.pos;
    while let Some(c) = self.peek() {
      if c == '<' {
        match self.peek_at(1) {
          Some('/') | Some('!') => break,
          Some(n) if is_tag_name_start(n) => break,
          _ => {
            self.pos += 1; // literal '<'
          }
        }
      } else {
        self.pos += 1;
      }
    }
    let raw: String = self.chars[start..self.pos].iter().collect();
    decode_entities(&raw)
  }

  fn skip_ws(&mut self) {
    while self.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
      self.pos += 1;
    }
  }
}

fn is_tag_name_start(c: char) -> bool {
  c.is_ascii_alphabetic()
}

fn is_tag_name_char(c: char) -> bool {
  c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':'
}

/// Decode the common HTML entities plus numeric (`&#NN;` / `&#xHH;`) forms.
/// Unknown named entities are left literal. This is the inverse of the
/// emitter's `&amp;`/`&lt;`/`&gt;` escaping, keeping parse/emit round-trips
/// stable for the entities the substrate produces.
fn decode_entities(input: &str) -> String {
  if !input.contains('&') {
    return input.to_string();
  }
  let bytes: Vec<char> = input.chars().collect();
  let mut out = String::with_capacity(input.len());
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == '&' {
      if let Some((decoded, len)) = decode_one_entity(&bytes[i..]) {
        out.push_str(&decoded);
        i += len;
        continue;
      }
    }
    out.push(bytes[i]);
    i += 1;
  }
  out
}

/// Decode a single `&...;` at the slice start; returns (text, consumed chars).
fn decode_one_entity(s: &[char]) -> Option<(String, usize)> {
  // Find the terminating ';' within a small window.
  let semi = s.iter().take(12).position(|&c| c == ';')?;
  let body: String = s[1..semi].iter().collect();
  let consumed = semi + 1;
  if let Some(rest) = body.strip_prefix('#') {
    let code = if let Some(hex) = rest.strip_prefix(['x', 'X']) {
      u32::from_str_radix(hex, 16).ok()?
    } else {
      rest.parse::<u32>().ok()?
    };
    let ch = char::from_u32(code)?;
    return Some((ch.to_string(), consumed));
  }
  let ch = match body.as_str() {
    "amp" => '&',
    "lt" => '<',
    "gt" => '>',
    "quot" => '"',
    "apos" => '\'',
    "nbsp" => '\u{a0}',
    _ => return None,
  };
  Some((ch.to_string(), consumed))
}

pub fn html_emit(value: &Value) -> Result<String> {
  let mut out = String::new();
  match value {
    Value::AttrSet(map) => {
      if markup_kind(map)? == "document" {
        html_emit_document(map, &mut out)?;
      } else {
        html_emit_node(value, &mut out)?;
      }
    }
    Value::List(items) => {
      for item in items.iter() {
        html_emit_node(item, &mut out)?;
      }
    }
    _ => return Err(anyhow!("html.emit: expected document or node")),
  }
  Ok(out)
}

pub(crate) fn json_to_value(value: &serde_json::Value) -> Value {
  match value {
    serde_json::Value::Null => Value::Null,
    serde_json::Value::Bool(flag) => Value::Bool(*flag),
    serde_json::Value::Number(number) => {
      if let Some(int) = number.as_i64() {
        Value::Int(int)
      } else {
        Value::Float(number.as_f64().unwrap_or_default())
      }
    }
    serde_json::Value::String(text) => Value::String(text.clone()),
    serde_json::Value::Array(items) => {
      let mut out = Vec::with_capacity(items.len());
      for item in items {
        out.push(json_to_value(item));
      }
      Value::List(Arc::new(out))
    }
    serde_json::Value::Object(map) => {
      let mut out = BTreeMap::new();
      for (key, item) in map {
        out.insert(key.clone(), json_to_value(item));
      }
      Value::AttrSet(Arc::new(out))
    }
  }
}

// 2026-05-06 (slice #78): pre-scan the JSON source for
// integer-shaped numeric tokens that don't fit in i64. Pre-fix
// shape: `builtins.fromJSON "999999999999999999999"` silently
// produced `Value::Float(1e+21)` — serde_json widens to f64 on
// overflow, losing precision. Real Nix errors on integer
// overflow during JSON parse. This helper walks the source byte-
// by-byte (with proper string-content skipping), identifies
// integer-shaped tokens (`-?\d+` not followed by `.` / `e` / `E`),
// and rejects any that don't parse as i64. Float tokens are
// left to serde_json's normal handling.
//
// Design choice: post-validate (after `serde_json::from_str`
// succeeds), not pre-validate. Reason: invalid JSON should
// produce serde's parse-error message first, and only valid
// JSON gets the overflow check. This keeps error messages
// clear about which failure mode applied.
pub(crate) fn check_json_no_int_overflow(src: &str) -> anyhow::Result<()> {
  let bytes = src.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    match bytes[i] {
      b'"' => {
        // Skip string contents (handle escape sequences so an
        // escaped quote doesn't end the string early).
        i += 1;
        while i < bytes.len() {
          if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
          }
          if bytes[i] == b'"' {
            i += 1;
            break;
          }
          i += 1;
        }
      }
      b'-' if i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() => {
        if let Some(consumed) = check_numeric_token(bytes, i)? {
          i += consumed;
        } else {
          i += 1;
        }
      }
      b'0'..=b'9' => {
        if let Some(consumed) = check_numeric_token(bytes, i)? {
          i += consumed;
        } else {
          i += 1;
        }
      }
      _ => i += 1,
    }
  }
  Ok(())
}

// Inspect the numeric token starting at `bytes[start]`. If the
// token is integer-shaped (no `.` / `e` / `E`) and doesn't fit
// in i64, return an error. If the token is float-shaped or
// integer-shaped-and-fits, return the byte length consumed so
// the caller can advance.
fn check_numeric_token(bytes: &[u8], start: usize) -> anyhow::Result<Option<usize>> {
  let mut i = start;
  if bytes[i] == b'-' {
    i += 1;
  }
  let digit_start = i;
  while i < bytes.len() && bytes[i].is_ascii_digit() {
    i += 1;
  }
  if i == digit_start {
    return Ok(None);
  }
  let int_end = i;
  // If a `.`, `e`, or `E` follows, the token is float-shaped.
  // Consume the rest of the float so the outer loop doesn't
  // re-scan the digits.
  if i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b'e' || bytes[i] == b'E') {
    while i < bytes.len()
      && (bytes[i].is_ascii_digit()
        || bytes[i] == b'.'
        || bytes[i] == b'e'
        || bytes[i] == b'E'
        || bytes[i] == b'+'
        || bytes[i] == b'-')
    {
      i += 1;
    }
    return Ok(Some(i - start));
  }
  // Integer-shaped token. Must fit in i64.
  let token =
    std::str::from_utf8(&bytes[start..int_end]).map_err(|_| anyhow!("invalid utf-8 in number"))?;
  if token.parse::<i64>().is_err() {
    return Err(anyhow!(
      "builtins.fromJSON: integer literal '{}' is too large for the i64 evaluator",
      token
    ));
  }
  Ok(Some(int_end - start))
}

pub(crate) fn value_to_json(value: &Value) -> Result<serde_json::Value> {
  match value {
    Value::Null => Ok(serde_json::Value::Null),
    Value::Bool(flag) => Ok(serde_json::Value::Bool(*flag)),
    Value::Int(number) => Ok(serde_json::Value::Number((*number).into())),
    Value::Float(number) => serde_json::Number::from_f64(*number)
      .map(serde_json::Value::Number)
      .ok_or_else(|| anyhow!("cannot encode NaN/inf into JSON")),
    Value::String(text) => Ok(serde_json::Value::String(text.clone())),
    Value::StringContext { text, .. } => Ok(serde_json::Value::String(text.clone())),
    Value::Path(path) => Ok(serde_json::Value::String(
      path.to_string_lossy().into_owned(),
    )),
    Value::List(items) => {
      let mut out = Vec::with_capacity(items.len());
      for item in items.iter() {
        out.push(value_to_json(item)?);
      }
      Ok(serde_json::Value::Array(out))
    }
    Value::AttrSet(map) => {
      let mut out = serde_json::Map::new();
      for (key, item) in map.iter() {
        out.insert(key.clone(), value_to_json(item)?);
      }
      Ok(serde_json::Value::Object(out))
    }
    Value::Lambda { .. } | Value::BuiltinPartial { .. } => {
      Err(anyhow!("cannot encode function value into JSON"))
    }
    Value::Thunk { .. } => {
      let forced = crate::interpret::force_value(value.clone())?;
      value_to_json(&forced)
    }
  }
}

fn markup_kind(map: &BTreeMap<String, Value>) -> Result<&str> {
  match map.get("kind") {
    Some(Value::String(kind)) => Ok(kind.as_str()),
    Some(_) => Err(anyhow!("markup.emit: kind must be string")),
    None => Err(anyhow!("markup.emit: missing kind")),
  }
}

fn xml_emit_document(map: &BTreeMap<String, Value>, out: &mut String) -> Result<()> {
  if let Some(Value::AttrSet(decl)) = map.get("decl") {
    xml_emit_decl(decl, out)?;
  }
  for child in children_from_attrset(map, "children", "xml.emit")? {
    xml_emit_node(&child, out)?;
  }
  Ok(())
}

fn xml_emit_node(node: &Value, out: &mut String) -> Result<()> {
  let Value::AttrSet(map) = node else {
    return Err(anyhow!(
      "xml.emit: node must be attrset, got {}",
      value_type_name(node)
    ));
  };
  match markup_kind(map)? {
    "document" => xml_emit_document(map, out),
    "element" => xml_emit_element(map, out),
    "text" => {
      let value = node_string(map, &["value", "text"])?
        .ok_or_else(|| anyhow!("xml.emit: text missing value"))?;
      out.push_str(&xml_escape_text(&value));
      Ok(())
    }
    "cdata" => {
      let value = node_string(map, &["value", "text"])?
        .ok_or_else(|| anyhow!("xml.emit: cdata missing value"))?;
      out.push_str("<![CDATA[");
      out.push_str(&value);
      out.push_str("]]>");
      Ok(())
    }
    "comment" => {
      let value =
        node_string(map, &["value"])?.ok_or_else(|| anyhow!("xml.emit: comment missing value"))?;
      out.push_str("<!--");
      out.push_str(&value);
      out.push_str("-->");
      Ok(())
    }
    "pi" => {
      let target =
        node_string(map, &["target"])?.ok_or_else(|| anyhow!("xml.emit: pi missing target"))?;
      let data = node_string(map, &["data"])?.unwrap_or_default();
      out.push_str("<?");
      out.push_str(&target);
      if !data.is_empty() {
        out.push(' ');
        out.push_str(&data);
      }
      out.push_str("?>");
      Ok(())
    }
    _ if map.contains_key("name") => xml_emit_element(map, out),
    other => Err(anyhow!("xml.emit: unknown kind '{}'", other)),
  }
}

fn xml_emit_element(map: &BTreeMap<String, Value>, out: &mut String) -> Result<()> {
  let name =
    node_string(map, &["name"])?.ok_or_else(|| anyhow!("xml.emit: element missing name"))?;
  let mut attrs = xml_attrs_from_attrset(map, "attrs")?;

  out.push('<');
  out.push_str(&name);

  if let Some(Value::AttrSet(ns_map)) = map.get("ns") {
    let mut ns_attrs = Vec::with_capacity(ns_map.len());
    for (prefix, value) in ns_map.iter() {
      let ns_uri = xml_value_to_string(value, "xml.emit: ns must be string-compatible")?;
      if prefix.is_empty() {
        ns_attrs.push(("xmlns".to_string(), ns_uri));
      } else {
        let mut attr_name = String::with_capacity("xmlns:".len() + prefix.len());
        attr_name.push_str("xmlns:");
        attr_name.push_str(prefix);
        ns_attrs.push((attr_name, ns_uri));
      }
    }
    ns_attrs.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    for (key, value) in ns_attrs {
      out.push(' ');
      out.push_str(&key);
      out.push_str("=\"");
      out.push_str(&xml_escape_attr(&value));
      out.push('"');
    }
    attrs.retain(|(key, _)| key != "xmlns" && !key.starts_with("xmlns:"));
  }

  attrs.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
  for (key, value) in attrs {
    out.push(' ');
    out.push_str(&key);
    out.push_str("=\"");
    out.push_str(&xml_escape_attr(&value));
    out.push('"');
  }

  let children = children_from_attrset(map, "children", "xml.emit")?;
  if children.is_empty() {
    out.push_str("/>");
    return Ok(());
  }

  out.push('>');
  for child in children {
    xml_emit_node(&child, out)?;
  }
  out.push_str("</");
  out.push_str(&name);
  out.push('>');
  Ok(())
}

fn xml_emit_decl(map: &BTreeMap<String, Value>, out: &mut String) -> Result<()> {
  let version =
    node_string(map, &["version"])?.ok_or_else(|| anyhow!("xml.emit: decl missing version"))?;
  out.push_str("<?xml version=\"");
  out.push_str(&xml_escape_attr(&version));
  out.push('"');

  if let Some(encoding) = node_string(map, &["encoding"])? {
    out.push_str(" encoding=\"");
    out.push_str(&xml_escape_attr(&encoding));
    out.push('"');
  }
  if let Some(standalone) = node_string(map, &["standalone"])? {
    out.push_str(" standalone=\"");
    out.push_str(&xml_escape_attr(&standalone));
    out.push('"');
  }
  out.push_str("?>");
  Ok(())
}


fn html_emit_document(map: &BTreeMap<String, Value>, out: &mut String) -> Result<()> {
  if let Some(doctype) = node_string(map, &["doctype"])? {
    out.push_str("<!DOCTYPE ");
    out.push_str(&xml_escape_attr(&doctype));
    out.push('>');
  }
  for child in children_from_attrset(map, "children", "html.emit")? {
    html_emit_node(&child, out)?;
  }
  Ok(())
}

fn html_emit_node(node: &Value, out: &mut String) -> Result<()> {
  let Value::AttrSet(map) = node else {
    return Err(anyhow!(
      "html.emit: node must be attrset, got {}",
      value_type_name(node)
    ));
  };
  match markup_kind(map)? {
    "document" => html_emit_document(map, out),
    "element" => html_emit_element(map, out),
    "text" => {
      let value = node_string(map, &["value", "text"])?
        .ok_or_else(|| anyhow!("html.emit: text missing value"))?;
      out.push_str(&xml_escape_text(&value));
      Ok(())
    }
    "comment" => {
      let value =
        node_string(map, &["value"])?.ok_or_else(|| anyhow!("html.emit: comment missing value"))?;
      out.push_str("<!--");
      out.push_str(&value);
      out.push_str("-->");
      Ok(())
    }
    other => Err(anyhow!("html.emit: unknown kind '{}'", other)),
  }
}

fn html_emit_element(map: &BTreeMap<String, Value>, out: &mut String) -> Result<()> {
  let name = ascii_lowercase_owned(
    &node_string(map, &["name"])?.ok_or_else(|| anyhow!("html.emit: element missing name"))?,
  );
  let mut attrs = html_attrs_from_attrset(map, "attrs")?;
  attrs.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
  let children = children_from_attrset(map, "children", "html.emit")?;

  out.push('<');
  out.push_str(&name);
  for (key, value) in attrs {
    out.push(' ');
    out.push_str(&key);
    out.push_str("=\"");
    out.push_str(&xml_escape_attr(&value));
    out.push('"');
  }

  if is_html_void_element(&name) {
    if !children.is_empty() {
      return Err(anyhow!("html.emit: void element must not have children"));
    }
    out.push('>');
    return Ok(());
  }

  out.push('>');
  for child in children {
    html_emit_node(&child, out)?;
  }
  out.push_str("</");
  out.push_str(&name);
  out.push('>');
  Ok(())
}

fn html_document_node(children: Vec<Value>, doctype: Option<String>) -> Value {
  let mut map = BTreeMap::new();
  map.insert("kind".to_string(), Value::String("document".to_string()));
  map.insert("children".to_string(), Value::List(Arc::new(children)));
  if let Some(doctype) = doctype {
    map.insert("doctype".to_string(), Value::String(doctype));
  }
  Value::AttrSet(Arc::new(map))
}

fn html_element_node(name: String, attrs: Vec<(String, String)>, children: Vec<Value>) -> Value {
  let mut map = BTreeMap::new();
  map.insert("kind".to_string(), Value::String("element".to_string()));
  map.insert("name".to_string(), Value::String(name));
  let mut attr_nodes = Vec::with_capacity(attrs.len());
  for (key, value) in attrs {
    attr_nodes.push(html_attr_node(key, value));
  }
  map.insert("attrs".to_string(), Value::List(Arc::new(attr_nodes)));
  map.insert("children".to_string(), Value::List(Arc::new(children)));
  Value::AttrSet(Arc::new(map))
}

fn html_text_node(value: String) -> Value {
  Value::AttrSet(Arc::new(BTreeMap::from([
    ("kind".to_string(), Value::String("text".to_string())),
    ("value".to_string(), Value::String(value)),
  ])))
}

fn html_comment_node(value: String) -> Value {
  Value::AttrSet(Arc::new(BTreeMap::from([
    ("kind".to_string(), Value::String("comment".to_string())),
    ("value".to_string(), Value::String(value)),
  ])))
}

fn html_attr_node(name: String, value: String) -> Value {
  Value::AttrSet(Arc::new(BTreeMap::from([
    ("name".to_string(), Value::String(name)),
    ("value".to_string(), Value::String(value)),
  ])))
}

fn node_string(map: &BTreeMap<String, Value>, keys: &[&str]) -> Result<Option<String>> {
  for key in keys {
    if let Some(value) = map.get(*key) {
      return Ok(Some(xml_value_to_string_for_key(value, key)?));
    }
  }
  Ok(None)
}

fn children_from_attrset(
  map: &BTreeMap<String, Value>,
  key: &str,
  context: &str,
) -> Result<Vec<Value>> {
  match map.get(key) {
    None => Ok(Vec::new()),
    Some(Value::List(items)) => Ok((**items).clone()),
    Some(other) => Err(anyhow!(
      "{}: children must be list, got {}",
      context,
      value_type_name(other)
    )),
  }
}

fn xml_attrs_from_attrset(
  map: &BTreeMap<String, Value>,
  key: &str,
) -> Result<Vec<(String, String)>> {
  match map.get(key) {
    None => Ok(Vec::new()),
    Some(Value::List(items)) => {
      let mut out = Vec::with_capacity(items.len());
      for item in items.iter() {
        out.push(xml_attr_from_node(item, "xml.emit")?);
      }
      Ok(out)
    }
    Some(Value::AttrSet(attrs)) => {
      let mut out = Vec::with_capacity(attrs.len());
      for (name, value) in attrs.iter() {
        out.push((
          name.clone(),
          xml_value_to_string(value, "xml.emit: attr value invalid")?,
        ));
      }
      Ok(out)
    }
    Some(other) => Err(anyhow!(
      "xml.emit: attrs must be list or attrset, got {}",
      value_type_name(other)
    )),
  }
}

fn html_attrs_from_attrset(
  map: &BTreeMap<String, Value>,
  key: &str,
) -> Result<Vec<(String, String)>> {
  match map.get(key) {
    None => Ok(Vec::new()),
    Some(Value::List(items)) => {
      let mut out = Vec::with_capacity(items.len());
      for item in items.iter() {
        let (name, value) = xml_attr_from_node(item, "html.emit")?;
        let lowered = ascii_lowercase_owned(&name);
        out.push((lowered.clone(), html_normalize_attr_value(&lowered, &value)));
      }
      Ok(out)
    }
    Some(Value::AttrSet(attrs)) => {
      let mut out = Vec::with_capacity(attrs.len());
      for (name, value) in attrs.iter() {
        let lowered = ascii_lowercase_owned(name);
        let value = xml_value_to_string(value, "html.emit: attr value invalid")?;
        out.push((lowered.clone(), html_normalize_attr_value(&lowered, &value)));
      }
      Ok(out)
    }
    Some(_) => Err(anyhow!("html.emit: attrs must be list or attrset")),
  }
}

fn xml_attr_from_node(node: &Value, context: &str) -> Result<(String, String)> {
  let Value::AttrSet(map) = node else {
    return Err(anyhow!("{}: attr node must be attrset", context));
  };
  let name =
    node_string(map, &["name"])?.ok_or_else(|| anyhow!("{}: attr missing name", context))?;
  let value = node_string(map, &["value", "text"])?
    .ok_or_else(|| anyhow!("{}: attr missing value", context))?;
  Ok((name, value))
}

// 2026-05-05 (slice #72): accept context-bearing strings.
// Pre-fix narrow `Value::String(text)` match rejected
// `Value::StringContext`, errored even for context-bearing
// strings used in XML emit.
fn xml_value_to_string(value: &Value, context: &str) -> Result<String> {
  coerce_xml_value_to_string(value).ok_or_else(|| anyhow!("{}", context))
}

fn xml_value_to_string_for_key(value: &Value, key: &str) -> Result<String> {
  coerce_xml_value_to_string(value)
    .ok_or_else(|| anyhow!("markup.emit: {} must be string-compatible", key))
}

fn coerce_xml_value_to_string(value: &Value) -> Option<String> {
  match value {
    Value::String(text) => Some(text.clone()),
    Value::StringContext { text, .. } => Some(text.clone()),
    Value::Int(number) => Some(number.to_string()),
    Value::Float(number) => Some(number.to_string()),
    Value::Bool(flag) => Some(flag.to_string()),
    Value::Null => Some(String::new()),
    Value::Path(path) => Some(path.to_string_lossy().into_owned()),
    _ => None,
  }
}

fn xml_escape_text(value: &str) -> Cow<'_, str> {
  xml_escape(value, false)
}

fn xml_escape_attr(value: &str) -> Cow<'_, str> {
  xml_escape(value, true)
}

fn xml_escape(value: &str, escape_quote: bool) -> Cow<'_, str> {
  let Some((start, _)) = value.char_indices().find(|(_, ch)| match ch {
    '&' | '<' | '>' => true,
    '"' if escape_quote => true,
    _ => false,
  }) else {
    return Cow::Borrowed(value);
  };

  let mut out = String::with_capacity(value.len() + 8);
  out.push_str(&value[..start]);
  for ch in value[start..].chars() {
    match ch {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' if escape_quote => out.push_str("&quot;"),
      _ => out.push(ch),
    }
  }
  Cow::Owned(out)
}

fn ascii_lowercase_owned(value: &str) -> String {
  if has_ascii_uppercase(value) {
    value.to_ascii_lowercase()
  } else {
    value.to_string()
  }
}


fn has_ascii_uppercase(value: &str) -> bool {
  value.bytes().any(|byte| byte.is_ascii_uppercase())
}

fn is_html_void_element(name: &str) -> bool {
  matches!(
    name,
    "area"
      | "base"
      | "br"
      | "col"
      | "embed"
      | "hr"
      | "img"
      | "input"
      | "link"
      | "meta"
      | "param"
      | "source"
      | "track"
      | "wbr"
  )
}

fn is_html_raw_text_element(name: &str) -> bool {
  matches!(name, "script" | "style")
}

fn is_html_boolean_attr(name: &str) -> bool {
  matches!(
    name,
    "allowfullscreen"
      | "async"
      | "autofocus"
      | "autoplay"
      | "checked"
      | "controls"
      | "default"
      | "defer"
      | "disabled"
      | "download"
      | "formnovalidate"
      | "hidden"
      | "ismap"
      | "itemscope"
      | "loop"
      | "multiple"
      | "muted"
      | "nomodule"
      | "novalidate"
      | "open"
      | "playsinline"
      | "readonly"
      | "required"
      | "reversed"
      | "selected"
      | "typemustmatch"
  )
}

fn html_normalize_attr_value(name: &str, value: &str) -> String {
  if is_html_boolean_attr(name)
    && (value.is_empty() || value.eq_ignore_ascii_case(name) || value.eq_ignore_ascii_case("true"))
  {
    return "true".to_string();
  }
  value.to_string()
}

fn value_type_name(value: &Value) -> &'static str {
  match value {
    Value::Null => "null",
    Value::Bool(_) => "bool",
    Value::Int(_) => "int",
    Value::Float(_) => "float",
    Value::String(_) | Value::StringContext { .. } => "string",
    Value::Path(_) => "path",
    Value::List(_) => "list",
    Value::AttrSet(_) => "set",
    Value::Lambda { .. } | Value::BuiltinPartial { .. } => "lambda",
    Value::Thunk { .. } => "thunk",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn xml_roundtrip_simple_ast() {
    let parsed = xml_parse(r#"<root a="1"><child>text</child></root>"#).expect("parse");
    let emitted = xml_emit(&parsed).expect("emit");
    assert_eq!(emitted, r#"<root a="1"><child>text</child></root>"#);
  }

  #[test]
  fn html_roundtrip_simple_ast() {
    let parsed = html_parse(r#"<div class="test">Hello</div>"#).expect("parse");
    let emitted = html_emit(&parsed).expect("emit");
    assert!(emitted.contains(r#"<div class="test">Hello</div>"#));
  }

  #[test]
  fn xml_escape_borrows_when_unchanged() {
    assert!(matches!(
      xml_escape_attr("plain-text"),
      Cow::Borrowed("plain-text")
    ));
    assert!(matches!(
      xml_escape_text("plain \" text"),
      Cow::Borrowed("plain \" text")
    ));
  }

  #[test]
  fn xml_escape_owns_when_escaping_needed() {
    assert_eq!(xml_escape_attr("a<&\"b").as_ref(), "a&lt;&amp;&quot;b");
    assert_eq!(xml_escape_text("a<&\"b").as_ref(), "a&lt;&amp;\"b");
  }
}
