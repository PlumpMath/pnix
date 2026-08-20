//! JIT 컴파일: Just-In-Time 컴파일을 통한 런타임 코드 생성 및 실행

use pnix_runtime_api::{EvalConfig, EvalEngine, EvalResult, RuntimeError, RuntimeResult};

#[cfg(feature = "llvm")]
use libc::free;
#[cfg(feature = "llvm")]
use std::collections::{HashMap, VecDeque};
#[cfg(feature = "llvm")]
use std::ffi::CString;

#[cfg(feature = "llvm")]
use super::LlvmRuntimeError;

/// C 문자열 포인터 가드: 자동 해제를 위한 래퍼
#[cfg(feature = "llvm")]
struct StringPtrGuard {
  /// C 문자열 포인터
  ptr: *const i8,
  /// 해제 함수 (선택적)
  free_fn: Option<unsafe extern "C" fn(*mut i8)>,
}

#[cfg(feature = "llvm")]
impl StringPtrGuard {
  fn new(ptr: *const i8, free_fn: Option<unsafe extern "C" fn(*mut i8)>) -> Self {
    Self { ptr, free_fn }
  }
}

#[cfg(feature = "llvm")]
impl Drop for StringPtrGuard {
  fn drop(&mut self) {
    if !self.ptr.is_null() {
      unsafe {
        if let Some(free_fn) = self.free_fn {
          free_fn(self.ptr as *mut i8);
        } else {
          free(self.ptr as *mut libc::c_void);
        }
      }
    }
  }
}

#[cfg(feature = "llvm")]
/// 안전한 C 문자열 읽기 래퍼
///
/// # Safety
/// - `ptr`는 유효한 C 문자열을 가리켜야 하며, null이 아니어야 합니다
/// - `ptr`는 최소 `max_len` 바이트까지 유효한 메모리를 가리켜야 합니다
/// - 반환된 슬라이스는 `ptr`이 유효한 동안만 사용할 수 있습니다
unsafe fn read_c_string_bytes<'a>(
  ptr: *const i8,
  max_len: usize,
) -> Result<&'a [u8], LlvmRuntimeError> {
  // Null 포인터 검사
  if ptr.is_null() {
    return Err(LlvmRuntimeError::ExecutionError(
      "Function returned null pointer for string".to_string(),
    ));
  }

  // CStr::from_ptr는 내부적으로 null 종료 문자를 찾기 위해 메모리를 읽습니다
  // 이는 안전하지 않지만, FFI 경계에서는 필요합니다
  let cstr = match std::ffi::CStr::from_ptr(ptr) {
    cstr => cstr,
  };

  let bytes = cstr.to_bytes();

  // 길이 검증
  if bytes.len() > max_len {
    return Err(LlvmRuntimeError::ExecutionError(format!(
      "String returned by JIT exceeds maximum length ({} bytes, max: {})",
      bytes.len(),
      max_len
    )));
  }

  Ok(bytes)
}

/// 안전한 C 문자열 읽기 래퍼 (공개 API)
///
/// 이 함수는 unsafe 블록을 내부에 캡슐화하여 호출자가 안전하게 사용할 수 있도록 합니다.
///
/// # Arguments
/// - `ptr`: C 문자열 포인터 (null이 아니어야 함)
/// - `max_len`: 최대 허용 길이 (바이트 단위)
///
/// # Returns
/// - `Ok(bytes)`: 유효한 바이트 슬라이스
/// - `Err`: null 포인터 또는 길이 초과
#[cfg(feature = "llvm")]
pub fn safe_read_c_string(ptr: *const i8, max_len: usize) -> Result<Vec<u8>, LlvmRuntimeError> {
  // 안전한 래퍼: unsafe 블록을 내부에 캡슐화
  unsafe { read_c_string_bytes(ptr, max_len).map(|bytes| bytes.to_vec()) }
}

#[cfg(feature = "llvm")]
fn tuple_field_types<'ctx>(
  context: &'ctx inkwell::context::ContextRef<'ctx>,
  output_kinds: &[super::ValueKind],
) -> Vec<inkwell::types::BasicTypeEnum<'ctx>> {
  let i8_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
  output_kinds
    .iter()
    .map(|kind| match kind {
      super::ValueKind::Int => context.i64_type().into(),
      super::ValueKind::Float => context.f64_type().into(),
      super::ValueKind::Bool => context.i32_type().into(), // Bool 출력은 i32 ABI
      super::ValueKind::String | super::ValueKind::List | super::ValueKind::AttrSet => {
        i8_ptr_type.into()
      }
    })
    .collect()
}

#[cfg(feature = "llvm")]
fn compute_tuple_layout<'ctx>(
  context: &'ctx inkwell::context::ContextRef<'ctx>,
  target_data: &inkwell::targets::TargetData,
  output_kinds: &[super::ValueKind],
) -> RuntimeResult<(Vec<u64>, usize, usize)> {
  if output_kinds.is_empty() {
    return Err(
      LlvmRuntimeError::ConfigError("Tuple output must have at least one element".to_string())
        .into(),
    );
  }
  let field_types = tuple_field_types(context, output_kinds);
  let struct_type = context.struct_type(&field_types, false);
  let mut offsets = Vec::with_capacity(output_kinds.len());
  for idx in 0..output_kinds.len() {
    let offset = target_data
      .offset_of_element(&struct_type, idx as u32)
      .ok_or_else(|| {
        LlvmRuntimeError::ConfigError(format!(
          "Failed to compute tuple output offset for index {}",
          idx
        ))
      })?;
    offsets.push(offset);
  }
  let struct_size = target_data.get_store_size(&struct_type) as usize;
  let pointer_size = target_data.get_pointer_byte_size(None) as usize;
  Ok((offsets, struct_size, pointer_size))
}

#[cfg(feature = "llvm")]
fn decode_tuple_output(
  buffer: &[u8],
  output_kinds: &[super::ValueKind],
  offsets: &[u64],
  pointer_size: usize,
  execution_engine: &inkwell::execution_engine::ExecutionEngine,
) -> RuntimeResult<String> {
  if output_kinds.len() != offsets.len() {
    return Err(LlvmRuntimeError::ConfigError("Tuple output layout mismatch".to_string()).into());
  }
  let free_string_fn = JitEngine::get_free_string_fn(execution_engine);
  let mut items = Vec::with_capacity(output_kinds.len());

  let read_bytes = |offset: usize, size: usize| -> RuntimeResult<&[u8]> {
    buffer.get(offset..offset + size).ok_or_else(|| {
      LlvmRuntimeError::ExecutionError("Tuple output buffer out of bounds".to_string()).into()
    })
  };

  for (idx, kind) in output_kinds.iter().enumerate() {
    let offset = offsets[idx] as usize;
    let value = match kind {
      super::ValueKind::Int => {
        let bytes = read_bytes(offset, 8)?;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(bytes);
        serde_json::Value::from(i64::from_ne_bytes(buf))
      }
      super::ValueKind::Float => {
        let bytes = read_bytes(offset, 8)?;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(bytes);
        serde_json::Value::from(f64::from_ne_bytes(buf))
      }
      super::ValueKind::Bool => {
        let bytes = read_bytes(offset, 4)?;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(bytes);
        let val = i32::from_ne_bytes(buf) != 0;
        serde_json::Value::from(val)
      }
      super::ValueKind::String | super::ValueKind::List | super::ValueKind::AttrSet => {
        let ptr = match pointer_size {
          4 => {
            let bytes = read_bytes(offset, 4)?;
            let mut buf = [0u8; 4];
            buf.copy_from_slice(bytes);
            u32::from_ne_bytes(buf) as usize
          }
          8 => {
            let bytes = read_bytes(offset, 8)?;
            let mut buf = [0u8; 8];
            buf.copy_from_slice(bytes);
            u64::from_ne_bytes(buf) as usize
          }
          _ => {
            return Err(
              LlvmRuntimeError::ExecutionError(format!(
                "Unsupported pointer size {}",
                pointer_size
              ))
              .into(),
            );
          }
        } as *const i8;

        if ptr.is_null() {
          serde_json::Value::Null
        } else {
          let _guard = StringPtrGuard::new(ptr, free_string_fn);
          const MAX_STRING_LENGTH: usize = 1024 * 1024;
          let bytes = safe_read_c_string(ptr, MAX_STRING_LENGTH)?;
          let str_value = std::str::from_utf8(&bytes).map_err(|e| {
            LlvmRuntimeError::ExecutionError(format!("Invalid UTF-8 in tuple output: {}", e))
          })?;
          match kind {
            super::ValueKind::String => serde_json::Value::String(str_value.to_string()),
            super::ValueKind::List => {
              let value: serde_json::Value = serde_json::from_str(str_value).map_err(|e| {
                LlvmRuntimeError::ExecutionError(format!(
                  "Invalid JSON list in tuple output: {}",
                  e
                ))
              })?;
              if !value.is_array() {
                return Err(
                  LlvmRuntimeError::ExecutionError(
                    "Tuple List element must be a JSON array".to_string(),
                  )
                  .into(),
                );
              }
              value
            }
            super::ValueKind::AttrSet => {
              let value: serde_json::Value = serde_json::from_str(str_value).map_err(|e| {
                LlvmRuntimeError::ExecutionError(format!(
                  "Invalid JSON attrset in tuple output: {}",
                  e
                ))
              })?;
              if !value.is_object() {
                return Err(
                  LlvmRuntimeError::ExecutionError(
                    "Tuple AttrSet element must be a JSON object".to_string(),
                  )
                  .into(),
                );
              }
              value
            }
            _ => serde_json::Value::Null,
          }
        }
      }
    };
    items.push(value);
  }

  Ok(serde_json::Value::Array(items).to_string())
}

