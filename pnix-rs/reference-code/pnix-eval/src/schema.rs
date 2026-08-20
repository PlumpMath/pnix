use crate::Value;
use anyhow::{anyhow, Result};
use regex::RegexBuilder;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

const REGEX_SIZE_LIMIT_BYTES: usize = 1_000_000;
const REGEX_DFA_SIZE_LIMIT_BYTES: usize = 1_000_000;

fn anchored_regex_pattern(pattern: &str) -> String {
  let mut anchored = String::with_capacity(pattern.len() + "(?s)^(?:)$".len());
  anchored.push_str("(?s)^(?:");
  anchored.push_str(pattern);
  anchored.push_str(")$");
  anchored
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SchemaError {
  path: Vec<String>,
  code: &'static str,
  message: String,
}

#[derive(Debug, Clone, Default)]
struct SchemaContext {
  // V-2: shares the schema attrset Arc — context construction is a
  // refcount bump instead of a registry deep-clone.
  registry: Option<Arc<BTreeMap<String, Value>>>,
}

pub fn schema_validate(schema: &Value, value: &Value) -> Result<Value> {
  let errors = collect_errors(schema, value)?;
  let ok = errors.is_empty();

  let mut out = BTreeMap::new();
  out.insert("success".to_string(), Value::Bool(ok));
  out.insert("ok".to_string(), Value::Bool(ok));
  out.insert("errors".to_string(), errors_to_value(&errors));
  Ok(Value::AttrSet(Arc::new(out)))
}

pub fn schema_normalize(schema: &Value, value: &Value) -> Result<Value> {
  let (root, ctx) = resolve_schema_root(schema)?;
  normalize_schema(&ctx, &root, value)
}

pub fn schema_explain(schema: &Value, value: &Value) -> Result<Value> {
  let errors = collect_errors(schema, value)?;
  if errors.is_empty() {
    return Ok(Value::String(String::new()));
  }

  let mut out = String::new();
  for error in errors {
    if !out.is_empty() {
      out.push('\n');
    }
    append_path(&mut out, &error.path);
    let _ = write!(&mut out, ": {}: {}", error.code, error.message);
  }
  Ok(Value::String(out))
}

fn collect_errors(schema: &Value, value: &Value) -> Result<Vec<SchemaError>> {
  let (root, ctx) = resolve_schema_root(schema)?;
  let mut errors = validate_schema(&ctx, &root, value, &["root".to_string()])?;
  sort_errors(&mut errors);
  Ok(errors)
}

fn resolve_schema_root(schema: &Value) -> Result<(Value, SchemaContext)> {
  let Value::AttrSet(map) = schema else {
    return Err(anyhow!("schema.validate: schema must be an attrset"));
  };

  if matches!(map.get("kind"), Some(Value::String(_))) {
    let registry = match map.get("registry") {
      Some(Value::AttrSet(reg)) => Some(reg.clone()),
      Some(_) => {
        return Err(anyhow!(
          "schema.validate: schema registry must be an attrset"
        ))
      }
      None => None,
    };
    return Ok((schema.clone(), SchemaContext { registry }));
  }

  if let Some(root) = map.get("root") {
    return Ok((
      root.clone(),
      SchemaContext {
        registry: Some(map.clone()),
      },
    ));
  }

  Err(anyhow!("schema.validate: schema must define kind or root"))
}

fn validate_schema(
  ctx: &SchemaContext,
  schema: &Value,
  value: &Value,
  path: &[String],
) -> Result<Vec<SchemaError>> {
  let Value::AttrSet(map) = schema else {
    return Err(anyhow!(
      "schema.validate: schema must be an attrset at {}",
      path.join(".")
    ));
  };

  match schema_kind(map)? {
    // 2026-05-05 (slice #72): also accept Value::StringContext.
    // Slice #50 established that context-bearing strings ARE
    // strings. Pre-fix `schemaValidate {kind="string"} "x${./p}"`
    // falsely reported a type mismatch.
    "string" => match value {
      Value::String(_) | Value::StringContext { .. } => {
        validate_constraints(map, value, "string", path)
      }
      _ => Ok(vec![type_mismatch(path, "string", value)]),
    },
    "int" => match value {
      Value::Int(_) => validate_constraints(map, value, "int", path),
      _ => Ok(vec![type_mismatch(path, "int", value)]),
    },
    "float" => match value {
      Value::Float(_) => validate_constraints(map, value, "float", path),
      _ => Ok(vec![type_mismatch(path, "float", value)]),
    },
    "bool" => match value {
      Value::Bool(_) => validate_constraints(map, value, "bool", path),
      _ => Ok(vec![type_mismatch(path, "bool", value)]),
    },
    "null" => match value {
      Value::Null => Ok(vec![]),
      _ => Ok(vec![type_mismatch(path, "null", value)]),
    },
    "path" => match value {
      Value::Path(_) => Ok(vec![]),
      _ => Ok(vec![type_mismatch(path, "path", value)]),
    },
    "list" => validate_list(ctx, map, value, path),
    "map" => validate_map(ctx, map, value, path),
    "record" => validate_record(ctx, map, value, path),
    "oneOf" => validate_one_of(ctx, map, value, path),
    "allOf" => validate_all_of(ctx, map, value, path),
    "literal" => validate_literal(map, value, path),
    "ref" => validate_ref(ctx, map, value, path),
    "tagged" => validate_tagged(ctx, map, value, path),
    other => Err(anyhow!(
      "schema.validate: unsupported schema kind {}",
      other
    )),
  }
}

fn normalize_schema(ctx: &SchemaContext, schema: &Value, value: &Value) -> Result<Value> {
  let Value::AttrSet(map) = schema else {
    return Ok(value.clone());
  };

  match schema_kind(map)? {
    "list" => normalize_list(ctx, map, value),
    "map" => normalize_map(ctx, map, value),
    "record" => normalize_record(ctx, map, value),
    "oneOf" => normalize_one_of(ctx, map, value),
    "allOf" => normalize_all_of(ctx, map, value),
    "ref" => normalize_ref(ctx, map, value),
    "tagged" => normalize_tagged(ctx, map, value),
    _ => Ok(value.clone()),
  }
}

fn validate_list(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
  path: &[String],
) -> Result<Vec<SchemaError>> {
  let Value::List(items) = value else {
    return Ok(vec![type_mismatch(path, "list", value)]);
  };
  let elem_schema = map
    .get("elem")
    .ok_or_else(|| anyhow!("schema.validate: list schema missing elem"))?;

  let mut errors = Vec::with_capacity(items.len());
  validate_len_constraints(map, items.len(), path, &mut errors)?;
  for (idx, item) in items.iter().enumerate() {
    let mut next_path = path.to_vec();
    next_path.push(idx.to_string());
    errors.extend(validate_schema(ctx, elem_schema, item, &next_path)?);
  }
  Ok(errors)
}

fn validate_map(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
  path: &[String],
) -> Result<Vec<SchemaError>> {
  let Value::AttrSet(items) = value else {
    return Ok(vec![type_mismatch(path, "map", value)]);
  };
  let key_schema = map
    .get("key")
    .ok_or_else(|| anyhow!("schema.validate: map schema missing key"))?;
  let value_schema = map
    .get("value")
    .ok_or_else(|| anyhow!("schema.validate: map schema missing value"))?;

  let mut errors = Vec::with_capacity(items.len().saturating_mul(2));
  for (key, item) in items.iter() {
    let mut next_path = path.to_vec();
    next_path.push(key.clone());
    errors.extend(validate_schema(
      ctx,
      key_schema,
      &Value::String(key.clone()),
      &next_path,
    )?);
    errors.extend(validate_schema(ctx, value_schema, item, &next_path)?);
  }
  Ok(errors)
}

fn validate_record(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
  path: &[String],
) -> Result<Vec<SchemaError>> {
  let Value::AttrSet(items) = value else {
    return Ok(vec![type_mismatch(path, "record", value)]);
  };
  let fields = schema_fields(map)?;
  let optional = schema_optional_items(map)?;
  let open = schema_open(map);

  let mut errors = Vec::with_capacity(fields.len() + items.len());
  for (field, field_schema) in fields {
    if let Some(field_value) = items.get(field) {
      let mut next_path = path.to_vec();
      next_path.push(field.clone());
      errors.extend(validate_schema(ctx, field_schema, field_value, &next_path)?);
    } else if !schema_optional_contains(optional, field) {
      let mut next_path = path.to_vec();
      next_path.push(field.clone());
      errors.push(SchemaError {
        path: next_path,
        code: "missing-field",
        message: format!("missing required field '{}'", field),
      });
    }
  }

  if !open {
    for field in items.keys() {
      if !fields.contains_key(field) {
        let mut next_path = path.to_vec();
        next_path.push(field.clone());
        errors.push(SchemaError {
          path: next_path,
          code: "unknown-field",
          message: format!("unknown field '{}'", field),
        });
      }
    }
  }

  Ok(errors)
}

fn validate_one_of(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
  path: &[String],
) -> Result<Vec<SchemaError>> {
  let variants = schema_variants(map)?;
  if variants.is_empty() {
    return Ok(vec![SchemaError {
      path: path.to_vec(),
      code: "constraint",
      message: "oneOf has no variants".to_string(),
    }]);
  }

  let mut best_errors: Option<Vec<SchemaError>> = None;
  for variant in variants {
    let errors = validate_schema(ctx, variant, value, path)?;
    if errors.is_empty() {
      return Ok(Vec::new());
    }
    if best_errors
      .as_ref()
      .map(|best| errors.len() < best.len())
      .unwrap_or(true)
    {
      best_errors = Some(errors);
    }
  }

  Ok(best_errors.unwrap_or_default())
}

fn validate_all_of(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
  path: &[String],
) -> Result<Vec<SchemaError>> {
  let variants = schema_variants(map)?;
  let mut errors = Vec::with_capacity(variants.len());
  for variant in variants {
    errors.extend(validate_schema(ctx, variant, value, path)?);
  }
  Ok(errors)
}

fn validate_literal(
  map: &BTreeMap<String, Value>,
  value: &Value,
  path: &[String],
) -> Result<Vec<SchemaError>> {
  let literal = map
    .get("value")
    .ok_or_else(|| anyhow!("schema.validate: literal schema missing value"))?;
  if schema_value_eq(literal, value) {
    Ok(Vec::new())
  } else {
    Ok(vec![SchemaError {
      path: path.to_vec(),
      code: "constraint",
      message: format!("expected literal {}", display_value(literal)),
    }])
  }
}

fn validate_ref(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
  path: &[String],
) -> Result<Vec<SchemaError>> {
  let Some(name) = schema_attr_string(map, "name")? else {
    return Err(anyhow!("schema.validate: ref schema missing name"));
  };
  let Some(registry) = &ctx.registry else {
    return Err(anyhow!("schema.validate: ref '{}' without registry", name));
  };
  let Some(target) = registry.get(name) else {
    return Err(anyhow!("schema.validate: unknown ref '{}'", name));
  };
  validate_schema(ctx, target, value, path)
}

fn validate_tagged(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
  path: &[String],
) -> Result<Vec<SchemaError>> {
  let Value::AttrSet(items) = value else {
    return Ok(vec![type_mismatch(path, "record", value)]);
  };
  let Some(tag_key) = schema_attr_string(map, "tag")? else {
    return Err(anyhow!("schema.validate: tagged schema missing tag"));
  };
  let variants = schema_variant_map(map)?;

  let Some(tag_value) = items.get(tag_key) else {
    let mut next_path = path.to_vec();
    next_path.push(tag_key.to_string());
    return Ok(vec![SchemaError {
      path: next_path,
      code: "missing-field",
      message: format!("missing required field '{}'", tag_key),
    }]);
  };
  let Value::String(tag_name) = tag_value else {
    let mut next_path = path.to_vec();
    next_path.push(tag_key.to_string());
    return Ok(vec![type_mismatch(&next_path, "string", tag_value)]);
  };
  let Some(variant) = variants.get(tag_name) else {
    let mut next_path = path.to_vec();
    next_path.push(tag_key.to_string());
    return Ok(vec![SchemaError {
      path: next_path,
      code: "constraint",
      message: format!("unknown tag '{}'", tag_name),
    }]);
  };

  validate_schema(ctx, variant, value, path)
}

fn validate_constraints(
  map: &BTreeMap<String, Value>,
  value: &Value,
  kind: &str,
  path: &[String],
) -> Result<Vec<SchemaError>> {
  let mut errors = Vec::new();

  if let Some(enum_values) = schema_enum(map)? {
    let default_value = map.get("default");
    let default_in_enum = default_value
      .map(|default_value| {
        enum_values
          .iter()
          .any(|candidate| schema_value_eq(candidate, default_value))
      })
      .unwrap_or(false);
    let value_matches_enum = enum_values
      .iter()
      .any(|candidate| schema_value_eq(candidate, value));
    let value_matches_default = default_value
      .map(|default_value| schema_value_eq(default_value, value))
      .unwrap_or(false);
    if !value_matches_enum && !value_matches_default {
      let mut expected = String::new();
      for (index, candidate) in enum_values.iter().enumerate() {
        if index > 0 {
          expected.push_str(", ");
        }
        expected.push_str(&display_value(candidate));
      }
      if let Some(default_value) = default_value.filter(|_| !default_in_enum) {
        if !expected.is_empty() {
          expected.push_str(", ");
        }
        expected.push_str(&display_value(default_value));
      }
      errors.push(SchemaError {
        path: path.to_vec(),
        code: "constraint",
        message: format!("expected one of [{}]", expected),
      });
    }
  }

  if kind == "string" {
    if let Some(pattern) = schema_attr_string(map, "pattern")? {
      let Value::String(text) = value else {
        return Ok(errors);
      };
      let re = RegexBuilder::new(&anchored_regex_pattern(pattern))
        .size_limit(REGEX_SIZE_LIMIT_BYTES)
        .dfa_size_limit(REGEX_DFA_SIZE_LIMIT_BYTES)
        .build()
        .map_err(|err| anyhow!("schema.validate: invalid regex: {}", err))?;
      if !re.is_match(text) {
        errors.push(SchemaError {
          path: path.to_vec(),
          code: "constraint",
          message: format!("string does not match pattern {}", pattern),
        });
      }
    }
    validate_len_constraints(map, value_string_len(value)?, path, &mut errors)?;
  }

  if kind == "int" {
    let Value::Int(number) = value else {
      return Ok(errors);
    };
    if let Some(min) = schema_number_int(map, "min")? {
      if *number < min {
        errors.push(SchemaError {
          path: path.to_vec(),
          code: "constraint",
          message: format!("expected min {}, got {}", min, number),
        });
      }
    }
    if let Some(max) = schema_number_int(map, "max")? {
      if *number > max {
        errors.push(SchemaError {
          path: path.to_vec(),
          code: "constraint",
          message: format!("expected max {}, got {}", max, number),
        });
      }
    }
  }

  if kind == "float" {
    let Value::Float(number) = value else {
      return Ok(errors);
    };
    if let Some(min) = schema_number_float(map, "min")? {
      if *number < min {
        errors.push(SchemaError {
          path: path.to_vec(),
          code: "constraint",
          message: format!("expected min {}, got {}", min, number),
        });
      }
    }
    if let Some(max) = schema_number_float(map, "max")? {
      if *number > max {
        errors.push(SchemaError {
          path: path.to_vec(),
          code: "constraint",
          message: format!("expected max {}, got {}", max, number),
        });
      }
    }
  }

  Ok(errors)
}

fn normalize_list(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
) -> Result<Value> {
  let Value::List(items) = value else {
    return Ok(value.clone());
  };
  let Some(elem_schema) = map.get("elem") else {
    return Ok(value.clone());
  };
  let mut out = Vec::with_capacity(items.len());
  for item in items.iter() {
    out.push(normalize_schema(ctx, elem_schema, item)?);
  }
  Ok(Value::List(Arc::new(out)))
}

fn normalize_map(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
) -> Result<Value> {
  let Value::AttrSet(items) = value else {
    return Ok(value.clone());
  };
  let Some(value_schema) = map.get("value") else {
    return Ok(value.clone());
  };
  let mut out = BTreeMap::new();
  for (key, item) in items.iter() {
    out.insert(key.clone(), normalize_schema(ctx, value_schema, item)?);
  }
  Ok(Value::AttrSet(Arc::new(out)))
}

fn normalize_record(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
) -> Result<Value> {
  let Value::AttrSet(items) = value else {
    return Ok(value.clone());
  };
  let fields = schema_fields(map)?;
  let mut out = BTreeMap::new();

  for (key, item) in items.iter() {
    if let Some(field_schema) = fields.get(key) {
      out.insert(key.clone(), normalize_schema(ctx, field_schema, item)?);
    } else {
      out.insert(key.clone(), item.clone());
    }
  }

  for (key, field_schema) in fields {
    if out.contains_key(key) {
      continue;
    }
    if let Some(default_value) = schema_default(field_schema)? {
      out.insert(
        key.clone(),
        normalize_schema(ctx, field_schema, default_value)?,
      );
    }
  }

  Ok(Value::AttrSet(Arc::new(out)))
}

fn normalize_one_of(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
) -> Result<Value> {
  for variant in schema_variants(map)? {
    if validate_schema(ctx, variant, value, &["root".to_string()])?.is_empty() {
      return normalize_schema(ctx, variant, value);
    }
  }
  Ok(value.clone())
}

fn normalize_all_of(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
) -> Result<Value> {
  let mut current = value.clone();
  for variant in schema_variants(map)? {
    current = normalize_schema(ctx, variant, &current)?;
  }
  Ok(current)
}

fn normalize_ref(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
) -> Result<Value> {
  let Some(name) = schema_attr_string(map, "name")? else {
    return Ok(value.clone());
  };
  let Some(registry) = &ctx.registry else {
    return Ok(value.clone());
  };
  let Some(target) = registry.get(name) else {
    return Ok(value.clone());
  };
  normalize_schema(ctx, target, value)
}

fn normalize_tagged(
  ctx: &SchemaContext,
  map: &BTreeMap<String, Value>,
  value: &Value,
) -> Result<Value> {
  let Value::AttrSet(items) = value else {
    return Ok(value.clone());
  };
  let Some(tag_key) = schema_attr_string(map, "tag")? else {
    return Ok(value.clone());
  };
  let Some(Value::String(tag_name)) = items.get(tag_key) else {
    return Ok(value.clone());
  };
  let variants = schema_variant_map(map)?;
  let Some(variant) = variants.get(tag_name) else {
    return Ok(value.clone());
  };
  normalize_schema(ctx, variant, value)
}

fn schema_kind<'a>(map: &'a BTreeMap<String, Value>) -> Result<&'a str> {
  match map.get("kind") {
    Some(Value::String(kind)) => Ok(kind.as_str()),
    Some(_) => Err(anyhow!("schema.validate: kind must be string")),
    None => Err(anyhow!("schema.validate: missing kind")),
  }
}

