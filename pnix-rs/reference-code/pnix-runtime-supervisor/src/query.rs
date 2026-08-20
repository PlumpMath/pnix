use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::cmp::Ordering;

const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 500;
const MAX_ORDER_BY: usize = 2;
const MAX_PREDICATES: usize = 50;
const MAX_DEPTH: usize = 10;

const DEFAULT_SELECT: &[&str] = &[
  "ns",
  "id",
  "status",
  "pid",
  "generation",
  "base_url",
  "desired_present",
  "fail_count",
  "paused",
];

#[derive(Debug, Clone, Deserialize, Default)]
pub struct QueryOrderBy {
  pub field: String,
  #[serde(default)]
  pub dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProcessQueryRequest {
  #[serde(default, rename = "where")]
  pub where_clause: Option<Value>,
  #[serde(default)]
  pub order_by: Vec<QueryOrderBy>,
  #[serde(default)]
  pub limit: Option<usize>,
  #[serde(default)]
  pub select: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProcessWatchRequest {
  #[serde(default)]
  pub cursor: Option<u64>,
  #[serde(default)]
  pub timeout_ms: Option<u64>,
  #[serde(default)]
  pub max: Option<usize>,
  #[serde(default)]
  pub include_object: Option<bool>,
  #[serde(default, rename = "where")]
  pub where_clause: Option<Value>,
  #[serde(default)]
  pub order_by: Vec<QueryOrderBy>,
  #[serde(default)]
  pub select: Vec<String>,
}

pub fn clamp_limit(limit: Option<usize>) -> usize {
  limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

pub fn clamp_watch_max(max: Option<usize>) -> usize {
  max.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

pub fn normalize_select(select: &[String]) -> Result<Vec<String>> {
  if select.is_empty() {
    return Ok(DEFAULT_SELECT.iter().map(|f| (*f).to_string()).collect());
  }
  let mut out = Vec::with_capacity(select.len());
  for field in select {
    if !is_allowed_field(field.as_str()) {
      anyhow::bail!("query.select includes unsupported field `{}`", field);
    }
    if !out.iter().any(|existing: &String| existing == field) {
      out.push(field.clone());
    }
  }
  Ok(out)
}

pub fn validate_query(request: &ProcessQueryRequest) -> Result<()> {
  if request.order_by.len() > MAX_ORDER_BY {
    anyhow::bail!("query.order_by supports at most {} fields", MAX_ORDER_BY);
  }
  for order in &request.order_by {
    if !is_allowed_field(order.field.as_str()) {
      anyhow::bail!(
        "query.order_by includes unsupported field `{}`",
        order.field
      );
    }
    if let Some(dir) = order.dir.as_deref() {
      let normalized = dir.to_ascii_lowercase();
      if normalized != "asc" && normalized != "desc" {
        anyhow::bail!("query.order_by direction must be asc|desc");
      }
    }
  }
  validate_where_expression(request.where_clause.as_ref())?;
  let _ = normalize_select(&request.select)?;
  Ok(())
}

pub fn apply_query_rows(
  mut rows: Vec<Value>,
  request: &ProcessQueryRequest,
  id_prefixes: &[String],
) -> Result<Vec<Value>> {
  validate_query(request)?;
  let select = normalize_select(&request.select)?;
  rows = rows
    .into_iter()
    .filter(|row| row_matches(row, request.where_clause.as_ref(), id_prefixes).unwrap_or(false))
    .collect();

  if !request.order_by.is_empty() {
    rows.sort_by(|a, b| compare_rows(a, b, &request.order_by));
  } else {
    rows.sort_by(|a, b| {
      let aid = field_value(a, "id").and_then(Value::as_str).unwrap_or("");
      let bid = field_value(b, "id").and_then(Value::as_str).unwrap_or("");
      aid.cmp(bid)
    });
  }

  let limit = clamp_limit(request.limit);
  if rows.len() > limit {
    rows.truncate(limit);
  }

  rows
    .into_iter()
    .map(|row| project_row(&row, &select))
    .collect::<Result<Vec<_>>>()
}

pub fn watch_query_from_request(request: &ProcessWatchRequest) -> ProcessQueryRequest {
  ProcessQueryRequest {
    where_clause: request.where_clause.clone(),
    order_by: request.order_by.clone(),
    limit: None,
    select: request.select.clone(),
  }
}

pub fn project_row(row: &Value, select: &[String]) -> Result<Value> {
  let obj = row
    .as_object()
    .context("process row must be object for projection")?;
  let mut out = Map::new();
  for field in select {
    if !is_allowed_field(field.as_str()) {
      anyhow::bail!("unsupported projection field `{}`", field);
    }
    if let Some(value) = obj.get(field) {
      out.insert(field.clone(), value.clone());
    } else {
      out.insert(field.clone(), Value::Null);
    }
  }
  Ok(Value::Object(out))
}

pub fn row_matches(
  row: &Value,
  where_clause: Option<&Value>,
  id_prefixes: &[String],
) -> Result<bool> {
  if !id_prefixes.is_empty() {
    let Some(id) = field_value(row, "id").and_then(Value::as_str) else {
      return Ok(false);
    };
    if !id_prefixes.iter().any(|prefix| id.starts_with(prefix)) {
      return Ok(false);
    }
  }
  let Some(expr) = where_clause else {
    return Ok(true);
  };
  let mut count = 0_usize;
  eval_expr(expr, row, 0, &mut count)
}

fn validate_where_expression(where_clause: Option<&Value>) -> Result<()> {
  let Some(expr) = where_clause else {
    return Ok(());
  };
  let mut count = 0_usize;
  validate_expr(expr, 0, &mut count)
}

fn validate_expr(expr: &Value, depth: usize, count: &mut usize) -> Result<()> {
  if depth > MAX_DEPTH {
    anyhow::bail!("query.where maximum depth exceeded ({})", MAX_DEPTH);
  }
  *count = count.saturating_add(1);
  if *count > MAX_PREDICATES {
    anyhow::bail!(
      "query.where maximum predicate count exceeded ({})",
      MAX_PREDICATES
    );
  }
  let obj = expr
    .as_object()
    .context("query.where expression must be object")?;
  if let Some(and) = obj.get("and") {
    let arr = and
      .as_array()
      .context("query.where.and must be an array expression")?;
    for child in arr {
      validate_expr(child, depth + 1, count)?;
    }
    return Ok(());
  }
  if let Some(or) = obj.get("or") {
    let arr = or
      .as_array()
      .context("query.where.or must be an array expression")?;
    for child in arr {
      validate_expr(child, depth + 1, count)?;
    }
    return Ok(());
  }
  if let Some(not) = obj.get("not") {
    validate_expr(not, depth + 1, count)?;
    return Ok(());
  }
  if let Some(field) = obj.get("field").and_then(Value::as_str) {
    if !is_allowed_field(field) {
      anyhow::bail!("query.where uses unsupported field `{}`", field);
    }
    let op = obj
      .get("op")
      .and_then(Value::as_str)
      .unwrap_or("=")
      .to_ascii_lowercase();
    if !is_allowed_op(op.as_str()) {
      anyhow::bail!("query.where uses unsupported op `{}`", op);
    }
    return Ok(());
  }
  if let Some(label) = obj.get("label") {
    let lobj = label
      .as_object()
      .context("query.where.label must be object")?;
    let _ = lobj
      .get("k")
      .and_then(Value::as_str)
      .context("query.where.label.k is required")?;
    let op = lobj
      .get("op")
      .and_then(Value::as_str)
      .unwrap_or("=")
      .to_ascii_lowercase();
    if !matches!(op.as_str(), "=" | "!=" | "in" | "exists" | "prefix") {
      anyhow::bail!("query.where.label uses unsupported op `{}`", op);
    }
    return Ok(());
  }
  anyhow::bail!("query.where expression is not recognized")
}

fn eval_expr(expr: &Value, row: &Value, depth: usize, count: &mut usize) -> Result<bool> {
  if depth > MAX_DEPTH {
    anyhow::bail!("query.where maximum depth exceeded ({})", MAX_DEPTH);
  }
  *count = count.saturating_add(1);
  if *count > MAX_PREDICATES {
    anyhow::bail!(
      "query.where maximum predicate count exceeded ({})",
      MAX_PREDICATES
    );
  }
  let obj = expr
    .as_object()
    .context("query.where expression must be object")?;

  if let Some(and) = obj.get("and") {
    let arr = and
      .as_array()
      .context("query.where.and must be an array expression")?;
    for child in arr {
      if !eval_expr(child, row, depth + 1, count)? {
        return Ok(false);
      }
    }
    return Ok(true);
  }
  if let Some(or) = obj.get("or") {
    let arr = or
      .as_array()
      .context("query.where.or must be an array expression")?;
    for child in arr {
      if eval_expr(child, row, depth + 1, count)? {
        return Ok(true);
      }
    }
    return Ok(false);
  }
  if let Some(not) = obj.get("not") {
    return Ok(!eval_expr(not, row, depth + 1, count)?);
  }
  if let Some(field) = obj.get("field").and_then(Value::as_str) {
    let op = obj
      .get("op")
      .and_then(Value::as_str)
      .unwrap_or("=")
      .to_ascii_lowercase();
    let left = field_value(row, field).unwrap_or(&Value::Null);
    return eval_condition(left, op.as_str(), obj);
  }
  if let Some(label) = obj.get("label") {
    return eval_label_condition(row, label);
  }
  anyhow::bail!("query.where expression is not recognized")
}

fn eval_label_condition(row: &Value, label: &Value) -> Result<bool> {
  let lobj = label
    .as_object()
    .context("query.where.label must be object")?;
  let key = lobj
    .get("k")
    .and_then(Value::as_str)
    .context("query.where.label.k is required")?;
  let op = lobj
    .get("op")
    .and_then(Value::as_str)
    .unwrap_or("=")
    .to_ascii_lowercase();
  let current = field_value(row, "labels")
    .and_then(Value::as_object)
    .and_then(|labels| labels.get(key))
    .cloned()
    .unwrap_or(Value::Null);
  let mut proxy = Map::new();
  proxy.insert("op".to_string(), Value::String(op));
  if let Some(value) = lobj.get("v") {
    proxy.insert("value".to_string(), value.clone());
  }
  if let Some(values) = lobj.get("values") {
    proxy.insert("values".to_string(), values.clone());
  }
  eval_condition(&current, proxy["op"].as_str().unwrap_or("="), &proxy)
}

fn eval_condition(left: &Value, op: &str, obj: &Map<String, Value>) -> Result<bool> {
  match op {
    "=" => Ok(match obj.get("value") {
      Some(value) => left == value,
      None => false,
    }),
    "!=" => Ok(match obj.get("value") {
      Some(value) => left != value,
      None => true,
    }),
    ">" | ">=" | "<" | "<=" => {
      let right = obj
        .get("value")
        .context("query.where comparison op requires `value`")?;
      let ordering = compare_scalar(left, right)
        .with_context(|| format!("cannot compare values for op `{}`", op))?;
      Ok(match op {
        ">" => ordering == Ordering::Greater,
        ">=" => ordering == Ordering::Greater || ordering == Ordering::Equal,
        "<" => ordering == Ordering::Less,
        "<=" => ordering == Ordering::Less || ordering == Ordering::Equal,
        _ => false,
      })
    }
    "prefix" => {
      let right = obj
        .get("value")
        .and_then(Value::as_str)
        .context("query.where prefix op requires string `value`")?;
      let left = left
        .as_str()
        .context("query.where prefix op requires string field")?;
      Ok(left.starts_with(right))
    }
    "in" => {
      let values = if let Some(values) = obj.get("values") {
        values
      } else {
        obj
          .get("value")
          .context("query.where in op requires `value` or `values`")?
      };
      let arr = values
        .as_array()
        .context("query.where in op requires array value")?;
      Ok(arr.iter().any(|candidate| candidate == left))
    }
    "between" => {
      let values = obj
        .get("value")
        .context("query.where between op requires `value` [min,max]")?;
      let arr = values
        .as_array()
        .context("query.where between op requires array value")?;
      if arr.len() != 2 {
        anyhow::bail!("query.where between op requires exactly 2 values");
      }
      let lower = compare_scalar(left, &arr[0]).context("between lower bound type mismatch")?;
      let upper = compare_scalar(left, &arr[1]).context("between upper bound type mismatch")?;
      Ok(
        (lower == Ordering::Greater || lower == Ordering::Equal)
          && (upper == Ordering::Less || upper == Ordering::Equal),
      )
    }
    "exists" => {
      let expected = obj.get("value").and_then(Value::as_bool).unwrap_or(true);
      Ok((!left.is_null()) == expected)
    }
    other => anyhow::bail!("unsupported operator `{}`", other),
  }
}

fn compare_rows(left: &Value, right: &Value, order_by: &[QueryOrderBy]) -> Ordering {
  for order in order_by {
    let a = field_value(left, order.field.as_str()).unwrap_or(&Value::Null);
    let b = field_value(right, order.field.as_str()).unwrap_or(&Value::Null);
    let mut cmp = compare_sort_value(a, b);
    if let Some(dir) = order.dir.as_deref() {
      if dir.eq_ignore_ascii_case("desc") {
        cmp = cmp.reverse();
      }
    }
    if cmp != Ordering::Equal {
      return cmp;
    }
  }
  let aid = field_value(left, "id")
    .and_then(Value::as_str)
    .unwrap_or("");
  let bid = field_value(right, "id")
    .and_then(Value::as_str)
    .unwrap_or("");
  aid.cmp(bid)
}

fn compare_sort_value(a: &Value, b: &Value) -> Ordering {
  if a.is_null() && b.is_null() {
    return Ordering::Equal;
  }
  if a.is_null() {
    return Ordering::Greater;
  }
  if b.is_null() {
    return Ordering::Less;
  }
  if let Some(ordering) = compare_scalar(a, b) {
    return ordering;
  }
  a.to_string().cmp(&b.to_string())
}

fn compare_scalar(a: &Value, b: &Value) -> Option<Ordering> {
  match (a, b) {
    (Value::Number(an), Value::Number(bn)) => {
      let af = an.as_f64()?;
      let bf = bn.as_f64()?;
      af.partial_cmp(&bf)
    }
    (Value::String(as_), Value::String(bs_)) => Some(as_.cmp(bs_)),
    (Value::Bool(ab), Value::Bool(bb)) => Some(ab.cmp(bb)),
    _ => None,
  }
}

fn field_value<'a>(row: &'a Value, field: &str) -> Option<&'a Value> {
  row.as_object().and_then(|obj| obj.get(field))
}

fn is_allowed_op(op: &str) -> bool {
  matches!(
    op,
    "=" | "!=" | ">" | ">=" | "<" | "<=" | "prefix" | "in" | "between" | "exists"
  )
}

fn is_allowed_field(field: &str) -> bool {
  matches!(
    field,
    "ns"
      | "id"
      | "logical_id"
      | "desired_present"
      | "desired_spec_hash"
      | "owner_namespace"
      | "owner_name"
      | "owner_invocation_id"
      | "lease_until_ms"
      | "status"
      | "pid"
      | "pgid"
      | "generation"
      | "observed_spec_hash"
      | "base_url"
      | "cgroup_path"
      | "paused"
      | "fail_count"
      | "next_retry_ms"
      | "health_fail_count"
      | "last_error"
      | "last_reconcile_ms"
      | "obs_ts_ms"
      | "rss_bytes"
      | "vmsize_bytes"
      | "threads_count"
      | "fd_count"
      | "cpu_pct"
      | "io_read_bytes"
      | "io_write_bytes"
      | "labels"
      | "depends_on"
  )
}
