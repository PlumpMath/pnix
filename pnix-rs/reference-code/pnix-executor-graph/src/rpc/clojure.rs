//! Clojure backend RPC client
//!
//! Calls clojure backend via HTTP JSON
//!
//! Stage-2: inputs/outputs map 기반으로 전환
//! - 요청: { "op": "call", "name": "...", "inputs": {...}, "args": [...] }
//! - 응답: { "status": "ok", "outputs": {...} } 또는 { "status": "ok", "result": ... }

use pnix_runtime_legacy::clojure_interop::NreplClient;
use serde_json::{json, Value};
use std::net::ToSocketAddrs;

use super::client::{RpcClient, RpcError};

/// Reserved input key used to hint clojure runtime eval target.
pub const EVAL_TARGET_INPUT_KEY: &str = "__pnix_eval_target";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalTarget {
  Jvm,
  Pnix,
  Verify,
}

impl EvalTarget {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Jvm => "jvm",
      Self::Pnix => "pnix",
      Self::Verify => "verify",
    }
  }

  pub fn parse(value: &str) -> Option<Self> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
      return None;
    }

    // Accept "jvm"/"pnix"/"verify" and Clojure keyword form ":jvm".
    // Reject auto-resolved or namespaced keywords such as "::pnix" and ":ns/pnix".
    let token = if let Some(rest) = trimmed.strip_prefix(':') {
      if rest.starts_with(':') {
        return None;
      }
      rest
    } else {
      trimmed
    };
    if token.is_empty() || token.contains(':') || token.contains('/') {
      return None;
    }

    match token.to_ascii_lowercase().as_str() {
      "jvm" => Some(Self::Jvm),
      "pnix" => Some(Self::Pnix),
      "verify" => Some(Self::Verify),
      _ => None,
    }
  }
}

pub fn parse_eval_target_value(value: &Value) -> Option<EvalTarget> {
  value.as_str().and_then(EvalTarget::parse)
}

pub fn inject_eval_target(args: Value, target: Option<EvalTarget>) -> Value {
  let Some(target) = target else {
    return args;
  };
  match args {
    Value::Object(mut obj) => {
      if !obj.contains_key(EVAL_TARGET_INPUT_KEY) {
        obj.insert(
          EVAL_TARGET_INPUT_KEY.to_string(),
          Value::String(target.as_str().to_string()),
        );
      }
      Value::Object(obj)
    }
    other => other,
  }
}

#[derive(Debug, Clone)]
struct DirectNreplConfig {
  host: String,
  port: u16,
  default_ns: Option<String>,
}

impl DirectNreplConfig {
  fn from_env() -> Result<Self, RpcError> {
    let host = std::env::var("PNIX_JVM_CLOJURE_NREPL_HOST")
      .ok()
      .map(|raw| raw.trim().to_string())
      .filter(|raw| !raw.is_empty())
      .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = std::env::var("PNIX_JVM_CLOJURE_NREPL_PORT")
      .ok()
      .map(|raw| raw.trim().to_string())
      .filter(|raw| !raw.is_empty())
      .map(|raw| {
        raw.parse::<u16>().map_err(|err| RpcError::Backend {
          name: "jvm.eval".to_string(),
          body: json!({
            "reason_code": "JVM_CLOJURE_INTEROP_DIRECT_CONFIG_INVALID",
            "message": format!("invalid PNIX_JVM_CLOJURE_NREPL_PORT '{}': {}", raw, err),
            "eval_target": "jvm"
          }),
        })
      })
      .transpose()?
      .unwrap_or(7888);
    let default_ns = std::env::var("PNIX_JVM_CLOJURE_NREPL_NS")
      .ok()
      .map(|raw| raw.trim().to_string())
      .filter(|raw| !raw.is_empty());
    Ok(Self {
      host,
      port,
      default_ns,
    })
  }
}

fn direct_backend_error(name: &str, reason_code: &str, message: impl Into<String>) -> RpcError {
  RpcError::Backend {
    name: name.to_string(),
    body: json!({
      "reason_code": reason_code,
      "message": message.into(),
      "eval_target": "jvm",
      "backend": "jvm_nrepl"
    }),
  }
}