fn schema_attr_string<'a>(map: &'a BTreeMap<String, Value>, key: &str) -> Result<Option<&'a str>> {
  match map.get(key) {
    None => Ok(None),
    Some(Value::String(value)) => Ok(Some(value.as_str())),
    Some(_) => Err(anyhow!("schema.validate: {} must be string", key)),
  }
}

fn schema_len(map: &BTreeMap<String, Value>, key: &str) -> Result<Option<usize>> {
  match map.get(key) {
    None => Ok(None),
    Some(Value::Int(value)) if *value >= 0 => Ok(Some(*value as usize)),
    Some(Value::Int(_)) => Err(anyhow!("schema.validate: {} must be non-negative", key)),
    Some(_) => Err(anyhow!("schema.validate: {} must be int", key)),
  }
}

fn schema_number_int(map: &BTreeMap<String, Value>, key: &str) -> Result<Option<i64>> {
  match map.get(key) {
    None => Ok(None),
    Some(Value::Int(value)) => Ok(Some(*value)),
    Some(_) => Err(anyhow!("schema.validate: {} must be int", key)),
  }
}

fn schema_number_float(map: &BTreeMap<String, Value>, key: &str) -> Result<Option<f64>> {
  match map.get(key) {
    None => Ok(None),
    Some(Value::Float(value)) => Ok(Some(*value)),
    Some(Value::Int(value)) => Ok(Some(*value as f64)),
    Some(_) => Err(anyhow!("schema.validate: {} must be float", key)),
  }
}