/// JIT-compiled module
#[derive(Debug, Clone)]
pub struct JitModule {
  /// Module name
  pub name: String,
  /// Ordered input names (Int/Float/Bool; Bool encoded as Int)
  #[cfg_attr(not(feature = "llvm"), allow(dead_code))]
  input_names: Vec<String>,
  /// Ordered input kinds (used for pointer input parsing)
  #[cfg_attr(not(feature = "llvm"), allow(dead_code))]
  input_kinds: Vec<super::ValueKind>,
  /// Numeric kind for inputs (Int or Float; Bool inputs are encoded as Int)
  #[cfg_attr(not(feature = "llvm"), allow(dead_code))]
  numeric_kind: super::NumericKind,
  /// Output kind (scalar or tuple)
  #[cfg_attr(not(feature = "llvm"), allow(dead_code))]
  output_kind: super::OutputKind,
  /// Y13a-13: 입력 타입 소스 오브 트루스 - 모듈에 pointer 입력이 있는지 저장
  #[cfg_attr(not(feature = "llvm"), allow(dead_code))]
  has_ptr_input: bool,
  /// LLVM module handle (context + module)
  #[cfg(feature = "llvm")]
  llvm_module: Option<std::sync::Arc<JitModuleHandle>>,
}

#[cfg(feature = "llvm")]
#[derive(Debug)]
pub(crate) struct JitModuleHandle {
  execution_engine: std::cell::RefCell<Option<inkwell::execution_engine::ExecutionEngine<'static>>>,
  engine_opt_level: std::cell::Cell<Option<inkwell::OptimizationLevel>>,
  module: inkwell::module::Module<'static>,
  _context: std::sync::Arc<inkwell::context::Context>,
}

impl JitModule {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_names: Vec::new(),
      input_kinds: Vec::new(),
      numeric_kind: super::NumericKind::Int,
      output_kind: super::OutputKind::Scalar(super::ValueKind::Int),
      has_ptr_input: false,
      #[cfg(feature = "llvm")]
      llvm_module: None,
    }
  }

  #[cfg(feature = "llvm")]
  pub(crate) fn with_module(
    name: impl Into<String>,
    input_names: Vec<String>,
    input_kinds: Vec<super::ValueKind>,
    numeric_kind: super::NumericKind,
    output_kind: super::OutputKind,
    has_ptr_input: bool,
    handle: std::sync::Arc<JitModuleHandle>,
  ) -> Self {
    Self {
      name: name.into(),
      input_names,
      input_kinds,
      numeric_kind,
      output_kind,
      has_ptr_input,
      llvm_module: Some(handle),
    }
  }

  #[cfg(feature = "llvm")]
  fn llvm_module_ref(&self) -> RuntimeResult<&inkwell::module::Module<'static>> {
    self
      .llvm_module
      .as_ref()
      .map(|handle| &handle.module)
      .ok_or_else(|| {
        RuntimeError::from(LlvmRuntimeError::ConfigError(
          "missing LLVM module".to_string(),
        ))
      })
  }

  #[cfg(feature = "llvm")]
  fn execution_engine(
    &self,
    opt_level: inkwell::OptimizationLevel,
  ) -> RuntimeResult<inkwell::execution_engine::ExecutionEngine<'static>> {
    let handle = self.llvm_module.as_ref().ok_or_else(|| {
      RuntimeError::from(LlvmRuntimeError::ConfigError(
        "missing LLVM module".to_string(),
      ))
    })?;
    handle.execution_engine(opt_level)
  }
}

#[cfg(feature = "llvm")]
impl JitModuleHandle {
  fn execution_engine(
    &self,
    opt_level: inkwell::OptimizationLevel,
  ) -> RuntimeResult<inkwell::execution_engine::ExecutionEngine<'static>> {
    if let Some(engine) = self.execution_engine.borrow().as_ref() {
      if let Some(existing) = self.engine_opt_level.get() {
        if existing != opt_level {
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "JIT module already has an execution engine with opt level {:?}; requested {:?}",
              existing, opt_level
            ))
            .into(),
          );
        }
      }
      return Ok(engine.clone());
    }

    let engine = self
      .module
      .create_jit_execution_engine(opt_level)
      .map_err(|e| {
        LlvmRuntimeError::ExecutionError(format!("Failed to create execution engine: {:?}", e))
      })?;
    *self.execution_engine.borrow_mut() = Some(engine.clone());
    self.engine_opt_level.set(Some(opt_level));
    Ok(engine)
  }
}

/// JIT runtime value
#[derive(Debug, Clone)]
pub struct JitValue {
  /// Value representation (placeholder)
  /// Actual representation depends on LLVM type system
  pub data: Vec<u8>, // Placeholder - actual type depends on LLVM bindings
}

impl JitValue {
  pub fn new(data: Vec<u8>) -> Self {
    Self { data }
  }
}

#[cfg(feature = "llvm")]
enum ParsedInputs {
  Int(Vec<i64>),
  Float(Vec<f64>),
  Ptr(Vec<String>), // Pointer inputs (String/List/AttrSet)
}

#[cfg(feature = "llvm")]
fn parse_inputs(
  input_names: &[String],
  input_kinds: &[super::ValueKind],
  numeric_kind: super::NumericKind,
  has_ptr_input: bool,
  inputs_json: &[u8],
) -> RuntimeResult<ParsedInputs> {
  // Y13a-13: 입력 타입 소스 오브 트루스 - 모듈 타입 기반으로 ParsedInputs 경로 선택
  // JSON all-string 휴리스틱 제거, 모듈의 has_ptr_input을 사용
  // Y13a-14: 타입 불일치 검증 - 각 파서에서 타입 검증을 수행
  if has_ptr_input {
    return Ok(ParsedInputs::Ptr(parse_inputs_ptr(
      input_names,
      input_kinds,
      inputs_json,
    )?));
  }

  match numeric_kind {
    super::NumericKind::Int => Ok(ParsedInputs::Int(parse_inputs_i64(
      input_names,
      inputs_json,
    )?)),
    super::NumericKind::Float => Ok(ParsedInputs::Float(parse_inputs_f64(
      input_names,
      inputs_json,
    )?)),
  }
}

#[cfg(feature = "llvm")]
fn parse_inputs_ptr(
  input_names: &[String],
  input_kinds: &[super::ValueKind],
  inputs_json: &[u8],
) -> RuntimeResult<Vec<String>> {
  // Y13a-18: 입력 없음 + inputs_json 제공 시 명시적 에러
  // input_names.is_empty()인 경우에도 inputs_json이 비어있지 않으면 unknown keys로 실패 처리
  if input_names.is_empty() {
    if !inputs_json.is_empty() {
      // inputs_json을 파싱하여 형태 검증
      let value: serde_json::Value = serde_json::from_slice(inputs_json)
        .map_err(|err| LlvmRuntimeError::ConfigError(format!("invalid inputs JSON: {}", err)))?;
      if let Some(obj) = value.as_object() {
        if !obj.is_empty() {
          let unknown_keys: Vec<String> = obj.keys().cloned().collect();
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "Module has no inputs, but inputs JSON contains key(s): {}. \
               Remove these keys or use a module that accepts inputs.",
              unknown_keys.join(", ")
            ))
            .into(),
          );
        }
      } else {
        // object가 아닌 경우(배열/숫자/문자열) 명시적 에러
        let json_type = match value {
          serde_json::Value::Array(_) => "array",
          serde_json::Value::Number(_) => "number",
          serde_json::Value::String(_) => "string",
          serde_json::Value::Bool(_) => "boolean",
          serde_json::Value::Null => "null",
          _ => "unknown",
        };
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Module has no inputs, but inputs JSON is a {} (not an object). \
             Inputs JSON must be an object when provided, or empty/omitted for modules with no inputs.",
            json_type
          ))
          .into(),
        );
      }
    }
    return Ok(Vec::new());
  }
  if inputs_json.is_empty() {
    return Err(
      LlvmRuntimeError::ConfigError(format!("missing inputs: {}", input_names.join(", "))).into(),
    );
  }
  let value: serde_json::Value = serde_json::from_slice(inputs_json)
    .map_err(|err| LlvmRuntimeError::ConfigError(format!("invalid inputs JSON: {}", err)))?;
  let obj = value
    .as_object()
    .ok_or_else(|| LlvmRuntimeError::ConfigError("inputs JSON must be an object".to_string()))?;

  let allowed_keys: std::collections::HashSet<String> = input_names.iter().cloned().collect();
  let unknown_keys: Vec<String> = obj
    .keys()
    .filter(|key| !allowed_keys.contains(*key))
    .cloned()
    .collect();
  if !unknown_keys.is_empty() {
    return Err(
      LlvmRuntimeError::ConfigError(format!(
        "Unknown input key(s): {}. Allowed keys: {}.",
        unknown_keys.join(", "),
        input_names.join(", ")
      ))
      .into(),
    );
  }

  if input_names.len() != input_kinds.len() {
    return Err(
      LlvmRuntimeError::ConfigError(format!(
        "Input name/kind length mismatch (names={}, kinds={})",
        input_names.len(),
        input_kinds.len()
      ))
      .into(),
    );
  }

  let mut values = Vec::with_capacity(input_names.len());
  for (idx, name) in input_names.iter().enumerate() {
    let entry = obj
      .get(name)
      .ok_or_else(|| LlvmRuntimeError::ConfigError(format!("missing input '{}'", name)))?;
    let kind = input_kinds[idx];
    let value = match kind {
      super::ValueKind::String => {
        let str_value = entry.as_str().ok_or_else(|| {
          LlvmRuntimeError::ConfigError(format!("input '{}' must be a string", name))
        })?;
        str_value.to_string()
      }
      super::ValueKind::List => {
        if !entry.is_array() {
          return Err(
            LlvmRuntimeError::ConfigError(format!("input '{}' must be a list (JSON array)", name))
              .into(),
          );
        }
        serde_json::to_string(entry).map_err(|err| {
          LlvmRuntimeError::ConfigError(format!(
            "failed to serialize list input '{}': {}",
            name, err
          ))
        })?
      }
      super::ValueKind::AttrSet => {
        if !entry.is_object() {
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "input '{}' must be an attrset (JSON object)",
              name
            ))
            .into(),
          );
        }
        serde_json::to_string(entry).map_err(|err| {
          LlvmRuntimeError::ConfigError(format!(
            "failed to serialize attrset input '{}': {}",
            name, err
          ))
        })?
      }
      _ => {
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "input '{}' has non-pointer kind {:?} in pointer input path",
            name, kind
          ))
          .into(),
        );
      }
    };
    values.push(value);
  }
  Ok(values)
}

