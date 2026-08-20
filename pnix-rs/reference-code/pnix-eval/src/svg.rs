use crate::markup::{json_to_value, value_to_json, xml_emit};
use crate::Value;
use anyhow::{anyhow, Result};
use serde_json::{Map, Number, Value as JsonValue};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub fn svg_schema_normalize(input: &Value) -> Result<Value> {
  let json = svg_input_to_json(input, "builtins.svgSchemaNormalize")?;
  let normalized = pnix_svg_core::svg_normalize_json(&json)
    .map_err(|err| anyhow!("builtins.svgSchemaNormalize: {}", err))?;
  Ok(json_to_value(&normalized))
}

pub fn svg_schema_validate(input: &Value) -> Result<Value> {
  let json = svg_input_to_json(input, "builtins.svgSchemaValidate")?;
  let version = svg_version_from_json(&json);
  let report = match pnix_svg_core::svg_validate_json(&json, &version) {
    Ok(()) => serde_json::json!({
      "success": true,
      "ok": true,
      "errors": [],
      "version": version,
    }),
    Err(errors) => serde_json::json!({
      "success": false,
      "ok": false,
      "errors": errors,
      "version": version,
    }),
  };
  Ok(json_to_value(&report))
}

pub fn svg_schema_explain(input: &Value) -> Result<Value> {
  let json = svg_input_to_json(input, "builtins.svgSchemaExplain")?;
  let version = svg_version_from_json(&json);
  let explanation = match pnix_svg_core::svg_validate_json(&json, &version) {
    Ok(()) => String::new(),
    Err(errors) => errors.join("\n"),
  };
  Ok(Value::String(explanation))
}

pub fn svg_emit(input: &Value) -> Result<String> {
  let mut json = svg_input_to_json(input, "builtins.svgEmit")?;
  json =
    pnix_svg_core::svg_normalize_json(&json).map_err(|err| anyhow!("builtins.svgEmit: {}", err))?;
  ensure_svg_namespace(&mut json);
  xml_emit(&json_to_value(&json)).map_err(|err| anyhow!("builtins.svgEmit: {}", err))
}

pub fn svg_render_packet(previous: &Value, next: &Value) -> Result<Value> {
  let previous_json = if matches!(previous, Value::Null) {
    None
  } else {
    Some(svg_input_to_json(previous, "builtins.svgRenderPacket")?)
  };
  let next_json = svg_input_to_json(next, "builtins.svgRenderPacket")?;
  let packet = svg_render_packet_json(previous_json.as_ref(), &next_json)?;
  Ok(json_to_value(&packet))
}

// 2026-05-05 (slice #72): accept context-bearing strings via
// as_str(). Same parity miss family as slice #50/#64/#71.
fn svg_input_to_json(input: &Value, builtin_name: &str) -> Result<JsonValue> {
  if let Some(xml) = input.as_str() {
    pnix_svg_core::svg_json_from_xml_str(xml)
      .map_err(|err| anyhow!("{builtin_name}: expected SVG XML string or XML JSON attrset: {err}"))
  } else {
    let json = value_to_json(input)?;
    let Some(root) = json.as_object() else {
      return Err(anyhow!(
        "{builtin_name}: expected SVG XML string or XML JSON attrset"
      ));
    };
    if !matches!(root.get("name"), Some(JsonValue::String(name)) if name == "svg") {
      return Err(anyhow!(
        "{builtin_name}: expected root SVG XML JSON attrset"
      ));
    }
    Ok(json)
  }
}

fn svg_version_from_json(json: &JsonValue) -> String {
  json
    .as_object()
    .and_then(|obj| obj.get("attrs"))
    .and_then(JsonValue::as_object)
    .and_then(|attrs| attrs.get("version"))
    .and_then(JsonValue::as_str)
    .map(ToString::to_string)
    .unwrap_or_else(|| "1.1".to_string())
}

fn ensure_svg_namespace(json: &mut JsonValue) {
  let Some(root) = json.as_object_mut() else {
    return;
  };
  if !matches!(root.get("name"), Some(JsonValue::String(name)) if name == "svg") {
    return;
  }
  let attrs = root
    .entry("attrs".to_string())
    .or_insert_with(|| JsonValue::Object(Map::new()));
  let Some(attrs) = attrs.as_object_mut() else {
    return;
  };
  attrs
    .entry("xmlns".to_string())
    .or_insert_with(|| JsonValue::String(pnix_svg_core::SVG_NAMESPACE.to_string()));
}