fn schema_enum(map: &BTreeMap<String, Value>) -> Result<Option<&[Value]>> {
  match map.get("enum") {
    None => Ok(None),
    Some(Value::List(items)) => Ok(Some(items)),
    Some(_) => Err(anyhow!("schema.validate: enum must be a list")),
  }
}

fn schema_fields(map: &BTreeMap<String, Value>) -> Result<&BTreeMap<String, Value>> {
  match map.get("fields") {
    Some(Value::AttrSet(fields)) => Ok(fields),
    Some(_) => Err(anyhow!("schema.validate: record fields must be attrset")),
    None => Err(anyhow!("schema.validate: record schema missing fields")),
  }
}

fn schema_variants(map: &BTreeMap<String, Value>) -> Result<&[Value]> {
  match map.get("variants") {
    Some(Value::List(variants)) => Ok(variants),
    Some(_) => Err(anyhow!("schema.validate: variants must be a list")),
    None => Err(anyhow!("schema.validate: variants missing")),
  }
}

fn schema_variant_map(map: &BTreeMap<String, Value>) -> Result<&BTreeMap<String, Value>> {
  match map.get("variants") {
    Some(Value::AttrSet(variants)) => Ok(variants),
    Some(_) => Err(anyhow!("schema.validate: tagged variants must be attrset")),
    None => Err(anyhow!("schema.validate: variants missing")),
  }
}