fn parse_code_arg(args: &Value) -> Result<String, RpcError> {
  match args {
    Value::String(code) => Ok(code.clone()),
    Value::Object(obj) => {
      for key in ["code", "form", "expr", "source", "__pnix_form"] {
        if let Some(value) = obj.get(key).and_then(|value| value.as_str()) {
          if !value.trim().is_empty() {
            return Ok(value.to_string());
          }
        }
      }
      Err(direct_backend_error(
        "jvm.eval",
        "JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT",
        "jvm.eval requires a non-empty string in args.code|form|expr|source|__pnix_form",
      ))
    }
    _ => Err(direct_backend_error(
      "jvm.eval",
      "JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT",
      "jvm.eval expects object or string payload",
    )),
  }
}

fn parse_optional_ns_arg(args: &Value, name: &str) -> Result<Option<String>, RpcError> {
  let Value::Object(obj) = args else {
    return Ok(None);
  };
  let Some(raw_ns) = obj.get("ns") else {
    return Ok(None);
  };
  let Some(raw_ns) = raw_ns.as_str() else {
    return Err(direct_backend_error(
      name,
      "JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT",
      "args.ns must be a non-empty string when provided",
    ));
  };
  let ns = raw_ns.trim();
  if ns.is_empty() {
    return Err(direct_backend_error(
      name,
      "JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT",
      "args.ns must be a non-empty string when provided",
    ));
  }
  Ok(Some(ns.to_string()))
}

fn parse_switch_ns_arg(args: &Value, name: &str) -> Result<String, RpcError> {
  match args {
    Value::String(raw) => {
      let ns = raw.trim();
      if ns.is_empty() {
        return Err(direct_backend_error(
          name,
          "JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT",
          "jvm.switch-ns requires non-empty namespace string payload or args.ns",
        ));
      }
      Ok(ns.to_string())
    }
    Value::Object(obj) => {
      let Some(raw_ns) = obj.get("ns") else {
        return Err(direct_backend_error(
          name,
          "JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT",
          "jvm.switch-ns requires non-empty namespace string payload or args.ns",
        ));
      };
      let Some(raw_ns) = raw_ns.as_str() else {
        return Err(direct_backend_error(
          name,
          "JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT",
          "jvm.switch-ns requires non-empty namespace string payload or args.ns",
        ));
      };
      let ns = raw_ns.trim();
      if ns.is_empty() {
        return Err(direct_backend_error(
          name,
          "JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT",
          "jvm.switch-ns requires non-empty namespace string payload or args.ns",
        ));
      }
      Ok(ns.to_string())
    }
    _ => Err(direct_backend_error(
      name,
      "JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT",
      "jvm.switch-ns requires non-empty namespace string payload or args.ns",
    )),
  }
}

fn parse_requested_eval_target(args: &Value, name: &str) -> Result<Option<EvalTarget>, RpcError> {
  let obj = match args {
    Value::Object(obj) => obj,
    _ => return Ok(None),
  };
  let mut parsed: Option<(&str, EvalTarget)> = None;
  for key in [EVAL_TARGET_INPUT_KEY, "eval_target", "pnix_eval_target"] {
    let Some(raw) = obj.get(key) else {
      continue;
    };
    let Some(raw_str) = raw.as_str() else {
      return Err(RpcError::Backend {
        name: name.to_string(),
        body: json!({
          "reason_code": "EVAL_TARGET_INVALID",
          "message": format!("{key} must be a string"),
          "eval_target": "jvm",
          "backend": "jvm_nrepl"
        }),
      });
    };
    let target = EvalTarget::parse(raw_str).ok_or_else(|| RpcError::Backend {
      name: name.to_string(),
      body: json!({
        "reason_code": "EVAL_TARGET_INVALID",
        "message": format!("Unsupported eval target '{}' in {}", raw_str, key),
        "eval_target": "jvm",
        "backend": "jvm_nrepl"
      }),
    })?;

    if let Some((existing_key, existing_target)) = parsed {
      if existing_target != target {
        return Err(RpcError::Backend {
          name: name.to_string(),
          body: json!({
            "reason_code": "EVAL_TARGET_INVALID",
            "message": format!(
              "conflicting eval target hints: {}={} vs {}={}",
              existing_key,
              existing_target.as_str(),
              key,
              target.as_str()
            ),
            "eval_target": "jvm",
            "backend": "jvm_nrepl"
          }),
        });
      }
    } else {
      parsed = Some((key, target));
    }
  }

  Ok(parsed.map(|(_, target)| target))
}