fn annotate_svg_sync_ids(
  node: &mut JsonValue,
  sync_prefix: &str,
  parent_pnix_address: Option<&str>,
  index: usize,
) {
  let Some(obj) = node.as_object_mut() else {
    return;
  };
  if !matches!(obj.get("kind"), Some(JsonValue::String(kind)) if kind == "element") {
    return;
  }

  let tag_name = obj
    .get("name")
    .and_then(JsonValue::as_str)
    .unwrap_or("svg-node")
    .to_string();

  let attrs = obj
    .entry("attrs".to_string())
    .or_insert_with(|| JsonValue::Object(Map::new()));
  let Some(attrs) = attrs.as_object_mut() else {
    return;
  };
  let node_id = svg_sync_node_id(
    sync_prefix,
    index,
    attrs.get("id").and_then(JsonValue::as_str),
  );
  attrs.insert(
    "data-node-id".to_string(),
    JsonValue::String(node_id.clone()),
  );
  attrs.insert("data-sync-id".to_string(), JsonValue::String(node_id));
  attrs.insert(
    "data-svg-tag".to_string(),
    JsonValue::String(tag_name.clone()),
  );
  let pnix_address = svg_pnix_address(
    parent_pnix_address,
    index,
    &tag_name,
    attrs.get("id").and_then(JsonValue::as_str),
  );
  attrs.insert(
    "data-pnix-address".to_string(),
    JsonValue::String(pnix_address.clone()),
  );

  if let Some(children) = obj.get_mut("children").and_then(JsonValue::as_array_mut) {
    for (child_index, child) in children.iter_mut().enumerate() {
      annotate_svg_sync_ids(child, sync_prefix, Some(&pnix_address), child_index);
    }
  }
}

fn svg_sync_node_id(sync_prefix: &str, index: usize, element_id: Option<&str>) -> String {
  if let Some(id) = element_id {
    let mut node_id = String::with_capacity("id:".len() + id.len());
    node_id.push_str("id:");
    node_id.push_str(id);
    return node_id;
  }

  let mut node_id = String::with_capacity("path::".len() + sync_prefix.len() + 20);
  node_id.push_str("path:");
  node_id.push_str(sync_prefix);
  node_id.push(':');
  push_usize_decimal(index, &mut node_id);
  node_id
}

fn svg_pnix_address(
  parent_pnix_address: Option<&str>,
  index: usize,
  tag_name: &str,
  element_id: Option<&str>,
) -> String {
  match parent_pnix_address {
    None => tag_name.to_string(),
    Some(parent) => match element_id {
      Some(id) if !id.is_empty() => {
        let mut address =
          String::with_capacity(parent.len() + tag_name.len() + id.len() + "/#".len());
        address.push_str(parent);
        address.push('/');
        address.push_str(tag_name);
        address.push('#');
        address.push_str(id);
        address
      }
      _ => {
        let mut address = String::with_capacity(parent.len() + tag_name.len() + "/:".len() + 20);
        address.push_str(parent);
        address.push('/');
        address.push_str(tag_name);
        address.push(':');
        push_usize_decimal(index, &mut address);
        address
      }
    },
  }
}

fn push_usize_decimal(value: usize, out: &mut String) {
  if value == 0 {
    out.push('0');
    return;
  }

  let mut value = value;
  let mut digits = [0u8; 20];
  let mut len = 0;
  while value > 0 {
    digits[len] = b'0' + (value % 10) as u8;
    len += 1;
    value /= 10;
  }
  for idx in (0..len).rev() {
    out.push(digits[idx] as char);
  }
}

fn svg_sync_plan_json(previous: Option<&JsonValue>, next: &JsonValue) -> JsonValue {
  let mut next_scene = next.clone();
  ensure_svg_namespace(&mut next_scene);
  annotate_svg_sync_ids(&mut next_scene, "root", None, 0);

  let mut ops = Vec::new();
  let mode = match previous {
    Some(previous) => {
      let mut previous_scene = previous.clone();
      ensure_svg_namespace(&mut previous_scene);
      annotate_svg_sync_ids(&mut previous_scene, "root", None, 0);
      diff_svg_nodes(&previous_scene, &next_scene, &mut ops);
      if ops.is_empty() {
        "noop"
      } else {
        "patch"
      }
    }
    None => {
      ops.push(serde_json::json!({
        "op": "mount",
        "node_id": next_scene
          .as_object()
          .map(svg_sync_id_of)
          .unwrap_or_else(|| "root".to_string()),
        "node_pnix_address": next_scene
          .as_object()
          .map(svg_pnix_address_of)
          .unwrap_or_else(|| "svg".to_string()),
        "target_pnix_address": next_scene
          .as_object()
          .map(svg_pnix_address_of)
          .unwrap_or_else(|| "svg".to_string()),
        "node": next_scene.clone(),
      }));
      "replace"
    }
  };

  serde_json::json!({
    "mode": mode,
    "changed": !ops.is_empty(),
    "patch_count": ops.len(),
    "scene": next_scene,
    "ops": ops,
  })
}

fn svg_fragment_from_json(node: &JsonValue) -> Result<String> {
  xml_emit(&json_to_value(node)).map_err(|err| anyhow!("svg fragment emit: {}", err))
}

fn svg_document_from_fragment(fragment: &str) -> String {
  const PREFIX: &str = "<!doctype html><html><head><meta charset=\"utf-8\"><title>pnix-svg-ssr</title></head><body data-pnix-svg-runtime=\"ssr\"><div id=\"pnix-svg-root\" data-pnix-svg-root=\"true\">";
  const SUFFIX: &str = "</div></body></html>";
  let mut document = String::with_capacity(PREFIX.len() + fragment.len() + SUFFIX.len());
  document.push_str(PREFIX);
  document.push_str(fragment);
  document.push_str(SUFFIX);
  document
}