fn schema_default(schema: &Value) -> Result<Option<&Value>> {
  let Value::AttrSet(map) = schema else {
    return Ok(None);
  };
  Ok(map.get("default"))
}

fn schema_optional_items(map: &BTreeMap<String, Value>) -> Result<&[Value]> {
  let Some(value) = map.get("optional") else {
    return Ok(&[]);
  };
  let Value::List(items) = value else {
    return Err(anyhow!(
      "schema.validate: optional must be a list of strings"
    ));
  };
  for item in items.iter() {
    let Value::String(_) = item else {
      return Err(anyhow!(
        "schema.validate: optional must be a list of strings"
      ));
    };
  }
  Ok(items)
}

fn schema_optional_contains(items: &[Value], field: &str) -> bool {
  items
    .iter()
    .any(|item| matches!(item, Value::String(name) if name == field))
}

fn schema_open(map: &BTreeMap<String, Value>) -> bool {
  matches!(map.get("open"), Some(Value::Bool(true)))
}

fn validate_len_constraints(
  map: &BTreeMap<String, Value>,
  len: usize,
  path: &[String],
  errors: &mut Vec<SchemaError>,
) -> Result<()> {
  if let Some(expected) = schema_len(map, "len")? {
    if len != expected {
      errors.push(SchemaError {
        path: path.to_vec(),
        code: "constraint",
        message: format!("expected length {}, got {}", expected, len),
      });
    }
  }
  if let Some(min_len) = schema_len(map, "minLen")? {
    if len < min_len {
      errors.push(SchemaError {
        path: path.to_vec(),
        code: "constraint",
        message: format!("expected min length {}, got {}", min_len, len),
      });
    }
  }
  if let Some(max_len) = schema_len(map, "maxLen")? {
    if len > max_len {
      errors.push(SchemaError {
        path: path.to_vec(),
        code: "constraint",
        message: format!("expected max length {}, got {}", max_len, len),
      });
    }
  }
  Ok(())
}

