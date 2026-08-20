use crate::markup::{json_to_value, value_to_json};
use crate::value::Env;
use crate::Value;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use std::borrow::Cow;
use std::sync::Arc;

// 2026-05-05 (slice #72): same StringContext parity miss as
// slice #50 / #64 / #71. The narrow `Value::String(expr)`
// match rejected context-bearing strings, falling through to
// the attrset path which errored with "expected ... XML
// string or XML JSON attrset" — misleading because the input
// IS a string. Fix uses `as_str()` to handle both variants.
pub fn mathml_xml_to_json(input: &Value) -> Result<Value> {
  let json = if let Some(expr) = input.as_str() {
    pnix_mathml_core::mathml_xml_json_from_expr_normalized(expr, eval_expr_to_json).ok_or_else(
      || anyhow!("builtins.mathmlXmlToJson: expected MathML XML or pnix XML JSON expression"),
    )?
  } else {
    let mut json = value_to_json(input)?;
    if pnix_mathml_core::xml_nodes_from_json(&json).is_empty() {
      return Err(anyhow!(
        "builtins.mathmlXmlToJson: expected MathML XML string or XML JSON attrset"
      ));
    }
    pnix_mathml_core::mathml_normalize_xml_json(&mut json);
    json
  };
  Ok(json_to_value(&json))
}

pub fn openmath_xml_to_json(input: &Value) -> Result<Value> {
  let json = if let Some(expr) = input.as_str() {
    pnix_openmath_core::openmath_xml_json_from_expr_normalized(expr, eval_expr_to_json).ok_or_else(
      || anyhow!("builtins.openmathXmlToJson: expected OpenMath XML or pnix XML JSON expression"),
    )?
  } else {
    let mut json = value_to_json(input)?;
    if pnix_openmath_core::xml_nodes_from_json(&json).is_empty() {
      return Err(anyhow!(
        "builtins.openmathXmlToJson: expected OpenMath XML string or XML JSON attrset"
      ));
    }
    pnix_openmath_core::openmath_normalize_xml_json(&mut json);
    json
  };
  Ok(json_to_value(&json))
}

pub fn mathml_emit(input: &Value) -> Result<String> {
  let json = value_to_json(input)?;
  let document = json
    .as_object()
    .and_then(|obj| obj.get("document"))
    .and_then(JsonValue::as_bool)
    .unwrap_or(false);
  let graph = mathml_graph_from_json(&json)
    .map_err(|err| anyhow!("builtins.mathmlEmit: invalid MathML JSON: {}", err))?;
  let opts = MathmlOptions::default();
  Ok(if document {
    mathml_graph_to_xml_document(&graph, &opts)
  } else {
    mathml_graph_to_xml_fragment(&graph, &opts)
  })
}

pub fn openmath_emit(input: &Value) -> Result<String> {
  let json = value_to_json(input)?;
  let document = json
    .as_object()
    .and_then(|obj| obj.get("document"))
    .and_then(JsonValue::as_bool)
    .unwrap_or(false);
  let graph = openmath_graph_from_json(&json)
    .map_err(|err| anyhow!("builtins.openmathEmit: invalid OpenMath JSON: {}", err))?;
  let opts = OpenmathOptions::default();
  Ok(if document {
    openmath_graph_to_xml_document(&graph, &opts)
  } else {
    openmath_graph_to_xml_fragment(&graph, &opts)
  })
}

fn eval_expr_to_json(expr: &str) -> Option<JsonValue> {
  let parsed = pnix_core::lang::pnix::parse_expr(expr).ok()?;
  let value = crate::interpret::eval(&parsed, &Env::new()).ok()?;
  value_to_json(&value).ok()
}

#[derive(Debug, Clone)]
pub struct MathmlOptions {
  pub display: Option<String>,
  pub mode: Option<String>,
  pub xmlns: Option<String>,
}