fn lower_svg_sync_op_to_html(op: &JsonValue) -> Result<JsonValue> {
  let op_obj = op
    .as_object()
    .ok_or_else(|| anyhow!("builtins.svgRenderPacket: sync op must be attrset"))?;
  let kind = op_obj
    .get("op")
    .and_then(JsonValue::as_str)
    .ok_or_else(|| anyhow!("builtins.svgRenderPacket: sync op missing kind"))?;

  match kind {
    "mount" | "replace" => {
      let node = op_obj
        .get("node")
        .ok_or_else(|| anyhow!("builtins.svgRenderPacket: sync op missing node"))?;
      let node_id = op_obj
        .get("node_id")
        .and_then(JsonValue::as_str)
        .or_else(|| {
          node
            .as_object()
            .and_then(|obj| obj.get("attrs"))
            .and_then(JsonValue::as_object)
            .and_then(|attrs| attrs.get("data-node-id"))
            .and_then(JsonValue::as_str)
        })
        .unwrap_or("path:root:0");
      let target_pnix_address = op_obj
        .get("target_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .or_else(|| {
          op_obj
            .get("node_pnix_address")
            .and_then(JsonValue::as_str)
            .map(ToString::to_string)
        })
        .or_else(|| node.as_object().map(svg_pnix_address_of))
        .unwrap_or_else(|| "svg".to_string());
      Ok(serde_json::json!({
        "op": kind,
        "target_node_id": node_id,
        "target_pnix_address": target_pnix_address,
        "html": svg_fragment_from_json(node)?,
      }))
    }
    "replace_children" => {
      let node_id = op_obj
        .get("node_id")
        .and_then(JsonValue::as_str)
        .unwrap_or("path:root:0");
      let target_pnix_address = op_obj
        .get("target_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .or_else(|| {
          op_obj
            .get("node_pnix_address")
            .and_then(JsonValue::as_str)
            .map(ToString::to_string)
        })
        .unwrap_or_else(|| "svg".to_string());
      let children = op_obj
        .get("children")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| anyhow!("builtins.svgRenderPacket: replace_children missing children"))?;
      let mut html_children = Vec::with_capacity(children.len());
      for child in children {
        html_children.push(svg_fragment_from_json(child)?);
      }
      Ok(serde_json::json!({
        "op": "replace_children",
        "target_node_id": node_id,
        "target_pnix_address": target_pnix_address,
        "children_html": html_children,
      }))
    }
    "insert_child" => {
      let node_id = op_obj
        .get("node_id")
        .and_then(JsonValue::as_str)
        .unwrap_or("path:root:0");
      let target_pnix_address = op_obj
        .get("target_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .or_else(|| {
          op_obj
            .get("node_pnix_address")
            .and_then(JsonValue::as_str)
            .map(ToString::to_string)
        })
        .unwrap_or_else(|| "svg".to_string());
      let child_node_id = op_obj
        .get("child_node_id")
        .and_then(JsonValue::as_str)
        .unwrap_or("path:root:0");
      let index = op_obj.get("index").and_then(JsonValue::as_u64).unwrap_or(0);
      let node = op_obj
        .get("node")
        .ok_or_else(|| anyhow!("builtins.svgRenderPacket: insert_child missing node"))?;
      let child_pnix_address = op_obj
        .get("child_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .or_else(|| node.as_object().map(svg_pnix_address_of))
        .unwrap_or_else(|| child_node_id.to_string());
      Ok(serde_json::json!({
        "op": "insert_child",
        "target_node_id": node_id,
        "target_pnix_address": target_pnix_address,
        "child_node_id": child_node_id,
        "child_pnix_address": child_pnix_address,
        "index": index,
        "html": svg_fragment_from_json(node)?,
      }))
    }
    "remove_child" => {
      let node_id = op_obj
        .get("node_id")
        .and_then(JsonValue::as_str)
        .unwrap_or("path:root:0");
      let target_pnix_address = op_obj
        .get("target_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .or_else(|| {
          op_obj
            .get("node_pnix_address")
            .and_then(JsonValue::as_str)
            .map(ToString::to_string)
        })
        .unwrap_or_else(|| "svg".to_string());
      let child_node_id = op_obj
        .get("child_node_id")
        .and_then(JsonValue::as_str)
        .unwrap_or("path:root:0");
      let child_pnix_address = op_obj
        .get("child_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| child_node_id.to_string());
      Ok(serde_json::json!({
        "op": "remove_child",
        "target_node_id": node_id,
        "target_pnix_address": target_pnix_address,
        "child_node_id": child_node_id,
        "child_pnix_address": child_pnix_address,
      }))
    }
    "reorder_children" => {
      let node_id = op_obj
        .get("node_id")
        .and_then(JsonValue::as_str)
        .unwrap_or("path:root:0");
      let target_pnix_address = op_obj
        .get("target_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .or_else(|| {
          op_obj
            .get("node_pnix_address")
            .and_then(JsonValue::as_str)
            .map(ToString::to_string)
        })
        .unwrap_or_else(|| "svg".to_string());
      let child_node_id_values = op_obj
        .get("child_node_ids")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| anyhow!("builtins.svgRenderPacket: reorder_children missing child ids"))?;
      let mut child_node_ids = Vec::with_capacity(child_node_id_values.len());
      for child_id in child_node_id_values {
        if let Some(child_id) = child_id.as_str() {
          child_node_ids.push(child_id.to_string());
        }
      }
      let child_pnix_addresses = op_obj
        .get("child_pnix_addresses")
        .and_then(JsonValue::as_array)
        .map(|items| {
          let mut out = Vec::with_capacity(items.len());
          for item in items {
            if let Some(item) = item.as_str() {
              out.push(item.to_string());
            }
          }
          out
        })
        .unwrap_or_default();
      Ok(serde_json::json!({
        "op": "reorder_children",
        "target_node_id": node_id,
        "target_pnix_address": target_pnix_address,
        "child_node_ids": child_node_ids,
        "child_pnix_addresses": child_pnix_addresses,
      }))
    }
    "update_attrs" => {
      let node_id = op_obj
        .get("node_id")
        .and_then(JsonValue::as_str)
        .unwrap_or("path:root:0");
      let target_pnix_address = op_obj
        .get("target_pnix_address")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .or_else(|| {
          op_obj
            .get("node_pnix_address")
            .and_then(JsonValue::as_str)
            .map(ToString::to_string)
        })
        .unwrap_or_else(|| "svg".to_string());
      let attrs = op_obj
        .get("attrs")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| anyhow!("builtins.svgRenderPacket: update_attrs missing attrs"))?;
      Ok(serde_json::json!({
        "op": "update_attrs",
        "target_node_id": node_id,
        "target_pnix_address": target_pnix_address,
        "attrs": svg_attrs_json(attrs),
      }))
    }
    other => Err(anyhow!(
      "builtins.svgRenderPacket: unsupported sync op {}",
      other
    )),
  }
}