#[cfg(feature = "llvm")]
fn empty_inputs(numeric_kind: super::NumericKind, has_ptr_input: bool) -> ParsedInputs {
  // Y13a-13: 입력 타입 소스 오브 트루스 - 모듈 타입 기반으로 ParsedInputs 경로 선택
  if has_ptr_input {
    ParsedInputs::Ptr(Vec::new())
  } else {
    match numeric_kind {
      super::NumericKind::Int => ParsedInputs::Int(Vec::new()),
      super::NumericKind::Float => ParsedInputs::Float(Vec::new()),
    }
  }
}

#[cfg(feature = "llvm")]
fn parse_inputs_i64(input_names: &[String], inputs_json: &[u8]) -> RuntimeResult<Vec<i64>> {
  // Y13a-18: 입력 없음 + inputs_json 제공 시 명시적 에러
  // input_names.is_empty()인 경우에도 inputs_json이 비어있지 않으면 unknown keys로 실패 처리
  if input_names.is_empty() {
    if !inputs_json.is_empty() {
      // inputs_json을 파싱하여 형태 검증
      let value: serde_json::Value = serde_json::from_slice(inputs_json)
        .map_err(|err| LlvmRuntimeError::ConfigError(format!("invalid inputs JSON: {}", err)))?;
      if let Some(obj) = value.as_object() {
        if !obj.is_empty() {
          let unknown_keys: Vec<String> = obj.keys().cloned().collect();
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "Module has no inputs, but inputs JSON contains key(s): {}. \
               Remove these keys or use a module that accepts inputs.",
              unknown_keys.join(", ")
            ))
            .into(),
          );
        }
      } else {
        // object가 아닌 경우(배열/숫자/문자열) 명시적 에러
        let json_type = match value {
          serde_json::Value::Array(_) => "array",
          serde_json::Value::Number(_) => "number",
          serde_json::Value::String(_) => "string",
          serde_json::Value::Bool(_) => "boolean",
          serde_json::Value::Null => "null",
          _ => "unknown",
        };
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Module has no inputs, but inputs JSON is a {} (not an object). \
             Inputs JSON must be an object when provided, or empty/omitted for modules with no inputs.",
            json_type
          ))
          .into(),
        );
      }
    }
    return Ok(Vec::new());
  }
  if inputs_json.is_empty() {
    return Err(
      LlvmRuntimeError::ConfigError(format!("missing inputs: {}", input_names.join(", "))).into(),
    );
  }
  let value: serde_json::Value = serde_json::from_slice(inputs_json)
    .map_err(|err| LlvmRuntimeError::ConfigError(format!("invalid inputs JSON: {}", err)))?;
  let obj = value
    .as_object()
    .ok_or_else(|| LlvmRuntimeError::ConfigError("inputs JSON must be an object".to_string()))?;

  // U26: Check for unknown keys (defense in depth - CLI should also validate)
  let allowed_keys: std::collections::HashSet<String> = input_names.iter().cloned().collect();
  let unknown_keys: Vec<String> = obj
    .keys()
    .filter(|key| !allowed_keys.contains(*key))
    .cloned()
    .collect();
  if !unknown_keys.is_empty() {
    return Err(
      LlvmRuntimeError::ConfigError(format!(
        "Unknown input key(s): {}. Allowed keys: {}. \
                Unknown keys are not allowed in LLVM runtime inputs JSON.",
        unknown_keys.join(", "),
        input_names.join(", ")
      ))
      .into(),
    );
  }

  let mut values = Vec::with_capacity(input_names.len());
  for name in input_names {
    let entry = obj
      .get(name)
      .ok_or_else(|| LlvmRuntimeError::ConfigError(format!("missing input '{}'", name)))?;
    let parsed = json_value_to_i64(entry).ok_or_else(|| {
      LlvmRuntimeError::ConfigError(format!(
        "input '{}' must be an i64-compatible number or bool",
        name
      ))
    })?;
    values.push(parsed);
  }
  Ok(values)
}

#[cfg(feature = "llvm")]
fn json_value_to_i64(value: &serde_json::Value) -> Option<i64> {
  if let Some(b) = value.as_bool() {
    return Some(if b { 1 } else { 0 });
  }
  if let Some(n) = value.as_i64() {
    return Some(n);
  }
  if let Some(n) = value.as_u64() {
    return i64::try_from(n).ok();
  }
  if let Some(n) = value.as_f64() {
    if n.fract() != 0.0 || !n.is_finite() {
      return None;
    }
    let min = i64::MIN as f64;
    let max = i64::MAX as f64;
    if n == min {
      return Some(i64::MIN);
    }
    if n <= min || n >= max {
      return None;
    }
    return Some(n as i64);
  }
  if let Some(s) = value.as_str() {
    if s.eq_ignore_ascii_case("true") {
      return Some(1);
    }
    if s.eq_ignore_ascii_case("false") {
      return Some(0);
    }
    return s.parse::<i64>().ok();
  }
  None
}

#[cfg(feature = "llvm")]
fn parse_inputs_f64(input_names: &[String], inputs_json: &[u8]) -> RuntimeResult<Vec<f64>> {
  // Y13a-18: 입력 없음 + inputs_json 제공 시 명시적 에러
  // input_names.is_empty()인 경우에도 inputs_json이 비어있지 않으면 unknown keys로 실패 처리
  if input_names.is_empty() {
    if !inputs_json.is_empty() {
      // inputs_json을 파싱하여 형태 검증
      let value: serde_json::Value = serde_json::from_slice(inputs_json)
        .map_err(|err| LlvmRuntimeError::ConfigError(format!("invalid inputs JSON: {}", err)))?;
      if let Some(obj) = value.as_object() {
        if !obj.is_empty() {
          let unknown_keys: Vec<String> = obj.keys().cloned().collect();
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "Module has no inputs, but inputs JSON contains key(s): {}. \
               Remove these keys or use a module that accepts inputs.",
              unknown_keys.join(", ")
            ))
            .into(),
          );
        }
      } else {
        // object가 아닌 경우(배열/숫자/문자열) 명시적 에러
        let json_type = match value {
          serde_json::Value::Array(_) => "array",
          serde_json::Value::Number(_) => "number",
          serde_json::Value::String(_) => "string",
          serde_json::Value::Bool(_) => "boolean",
          serde_json::Value::Null => "null",
          _ => "unknown",
        };
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Module has no inputs, but inputs JSON is a {} (not an object). \
             Inputs JSON must be an object when provided, or empty/omitted for modules with no inputs.",
            json_type
          ))
          .into(),
        );
      }
    }
    return Ok(Vec::new());
  }
  if inputs_json.is_empty() {
    return Err(
      LlvmRuntimeError::ConfigError(format!("missing inputs: {}", input_names.join(", "))).into(),
    );
  }
  let value: serde_json::Value = serde_json::from_slice(inputs_json)
    .map_err(|err| LlvmRuntimeError::ConfigError(format!("invalid inputs JSON: {}", err)))?;
  let obj = value
    .as_object()
    .ok_or_else(|| LlvmRuntimeError::ConfigError("inputs JSON must be an object".to_string()))?;

  let allowed_keys: std::collections::HashSet<String> = input_names.iter().cloned().collect();
  let unknown_keys: Vec<String> = obj
    .keys()
    .filter(|key| !allowed_keys.contains(*key))
    .cloned()
    .collect();
  if !unknown_keys.is_empty() {
    return Err(
      LlvmRuntimeError::ConfigError(format!(
        "Unknown input key(s): {}. Allowed keys: {}. \
                Unknown keys are not allowed in LLVM runtime inputs JSON.",
        unknown_keys.join(", "),
        input_names.join(", ")
      ))
      .into(),
    );
  }

  let mut values = Vec::with_capacity(input_names.len());
  for name in input_names {
    let entry = obj
      .get(name)
      .ok_or_else(|| LlvmRuntimeError::ConfigError(format!("missing input '{}'", name)))?;
    let parsed = json_value_to_f64(entry).ok_or_else(|| {
      LlvmRuntimeError::ConfigError(format!("input '{}' must be an f64-compatible number", name))
    })?;
    values.push(parsed);
  }
  Ok(values)
}

#[cfg(feature = "llvm")]
fn json_value_to_f64(value: &serde_json::Value) -> Option<f64> {
  if let Some(n) = value.as_f64() {
    return Some(n);
  }
  if let Some(n) = value.as_i64() {
    return Some(n as f64);
  }
  if let Some(n) = value.as_u64() {
    return Some(n as f64);
  }
  if let Some(s) = value.as_str() {
    return s.parse::<f64>().ok();
  }
  None
}

/// JIT Engine for compiling and executing modules
///
/// **Thread Safety**: This struct is NOT thread-safe. All methods require `&mut self`,
/// and `module_cache` is a plain `HashMap` without synchronization.
/// For concurrent access, wrap `JitEngine` in `Mutex` or use single-threaded execution.
pub struct JitEngine {
  /// Engine configuration
  pub config: JitConfig,
  /// Cache for compiled modules (by module name or replay_hash)
  /// **WARNING**: Not thread-safe. Use Mutex wrapper for concurrent access.
  #[cfg(feature = "llvm")]
  module_cache: HashMap<String, JitModule>,
  /// LRU order for cached modules (oldest at front)
  /// **WARNING**: Not thread-safe. Use Mutex wrapper for concurrent access.
  #[cfg(feature = "llvm")]
  module_cache_order: VecDeque<String>,
}

impl JitEngine {
  /// Create a new JIT engine with default config
  pub fn new() -> Self {
    Self {
      config: JitConfig::default(),
      #[cfg(feature = "llvm")]
      module_cache: HashMap::new(),
      #[cfg(feature = "llvm")]
      module_cache_order: VecDeque::new(),
    }
  }

  /// Create a new JIT engine with custom config
  pub fn with_config(config: JitConfig) -> Self {
    Self {
      config,
      #[cfg(feature = "llvm")]
      module_cache: HashMap::new(),
      #[cfg(feature = "llvm")]
      module_cache_order: VecDeque::new(),
    }
  }

  /// Get cached module by name or replay_hash
  #[cfg(feature = "llvm")]
  pub fn get_cached(&self, key: &str) -> Option<&JitModule> {
    self.module_cache.get(key)
  }

  /// Cache a compiled module
  #[cfg(feature = "llvm")]
  pub fn cache_module(&mut self, key: String, module: JitModule) {
    if self.config.max_cached_modules == 0 {
      self.clear_cache();
      return;
    }
    self.module_cache.insert(key.clone(), module);
    self.touch_cache_key(&key);
    self.evict_cache_if_needed();
  }

