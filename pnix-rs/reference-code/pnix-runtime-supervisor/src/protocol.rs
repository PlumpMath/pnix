use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
  pub op: String,
  pub id: u64,
  #[serde(default)]
  pub name: String,
  #[serde(default)]
  pub inputs: Value,
  #[serde(default)]
  pub args: Value,
  #[serde(default)]
  pub token: String,
  #[serde(default)]
  pub caps: Vec<String>,
}

pub fn request_payload(req: &RpcRequest) -> &Value {
  if !req.inputs.is_null() {
    &req.inputs
  } else {
    &req.args
  }
}

pub fn ok_outputs(id: u64, outputs: Value) -> Value {
  json!({
    "status": "ok",
    "id": id,
    "outputs": outputs,
  })
}

pub fn ok_list(id: u64, morphisms: Vec<&'static str>) -> Value {
  json!({
    "status": "ok",
    "id": id,
    "morphisms": morphisms,
  })
}

pub fn err(id: u64, message: impl Into<String>, data: Option<Value>) -> Value {
  let mut obj = serde_json::Map::new();
  obj.insert("message".to_string(), Value::String(message.into()));
  if let Some(data) = data {
    obj.insert("data".to_string(), data);
  }

  json!({
    "status": "error",
    "id": id,
    "error": Value::Object(obj),
  })
}