fn diff_svg_nodes(previous: &JsonValue, next: &JsonValue, ops: &mut Vec<JsonValue>) {
  if previous == next {
    return;
  }

  let Some(previous_obj) = previous.as_object() else {
    ops.push(svg_replace_sync_op(next));
    return;
  };
  let Some(next_obj) = next.as_object() else {
    ops.push(svg_replace_sync_op(next));
    return;
  };

  let previous_id = svg_sync_id_of(previous_obj);
  let next_id = svg_sync_id_of(next_obj);
  let previous_name = previous_obj.get("name").and_then(JsonValue::as_str);
  let next_name = next_obj.get("name").and_then(JsonValue::as_str);
  let previous_kind = previous_obj.get("kind").and_then(JsonValue::as_str);
  let next_kind = next_obj.get("kind").and_then(JsonValue::as_str);

  if previous_id != next_id || previous_name != next_name || previous_kind != next_kind {
    ops.push(svg_replace_sync_op(next));
    return;
  }

  if previous_obj.get("value") != next_obj.get("value") {
    ops.push(svg_replace_sync_op(next));
    return;
  }

  if previous_obj.get("attrs") != next_obj.get("attrs") {
    ops.push(serde_json::json!({
      "op": "update_attrs",
      "node_id": next_id,
      "node_pnix_address": svg_pnix_address_of(next_obj),
      "target_pnix_address": svg_pnix_address_of(next_obj),
      "attrs": next_obj.get("attrs").cloned().unwrap_or(JsonValue::Object(Map::new())),
    }));
  }

  let previous_children = previous_obj
    .get("children")
    .and_then(JsonValue::as_array)
    .cloned()
    .unwrap_or_default();
  let next_children = next_obj
    .get("children")
    .and_then(JsonValue::as_array)
    .cloned()
    .unwrap_or_default();

  let next_pnix_address = svg_pnix_address_of(next_obj);

  if diff_svg_children(
    &next_id,
    &next_pnix_address,
    &previous_children,
    &next_children,
    ops,
  ) {
    return;
  }

  if previous_children.len() != next_children.len()
    || svg_child_id_order(&previous_children) != svg_child_id_order(&next_children)
  {
    ops.push(serde_json::json!({
      "op": "replace_children",
      "node_id": next_id,
      "node_pnix_address": next_pnix_address,
      "target_pnix_address": next_pnix_address,
      "children": next_children,
    }));
    return;
  }

  for (previous_child, next_child) in previous_children.iter().zip(next_children.iter()) {
    diff_svg_nodes(previous_child, next_child, ops);
  }
}