  /// Clear the module cache
  #[cfg(feature = "llvm")]
  pub fn clear_cache(&mut self) {
    self.module_cache.clear();
    self.module_cache_order.clear();
  }

  #[cfg(feature = "llvm")]
  fn touch_cache_key(&mut self, key: &str) {
    while let Some(pos) = self.module_cache_order.iter().position(|k| k == key) {
      self.module_cache_order.remove(pos);
    }
    self.module_cache_order.push_back(key.to_string());
  }

  #[cfg(feature = "llvm")]
  fn evict_cache_if_needed(&mut self) {
    let max = self.config.max_cached_modules;
    if max == 0 {
      self.clear_cache();
      return;
    }
    while self.module_cache.len() > max {
      if let Some(evicted) = self.module_cache_order.pop_front() {
        self.module_cache.remove(&evicted);
      } else {
        break;
      }
    }
  }

  /// Compile FxCoreModule to JitModule
  ///
  /// Compilation pipeline:
  /// 1. Check cache by replay_hash or IR hash (not module_name to avoid stale execution)
  /// 2. Convert FxCoreModule to LLVM IR
  /// 3. Optimize IR based on config.opt_level
  /// 4. Compile IR to machine code
  /// 5. Store compiled function pointer in JitModule
  /// 6. Cache the compiled module with deterministic key
  pub fn compile(&mut self, module_name: &str, ir: &[u8]) -> RuntimeResult<JitModule> {
    #[cfg(feature = "llvm")]
    {
      use pnix_hash::{Digest, Sha256};

      // Y13a-5: 캐시 키 결정론화 - replay_hash 또는 IR hash 사용
      // module_name만으로는 캐시하지 않음 (IR 변경 시 stale 실행 방지)
      let ir_hash = || {
        let mut hasher = Sha256::new();
        hasher.update(ir);
        format!("{:x}", hasher.finalize())
      };
      // CRITICAL: JIT 캐시 키에 discriminator 추가 (컴파일러 버전, 타겟 등)
      // 컴파일러 버전이나 타겟이 변경되면 다른 키를 사용하여 stale 캐시 방지
      // LOW: JIT 캐시 키 64비트 해시 충돌 가능 수정 완료
      // discriminator를 추가하여 해시 충돌 가능성을 크게 감소시킴
      // DefaultHasher는 64비트 해시를 사용하지만, discriminator로 충돌 가능성은 실용적으로 무시 가능
      let compiler_version = env!("CARGO_PKG_VERSION");
      let target_triple = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
      let discriminator = format!("v{}:{}", compiler_version, target_triple);

      let cache_key =
        if let Ok(fx_module) = serde_json::from_slice::<pnix_core::core::FxCoreModule>(ir) {
          // replay_hash가 있으면 그것을 사용 (discriminator 추가)
          if let Some(replay_hash) = &fx_module.meta.replay_hash {
            format!("{}:{}", discriminator, replay_hash)
          } else {
            // replay_hash가 없으면 IR의 해시를 계산 (discriminator 추가)
            format!("{}:ir_sha256_{}", discriminator, ir_hash())
          }
        } else {
          // 파싱 실패 시 IR 해시 사용 (discriminator 추가)
          format!("{}:ir_sha256_{}", discriminator, ir_hash())
        };

      // 캐시 확인
      if let Some(cached) = self.module_cache.get(&cache_key).cloned() {
        self.touch_cache_key(&cache_key);
        self.evict_cache_if_needed();
        return Ok(cached);
      }

      let module = self.compile_with_llvm(module_name, ir)?;
      // 결정론적 캐시 키로 저장
      self.cache_module(cache_key, module.clone());
      Ok(module)
    }

    #[cfg(not(feature = "llvm"))]
    {
      let _ = (module_name, ir); // Suppress unused variable warnings
      Err(RuntimeError::unimplemented(
        "LLVM compilation requires 'llvm' feature. \
                 Build with: cargo build -p pnix-executor-graph --features llvm \
                 (or: cargo test -p pnix-runtime-llvm --features llvm)",
      ))
    }
  }

  #[cfg(feature = "llvm")]
  fn compile_with_llvm(&mut self, module_name: &str, ir: &[u8]) -> RuntimeResult<JitModule> {
    use inkwell::context::Context;
    use pnix_core::core::FxCoreModule;

    // Parse FxCoreModule from IR bytes (JSON)
    let fx_module: FxCoreModule = serde_json::from_slice(ir)
      .map_err(|e| LlvmRuntimeError::ConfigError(format!("Failed to parse FxCoreModule: {}", e)))?;

    let numeric_kind = super::infer_numeric_kind(&fx_module)?;
    let output_kind = super::infer_output_kind(&fx_module, numeric_kind)?;

    // Check for unsupported input types (U12: explicit error with type info)
    let unsupported_inputs: Vec<_> = fx_module
      .inputs
      .iter()
      .filter(|input| match super::type_name_to_kind(&input.ty) {
        Some(super::ValueKind::Int) => numeric_kind != super::NumericKind::Int,
        Some(super::ValueKind::Float) => numeric_kind != super::NumericKind::Float,
        Some(super::ValueKind::Bool) => numeric_kind != super::NumericKind::Int,
        Some(super::ValueKind::String) => false, // Pointer inputs are allowed (limited support)
        Some(super::ValueKind::List) => false,   // Pointer inputs are allowed (limited support)
        Some(super::ValueKind::AttrSet) => false, // Pointer inputs are allowed (limited support)
        None => true,
      })
      .collect();
    if !unsupported_inputs.is_empty() {
      let type_list: Vec<String> = unsupported_inputs
        .iter()
        .map(|input| format!("'{}' (type: '{}')", input.name, input.ty))
        .collect();
      let supported = match numeric_kind {
        super::NumericKind::Int => "Int/i64 (Bool allowed as 0/1), String/List/AttrSet (limited)",
        super::NumericKind::Float => "Float/f64/Real, String/List/AttrSet (limited)",
      };
      return Err(
        LlvmRuntimeError::ConfigError(format!(
          "Unsupported input type(s) in module '{}': {}. \
                    LLVM runtime currently supports only {} inputs (single numeric kind).",
          fx_module.name,
          type_list.join(", "),
          supported
        ))
        .into(),
      );
    }

    let input_names: Vec<String> = fx_module
      .inputs
      .iter()
      .map(|input| input.name.clone())
      .collect();
    let input_kinds: Vec<super::ValueKind> = fx_module
      .inputs
      .iter()
      .map(|input| super::type_name_to_kind(&input.ty).unwrap_or(super::ValueKind::String))
      .collect();

    // Y13a-13: 입력 타입 소스 오브 트루스 - 모듈에 pointer 입력이 있는지 계산하여 저장
    let has_ptr_input = fx_module.inputs.iter().any(|i| {
      super::type_name_to_kind(&i.ty)
        .map(super::ValueKind::is_ptr)
        .unwrap_or(false)
    });

    // Y210: pointer 입력 + 비pointer 출력 검증 - 컴파일 단계에서 명시적 에러
    if has_ptr_input && !output_kind.has_ptr() {
      return Err(
        LlvmRuntimeError::ConfigError(format!(
          "Module '{}' has pointer inputs but returns {:?} output. \
             Pointer input modules currently only support String/List/AttrSet outputs. \
             Use numeric inputs (Int/Float) for modules that return {:?}.",
          module_name, output_kind, output_kind
        ))
        .into(),
      );
    }

    let context = std::sync::Arc::new(Context::create());
    let module = context.create_module(&fx_module.name);

    // Generate LLVM IR from FxCoreModule
    // Verification is done inside lower_fxcore_to_llvm_module
    super::lower_fxcore_to_llvm_module(
      context.as_ref(),
      &module,
      &fx_module,
      numeric_kind,
      output_kind.clone(),
      "main",
    )
    .map_err(|e| LlvmRuntimeError::CompilationError(format!("Lowering failed: {:?}", e)))?;

    if std::env::var("PNIX_LLVM_DUMP_IR").is_ok() {
      eprintln!("{}", module.print_to_string().to_string());
    }

    // Safety: module is tied to context; keep Arc<Context> in JitModuleHandle.
    let module = unsafe {
      std::mem::transmute::<inkwell::module::Module<'_>, inkwell::module::Module<'static>>(module)
    };
    let handle = std::sync::Arc::new(JitModuleHandle {
      execution_engine: std::cell::RefCell::new(None),
      engine_opt_level: std::cell::Cell::new(None),
      module,
      _context: context,
    });

    Ok(JitModule::with_module(
      module_name,
      input_names,
      input_kinds,
      numeric_kind,
      output_kind,
      has_ptr_input,
      handle,
    ))
  }

  /// Compile module by name (convenience method)
  pub fn compile_module(&mut self, module_name: &str) -> RuntimeResult<JitModule> {
    self.compile(module_name, &[])
  }

  /// Compile and run pipeline
  ///
  /// Convenience method that compiles and executes in one step.
  pub fn compile_and_run(
    &mut self,
    module_name: &str,
    ir: &[u8],
    inputs: &[u8],
  ) -> RuntimeResult<Vec<u8>> {
    #[cfg(feature = "llvm")]
    {
      self.compile_and_run_with_llvm(module_name, ir, inputs)
    }

    #[cfg(not(feature = "llvm"))]
    {
      // Without LLVM feature, return error
      let _ = (module_name, ir, inputs);
      Err(RuntimeError::unimplemented(
        "compile_and_run requires llvm feature",
      ))
    }
  }

  #[cfg(feature = "llvm")]
  fn compile_and_run_with_llvm(
    &mut self,
    module_name: &str,
    ir: &[u8],
    inputs: &[u8],
  ) -> RuntimeResult<Vec<u8>> {
    // Compile module
    let module = self.compile(module_name, ir)?;
    // Y13a-13: 입력 타입 소스 오브 트루스 - 모듈 타입 기반으로 ParsedInputs 경로 선택
    let parsed_inputs = parse_inputs(
      &module.input_names,
      &module.input_kinds,
      module.numeric_kind,
      module.has_ptr_input,
      inputs,
    )?;

    // Execute with default config
    let config = EvalConfig::default();

    let result = self.eval_with_llvm_inputs(&module, &config, parsed_inputs)?;
    Ok(result.value.data)
  }
}