impl Default for MathmlOptions {
  fn default() -> Self {
    Self {
      display: Some("block".to_string()),
      mode: Some("math".to_string()),
      xmlns: Some("http://www.w3.org/1998/Math/MathML".to_string()),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathmlGraphSpec {
  #[serde(default)]
  pub display: Option<String>,
  #[serde(default)]
  pub mode: Option<String>,
  #[serde(default)]
  pub xmlns: Option<String>,
  #[serde(default)]
  pub elements: Vec<MathmlElementSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MathmlElementSpec {
  Generic {
    #[serde(default, alias = "kind")]
    kind: Option<String>,
    #[serde(default, alias = "name", alias = "tag")]
    name: String,
    #[serde(default, alias = "node_id", alias = "node-id")]
    id: Option<String>,
    #[serde(default, alias = "attrs", alias = "attributes")]
    attrs: Map<String, JsonValue>,
    #[serde(default, alias = "nodes", alias = "children")]
    children: Vec<MathmlElementSpec>,
    #[serde(default, alias = "text", alias = "value")]
    text: Option<String>,
  },
  Text {
    #[serde(default, alias = "kind")]
    kind: Option<String>,
    #[serde(default, alias = "text", alias = "value")]
    text: String,
  },
}

pub fn mathml_graph_from_json(value: &JsonValue) -> serde_json::Result<MathmlGraphSpec> {
  serde_json::from_value(value.clone())
}

pub fn mathml_graph_to_xml_fragment(graph: &MathmlGraphSpec, opts: &MathmlOptions) -> String {
  let mut out = String::new();
  emit_mathml_element(&mut out, graph, opts, 0);
  out
}

pub fn mathml_graph_to_xml_document(graph: &MathmlGraphSpec, opts: &MathmlOptions) -> String {
  let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
  emit_mathml_element(&mut out, graph, opts, 0);
  out
}

fn emit_mathml_element(
  out: &mut String,
  graph: &MathmlGraphSpec,
  opts: &MathmlOptions,
  indent: usize,
) {
  let indent_str = "  ".repeat(indent);
  out.push_str(&indent_str);
  out.push_str("<math");

  let default_xmlns = "http://www.w3.org/1998/Math/MathML".to_string();
  let xmlns = graph
    .xmlns
    .as_ref()
    .or(opts.xmlns.as_ref())
    .unwrap_or(&default_xmlns);
  push_xml_attr(out, "xmlns", xmlns);

  if let Some(display) = graph.display.as_ref().or(opts.display.as_ref()) {
    push_xml_attr(out, "display", display);
  }
  if let Some(mode) = graph.mode.as_ref().or(opts.mode.as_ref()) {
    push_xml_attr(out, "mode", mode);
  }

  out.push_str(">\n");
  for child in &graph.elements {
    emit_mathml_node(out, child, indent + 1);
  }
  out.push_str(&indent_str);
  out.push_str("</math>\n");
}

fn emit_mathml_node(out: &mut String, node: &MathmlElementSpec, indent: usize) {
  let indent_str = "  ".repeat(indent);
  match node {
    MathmlElementSpec::Text { text, .. } => {
      let escaped = escape_xml_text(text);
      if !escaped.trim().is_empty() {
        out.push_str(&escaped);
      }
    }
    MathmlElementSpec::Generic {
      name,
      id,
      attrs,
      children,
      text,
      ..
    } => {
      out.push_str(&indent_str);
      out.push('<');
      out.push_str(name);
      for (key, value) in attrs {
        if let Some(attr_value) = value_to_xml_attr_string(value) {
          push_xml_attr(out, key, &attr_value);
        }
      }
      if let Some(id) = id {
        push_xml_attr(out, "id", id);
      }

      if children.is_empty() && text.is_none() {
        out.push_str(" />\n");
        return;
      }

      out.push('>');
      if let Some(text) = text {
        if children.is_empty() {
          let escaped = escape_xml_text(text);
          if !escaped.trim().is_empty() {
            out.push_str(&escaped);
          }
        }
      }
      if !children.is_empty() {
        out.push('\n');
        for child in children {
          emit_mathml_node(out, child, indent + 1);
        }
        out.push_str(&indent_str);
        out.push_str("</");
        out.push_str(name);
        out.push_str(">\n");
      } else {
        out.push_str("</");
        out.push_str(name);
        out.push_str(">\n");
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct OpenmathOptions {
  pub xmlns: Option<String>,
}

impl Default for OpenmathOptions {
  fn default() -> Self {
    Self {
      xmlns: Some("http://www.openmath.org/OpenMath".to_string()),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenmathGraphSpec {
  #[serde(default)]
  pub xmlns: Option<String>,
  #[serde(default)]
  pub cd_base: Option<String>,
  #[serde(default)]
  pub expressions: Vec<OpenmathElementSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenmathElementSpec {
  Generic {
    #[serde(default, alias = "kind")]
    kind: Option<String>,
    #[serde(default, alias = "name", alias = "tag")]
    name: String,
    #[serde(default, alias = "node_id", alias = "node-id")]
    id: Option<String>,
    #[serde(default, alias = "attrs", alias = "attributes")]
    attrs: Map<String, JsonValue>,
    #[serde(default, alias = "nodes", alias = "children")]
    children: Vec<OpenmathElementSpec>,
    #[serde(default, alias = "text", alias = "value")]
    text: Option<String>,
  },
  Text {
    #[serde(default, alias = "kind")]
    kind: Option<String>,
    #[serde(default, alias = "text", alias = "value")]
    text: String,
  },
}

pub fn openmath_graph_from_json(value: &JsonValue) -> serde_json::Result<OpenmathGraphSpec> {
  serde_json::from_value(value.clone())
}

pub fn openmath_graph_to_xml_fragment(graph: &OpenmathGraphSpec, opts: &OpenmathOptions) -> String {
  let mut out = String::new();
  emit_openmath_element(&mut out, graph, opts, 0);
  out
}

pub fn openmath_graph_to_xml_document(graph: &OpenmathGraphSpec, opts: &OpenmathOptions) -> String {
  let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
  emit_openmath_element(&mut out, graph, opts, 0);
  out
}

fn emit_openmath_element(
  out: &mut String,
  graph: &OpenmathGraphSpec,
  opts: &OpenmathOptions,
  indent: usize,
) {
  let indent_str = "  ".repeat(indent);
  let default_xmlns = "http://www.openmath.org/OpenMath".to_string();
  let xmlns = graph
    .xmlns
    .as_ref()
    .or(opts.xmlns.as_ref())
    .unwrap_or(&default_xmlns);

  if graph.expressions.is_empty() {
    out.push_str(&indent_str);
    out.push_str("<OMOBJ");
    push_xml_attr(out, "xmlns", xmlns);
    out.push_str("></OMOBJ>");
    return;
  }

  if graph.expressions.len() == 1 {
    emit_openmath_element_spec(out, &graph.expressions[0], xmlns.as_str(), indent);
  } else {
    out.push_str(&indent_str);
    out.push_str("<OMOBJ");
    push_xml_attr(out, "xmlns", xmlns);
    out.push_str(">\n");
    for expr in &graph.expressions {
      emit_openmath_element_spec(out, expr, xmlns.as_str(), indent + 1);
    }
    out.push_str(&indent_str);
    out.push_str("</OMOBJ>");
  }
}

fn emit_openmath_element_spec(
  out: &mut String,
  spec: &OpenmathElementSpec,
  xmlns: &str,
  indent: usize,
) {
  let indent_str = "  ".repeat(indent);
  match spec {
    OpenmathElementSpec::Text { text, .. } => {
      out.push_str(&indent_str);
      out.push_str(&escape_xml_text(text));
    }
    OpenmathElementSpec::Generic {
      name,
      attrs,
      children,
      text,
      id,
      ..
    } => {
      let tag_name = name.to_uppercase();
      out.push_str(&indent_str);
      out.push('<');
      out.push_str(&tag_name);
      if xmlns != "http://www.openmath.org/OpenMath" {
        push_xml_attr(out, "xmlns", xmlns);
      }
      if let Some(id) = id {
        let escaped_id = escape_xml_attr(id);
        out.push_str(" id=\"");
        out.push_str(&escaped_id);
        out.push_str("\" data-node-id=\"");
        out.push_str(&escaped_id);
        out.push('"');
      }
      for (key, value) in attrs {
        if let Some(value) = value_to_xml_attr_string(value) {
          push_xml_attr(out, key, &value);
        }
      }

      if !children.is_empty() || text.is_some() {
        out.push('>');
        if !children.is_empty() {
          out.push('\n');
          for child in children {
            emit_openmath_element_spec(out, child, xmlns, indent + 1);
          }
          out.push_str(&indent_str);
        } else if let Some(text) = text {
          out.push_str(&escape_xml_text(text));
        }
        out.push_str("</");
        out.push_str(&tag_name);
        out.push('>');
      } else {
        out.push_str("/>");
      }
      out.push('\n');
    }
  }
}

fn push_xml_attr(out: &mut String, key: &str, value: &str) {
  out.push(' ');
  out.push_str(key);
  out.push_str("=\"");
  out.push_str(&escape_xml_attr(value));
  out.push('"');
}

fn value_to_xml_attr_string(value: &JsonValue) -> Option<String> {
  match value {
    JsonValue::String(text) => Some(text.clone()),
    JsonValue::Number(number) => Some(number.to_string()),
    JsonValue::Bool(flag) => Some(flag.to_string()),
    JsonValue::Null => None,
    JsonValue::Array(items) => {
      let mut out = String::new();
      for item in items {
        let Some(part) = value_to_xml_attr_string(item) else {
          continue;
        };
        if !out.is_empty() {
          out.push(' ');
        }
        out.push_str(&part);
      }
      (!out.is_empty()).then_some(out)
    }
    JsonValue::Object(_) => serde_json::to_string(value).ok(),
  }
}

fn escape_xml_attr(text: &str) -> Cow<'_, str> {
  escape_xml(text, true)
}

fn escape_xml_text(text: &str) -> Cow<'_, str> {
  escape_xml(text, false)
}

fn escape_xml(text: &str, attr: bool) -> Cow<'_, str> {
  let Some((start, first_char)) = text.char_indices().find(|(_, ch)| match ch {
    '&' | '<' | '>' => true,
    '"' | '\'' if attr => true,
    _ => false,
  }) else {
    return Cow::Borrowed(text);
  };

  let mut out = String::with_capacity(text.len() + 8);
  out.push_str(&text[..start]);
  push_escaped_xml_char(&mut out, first_char, attr);
  for ch in text[start + first_char.len_utf8()..].chars() {
    push_escaped_xml_char(&mut out, ch, attr);
  }
  Cow::Owned(out)
}

fn push_escaped_xml_char(out: &mut String, ch: char, attr: bool) {
  match ch {
    '&' => out.push_str("&amp;"),
    '<' => out.push_str("&lt;"),
    '>' => out.push_str("&gt;"),
    '"' if attr => out.push_str("&quot;"),
    '\'' if attr => out.push_str("&apos;"),
    _ => out.push(ch),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::BTreeMap;

  #[test]
  fn math_xml_attr_array_preserves_join_surface() {
    let value = JsonValue::Array(vec![
      JsonValue::String("0".to_string()),
      JsonValue::Number(serde_json::Number::from(10)),
      JsonValue::Null,
      JsonValue::Bool(true),
    ]);
    assert_eq!(
      value_to_xml_attr_string(&value).as_deref(),
      Some("0 10 true")
    );
  }

  #[test]
  fn mathml_roundtrip_from_graph_and_xml() {
    let graph = Value::AttrSet(Arc::new(BTreeMap::from([
      ("kind".to_string(), Value::String("mathml".to_string())),
      ("display".to_string(), Value::String("inline".to_string())),
      (
        "elements".to_string(),
        Value::List(Arc::new(vec![Value::AttrSet(Arc::new(BTreeMap::from([
          ("name".to_string(), Value::String("msqrt".to_string())),
          (
            "children".to_string(),
            Value::List(Arc::new(vec![Value::AttrSet(Arc::new(BTreeMap::from([
              ("name".to_string(), Value::String("mn".to_string())),
              ("text".to_string(), Value::String("9".to_string())),
            ])))])),
          ),
        ])))])),
      ),
    ])));

    let xml = mathml_emit(&graph).expect("emit");
    assert!(xml.contains("<math"));
    let json = mathml_xml_to_json(&Value::String(xml)).expect("json");
    match json {
      Value::AttrSet(map) => assert_eq!(map.get("name").and_then(Value::as_str), Some("math")),
      other => panic!("expected attrset, got {:?}", other),
    }
  }

  #[test]
  fn openmath_roundtrip_from_graph_and_xml() {
    let graph = Value::AttrSet(Arc::new(BTreeMap::from([(
      "expressions".to_string(),
      Value::List(Arc::new(vec![Value::AttrSet(Arc::new(BTreeMap::from([
        ("name".to_string(), Value::String("OMA".to_string())),
        (
          "children".to_string(),
          Value::List(Arc::new(vec![
            Value::AttrSet(Arc::new(BTreeMap::from([
              ("name".to_string(), Value::String("OMS".to_string())),
              (
                "attrs".to_string(),
                Value::AttrSet(Arc::new(BTreeMap::from([
                  ("cd".to_string(), Value::String("pnix-arith".to_string())),
                  ("name".to_string(), Value::String("plus".to_string())),
                ]))),
              ),
            ]))),
            Value::AttrSet(Arc::new(BTreeMap::from([
              ("name".to_string(), Value::String("OMI".to_string())),
              ("text".to_string(), Value::String("1".to_string())),
            ]))),
          ])),
        ),
      ])))])),
    )])));

    let xml = openmath_emit(&graph).expect("emit");
    assert!(xml.contains("<OMA"));
    let json = openmath_xml_to_json(&Value::String(xml)).expect("json");
    match json {
      Value::AttrSet(map) => assert_eq!(map.get("name").and_then(Value::as_str), Some("OMA")),
      other => panic!("expected attrset, got {:?}", other),
    }
  }

  #[test]
  fn math_xml_escape_borrows_when_unchanged() {
    assert!(matches!(
      escape_xml_attr("pnix-arith-plus"),
      Cow::Borrowed("pnix-arith-plus")
    ));
    assert!(matches!(escape_xml_text("x + y"), Cow::Borrowed("x + y")));
  }

  #[test]
  fn math_xml_escape_owns_when_escaping_needed() {
    assert_eq!(
      escape_xml_attr("a<&\"'b").as_ref(),
      "a&lt;&amp;&quot;&apos;b"
    );
    assert_eq!(escape_xml_text("a<&\"'b").as_ref(), "a&lt;&amp;\"'b");
  }
}