fn diff_svg_children(
  parent_id: &str,
  parent_pnix_address: &str,
  previous_children: &[JsonValue],
  next_children: &[JsonValue],
  ops: &mut Vec<JsonValue>,
) -> bool {
  let Some((previous_ids, previous_lookup)) = svg_index_children(previous_children) else {
    return false;
  };
  let Some((next_ids, next_lookup)) = svg_index_children(next_children) else {
    return false;
  };

  let previous_set = previous_ids.iter().cloned().collect::<BTreeSet<_>>();
  let next_set = next_ids.iter().cloned().collect::<BTreeSet<_>>();

  for removed_id in previous_ids.iter().filter(|id| !next_set.contains(*id)) {
    let child_pnix_address = previous_lookup
      .get(removed_id)
      .and_then(JsonValue::as_object)
      .map(svg_pnix_address_of)
      .unwrap_or_else(|| removed_id.clone());
    ops.push(serde_json::json!({
      "op": "remove_child",
      "node_id": parent_id,
      "node_pnix_address": parent_pnix_address,
      "target_pnix_address": parent_pnix_address,
      "child_node_id": removed_id,
      "child_pnix_address": child_pnix_address,
    }));
  }

  for (index, next_id) in next_ids.iter().enumerate() {
    if !previous_set.contains(next_id) {
      let Some(node) = next_lookup.get(next_id) else {
        return false;
      };
      ops.push(serde_json::json!({
        "op": "insert_child",
        "node_id": parent_id,
        "node_pnix_address": parent_pnix_address,
        "target_pnix_address": parent_pnix_address,
        "index": index,
        "child_node_id": next_id,
        "child_pnix_address": node
          .as_object()
          .map(svg_pnix_address_of)
          .unwrap_or_else(|| next_id.clone()),
        "node": node,
      }));
    }
  }

  if previous_ids != next_ids {
    let mut child_pnix_addresses = Vec::with_capacity(next_ids.len());
    for child_id in &next_ids {
      if let Some(address) = next_lookup
        .get(child_id)
        .and_then(JsonValue::as_object)
        .map(svg_pnix_address_of)
      {
        child_pnix_addresses.push(address);
      }
    }
    ops.push(serde_json::json!({
      "op": "reorder_children",
      "node_id": parent_id,
      "node_pnix_address": parent_pnix_address,
      "target_pnix_address": parent_pnix_address,
      "child_node_ids": next_ids,
      "child_pnix_addresses": child_pnix_addresses,
    }));
  }

  for next_id in next_ids.iter().filter(|id| previous_set.contains(*id)) {
    let Some(previous_child) = previous_lookup.get(next_id) else {
      return false;
    };
    let Some(next_child) = next_lookup.get(next_id) else {
      return false;
    };
    diff_svg_nodes(previous_child, next_child, ops);
  }

  true
}

fn svg_replace_sync_op(next: &JsonValue) -> JsonValue {
  serde_json::json!({
    "op": "replace",
    "node_id": next
      .as_object()
      .map(svg_sync_id_of)
      .unwrap_or_else(|| "root".to_string()),
    "node_pnix_address": next
      .as_object()
      .map(svg_pnix_address_of)
      .unwrap_or_else(|| "svg".to_string()),
    "target_pnix_address": next
      .as_object()
      .map(svg_pnix_address_of)
      .unwrap_or_else(|| "svg".to_string()),
    "node": next,
  })
}

fn svg_sync_id_of(obj: &Map<String, JsonValue>) -> String {
  obj
    .get("attrs")
    .and_then(JsonValue::as_object)
    .and_then(|attrs| attrs.get("data-sync-id"))
    .and_then(JsonValue::as_str)
    .unwrap_or("root")
    .to_string()
}

fn svg_pnix_address_of(obj: &Map<String, JsonValue>) -> String {
  obj
    .get("attrs")
    .and_then(JsonValue::as_object)
    .and_then(|attrs| attrs.get("data-pnix-address"))
    .and_then(JsonValue::as_str)
    .map(ToString::to_string)
    .or_else(|| {
      obj
        .get("name")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
    })
    .unwrap_or_else(|| "svg".to_string())
}

fn svg_child_id_order(children: &[JsonValue]) -> Vec<String> {
  let mut out = Vec::with_capacity(children.len());
  for child in children {
    out.push(
      child
        .as_object()
        .map(svg_sync_id_of)
        .unwrap_or_else(|| "<leaf>".to_string()),
    );
  }
  out
}

fn svg_index_children(
  children: &[JsonValue],
) -> Option<(Vec<String>, BTreeMap<String, JsonValue>)> {
  let mut ids = Vec::with_capacity(children.len());
  let mut seen = BTreeSet::new();
  let mut lookup = BTreeMap::new();

  for child in children {
    let obj = child.as_object()?;
    let child_id = svg_sync_id_of(obj);
    if !seen.insert(child_id.clone()) {
      return None;
    }
    ids.push(child_id.clone());
    lookup.insert(child_id, child.clone());
  }

  Some((ids, lookup))
}

fn svg_attrs_json(attrs: &Map<String, JsonValue>) -> JsonValue {
  let mut rendered = Map::new();
  for (key, value) in attrs {
    if let Some(rendered_value) = svg_json_attr_string(value) {
      rendered.insert(key.clone(), JsonValue::String(rendered_value));
    }
  }
  JsonValue::Object(rendered)
}

fn svg_json_attr_string(value: &JsonValue) -> Option<String> {
  match value {
    JsonValue::Null => None,
    JsonValue::Bool(flag) => Some(flag.to_string()),
    JsonValue::Number(number) => Some(number.to_string()),
    JsonValue::String(text) => Some(text.clone()),
    JsonValue::Array(items) => {
      let mut out = String::new();
      for item in items {
        let Some(part) = svg_json_attr_string(item) else {
          continue;
        };
        if !out.is_empty() {
          out.push(' ');
        }
        out.push_str(&part);
      }
      (!out.is_empty()).then_some(out)
    }
    JsonValue::Object(_) => None,
  }
}