fn normalize_direct_result(value: String) -> Value {
  serde_json::from_str::<Value>(value.trim()).unwrap_or(Value::String(value))
}

fn connect_nrepl(config: &DirectNreplConfig, name: &str) -> Result<NreplClient, RpcError> {
  let addr = format!("{}:{}", config.host, config.port);
  if addr.to_socket_addrs().is_err() {
    return Err(direct_backend_error(
      name,
      "JVM_CLOJURE_INTEROP_DIRECT_CONFIG_INVALID",
      format!("invalid nREPL endpoint '{}'", addr),
    ));
  }
  NreplClient::connect(&config.host, config.port).map_err(|err| {
    direct_backend_error(
      name,
      "JVM_CLOJURE_INTEROP_DIRECT_CONNECT_FAIL",
      format!("failed to connect nREPL {}: {}", addr, err),
    )
  })
}

fn apply_switch_ns(client: &NreplClient, ns: &str, name: &str) -> Result<(), RpcError> {
  client.switch_ns(ns).map_err(|err| {
    direct_backend_error(
      name,
      "JVM_CLOJURE_INTEROP_DIRECT_SWITCH_NS_FAIL",
      format!("failed to switch namespace to '{}': {}", ns, err),
    )
  })
}

fn eval_direct(
  client: &NreplClient,
  name: &str,
  code: &str,
) -> Result<serde_json::Value, RpcError> {
  let raw = client.eval(code).map_err(|err| {
    direct_backend_error(
      name,
      "JVM_CLOJURE_INTEROP_DIRECT_EVAL_FAIL",
      format!("nREPL eval failed: {}", err),
    )
  })?;
  Ok(normalize_direct_result(raw))
}