// 2026-05-05 (slice #72): accept context-bearing strings.
fn value_string_len(value: &Value) -> Result<usize> {
  match value {
    Value::String(text) => Ok(text.chars().count()),
    Value::StringContext { text, .. } => Ok(text.chars().count()),
    _ => Err(anyhow!("schema.validate: expected string")),
  }
}

fn errors_to_value(errors: &[SchemaError]) -> Value {
  let mut out = Vec::with_capacity(errors.len());
  for error in errors {
    let mut map = BTreeMap::new();
    map.insert("path".to_string(), path_to_value(&error.path));
    map.insert("code".to_string(), Value::String(error.code.to_string()));
    map.insert("message".to_string(), Value::String(error.message.clone()));
    out.push(Value::AttrSet(Arc::new(map)));
  }
  Value::List(Arc::new(out))
}

fn path_to_value(path: &[String]) -> Value {
  let mut out = Vec::with_capacity(path.len());
  for part in path {
    out.push(Value::String(part.clone()));
  }
  Value::List(Arc::new(out))
}

fn append_path(out: &mut String, path: &[String]) {
  for (index, part) in path.iter().enumerate() {
    if index > 0 {
      out.push('.');
    }
    out.push_str(part);
  }
}

fn type_mismatch(path: &[String], expected: &str, value: &Value) -> SchemaError {
  SchemaError {
    path: path.to_vec(),
    code: "type",
    message: format!("expected {}, got {}", expected, value_type_name(value)),
  }
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

fn schema_value_eq(lhs: &Value, rhs: &Value) -> bool {
  match (lhs, rhs) {
    (Value::Null, Value::Null) => true,
    (Value::Bool(a), Value::Bool(b)) => a == b,
    (Value::Int(a), Value::Int(b)) => a == b,
    (Value::Float(a), Value::Float(b)) => a == b,
    (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => (*a as f64) == *b,
    (Value::String(a), Value::String(b)) => a == b,
    (Value::Path(a), Value::Path(b)) => a == b,
    (Value::List(a), Value::List(b)) => {
      a.len() == b.len()
        && a
          .iter()
          .zip(b.iter())
          .all(|(left, right)| schema_value_eq(left, right))
    }
    (Value::AttrSet(a), Value::AttrSet(b)) => {
      a.len() == b.len()
        && a.iter().all(|(key, left)| {
          b.get(key)
            .map(|right| schema_value_eq(left, right))
            .unwrap_or(false)
        })
    }
    _ => false,
  }
}

fn display_value(value: &Value) -> String {
  format!("{}", value)
}

fn sort_errors(errors: &mut [SchemaError]) {
  errors.sort_by(|lhs, rhs| {
    let path_cmp = lhs.path.cmp(&rhs.path);
    if path_cmp != Ordering::Equal {
      return path_cmp;
    }
    let code_cmp = lhs.code.cmp(rhs.code);
    if code_cmp != Ordering::Equal {
      return code_cmp;
    }
    lhs.message.cmp(&rhs.message)
  });
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn validates_record_and_optional_fields() {
    let schema = Value::AttrSet(Arc::new(BTreeMap::from([
      ("kind".to_string(), Value::String("record".to_string())),
      (
        "fields".to_string(),
        Value::AttrSet(Arc::new(BTreeMap::from([
          (
            "name".to_string(),
            Value::AttrSet(Arc::new(BTreeMap::from([(
              "kind".to_string(),
              Value::String("string".to_string()),
            )]))),
          ),
          (
            "count".to_string(),
            Value::AttrSet(Arc::new(BTreeMap::from([(
              "kind".to_string(),
              Value::String("int".to_string()),
            )]))),
          ),
        ]))),
      ),
      (
        "optional".to_string(),
        Value::List(Arc::new(vec![Value::String("count".to_string())])),
      ),
    ])));

    let report = schema_validate(
      &schema,
      &Value::AttrSet(Arc::new(BTreeMap::from([(
        "name".to_string(),
        Value::String("ok".to_string()),
      )]))),
    )
    .expect("validate");

    match report {
      Value::AttrSet(map) => {
        assert!(matches!(map.get("ok"), Some(Value::Bool(true))));
      }
      other => panic!("expected report attrset, got {:?}", other),
    }
  }

  #[test]
  fn normalize_adds_defaults() {
    let field_schema = Value::AttrSet(Arc::new(BTreeMap::from([
      ("kind".to_string(), Value::String("string".to_string())),
      ("default".to_string(), Value::String("fallback".to_string())),
    ])));
    let schema = Value::AttrSet(Arc::new(BTreeMap::from([
      ("kind".to_string(), Value::String("record".to_string())),
      (
        "fields".to_string(),
        Value::AttrSet(Arc::new(BTreeMap::from([(
          "name".to_string(),
          field_schema,
        )]))),
      ),
    ])));

    let normalized =
      schema_normalize(&schema, &Value::AttrSet(Arc::new(BTreeMap::new()))).expect("norm");
    match normalized {
      Value::AttrSet(map) => {
        assert_eq!(map.get("name").and_then(Value::as_str), Some("fallback"));
      }
      other => panic!("expected attrset, got {:?}", other),
    }
  }
}