fn svg_html_payload_from_sync_plan(plan: &JsonValue) -> Result<JsonValue> {
  let plan_obj = plan
    .as_object()
    .ok_or_else(|| anyhow!("builtins.svgRenderPacket: sync plan must be attrset"))?;
  let scene = plan_obj
    .get("scene")
    .ok_or_else(|| anyhow!("builtins.svgRenderPacket: sync plan missing scene"))?;
  let scene_ops = plan_obj
    .get("ops")
    .and_then(JsonValue::as_array)
    .ok_or_else(|| anyhow!("builtins.svgRenderPacket: sync plan missing ops"))?;
  let fragment = svg_fragment_from_json(scene)?;
  let mut html_ops = Vec::with_capacity(scene_ops.len());
  for op in scene_ops {
    html_ops.push(lower_svg_sync_op_to_html(op)?);
  }

  Ok(serde_json::json!({
    "protocol": "pnix.svg.html.v1",
    "engine": "svg-inline-ssr",
    "mode": plan_obj.get("mode").cloned().unwrap_or(JsonValue::String("noop".to_string())),
    "changed": plan_obj.get("changed").cloned().unwrap_or(JsonValue::Bool(false)),
    "patch_count": plan_obj.get("patch_count").cloned().unwrap_or(JsonValue::Number(Number::from(0))),
    "scene": scene.clone(),
    "scene_ops": JsonValue::Array(scene_ops.to_vec()),
    "fragment": fragment,
    "html": svg_document_from_fragment(&svg_fragment_from_json(scene)?),
    "ops": JsonValue::Array(html_ops),
  }))
}

fn render_memory_json(family: &str, scene_kind: &str, surfaces: &[&str]) -> JsonValue {
  serde_json::json!({
    "protocol": "pnix.world.memory.v1",
    "family": family,
    "scene_kind": scene_kind,
    "surfaces": surfaces,
    "replayable": true,
    "apply_contract": "append-only-owner-packet",
  })
}