fn call_direct_nrepl_blocking(
  config: DirectNreplConfig,
  sym: &str,
  args: Value,
) -> Result<Value, RpcError> {
  let op = sym.trim().to_ascii_lowercase();
  let name = format!("jvm.{}", op);
  let requested_target = parse_requested_eval_target(&args, &name)?;
  let supported = matches!(
    op.as_str(),
    "eval"
      | "call"
      | "form"
      | "macroexpand"
      | "macroexpand-all"
      | "macroexpand_all"
      | "switch-ns"
      | "switch_ns"
  );
  if !supported {
    return Err(direct_backend_error(
      &name,
      "JVM_CLOJURE_INTEROP_DIRECT_UNSUPPORTED_OP",
      format!("unsupported jvm direct op '{}'", sym),
    ));
  }

  if let Some(target) = requested_target {
    if target != EvalTarget::Jvm {
      return Err(direct_backend_error(
        &name,
        "JVM_CLOJURE_INTEROP_DIRECT_UNSUPPORTED_TARGET",
        format!(
          "direct jvm nREPL backend only supports :jvm eval target (requested: {})",
          target.as_str()
        ),
      ));
    }
  }

  let switch_ns = if matches!(op.as_str(), "switch-ns" | "switch_ns") {
    Some(parse_switch_ns_arg(&args, &name)?)
  } else {
    None
  };
  let request_ns = if switch_ns.is_some() {
    None
  } else {
    parse_optional_ns_arg(&args, &name)?
  };

  let client = connect_nrepl(&config, &name)?;

  if let Some(default_ns) = config.default_ns.as_deref() {
    apply_switch_ns(&client, default_ns, &name)?;
  }
  if let Some(request_ns) = request_ns.as_deref() {
    apply_switch_ns(&client, request_ns, &name)?;
  }

  match op.as_str() {
    "eval" | "call" | "form" => {
      let code = parse_code_arg(&args)?;
      let result = eval_direct(&client, &name, &code)?;
      Ok(json!({
        "status": "ok",
        "result": result,
        "eval_target": "jvm",
        "backend": "jvm_nrepl",
        "op": op
      }))
    }
    "macroexpand" => {
      let form = parse_code_arg(&args)?;
      let escaped = form.replace('\\', "\\\\").replace('"', "\\\"");
      let code = format!("(let [f (read-string \"{escaped}\")] (pr-str (macroexpand f)))");
      let result = eval_direct(&client, &name, &code)?;
      Ok(json!({
        "status": "ok",
        "result": result,
        "eval_target": "jvm",
        "backend": "jvm_nrepl",
        "op": "macroexpand"
      }))
    }
    "macroexpand-all" | "macroexpand_all" => {
      let form = parse_code_arg(&args)?;
      let escaped = form.replace('\\', "\\\\").replace('"', "\\\"");
      let code = format!(
        "(do (require 'clojure.walk) (let [f (read-string \"{escaped}\")] (pr-str (clojure.walk/macroexpand-all f))))"
      );
      let result = eval_direct(&client, &name, &code)?;
      Ok(json!({
        "status": "ok",
        "result": result,
        "eval_target": "jvm",
        "backend": "jvm_nrepl",
        "op": "macroexpand-all"
      }))
    }
    "switch-ns" | "switch_ns" => {
      let ns = switch_ns.ok_or_else(|| {
        direct_backend_error(
          &name,
          "JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT",
          "jvm.switch-ns requires non-empty namespace string payload or args.ns",
        )
      })?;
      apply_switch_ns(&client, &ns, &name)?;
      Ok(json!({
        "status": "ok",
        "result": "nil",
        "ns": ns,
        "eval_target": "jvm",
        "backend": "jvm_nrepl",
        "op": "switch-ns"
      }))
    }
    _ => Err(direct_backend_error(
      &name,
      "JVM_CLOJURE_INTEROP_DIRECT_UNSUPPORTED_OP",
      format!("unsupported jvm direct op '{}'", sym),
    )),
  }
}

pub async fn call_direct_nrepl(sym: &str, args: Value) -> Result<Value, RpcError> {
  let config = DirectNreplConfig::from_env()?;
  call_direct_nrepl_with_config(config, sym, args).await
}

async fn call_direct_nrepl_with_config(
  config: DirectNreplConfig,
  sym: &str,
  args: Value,
) -> Result<Value, RpcError> {
  let op_name = sym.to_string();
  tokio::task::spawn_blocking(move || call_direct_nrepl_blocking(config, &op_name, args))
    .await
    .map_err(|err| {
      direct_backend_error(
        &format!("jvm.{}", sym),
        "JVM_CLOJURE_INTEROP_DIRECT_RUNTIME_FAIL",
        format!("direct interop task join failed: {}", err),
      )
    })?
}

/// Call a clojure backend morphism
///
/// # Arguments
/// * `name` - Morphism name (prefix removed, e.g., "solve-linear")
/// * `args` - JSON value (Stage-2: map 선호, Stage-1 호환: array도 지원)
pub async fn call(client: &RpcClient, name: &str, args: Value) -> Result<Value, RpcError> {
  client.call(name, args).await
}