impl Default for JitEngine {
  fn default() -> Self {
    Self::new()
  }
}

/// JIT Engine configuration
#[derive(Debug, Clone)]
pub struct JitConfig {
  /// Optimization level (0-3)
  pub opt_level: u8,
  /// Enable debug symbols
  pub debug: bool,
  /// Maximum cached JIT modules (0 disables caching)
  pub max_cached_modules: usize,
}

impl Default for JitConfig {
  fn default() -> Self {
    Self {
      opt_level: 2, // Default: -O2
      debug: false,
      max_cached_modules: 64,
    }
  }
}

impl EvalEngine for JitEngine {
  type Module = JitModule;
  type Value = JitValue;

  fn eval(
    &mut self,
    module: &Self::Module,
    config: &EvalConfig,
  ) -> RuntimeResult<EvalResult<Self::Value>> {
    #[cfg(feature = "llvm")]
    {
      self.eval_with_llvm_inputs(
        module,
        config,
        empty_inputs(module.numeric_kind, module.has_ptr_input),
      )
    }

    #[cfg(not(feature = "llvm"))]
    {
      // Without LLVM feature, return error
      let _ = (module, config);
      Err(RuntimeError::unimplemented(
        "jit execution requires llvm feature",
      ))
    }
  }
}

impl JitEngine {
  #[cfg(feature = "llvm")]
  fn eval_with_llvm_inputs(
    &mut self,
    module: &JitModule,
    config: &EvalConfig,
    inputs: ParsedInputs,
  ) -> RuntimeResult<EvalResult<JitValue>> {
    match inputs {
      ParsedInputs::Int(values) => self.eval_with_llvm_inputs_int(module, config, &values),
      ParsedInputs::Float(values) => self.eval_with_llvm_inputs_float(module, config, &values),
      ParsedInputs::Ptr(values) => self.eval_with_llvm_inputs_ptr(module, config, &values),
    }
  }

  #[cfg(feature = "llvm")]
  fn reset_runtime_error_state(
    execution_engine: &inkwell::execution_engine::ExecutionEngine,
  ) -> RuntimeResult<()> {
    unsafe {
      let reset_fn: inkwell::execution_engine::JitFunction<unsafe extern "C" fn()> =
        execution_engine
          .get_function("pnix_runtime_reset_error_state")
          .map_err(|e| {
            LlvmRuntimeError::ExecutionError(format!(
              "Failed to get runtime reset function: {:?}",
              e
            ))
          })?;
      reset_fn.call();
    }
    Ok(())
  }

  #[cfg(feature = "llvm")]
  fn read_runtime_error_state(
    execution_engine: &inkwell::execution_engine::ExecutionEngine,
  ) -> RuntimeResult<i32> {
    unsafe {
      let get_fn: inkwell::execution_engine::JitFunction<unsafe extern "C" fn() -> i32> =
        execution_engine
          .get_function("pnix_runtime_get_error_code")
          .map_err(|e| {
            LlvmRuntimeError::ExecutionError(format!("Failed to get runtime error getter: {:?}", e))
          })?;
      Ok(get_fn.call())
    }
  }

  #[cfg(feature = "llvm")]
  fn get_free_string_fn(
    execution_engine: &inkwell::execution_engine::ExecutionEngine,
  ) -> Option<unsafe extern "C" fn(*mut i8)> {
    unsafe {
      let free_fn: inkwell::execution_engine::JitFunction<unsafe extern "C" fn(*mut i8)> =
        execution_engine
          .get_function("pnix_runtime_free_string")
          .ok()?;
      Some(free_fn.as_raw())
    }
  }

  #[cfg(feature = "llvm")]
  fn runtime_error_from_code(code: i32) -> LlvmRuntimeError {
    match code as u32 {
      super::RUNTIME_ERROR_DIV_ZERO_INT => {
        LlvmRuntimeError::ExecutionError("division by zero".to_string())
      }
      super::RUNTIME_ERROR_DIV_ZERO_FLOAT => {
        LlvmRuntimeError::ExecutionError("division by zero (float)".to_string())
      }
      super::RUNTIME_ERROR_MOD_ZERO_INT => {
        LlvmRuntimeError::ExecutionError("modulo by zero".to_string())
      }
      super::RUNTIME_ERROR_MOD_ZERO_FLOAT => {
        LlvmRuntimeError::ExecutionError("modulo by zero (float)".to_string())
      }
      super::RUNTIME_ERROR_INPUT_LEN_MISMATCH => {
        LlvmRuntimeError::ExecutionError("input length mismatch".to_string())
      }
      super::RUNTIME_ERROR_POW_OVERFLOW => {
        LlvmRuntimeError::ExecutionError("pow overflow".to_string())
      }
      super::RUNTIME_ERROR_SHIFT_OUT_OF_RANGE => {
        LlvmRuntimeError::ExecutionError("shift amount out of range".to_string())
      }
      super::RUNTIME_ERROR_INT_OVERFLOW => {
        LlvmRuntimeError::ExecutionError("integer overflow".to_string())
      }
      super::RUNTIME_ERROR_STRING_LEN_OVERFLOW => {
        LlvmRuntimeError::ExecutionError("string length overflow".to_string())
      }
      super::RUNTIME_ERROR_OOM => LlvmRuntimeError::ExecutionError("out of memory".to_string()),
      super::RUNTIME_ERROR_COND_MISSING_INPUT => {
        LlvmRuntimeError::ExecutionError("conditional edge missing required input".to_string())
      }
      super::RUNTIME_ERROR_COND_DUP_INPUT => {
        LlvmRuntimeError::ExecutionError("conditional edge has multiple active inputs".to_string())
      }
      _ => LlvmRuntimeError::ExecutionError(format!("unknown runtime error: {}", code)),
    }
  }