fn svg_render_packet_json(previous: Option<&JsonValue>, next: &JsonValue) -> Result<JsonValue> {
  let sync = svg_sync_plan_json(previous, next);
  let html = svg_html_payload_from_sync_plan(&sync)?;

  Ok(serde_json::json!({
    "protocol": "pnix.render.packet.v1",
    "family": "svg",
    "process_api": true,
    "scene": sync.get("scene").cloned().unwrap_or_else(|| next.clone()),
    "sync": sync,
    "lowerings": {
      "html": html,
      "wgpu": {
        "status": "pending",
        "reason": "direct svg render packet exists, but wgpu lowerer is not implemented yet",
      },
    },
    "memory": render_memory_json("svg", "vector-2d-scene", &["svg", "html", "js", "ai"]),
    "interfaces": {
      "process_mode": "api-packet",
      "authoring_roles": ["scene", "vector-2d"],
      "runtime_roles": ["sync", "html", "replay"],
      "pending_roles": ["wgpu"],
    },
  }))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn svg_schema_normalize_accepts_xml_string() {
    let normalized = svg_schema_normalize(&Value::String(
      "<svg><a href=\"#demo\"/><circle cx=\"5\" cy=\"5\" r=\"2\"/></svg>".to_string(),
    ))
    .expect("normalize");

    let Value::AttrSet(root) = normalized else {
      panic!("normalized svg must be attrset");
    };
    let Some(Value::List(children)) = root.get("children") else {
      panic!("children missing");
    };
    assert!(children.iter().any(|child| {
      matches!(
        child,
        Value::AttrSet(node)
          if matches!(node.get("name"), Some(Value::String(name)) if name == "a")
            && matches!(node.get("attrs"), Some(Value::AttrSet(attrs)) if matches!(attrs.get("show"), Some(Value::String(show)) if show == "replace"))
      )
    }));
  }

  #[test]
  fn svg_json_attr_array_preserves_join_surface() {
    let value = JsonValue::Array(vec![
      JsonValue::String("0".to_string()),
      JsonValue::Number(serde_json::Number::from(10)),
      JsonValue::Null,
      JsonValue::Bool(true),
    ]);
    assert_eq!(svg_json_attr_string(&value).as_deref(), Some("0 10 true"));
  }

  #[test]
  fn svg_sync_index_builders_preserve_surface() {
    assert_eq!(svg_sync_node_id("root/svg", 12, None), "path:root/svg:12");
    assert_eq!(svg_sync_node_id("root/svg", 5, Some("dot")), "id:dot");
    assert_eq!(
      svg_pnix_address(Some("svg"), 7, "circle", None),
      "svg/circle:7"
    );
    assert_eq!(
      svg_pnix_address(Some("svg"), 7, "circle", Some("dot")),
      "svg/circle#dot"
    );
    assert_eq!(svg_pnix_address(None, 0, "svg", None), "svg");
  }

  #[test]
  fn svg_push_usize_decimal_preserves_surface() {
    let mut out = String::new();
    push_usize_decimal(0, &mut out);
    out.push(',');
    push_usize_decimal(42, &mut out);
    out.push(',');
    push_usize_decimal(usize::MAX, &mut out);
    assert_eq!(out, format!("0,42,{}", usize::MAX));
  }

  #[test]
  fn svg_schema_validate_reports_version_and_errors() {
    let report = svg_schema_validate(&Value::AttrSet(
      [
        ("kind".to_string(), Value::String("element".to_string())),
        ("name".to_string(), Value::String("svg".to_string())),
        (
          "attrs".to_string(),
          Value::AttrSet(Arc::new(
            [("version".to_string(), Value::String("3.0".to_string()))]
              .into_iter()
              .collect(),
          )),
        ),
        (
          "children".to_string(),
          Value::List(Arc::new(vec![Value::AttrSet(
            [
              ("kind".to_string(), Value::String("element".to_string())),
              ("name".to_string(), Value::String("circle".to_string())),
              (
                "attrs".to_string(),
                Value::AttrSet(Arc::new(
                  [("bogus".to_string(), Value::String("1".to_string()))]
                    .into_iter()
                    .collect(),
                )),
              ),
            ]
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .into(),
          )])),
        ),
      ]
      .into_iter()
      .collect::<BTreeMap<_, _>>()
      .into(),
    ))
    .expect("validate");

    let Value::AttrSet(map) = report else {
      panic!("validate report must be attrset");
    };
    assert!(matches!(map.get("ok"), Some(Value::Bool(false))));
    assert!(matches!(
      map.get("version"),
      Some(Value::String(version)) if version == "3.0"
    ));
    assert!(matches!(
      map.get("errors"),
      Some(Value::List(errors))
        if errors.iter().any(|err| matches!(err, Value::String(text) if text.contains("unsupported version '3.0'")))
    ));
  }

  #[test]
  fn svg_emit_roundtrips_normalized_svg() {
    let emitted = svg_emit(&Value::String(
      "<svg width=\"10\" height=\"10\"><circle id=\"dot\" cx=\"5\" cy=\"5\" r=\"2\"/></svg>"
        .to_string(),
    ))
    .expect("emit");
    assert!(emitted.contains("<svg"));
    assert!(emitted.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(emitted.contains("<circle"));
  }

  #[test]
  fn svg_render_packet_carries_html_lowering_and_memory() {
    let packet = svg_render_packet(
      &Value::String(
        "<svg width=\"10\" height=\"10\"><circle id=\"dot\" cx=\"5\" cy=\"5\" r=\"2\"/></svg>"
          .to_string(),
      ),
      &Value::String(
        "<svg width=\"10\" height=\"10\"><circle id=\"dot\" cx=\"7\" cy=\"5\" r=\"2\" fill=\"blue\"/></svg>"
          .to_string(),
      ),
    )
    .expect("packet");

    let Value::AttrSet(map) = packet else {
      panic!("svg render packet must be attrset");
    };
    assert!(matches!(
      map.get("protocol"),
      Some(Value::String(protocol)) if protocol == "pnix.render.packet.v1"
    ));
    assert!(matches!(
      map.get("family"),
      Some(Value::String(family)) if family == "svg"
    ));
    assert!(matches!(
      map.get("memory"),
      Some(Value::AttrSet(memory))
        if matches!(memory.get("protocol"), Some(Value::String(protocol)) if protocol == "pnix.world.memory.v1")
          && matches!(memory.get("surfaces"), Some(Value::List(surfaces)) if surfaces.iter().any(|item| matches!(item, Value::String(name) if name == "svg")))
    ));
    assert!(matches!(
      map.get("lowerings"),
      Some(Value::AttrSet(lowerings))
        if matches!(lowerings.get("html"), Some(Value::AttrSet(html)) if matches!(html.get("protocol"), Some(Value::String(protocol)) if protocol == "pnix.svg.html.v1"))
          && matches!(lowerings.get("wgpu"), Some(Value::AttrSet(wgpu)) if matches!(wgpu.get("status"), Some(Value::String(status)) if status == "pending"))
    ));
    let Some(Value::AttrSet(lowerings)) = map.get("lowerings") else {
      panic!("lowerings missing");
    };
    let Some(Value::AttrSet(html)) = lowerings.get("html") else {
      panic!("html lowering missing");
    };
    assert!(matches!(
      html.get("fragment"),
      Some(Value::String(fragment))
        if fragment.contains(r#"data-pnix-address="svg""#)
          && fragment.contains(r#"data-pnix-address="svg/circle#dot""#)
    ));
    assert!(matches!(
      html.get("ops"),
      Some(Value::List(ops))
        if ops.iter().any(|op| matches!(op, Value::AttrSet(attrs)
          if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "update_attrs")
            && matches!(attrs.get("target_pnix_address"), Some(Value::String(address)) if address == "svg/circle#dot")))
    ));
  }

  #[test]
  fn svg_render_packet_prefers_update_attrs_for_stable_node_ids() {
    let packet = svg_render_packet(
      &Value::String(
        "<svg width=\"10\" height=\"10\"><circle id=\"dot\" cx=\"5\" cy=\"5\" r=\"2\"/></svg>"
          .to_string(),
      ),
      &Value::String(
        "<svg width=\"10\" height=\"10\"><circle id=\"dot\" cx=\"7\" cy=\"5\" r=\"2\" fill=\"blue\"/></svg>"
          .to_string(),
      ),
    )
    .expect("packet");

    let Value::AttrSet(map) = packet else {
      panic!("svg render packet must be attrset");
    };
    let Some(Value::AttrSet(sync)) = map.get("sync") else {
      panic!("sync missing");
    };
    assert!(matches!(sync.get("mode"), Some(Value::String(mode)) if mode == "patch"));
    assert!(matches!(
      sync.get("ops"),
      Some(Value::List(ops))
        if ops.iter().any(|op| matches!(op, Value::AttrSet(attrs)
          if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "update_attrs")
            && matches!(attrs.get("node_id"), Some(Value::String(node_id)) if node_id == "id:dot")
            && matches!(attrs.get("target_pnix_address"), Some(Value::String(address)) if address == "svg/circle#dot")))
    ));
  }

  #[test]
  fn svg_render_packet_emits_child_delta_ops_for_stable_parent() {
    let packet = svg_render_packet(
      &Value::String(
        "<svg width=\"10\" height=\"10\"><circle id=\"a\" cx=\"1\" cy=\"1\" r=\"1\"/><circle id=\"b\" cx=\"2\" cy=\"2\" r=\"1\"/><circle id=\"d\" cx=\"3\" cy=\"3\" r=\"1\"/></svg>"
          .to_string(),
      ),
      &Value::String(
        "<svg width=\"10\" height=\"10\"><circle id=\"b\" cx=\"2\" cy=\"2\" r=\"1\" fill=\"blue\"/><circle id=\"a\" cx=\"1\" cy=\"1\" r=\"1\"/><rect id=\"c\" width=\"1\" height=\"1\"/></svg>"
          .to_string(),
      ),
    )
    .expect("packet");

    let Value::AttrSet(map) = packet else {
      panic!("svg render packet must be attrset");
    };
    let Some(Value::AttrSet(sync)) = map.get("sync") else {
      panic!("sync missing");
    };
    let Some(Value::List(ops)) = sync.get("ops") else {
      panic!("sync ops missing");
    };
    assert!(ops.iter().any(|op| matches!(op, Value::AttrSet(attrs)
      if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "remove_child")
        && matches!(attrs.get("node_pnix_address"), Some(Value::String(address)) if address == "svg")
        && matches!(attrs.get("child_node_id"), Some(Value::String(node_id)) if node_id == "id:d")
        && matches!(attrs.get("child_pnix_address"), Some(Value::String(address)) if address == "svg/circle#d"))));
    assert!(ops.iter().any(|op| matches!(op, Value::AttrSet(attrs)
      if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "insert_child")
        && matches!(attrs.get("node_pnix_address"), Some(Value::String(address)) if address == "svg")
        && matches!(attrs.get("child_node_id"), Some(Value::String(node_id)) if node_id == "id:c")
        && matches!(attrs.get("child_pnix_address"), Some(Value::String(address)) if address == "svg/rect#c"))));
    assert!(ops.iter().any(|op| matches!(op, Value::AttrSet(attrs)
      if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "reorder_children")
        && matches!(attrs.get("node_pnix_address"), Some(Value::String(address)) if address == "svg")
        && matches!(attrs.get("child_node_ids"), Some(Value::List(child_ids))
          if matches!(child_ids.as_slice(),
            [Value::String(first), Value::String(second), Value::String(third)]
              if first == "id:b" && second == "id:a" && third == "id:c")))));

    let Some(Value::AttrSet(lowerings)) = map.get("lowerings") else {
      panic!("lowerings missing");
    };
    let Some(Value::AttrSet(html)) = lowerings.get("html") else {
      panic!("html lowering missing");
    };
    let Some(Value::List(html_ops)) = html.get("ops") else {
      panic!("html ops missing");
    };
    assert!(html_ops.iter().any(|op| matches!(op, Value::AttrSet(attrs)
      if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "remove_child")
        && matches!(attrs.get("target_pnix_address"), Some(Value::String(address)) if address == "svg")
        && matches!(attrs.get("child_node_id"), Some(Value::String(node_id)) if node_id == "id:d")
        && matches!(attrs.get("child_pnix_address"), Some(Value::String(address)) if address == "svg/circle#d"))));
    assert!(html_ops.iter().any(|op| matches!(op, Value::AttrSet(attrs)
      if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "insert_child")
        && matches!(attrs.get("target_pnix_address"), Some(Value::String(address)) if address == "svg")
        && matches!(attrs.get("child_node_id"), Some(Value::String(node_id)) if node_id == "id:c")
        && matches!(attrs.get("child_pnix_address"), Some(Value::String(address)) if address == "svg/rect#c")
        && matches!(attrs.get("html"), Some(Value::String(fragment)) if fragment.contains("<rect")))));
    assert!(html_ops.iter().any(|op| matches!(op, Value::AttrSet(attrs)
      if matches!(attrs.get("op"), Some(Value::String(kind)) if kind == "reorder_children")
        && matches!(attrs.get("target_pnix_address"), Some(Value::String(address)) if address == "svg")
        && matches!(attrs.get("child_node_ids"), Some(Value::List(child_ids))
          if matches!(child_ids.as_slice(),
            [Value::String(first), Value::String(second), Value::String(third)]
              if first == "id:b" && second == "id:a" && third == "id:c")))));
  }
}