/// List available morphisms from clojure backend
#[allow(dead_code)]
pub async fn list(client: &RpcClient) -> Result<Vec<String>, RpcError> {
  client.list().await
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;
  use std::collections::BTreeMap;
  use std::io::{ErrorKind, Read, Write};
  use std::net::{TcpListener, TcpStream};
  use std::thread;

  fn encode_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(bytes.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(bytes);
  }

  fn encode_string(out: &mut Vec<u8>, value: &str) {
    encode_bytes(out, value.as_bytes());
  }

  fn encode_response(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(b'd');
    for (key, value) in entries {
      encode_string(&mut out, key);
      encode_string(&mut out, value);
    }
    encode_string(&mut out, "status");
    out.push(b'l');
    encode_string(&mut out, "done");
    out.push(b'e');
    out.push(b'e');
    out
  }

  fn decode_string(input: &[u8], mut idx: usize) -> Option<(String, usize)> {
    if idx >= input.len() || !input[idx].is_ascii_digit() {
      return None;
    }
    let start = idx;
    while idx < input.len() && input[idx].is_ascii_digit() {
      idx += 1;
    }
    if idx >= input.len() || input[idx] != b':' {
      return None;
    }
    let len = std::str::from_utf8(&input[start..idx])
      .ok()?
      .parse::<usize>()
      .ok()?;
    let content_start = idx + 1;
    let content_end = content_start.checked_add(len)?;
    if content_end > input.len() {
      return None;
    }
    let value = String::from_utf8_lossy(&input[content_start..content_end]).to_string();
    Some((value, content_end))
  }

  fn decode_request(buffer: &[u8]) -> Option<(BTreeMap<String, String>, usize)> {
    if buffer.first().copied()? != b'd' {
      return None;
    }
    let mut idx = 1;
    let mut map = BTreeMap::new();
    while idx < buffer.len() {
      if buffer[idx] == b'e' {
        return Some((map, idx + 1));
      }
      let (key, next) = decode_string(buffer, idx)?;
      idx = next;
      if idx >= buffer.len() {
        return None;
      }
      match buffer[idx] {
        b'l' => {
          // only status list is expected in test responses, not requests.
          return None;
        }
        b'i' => return None,
        b'd' => return None,
        _ => {
          let (value, next_value) = decode_string(buffer, idx)?;
          idx = next_value;
          map.insert(key, value);
        }
      }
    }
    None
  }

  fn read_bencode_request(
    stream: &mut TcpStream,
    buffer: &mut Vec<u8>,
  ) -> BTreeMap<String, String> {
    loop {
      if let Some((decoded, used)) = decode_request(buffer) {
        buffer.drain(0..used);
        return decoded;
      }
      let mut chunk = [0_u8; 4096];
      let read = stream.read(&mut chunk).expect("read request");
      assert!(read > 0, "connection closed before request completed");
      buffer.extend_from_slice(&chunk[..read]);
    }
  }

  fn bind_fake_nrepl_or_skip() -> Option<TcpListener> {
    match TcpListener::bind("127.0.0.1:0") {
      Ok(listener) => Some(listener),
      Err(err) if err.kind() == ErrorKind::PermissionDenied => {
        eprintln!(
          "skip fake nrepl socket bind in restricted environment: {}",
          err
        );
        None
      }
      Err(err) => panic!("bind fake nrepl: {}", err),
    }
  }

  #[tokio::test]
  async fn direct_nrepl_eval_switch_ns_and_macroexpand() {
    let Some(listener) = bind_fake_nrepl_or_skip() else {
      return;
    };
    let port = listener.local_addr().expect("addr").port();
    let server = thread::spawn(move || {
      for expected in ["eval", "switch-ns", "macroexpand"] {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buffer = Vec::new();

        // each direct call opens a fresh client session, so first frame is always clone.
        let req = read_bencode_request(&mut stream, &mut buffer);
        assert_eq!(req.get("op"), Some(&"clone".to_string()));
        stream
          .write_all(&encode_response(&[("new-session", "sess-1")]))
          .expect("write clone response");

        let req = read_bencode_request(&mut stream, &mut buffer);
        assert_eq!(req.get("op"), Some(&"eval".to_string()));
        match expected {
          "eval" => {
            assert_eq!(req.get("code"), Some(&"(+ 1 2)".to_string()));
            stream
              .write_all(&encode_response(&[("value", "3")]))
              .expect("write eval response");
          }
          "switch-ns" => {
            assert!(
              req
                .get("code")
                .map(|code| code.contains("(create-ns 'user.test)"))
                .unwrap_or(false),
              "expected switch-ns code payload"
            );
            stream
              .write_all(&encode_response(&[("value", "nil")]))
              .expect("write switch response");
          }
          "macroexpand" => {
            assert!(
              req
                .get("code")
                .map(|code| code.contains("macroexpand"))
                .unwrap_or(false),
              "expected macroexpand payload"
            );
            stream
              .write_all(&encode_response(&[("value", "(if true (do 1))")]))
              .expect("write macroexpand response");
          }
          _ => unreachable!("unsupported expected op"),
        }
      }
    });

    let config = DirectNreplConfig {
      host: "127.0.0.1".to_string(),
      port,
      default_ns: None,
    };

    let eval = call_direct_nrepl_with_config(config.clone(), "eval", json!({"code": "(+ 1 2)"}))
      .await
      .expect("eval call");
    assert_eq!(eval.get("status").and_then(|v| v.as_str()), Some("ok"));
    assert_eq!(eval.get("result"), Some(&json!(3)));

    let switch_ns =
      call_direct_nrepl_with_config(config.clone(), "switch-ns", json!({"ns": "user.test"}))
        .await
        .expect("switch-ns call");
    assert_eq!(switch_ns.get("status").and_then(|v| v.as_str()), Some("ok"));
    assert_eq!(
      switch_ns.get("ns").and_then(|v| v.as_str()),
      Some("user.test")
    );

    let macroexpand =
      call_direct_nrepl_with_config(config, "macroexpand", json!({"form": "(when true 1)"}))
        .await
        .expect("macroexpand call");
    assert_eq!(
      macroexpand.get("status").and_then(|v| v.as_str()),
      Some("ok")
    );
    assert_eq!(macroexpand.get("result"), Some(&json!("(if true (do 1))")));

    server.join().expect("server join");
  }

  #[tokio::test]
  async fn direct_nrepl_eval_string_payload_does_not_switch_ns() {
    let Some(listener) = bind_fake_nrepl_or_skip() else {
      return;
    };
    let port = listener.local_addr().expect("addr").port();
    let server = thread::spawn(move || {
      let (mut stream, _) = listener.accept().expect("accept");
      let mut buffer = Vec::new();

      let req = read_bencode_request(&mut stream, &mut buffer);
      assert_eq!(req.get("op"), Some(&"clone".to_string()));
      stream
        .write_all(&encode_response(&[("new-session", "sess-1")]))
        .expect("write clone response");

      let req = read_bencode_request(&mut stream, &mut buffer);
      assert_eq!(req.get("op"), Some(&"eval".to_string()));
      assert_eq!(req.get("code"), Some(&"(+ 4 5)".to_string()));
      stream
        .write_all(&encode_response(&[("value", "9")]))
        .expect("write eval response");
    });

    let config = DirectNreplConfig {
      host: "127.0.0.1".to_string(),
      port,
      default_ns: None,
    };

    let eval = call_direct_nrepl_with_config(config, "eval", json!("(+ 4 5)"))
      .await
      .expect("eval call");
    assert_eq!(eval.get("status").and_then(|v| v.as_str()), Some("ok"));
    assert_eq!(eval.get("result"), Some(&json!(9)));

    server.join().expect("server join");
  }

  #[tokio::test]
  async fn direct_nrepl_rejects_unknown_op() {
    let config = DirectNreplConfig {
      host: "127.0.0.1".to_string(),
      port: 65530,
      default_ns: None,
    };
    let err = call_direct_nrepl_with_config(config, "unknown-op", json!({}))
      .await
      .expect_err("unknown op must fail");
    let body = match err {
      RpcError::Backend { body, .. } => body,
      other => panic!("unexpected error variant: {}", other),
    };
    assert_eq!(
      body.get("reason_code").and_then(|v| v.as_str()),
      Some("JVM_CLOJURE_INTEROP_DIRECT_UNSUPPORTED_OP")
    );
  }

  #[tokio::test]
  async fn direct_nrepl_rejects_non_jvm_eval_target() {
    let config = DirectNreplConfig {
      host: "127.0.0.1".to_string(),
      port: 65530,
      default_ns: None,
    };
    let err = call_direct_nrepl_with_config(
      config,
      "eval",
      json!({"code": "(+ 1 2)", "__pnix_eval_target": ":pnix"}),
    )
    .await
    .expect_err("non-jvm eval target must fail");
    let body = match err {
      RpcError::Backend { body, .. } => body,
      other => panic!("unexpected error variant: {}", other),
    };
    assert_eq!(
      body.get("reason_code").and_then(|v| v.as_str()),
      Some("JVM_CLOJURE_INTEROP_DIRECT_UNSUPPORTED_TARGET")
    );
  }

  #[tokio::test]
  async fn direct_nrepl_rejects_invalid_eval_target_value() {
    let config = DirectNreplConfig {
      host: "127.0.0.1".to_string(),
      port: 65530,
      default_ns: None,
    };
    let err = call_direct_nrepl_with_config(
      config,
      "eval",
      json!({"code": "(+ 1 2)", "__pnix_eval_target": ":bad-target"}),
    )
    .await
    .expect_err("invalid eval target must fail");
    let body = match err {
      RpcError::Backend { body, .. } => body,
      other => panic!("unexpected error variant: {}", other),
    };
    assert_eq!(
      body.get("reason_code").and_then(|v| v.as_str()),
      Some("EVAL_TARGET_INVALID")
    );
  }

  #[tokio::test]
  async fn direct_nrepl_rejects_non_string_ns_hint() {
    let config = DirectNreplConfig {
      host: "127.0.0.1".to_string(),
      port: 65530,
      default_ns: None,
    };
    let err = call_direct_nrepl_with_config(
      config,
      "eval",
      json!({"code": "(+ 1 2)", "ns": {"bad": true}}),
    )
    .await
    .expect_err("non-string ns hint must fail closed");
    let body = match err {
      RpcError::Backend { body, .. } => body,
      other => panic!("unexpected error variant: {}", other),
    };
    assert_eq!(
      body.get("reason_code").and_then(|v| v.as_str()),
      Some("JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT")
    );
  }

  #[tokio::test]
  async fn direct_nrepl_rejects_empty_switch_ns_payload() {
    let config = DirectNreplConfig {
      host: "127.0.0.1".to_string(),
      port: 65530,
      default_ns: None,
    };
    let err = call_direct_nrepl_with_config(config, "switch-ns", json!({"ns": "   "}))
      .await
      .expect_err("empty switch-ns payload must fail closed");
    let body = match err {
      RpcError::Backend { body, .. } => body,
      other => panic!("unexpected error variant: {}", other),
    };
    assert_eq!(
      body.get("reason_code").and_then(|v| v.as_str()),
      Some("JVM_CLOJURE_INTEROP_DIRECT_INVALID_INPUT")
    );
  }

  #[tokio::test]
  async fn direct_nrepl_rejects_non_jvm_eval_target_via_legacy_key() {
    let config = DirectNreplConfig {
      host: "127.0.0.1".to_string(),
      port: 65530,
      default_ns: None,
    };
    let err = call_direct_nrepl_with_config(
      config,
      "eval",
      json!({"code": "(+ 1 2)", "eval_target": "verify"}),
    )
    .await
    .expect_err("legacy non-jvm eval target must fail");
    let body = match err {
      RpcError::Backend { body, .. } => body,
      other => panic!("unexpected error variant: {}", other),
    };
    assert_eq!(
      body.get("reason_code").and_then(|v| v.as_str()),
      Some("JVM_CLOJURE_INTEROP_DIRECT_UNSUPPORTED_TARGET")
    );
  }

  #[test]
  fn parse_requested_eval_target_accepts_legacy_keys() {
    assert_eq!(
      parse_requested_eval_target(&json!({"eval_target": ":verify"}), "jvm.eval")
        .expect("legacy eval_target should parse"),
      Some(EvalTarget::Verify)
    );
    assert_eq!(
      parse_requested_eval_target(&json!({"pnix_eval_target": "jvm"}), "jvm.eval")
        .expect("legacy pnix_eval_target should parse"),
      Some(EvalTarget::Jvm)
    );
  }

  #[test]
  fn parse_requested_eval_target_rejects_conflicting_hints() {
    let err = parse_requested_eval_target(
      &json!({"__pnix_eval_target": ":jvm", "eval_target": ":pnix"}),
      "jvm.eval",
    )
    .expect_err("conflicting eval target hints must fail");
    let body = match err {
      RpcError::Backend { body, .. } => body,
      other => panic!("unexpected error variant: {}", other),
    };
    assert_eq!(
      body.get("reason_code").and_then(|v| v.as_str()),
      Some("EVAL_TARGET_INVALID")
    );
  }

  #[test]
  fn parse_requested_eval_target_rejects_non_string_legacy_values() {
    let err = parse_requested_eval_target(&json!({"eval_target": 123}), "jvm.eval")
      .expect_err("non-string eval target hint must fail");
    let body = match err {
      RpcError::Backend { body, .. } => body,
      other => panic!("unexpected error variant: {}", other),
    };
    assert_eq!(
      body.get("reason_code").and_then(|v| v.as_str()),
      Some("EVAL_TARGET_INVALID")
    );
  }

  #[test]
  fn parse_requested_eval_target_accepts_consistent_multi_key_hints() {
    assert_eq!(
      parse_requested_eval_target(
        &json!({
          "__pnix_eval_target": ":jvm",
          "eval_target": "jvm",
          "pnix_eval_target": "JVM"
        }),
        "jvm.eval"
      )
      .expect("consistent hints should parse"),
      Some(EvalTarget::Jvm)
    );
  }

  #[test]
  fn parse_requested_eval_target_rejects_invalid_legacy_even_with_primary_hint() {
    let err = parse_requested_eval_target(
      &json!({
        "__pnix_eval_target": "jvm",
        "eval_target": {"bad": true}
      }),
      "jvm.eval",
    )
    .expect_err("invalid secondary hint must fail closed");
    let body = match err {
      RpcError::Backend { body, .. } => body,
      other => panic!("unexpected error variant: {}", other),
    };
    assert_eq!(
      body.get("reason_code").and_then(|v| v.as_str()),
      Some("EVAL_TARGET_INVALID")
    );
  }

  #[test]
  fn parse_eval_target_accepts_known_values() {
    assert_eq!(EvalTarget::parse("jvm"), Some(EvalTarget::Jvm));
    assert_eq!(EvalTarget::parse("Jvm"), Some(EvalTarget::Jvm));
    assert_eq!(EvalTarget::parse(":pnix"), Some(EvalTarget::Pnix));
    assert_eq!(EvalTarget::parse(" verify "), Some(EvalTarget::Verify));
    assert_eq!(EvalTarget::parse("unknown"), None);
  }

  #[test]
  fn parse_eval_target_rejects_ambiguous_keyword_forms() {
    assert_eq!(EvalTarget::parse("::pnix"), None);
    assert_eq!(EvalTarget::parse(":ns/pnix"), None);
    assert_eq!(EvalTarget::parse(":pnix:verify"), None);
    assert_eq!(EvalTarget::parse(":"), None);
  }

  #[test]
  fn inject_eval_target_sets_reserved_key_for_object_inputs() {
    let out = inject_eval_target(json!({"a": 1}), Some(EvalTarget::Pnix));
    assert_eq!(out.get("a"), Some(&json!(1)));
    assert_eq!(
      out.get(EVAL_TARGET_INPUT_KEY),
      Some(&json!(EvalTarget::Pnix.as_str()))
    );
  }

  #[test]
  fn inject_eval_target_keeps_non_object_payload_unchanged() {
    let payload = json!([1, 2, 3]);
    let out = inject_eval_target(payload.clone(), Some(EvalTarget::Jvm));
    assert_eq!(out, payload);
  }

  #[test]
  fn inject_eval_target_preserves_existing_reserved_key() {
    let out = inject_eval_target(
      json!({"a": 1, "__pnix_eval_target": "verify"}),
      Some(EvalTarget::Pnix),
    );
    assert_eq!(out.get("a"), Some(&json!(1)));
    assert_eq!(out.get(EVAL_TARGET_INPUT_KEY), Some(&json!("verify")));
  }
}