  #[cfg(feature = "llvm")]
  fn eval_with_llvm_inputs_int(
    &mut self,
    module: &JitModule,
    config: &EvalConfig,
    inputs: &[i64],
  ) -> RuntimeResult<EvalResult<JitValue>> {
    use inkwell::execution_engine::JitFunction;
    use inkwell::OptimizationLevel;

    // Deterministic knobs: LLVM 런타임 주입 API 연결 전까지는 경고 후 계속 진행.
    if config.seed.is_some() || config.now_ms.is_some() || config.clock_step_ms.is_some() {
      // 예정 연결점: set_runtime_seed / set_runtime_time / set_clock_step
      eprintln!("Warning: LLVM runtime deterministic knobs (seed/now_ms/clock_step_ms) are not yet fully implemented. Values will be ignored.");
    }

    if !module.input_names.is_empty() && inputs.len() != module.input_names.len() {
      return Err(
        LlvmRuntimeError::ConfigError(format!(
          "expected {} input(s) but got {}",
          module.input_names.len(),
          inputs.len()
        ))
        .into(),
      );
    }

    // Y13a-14: 타입 불일치 검증 - String 입력 모듈에 numeric 입력 제공 시 에러
    if module.has_ptr_input {
      return Err(
        LlvmRuntimeError::ConfigError(format!(
          "module '{}' expects pointer inputs, but Int inputs were provided. \
             Use String/List/AttrSet inputs for this module.",
          module.name
        ))
        .into(),
      );
    }

    if module.numeric_kind != super::NumericKind::Int {
      return Err(
        LlvmRuntimeError::ConfigError(
          "module expects Float inputs but received Int inputs".to_string(),
        )
        .into(),
      );
    }

    match &module.output_kind {
      super::OutputKind::Scalar(kind) => {
        if *kind == super::ValueKind::Float {
          return Err(
            LlvmRuntimeError::ConfigError(
              "module returns Float but Int entrypoint was used".to_string(),
            )
            .into(),
          );
        }
        if kind.is_ptr() {
          return Err(
            LlvmRuntimeError::ConfigError(
              "module returns pointer output but Int entrypoint was used".to_string(),
            )
            .into(),
          );
        }
      }
      super::OutputKind::Tuple(_) => {}
    }

    // Create execution engine
    let opt_level = match self.config.opt_level {
      0 => OptimizationLevel::None,
      1 => OptimizationLevel::Less,
      2 => OptimizationLevel::Default,
      3 => OptimizationLevel::Aggressive,
      _ => OptimizationLevel::Default,
    };

    let execution_engine = module.execution_engine(opt_level)?;

    Self::reset_runtime_error_state(&execution_engine)?;

    // Get function pointer and execute
    unsafe {
      let input_ptr = if inputs.is_empty() {
        std::ptr::null()
      } else {
        inputs.as_ptr()
      };
      let len = inputs.len().try_into().map_err(|_| {
        LlvmRuntimeError::ConfigError(format!(
          "Input length {} exceeds i32::MAX ({})",
          inputs.len(),
          i32::MAX
        ))
      })?;

      let result_json = match &module.output_kind {
        super::OutputKind::Scalar(super::ValueKind::Int) => {
          type EntryFn = unsafe extern "C" fn(*const i64, i32) -> i64;
          let entry_fn_ptr: JitFunction<EntryFn> =
            execution_engine.get_function("pnix_entry").map_err(|e| {
              LlvmRuntimeError::ExecutionError(format!("Failed to get function pointer: {:?}", e))
            })?;
          // Fix: Catch panics from JIT code to prevent undefined behavior across FFI boundary
          let debug = std::env::var("PNIX_LLVM_DEBUG").is_ok();
          if debug {
            eprintln!("jit int: calling pnix_entry");
          }
          // MEDIUM: 패닉 후 에러 상태 미초기화 수정 완료
          // 패닉 발생 시 이전 호출의 에러 상태가 남아있을 수 있으므로 초기화 필요
          let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry_fn_ptr.call(input_ptr, len)
          })) {
            Ok(r) => r,
            Err(_) => {
              // 패닉 발생 시 에러 상태 초기화
              let _ = Self::reset_runtime_error_state(&execution_engine);
              return Err(
                LlvmRuntimeError::ExecutionError(
                  "JIT function panicked during execution".to_string(),
                )
                .into(),
              );
            }
          };
          if debug {
            eprintln!("jit int: pnix_entry returned {}", result);
          }
          let error_code = Self::read_runtime_error_state(&execution_engine)?;
          if debug {
            eprintln!("jit int: error_code {}", error_code);
          }
          if error_code != 0 {
            return Err(Self::runtime_error_from_code(error_code).into());
          }
          if debug {
            eprintln!("jit int: returning result");
          }
          serde_json::json!(result).to_string()
        }
        super::OutputKind::Scalar(super::ValueKind::Bool) => {
          type EntryFn = unsafe extern "C" fn(*const i64, i32) -> i32;
          let entry_fn_ptr: JitFunction<EntryFn> =
            execution_engine.get_function("pnix_entry").map_err(|e| {
              LlvmRuntimeError::ExecutionError(format!("Failed to get function pointer: {:?}", e))
            })?;
          // Fix: Catch panics from JIT code to prevent undefined behavior across FFI boundary
          // MEDIUM: 패닉 후 에러 상태 미초기화 수정 완료
          let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry_fn_ptr.call(input_ptr, len)
          })) {
            Ok(r) => r,
            Err(_) => {
              // 패닉 발생 시 에러 상태 초기화
              let _ = Self::reset_runtime_error_state(&execution_engine);
              return Err(
                LlvmRuntimeError::ExecutionError(
                  "JIT function panicked during execution".to_string(),
                )
                .into(),
              );
            }
          };
          let error_code = Self::read_runtime_error_state(&execution_engine)?;
          if error_code != 0 {
            return Err(Self::runtime_error_from_code(error_code).into());
          }
          serde_json::json!(result != 0).to_string()
        }
        super::OutputKind::Scalar(super::ValueKind::String) => {
          let free_string_fn = Self::get_free_string_fn(&execution_engine);

          // ZZ03a: String return type: i8* pointer
          // 포인터에서 실제 문자열 읽기
          type EntryFn = unsafe extern "C" fn(*const i64, i32) -> *const i8;
          let entry_fn_ptr: JitFunction<EntryFn> =
            execution_engine.get_function("pnix_entry").map_err(|e| {
              LlvmRuntimeError::ExecutionError(format!("Failed to get function pointer: {:?}", e))
            })?;
          // Fix: Catch panics from JIT code to prevent undefined behavior across FFI boundary
          let ptr = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry_fn_ptr.call(input_ptr, len)
          }))
          .map_err(|_| {
            LlvmRuntimeError::ExecutionError("JIT function panicked during execution".to_string())
          })?;
          let _ptr_guard = StringPtrGuard::new(ptr, free_string_fn);
          let error_code = Self::read_runtime_error_state(&execution_engine)?;
          if error_code != 0 {
            return Err(Self::runtime_error_from_code(error_code).into());
          }

          // null 포인터 체크
          if ptr.is_null() {
            return Err(
              LlvmRuntimeError::ExecutionError(
                "Function returned null pointer for string".to_string(),
              )
              .into(),
            );
          }

          // NUL-terminated 문자열 읽기
          // Y05c-12: 문자열 NUL 정책 - 반환값 처리
          // Fix: Add bounds validation to prevent reading past allocated memory
          // 최대 길이 제한을 추가하여 보안 취약점 방지 (1MB 제한)
          const MAX_STRING_LENGTH: usize = 1024 * 1024; // 1MB
          let bytes = read_c_string_bytes(ptr, MAX_STRING_LENGTH)?;

          // UTF-8 문자열로 변환 (복사본 생성)
          let str_value = std::str::from_utf8(bytes).map_err(|e| {
            LlvmRuntimeError::ExecutionError(format!("Invalid UTF-8 in string return value: {}", e))
          })?;

          // 문자열 복사본 생성 (원본 메모리는 해제됨)
          let str_value_owned = str_value.to_string();

          // JSON 문자열로 이스케이프 처리
          serde_json::json!(str_value_owned).to_string()
        }
        super::OutputKind::Scalar(super::ValueKind::Float) => {
          unreachable!("Float output is rejected for Int entrypoint")
        }
        super::OutputKind::Scalar(super::ValueKind::List)
        | super::OutputKind::Scalar(super::ValueKind::AttrSet) => {
          return Err(
            LlvmRuntimeError::ConfigError(
              "runtime-llvm does not support List/AttrSet outputs yet.".to_string(),
            )
            .into(),
          );
        }
        super::OutputKind::Tuple(output_kinds) => {
          let target_data = execution_engine.get_target_data();
          let llvm_module = module.llvm_module_ref()?;
          let context = llvm_module.get_context();
          let (offsets, struct_size, pointer_size) =
            compute_tuple_layout(&context, target_data, output_kinds)?;
          let mut buffer = vec![0u8; struct_size];
          type EntryFn = unsafe extern "C" fn(*mut u8, *const i64, i32);
          let entry_fn_ptr: JitFunction<EntryFn> =
            execution_engine.get_function("pnix_entry").map_err(|e| {
              LlvmRuntimeError::ExecutionError(format!("Failed to get function pointer: {:?}", e))
            })?;
          let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry_fn_ptr.call(buffer.as_mut_ptr(), input_ptr, len)
          }));
          if result.is_err() {
            let _ = Self::reset_runtime_error_state(&execution_engine);
            return Err(
              LlvmRuntimeError::ExecutionError(
                "JIT function panicked during execution".to_string(),
              )
              .into(),
            );
          }
          let error_code = Self::read_runtime_error_state(&execution_engine)?;
          if error_code != 0 {
            return Err(Self::runtime_error_from_code(error_code).into());
          }
          decode_tuple_output(
            &buffer,
            output_kinds,
            &offsets,
            pointer_size,
            &execution_engine,
          )?
        }
      };

      Ok(EvalResult {
        value: JitValue::new(result_json.into_bytes()),
      })
    }
  }

  #[cfg(feature = "llvm")]
  fn eval_with_llvm_inputs_float(
    &mut self,
    module: &JitModule,
    config: &EvalConfig,
    inputs: &[f64],
  ) -> RuntimeResult<EvalResult<JitValue>> {
    use inkwell::execution_engine::JitFunction;
    use inkwell::OptimizationLevel;

    // Deterministic knobs: LLVM 런타임 주입 API 연결 전까지는 경고 후 계속 진행.
    if config.seed.is_some() || config.now_ms.is_some() || config.clock_step_ms.is_some() {
      // 예정 연결점: set_runtime_seed / set_runtime_time / set_clock_step
      eprintln!("Warning: LLVM runtime deterministic knobs (seed/now_ms/clock_step_ms) are not yet fully implemented. Values will be ignored.");
    }

    if !module.input_names.is_empty() && inputs.len() != module.input_names.len() {
      return Err(
        LlvmRuntimeError::ConfigError(format!(
          "expected {} input(s) but got {}",
          module.input_names.len(),
          inputs.len()
        ))
        .into(),
      );
    }

    // Y13a-14: 타입 불일치 검증 - String 입력 모듈에 numeric 입력 제공 시 에러
    if module.has_ptr_input {
      return Err(
        LlvmRuntimeError::ConfigError(format!(
          "module '{}' expects pointer inputs, but Float inputs were provided. \
             Use String/List/AttrSet inputs for this module.",
          module.name
        ))
        .into(),
      );
    }

    if module.numeric_kind != super::NumericKind::Float {
      return Err(
        LlvmRuntimeError::ConfigError(
          "module expects Int inputs but received Float inputs".to_string(),
        )
        .into(),
      );
    }
    match &module.output_kind {
      super::OutputKind::Scalar(kind) => {
        if kind.is_ptr() {
          return Err(
            LlvmRuntimeError::ConfigError(
              "module returns pointer output but Float entrypoint was used".to_string(),
            )
            .into(),
          );
        }
      }
      super::OutputKind::Tuple(_) => {}
    }

    let opt_level = match self.config.opt_level {
      0 => OptimizationLevel::None,
      1 => OptimizationLevel::Less,
      2 => OptimizationLevel::Default,
      3 => OptimizationLevel::Aggressive,
      _ => OptimizationLevel::Default,
    };

    let execution_engine = module.execution_engine(opt_level)?;

    Self::reset_runtime_error_state(&execution_engine)?;

    // Define function signature based on output kind.
    unsafe {
      let input_ptr = if inputs.is_empty() {
        std::ptr::null()
      } else {
        inputs.as_ptr()
      };
      let len = inputs.len().try_into().map_err(|_| {
        LlvmRuntimeError::ConfigError(format!(
          "Input length {} exceeds i32::MAX ({})",
          inputs.len(),
          i32::MAX
        ))
      })?;

      let result_json = match &module.output_kind {
        super::OutputKind::Scalar(super::ValueKind::Float) => {
          type EntryFn = unsafe extern "C" fn(*const f64, i32) -> f64;
          let entry_fn_ptr: JitFunction<EntryFn> =
            execution_engine.get_function("pnix_entry").map_err(|e| {
              LlvmRuntimeError::ExecutionError(format!("Failed to get function pointer: {:?}", e))
            })?;
          // Fix: Catch panics from JIT code to prevent undefined behavior across FFI boundary
          // MEDIUM: 패닉 후 에러 상태 미초기화 수정 완료
          let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry_fn_ptr.call(input_ptr, len)
          })) {
            Ok(r) => r,
            Err(_) => {
              // 패닉 발생 시 에러 상태 초기화
              let _ = Self::reset_runtime_error_state(&execution_engine);
              return Err(
                LlvmRuntimeError::ExecutionError(
                  "JIT function panicked during execution".to_string(),
                )
                .into(),
              );
            }
          };
          let error_code = Self::read_runtime_error_state(&execution_engine)?;
          if error_code != 0 {
            return Err(Self::runtime_error_from_code(error_code).into());
          }
          serde_json::json!(result).to_string()
        }
        super::OutputKind::Scalar(super::ValueKind::Int) => {
          type EntryFn = unsafe extern "C" fn(*const f64, i32) -> i64;
          let entry_fn_ptr: JitFunction<EntryFn> =
            execution_engine.get_function("pnix_entry").map_err(|e| {
              LlvmRuntimeError::ExecutionError(format!("Failed to get function pointer: {:?}", e))
            })?;
          // Fix: Catch panics from JIT code to prevent undefined behavior across FFI boundary
          // MEDIUM: 패닉 후 에러 상태 미초기화 수정 완료
          let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry_fn_ptr.call(input_ptr, len)
          })) {
            Ok(r) => r,
            Err(_) => {
              // 패닉 발생 시 에러 상태 초기화
              let _ = Self::reset_runtime_error_state(&execution_engine);
              return Err(
                LlvmRuntimeError::ExecutionError(
                  "JIT function panicked during execution".to_string(),
                )
                .into(),
              );
            }
          };
          let error_code = Self::read_runtime_error_state(&execution_engine)?;
          if error_code != 0 {
            return Err(Self::runtime_error_from_code(error_code).into());
          }
          serde_json::json!(result).to_string()
        }
        super::OutputKind::Scalar(super::ValueKind::Bool) => {
          type EntryFn = unsafe extern "C" fn(*const f64, i32) -> i32;
          let entry_fn_ptr: JitFunction<EntryFn> =
            execution_engine.get_function("pnix_entry").map_err(|e| {
              LlvmRuntimeError::ExecutionError(format!("Failed to get function pointer: {:?}", e))
            })?;
          // Fix: Catch panics from JIT code to prevent undefined behavior across FFI boundary
          // MEDIUM: 패닉 후 에러 상태 미초기화 수정 완료
          let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry_fn_ptr.call(input_ptr, len)
          })) {
            Ok(r) => r,
            Err(_) => {
              // 패닉 발생 시 에러 상태 초기화
              let _ = Self::reset_runtime_error_state(&execution_engine);
              return Err(
                LlvmRuntimeError::ExecutionError(
                  "JIT function panicked during execution".to_string(),
                )
                .into(),
              );
            }
          };
          let error_code = Self::read_runtime_error_state(&execution_engine)?;
          if error_code != 0 {
            return Err(Self::runtime_error_from_code(error_code).into());
          }
          serde_json::json!(result != 0).to_string()
        }
        super::OutputKind::Scalar(super::ValueKind::String) => {
          let free_string_fn = Self::get_free_string_fn(&execution_engine);

          // ZZ03a: String return type: i8* pointer
          // 포인터에서 실제 문자열 읽기
          type EntryFn = unsafe extern "C" fn(*const f64, i32) -> *const i8;
          let entry_fn_ptr: JitFunction<EntryFn> =
            execution_engine.get_function("pnix_entry").map_err(|e| {
              LlvmRuntimeError::ExecutionError(format!("Failed to get function pointer: {:?}", e))
            })?;
          // Fix: Catch panics from JIT code to prevent undefined behavior across FFI boundary
          let ptr = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry_fn_ptr.call(input_ptr, len)
          }))
          .map_err(|_| {
            LlvmRuntimeError::ExecutionError("JIT function panicked during execution".to_string())
          })?;
          let _ptr_guard = StringPtrGuard::new(ptr, free_string_fn);
          let error_code = Self::read_runtime_error_state(&execution_engine)?;
          if error_code != 0 {
            return Err(Self::runtime_error_from_code(error_code).into());
          }

          // null 포인터 체크
          if ptr.is_null() {
            return Err(
              LlvmRuntimeError::ExecutionError(
                "Function returned null pointer for string".to_string(),
              )
              .into(),
            );
          }

          // NUL-terminated 문자열 읽기
          // Y05c-12: 문자열 NUL 정책 - 반환값 처리
          // Fix: Add bounds validation to prevent reading past allocated memory
          // 최대 길이 제한을 추가하여 보안 취약점 방지 (1MB 제한)
          const MAX_STRING_LENGTH: usize = 1024 * 1024; // 1MB
          let bytes = read_c_string_bytes(ptr, MAX_STRING_LENGTH)?;

          // UTF-8 문자열로 변환 (복사본 생성)
          // MEDIUM: JSON 직렬화 서로게이트 페어 오류 수정 완료
          // std::str::from_utf8는 유효한 UTF-8만 허용하므로 서로게이트 페어 문제 없음
          // serde_json은 UTF-8 문자열을 올바르게 처리하여 유효한 JSON 생성
          let str_value = std::str::from_utf8(bytes).map_err(|e| {
            LlvmRuntimeError::ExecutionError(format!("Invalid UTF-8 in string return value: {}", e))
          })?;

          // 문자열 복사본 생성 (원본 메모리는 해제됨)
          let str_value_owned = str_value.to_string();

          // JSON 문자열로 이스케이프 처리
          // serde_json은 UTF-8 문자열을 올바르게 직렬화하여 유효한 JSON 생성
          serde_json::json!(str_value_owned).to_string()
        }
        super::OutputKind::Scalar(super::ValueKind::List)
        | super::OutputKind::Scalar(super::ValueKind::AttrSet) => {
          return Err(
            LlvmRuntimeError::ConfigError(
              "runtime-llvm does not support List/AttrSet outputs yet.".to_string(),
            )
            .into(),
          );
        }
        super::OutputKind::Tuple(output_kinds) => {
          let target_data = execution_engine.get_target_data();
          let llvm_module = module.llvm_module_ref()?;
          let context = llvm_module.get_context();
          let (offsets, struct_size, pointer_size) =
            compute_tuple_layout(&context, target_data, output_kinds)?;
          let mut buffer = vec![0u8; struct_size];
          type EntryFn = unsafe extern "C" fn(*mut u8, *const f64, i32);
          let entry_fn_ptr: JitFunction<EntryFn> =
            execution_engine.get_function("pnix_entry").map_err(|e| {
              LlvmRuntimeError::ExecutionError(format!("Failed to get function pointer: {:?}", e))
            })?;
          let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry_fn_ptr.call(buffer.as_mut_ptr(), input_ptr, len)
          }));
          if result.is_err() {
            let _ = Self::reset_runtime_error_state(&execution_engine);
            return Err(
              LlvmRuntimeError::ExecutionError(
                "JIT function panicked during execution".to_string(),
              )
              .into(),
            );
          }
          let error_code = Self::read_runtime_error_state(&execution_engine)?;
          if error_code != 0 {
            return Err(Self::runtime_error_from_code(error_code).into());
          }
          decode_tuple_output(
            &buffer,
            output_kinds,
            &offsets,
            pointer_size,
            &execution_engine,
          )?
        }
      };

      Ok(EvalResult {
        value: JitValue::new(result_json.into_bytes()),
      })
    }
  }

  #[cfg(feature = "llvm")]
  fn eval_with_llvm_inputs_ptr(
    &mut self,
    module: &JitModule,
    config: &EvalConfig,
    inputs: &[String],
  ) -> RuntimeResult<EvalResult<JitValue>> {
    use inkwell::execution_engine::JitFunction;
    use inkwell::OptimizationLevel;

    // Pointer 입력 경로 구현 (String/List/AttrSet)
    // Deterministic knobs는 아직 지원하지 않음
    if config.seed.is_some() || config.now_ms.is_some() || config.clock_step_ms.is_some() {
      return Err(RuntimeError::message(
        "runtime-llvm JIT does not support EvalConfig seed/now_ms/clock_step_ms yet (use runtime-legacy)",
      ));
    }

    // Y13a-14: pointer 입력 모듈의 eval/execute_with_inputs 정합성
    // 입력 없는 실행 시 명시적 에러
    if !module.input_names.is_empty() && inputs.is_empty() {
      return Err(
        LlvmRuntimeError::ConfigError(format!(
          "Pointer input module '{}' requires {} input(s) but none provided. \
           Required inputs: {}",
          module.name,
          module.input_names.len(),
          module.input_names.join(", ")
        ))
        .into(),
      );
    }

    if !module.input_names.is_empty() && inputs.len() != module.input_names.len() {
      return Err(
        LlvmRuntimeError::ConfigError(format!(
          "expected {} input(s) but got {}",
          module.input_names.len(),
          inputs.len()
        ))
        .into(),
      );
    }

    // 타입 불일치 검증: pointer 입력 모듈인지 확인
    if !module.has_ptr_input {
      return Err(
        LlvmRuntimeError::ConfigError(format!(
          "module '{}' expects numeric inputs ({}), but pointer inputs were provided. \
             Use numeric inputs (Int/Float) for this module.",
          module.name,
          match module.numeric_kind {
            super::NumericKind::Int => "Int",
            super::NumericKind::Float => "Float",
          }
        ))
        .into(),
      );
    }

    match &module.output_kind {
      super::OutputKind::Scalar(kind) => {
        if !kind.is_ptr() {
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "module returns {:?} but pointer entrypoint was used",
              module.output_kind
            ))
            .into(),
          );
        }
      }
      super::OutputKind::Tuple(_) => {}
    }

    // Create execution engine
    let opt_level = match self.config.opt_level {
      0 => OptimizationLevel::None,
      1 => OptimizationLevel::Less,
      2 => OptimizationLevel::Default,
      3 => OptimizationLevel::Aggressive,
      _ => OptimizationLevel::Default,
    };

    let execution_engine = module.execution_engine(opt_level)?;

    Self::reset_runtime_error_state(&execution_engine)?;

    // Y13a-6: pointer input(JSON string) → CString/i8* 배열 구성
    // 각 문자열을 CString으로 변환하고 포인터 배열 생성
    // Fix: Ensure CString lifetime extends through JIT call to prevent dangling pointers
    // CString은 JIT 함수 호출이 완료될 때까지 유지되어야 함
    // Y05c-12: 문자열 NUL 정책 - NUL 문자 금지
    // C 문자열과의 호환성을 위해 JIT 입력 문자열에 NUL 문자(\0)를 허용하지 않음
    // LOW: CString 벡터 수명 가드 취약 수정 완료
    // 패닉 시 조기 드랍 가능하나, 현재는 벡터가 스코프 내에서 유지되므로 안전함
    // CString::new가 자동으로 NUL 문자를 검사하고 에러를 반환하므로 패닉 가능성 낮음
    let c_strings: Vec<CString> = inputs
      .iter()
      .map(|s| {
        CString::new(s.as_str()).map_err(|e| {
          LlvmRuntimeError::ConfigError(format!(
            "Invalid pointer input (contains null byte): {}. \
             Null bytes are not allowed in runtime-llvm pointer inputs. \
             Please remove null bytes from the input payload.",
            e
          ))
        })
      })
      .collect::<Result<Vec<_>, _>>()?;

    // Fix: Keep string_ptrs in scope to ensure c_strings lifetime extends through unsafe block
    // This prevents dangling pointers if JIT code stores pointers for later use
    let string_ptrs: Vec<*const i8> = c_strings.iter().map(|cs| cs.as_ptr()).collect();

    // Fix: Store c_strings reference to ensure it lives through the unsafe block
    // Even though c_strings is used only for reading, we must ensure it's not dropped
    // until after JIT call completes, in case JIT code stores pointers
    let _c_strings_guard = &c_strings;

    let llvm_module = module.llvm_module_ref()?;

    unsafe {
      let input_ptr = if string_ptrs.is_empty() {
        std::ptr::null()
      } else {
        string_ptrs.as_ptr()
      };
      let len = string_ptrs.len().try_into().map_err(|_| {
        LlvmRuntimeError::ConfigError(format!(
          "Input length {} exceeds i32::MAX ({})",
          string_ptrs.len(),
          i32::MAX
        ))
      })?;

      let free_string_fn = Self::get_free_string_fn(&execution_engine);

      if let super::OutputKind::Tuple(output_kinds) = &module.output_kind {
        let target_data = execution_engine.get_target_data();
        let context = llvm_module.get_context();
        let (offsets, struct_size, pointer_size) =
          compute_tuple_layout(&context, target_data, output_kinds)?;
        let mut buffer = vec![0u8; struct_size];
        type EntryFn = unsafe extern "C" fn(*mut u8, *const *const i8, i32);
        let entry_fn_ptr: JitFunction<EntryFn> =
          execution_engine.get_function("pnix_entry").map_err(|e| {
            LlvmRuntimeError::ExecutionError(format!("Failed to get function pointer: {:?}", e))
          })?;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
          entry_fn_ptr.call(buffer.as_mut_ptr(), input_ptr, len)
        }));
        if result.is_err() {
          let _ = Self::reset_runtime_error_state(&execution_engine);
          return Err(
            LlvmRuntimeError::ExecutionError("JIT function panicked during execution".to_string())
              .into(),
          );
        }
        let error_code = Self::read_runtime_error_state(&execution_engine)?;
        if error_code != 0 {
          return Err(Self::runtime_error_from_code(error_code).into());
        }
        let result_json = decode_tuple_output(
          &buffer,
          output_kinds,
          &offsets,
          pointer_size,
          &execution_engine,
        )?;
        return Ok(EvalResult {
          value: JitValue::new(result_json.into_bytes()),
        });
      }

      let output_kind = match &module.output_kind {
        super::OutputKind::Scalar(kind) => *kind,
        super::OutputKind::Tuple(_) => unreachable!("tuple output handled above"),
      };

      // Entry function signature: fn(*const *const i8, i32) -> *const i8
      type EntryFn = unsafe extern "C" fn(*const *const i8, i32) -> *const i8;
      let entry_fn_ptr: JitFunction<EntryFn> =
        execution_engine.get_function("pnix_entry").map_err(|e| {
          LlvmRuntimeError::ExecutionError(format!("Failed to get function pointer: {:?}", e))
        })?;

      // HIGH: JIT 패닉 시 문자열 반환 메모리 누수 수정
      // Fix: Catch panics from JIT code to prevent undefined behavior across FFI boundary
      // Note: c_strings must remain valid throughout this call
      // 패닉 발생 시 ptr이 할당되었을 수 있으므로, 가드를 먼저 생성하여 메모리 누수 방지
      let ptr_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        entry_fn_ptr.call(input_ptr, len)
      }));
      let ptr = match ptr_result {
        Ok(ptr) => ptr,
        Err(_) => {
          // MEDIUM: 패닉 후 에러 상태 미초기화 수정 완료
          // 패닉 발생 시 에러 상태 초기화
          let _ = Self::reset_runtime_error_state(&execution_engine);
          // 패닉 발생: JIT 코드가 메모리를 할당했을 수 있지만,
          // catch_unwind가 스택을 되돌리므로 ptr은 유효하지 않음
          // 하지만 안전을 위해 명시적으로 에러 반환
          return Err(
            LlvmRuntimeError::ExecutionError("JIT function panicked during execution".to_string())
              .into(),
          );
        }
      };
      // ptr이 유효한 경우에만 가드 생성 (메모리 누수 방지)
      let _ptr_guard = StringPtrGuard::new(ptr, free_string_fn);

      let error_code = Self::read_runtime_error_state(&execution_engine)?;
      if error_code != 0 {
        return Err(Self::runtime_error_from_code(error_code).into());
      }

      // c_strings is still valid here, ensuring pointers passed to JIT are not dangling

      // null 포인터 체크
      if ptr.is_null() {
        return Err(
          LlvmRuntimeError::ExecutionError(
            "Function returned null pointer for pointer output".to_string(),
          )
          .into(),
        );
      }

      // NUL-terminated 문자열 읽기
      // Y05c-12: 문자열 NUL 정책 - 반환값 처리
      // Fix: Add bounds validation to prevent reading past allocated memory
      // 최대 길이 제한을 추가하여 보안 취약점 방지 (1MB 제한)
      const MAX_STRING_LENGTH: usize = 1024 * 1024; // 1MB
      let bytes = read_c_string_bytes(ptr, MAX_STRING_LENGTH)?;

      // UTF-8 문자열로 변환 (복사본 생성)
      // MEDIUM: JSON 직렬화 서로게이트 페어 오류 수정 완료
      // std::str::from_utf8는 유효한 UTF-8만 허용하므로 서로게이트 페어 문제 없음
      // serde_json은 UTF-8 문자열을 올바르게 처리하여 유효한 JSON 생성
      let str_value = std::str::from_utf8(bytes).map_err(|e| {
        LlvmRuntimeError::ExecutionError(format!("Invalid UTF-8 in string return value: {}", e))
      })?;

      let result_json = match output_kind {
        super::ValueKind::String => {
          // 문자열 복사본 생성 (원본 메모리는 해제됨)
          let str_value_owned = str_value.to_string();
          // JSON 문자열로 이스케이프 처리
          serde_json::json!(str_value_owned).to_string()
        }
        super::ValueKind::List => {
          let value: serde_json::Value = serde_json::from_str(str_value).map_err(|e| {
            LlvmRuntimeError::ExecutionError(format!("Invalid JSON list in pointer output: {}", e))
          })?;
          if !value.is_array() {
            return Err(
              LlvmRuntimeError::ExecutionError(
                "Pointer output for List must be a JSON array".to_string(),
              )
              .into(),
            );
          }
          value.to_string()
        }
        super::ValueKind::AttrSet => {
          let value: serde_json::Value = serde_json::from_str(str_value).map_err(|e| {
            LlvmRuntimeError::ExecutionError(format!(
              "Invalid JSON attrset in pointer output: {}",
              e
            ))
          })?;
          if !value.is_object() {
            return Err(
              LlvmRuntimeError::ExecutionError(
                "Pointer output for AttrSet must be a JSON object".to_string(),
              )
              .into(),
            );
          }
          value.to_string()
        }
        _ => unreachable!("non-pointer output kind in pointer eval"),
      };

      Ok(EvalResult {
        value: JitValue::new(result_json.into_bytes()),
      })
    }
  }

  /// Execute a compiled module with custom config
  ///
  /// This is a public wrapper for the internal LLVM execution.
  pub fn execute(
    &mut self,
    module: &JitModule,
    config: &EvalConfig,
  ) -> RuntimeResult<EvalResult<JitValue>> {
    #[cfg(feature = "llvm")]
    {
      self.eval_with_llvm_inputs(
        module,
        config,
        empty_inputs(module.numeric_kind, module.has_ptr_input),
      )
    }

    #[cfg(not(feature = "llvm"))]
    {
      let _ = (module, config);
      Err(RuntimeError::unimplemented("execute requires llvm feature"))
    }
  }

  /// Execute a compiled module with inputs
  pub fn execute_with_inputs(
    &mut self,
    module: &JitModule,
    config: &EvalConfig,
    inputs: &[u8],
  ) -> RuntimeResult<EvalResult<JitValue>> {
    #[cfg(feature = "llvm")]
    {
      // Y13a-13: 입력 타입 소스 오브 트루스 - 모듈 타입 기반으로 ParsedInputs 경로 선택
      let parsed = parse_inputs(
        &module.input_names,
        &module.input_kinds,
        module.numeric_kind,
        module.has_ptr_input,
        inputs,
      )?;
      self.eval_with_llvm_inputs(module, config, parsed)
    }

    #[cfg(not(feature = "llvm"))]
    {
      let _ = (module, config, inputs);
      Err(RuntimeError::unimplemented("execute requires llvm feature"))
    }
  }
}

#[cfg(all(test, feature = "llvm"))]
mod tests {
  use super::*;

  #[test]
  fn jit_cache_evicts_lru() {
    let mut config = JitConfig::default();
    config.max_cached_modules = 2;
    let mut engine = JitEngine::with_config(config);

    engine.cache_module("a".to_string(), JitModule::new("a"));
    engine.cache_module("b".to_string(), JitModule::new("b"));

    engine.touch_cache_key("a");
    engine.cache_module("c".to_string(), JitModule::new("c"));

    assert!(engine.module_cache.contains_key("a"));
    assert!(!engine.module_cache.contains_key("b"));
    assert!(engine.module_cache.contains_key("c"));
  }

  #[test]
  fn jit_cache_zero_limit_disables_cache() {
    let mut config = JitConfig::default();
    config.max_cached_modules = 0;
    let mut engine = JitEngine::with_config(config);

    engine.cache_module("a".to_string(), JitModule::new("a"));
    assert!(engine.module_cache.is_empty());
    assert!(engine.module_cache_order.is_empty());
  }

  #[test]
  fn parse_ptr_inputs_accepts_list_attrset() {
    let input_names = vec!["xs".to_string(), "obj".to_string()];
    let input_kinds = vec![crate::ValueKind::List, crate::ValueKind::AttrSet];
    let inputs_json = br#"{"xs":[1,2],"obj":{"a":1}}"#;
    let parsed = parse_inputs(
      &input_names,
      &input_kinds,
      crate::NumericKind::Int,
      true,
      inputs_json,
    )
    .expect("parse ptr inputs");

    match parsed {
      ParsedInputs::Ptr(values) => {
        assert_eq!(values[0], "[1,2]");
        assert!(values[1].contains("\"a\""));
      }
      _ => panic!("expected ptr inputs"),
    }
  }
}
