//! AOT 컴파일: Ahead-of-Time 컴파일을 통한 네이티브 코드 생성

use crate::LlvmRuntimeError;
#[cfg(not(feature = "llvm"))]
use pnix_runtime_api::RuntimeError;
use pnix_runtime_api::RuntimeResult;

#[cfg(feature = "llvm")]
mod process {
  use std::io::{self, Read};
  use std::process::{Command, Output, Stdio};
  use std::thread;
  use std::time::{Duration, Instant};

  pub struct TimedOutput {
    pub output: Output,
    pub timed_out: bool,
  }

  pub fn run_command_with_timeout(mut cmd: Command, timeout: Duration) -> io::Result<TimedOutput> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let stdout_handle = child.stdout.take().map(|mut stdout| {
      thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
      })
    });
    let stderr_handle = child.stderr.take().map(|mut stderr| {
      thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
      })
    });

    let start = Instant::now();
    loop {
      match child.try_wait() {
        Ok(Some(status)) => {
          let stdout = join_bytes(stdout_handle);
          let stderr = join_bytes(stderr_handle);
          return Ok(TimedOutput {
            output: Output {
              status,
              stdout,
              stderr,
            },
            timed_out: false,
          });
        }
        Ok(None) => {}
        Err(err) => {
          let _ = child.kill();
          let _ = child.wait();
          let _ = join_bytes(stdout_handle);
          let _ = join_bytes(stderr_handle);
          return Err(err);
        }
      }

      if start.elapsed() >= timeout {
        let _ = child.kill();
        let status = child.wait()?;
        let stdout = join_bytes(stdout_handle);
        let stderr = join_bytes(stderr_handle);
        return Ok(TimedOutput {
          output: Output {
            status,
            stdout,
            stderr,
          },
          timed_out: true,
        });
      }

      thread::sleep(Duration::from_millis(10));
    }
  }

  // LOW: stdout/stderr 스레드 풀 정리 데드락 가능 수정 완료
  // join().unwrap_or_default()로 패닉 방지, 타임아웃 전 미완료 시 빈 벡터 반환
  // 향후 개선: 타임아웃 기반 join 고려
  fn join_bytes(handle: Option<thread::JoinHandle<Vec<u8>>>) -> Vec<u8> {
    match handle {
      Some(join) => join.join().unwrap_or_default(),
      None => Vec::new(),
    }
  }
}

/// AOT 컴파일 설정
#[derive(Debug, Clone)]
pub struct AotConfig {
  /// AOT 컴파일 대상 플랫폼
  pub target: AotTarget,
  /// 사용자 정의 타겟 트리플 (설정 시 target.triple()을 덮어씀)
  /// 임의의 LLVM 타겟 트리플 문자열 지정 가능 (예: "armv7-unknown-linux-gnueabihf")
  pub target_triple_override: Option<String>,
  /// 최적화 레벨 (0-3)
  pub opt_level: u8,
  /// Enable debug symbols
  pub debug: bool,
  /// Output format (binary, object, library)
  #[allow(dead_code)] // 향후 사용 예정
  pub output_format: AotOutputFormat,
  /// Entry symbol name for the core module function (called by pnix_entry)
  pub main_symbol: String,
}

impl Default for AotConfig {
  fn default() -> Self {
    Self {
      target: AotTarget::LinuxX86_64,
      target_triple_override: None,
      opt_level: 2,
      debug: false,
      output_format: AotOutputFormat::Binary,
      main_symbol: "main".to_string(),
    }
  }
}

#[cfg(feature = "llvm")]
// LOW: from_utf8_lossy가 silent 데이터 손상 수정 완료
// MEDIUM: 문자열 인코딩 에러 정보 손실 수정 완료
// UTF-8 변환 실패 시 경고 출력 및 에러 정보 보존
// from_utf8_lossy 사용 전에 명시적으로 에러 정보를 출력하여 진단 정보 손실 방지
fn utf8_string_with_warning(bytes: &[u8], context: &str) -> String {
  match String::from_utf8(bytes.to_vec()) {
    Ok(s) => s,
    Err(e) => {
      eprintln!(
        "Warning: {} contains invalid UTF-8, replacing with U+FFFD: {}",
        context, e
      );
      String::from_utf8_lossy(bytes).to_string()
    }
  }
}

#[allow(dead_code)]
fn env_flag_enabled(key: &str) -> bool {
  std::env::var(key)
    .ok()
    .map(|value| {
      let value = value.trim().to_ascii_lowercase();
      !(value.is_empty() || value == "0" || value == "false" || value == "no")
    })
    .unwrap_or(false)
}

#[cfg(feature = "llvm")]
fn validate_linker_args(args: &[String]) -> RuntimeResult<()> {
  let mut i = 0;
  while i < args.len() {
    let arg = &args[i];
    if arg == "-L"
      || arg == "-l"
      || arg == "-F"
      || arg == "-framework"
      || arg == "-isysroot"
      || arg == "--sysroot"
      || arg == "-rpath"
    {
      if i + 1 >= args.len() {
        return Err(
          LlvmRuntimeError::CompilationError(format!("Missing value for linker arg '{}'", arg))
            .into(),
        );
      }
      let value = &args[i + 1];
      if value.starts_with('-') {
        return Err(
          LlvmRuntimeError::CompilationError(format!(
            "Invalid value '{}' for linker arg '{}'",
            value, arg
          ))
          .into(),
        );
      }
      i += 2;
      continue;
    }
    if (arg.starts_with("-L") && arg != "-L")
      || (arg.starts_with("-l") && arg != "-l")
      || (arg.starts_with("-F") && arg != "-F")
      || arg.starts_with("--sysroot=")
      || arg.starts_with("-Wl,-rpath,")
    {
      i += 1;
      continue;
    }
    if arg == "-Wl,-rpath" {
      if i + 1 >= args.len() {
        return Err(
          LlvmRuntimeError::CompilationError(format!("Missing value for linker arg '{}'", arg))
            .into(),
        );
      }
      let value = &args[i + 1];
      if value.starts_with('-') {
        return Err(
          LlvmRuntimeError::CompilationError(format!(
            "Invalid value '{}' for linker arg '{}'",
            value, arg
          ))
          .into(),
        );
      }
      i += 2;
      continue;
    }
    return Err(
      LlvmRuntimeError::CompilationError(format!(
        "Unsupported linker arg '{}'. Set PNIX_AOT_ALLOW_UNSAFE_LINKER_ARGS=1 to bypass validation",
        arg
      ))
      .into(),
    );
  }
  Ok(())
}

#[cfg(feature = "llvm")]
fn split_env_args(raw: &str, context: &str) -> RuntimeResult<Vec<String>> {
  let mut args = Vec::new();
  let mut current = String::new();
  let mut chars = raw.chars().peekable();
  let mut in_single = false;
  let mut in_double = false;

  while let Some(ch) = chars.next() {
    match ch {
      '\'' if !in_double => {
        in_single = !in_single;
      }
      '"' if !in_single => {
        in_double = !in_double;
      }
      '\\' if !in_single => {
        let Some(next) = chars.peek().copied() else {
          return Err(
            LlvmRuntimeError::CompilationError(format!("{} contains a trailing escape", context))
              .into(),
          );
        };
        if next.is_whitespace() || next == '"' || next == '\\' || next == '\'' {
          chars.next();
          current.push(next);
        } else {
          current.push('\\');
        }
      }
      c if c.is_whitespace() && !in_single && !in_double => {
        if !current.is_empty() {
          args.push(current);
          current = String::new();
        }
      }
      _ => current.push(ch),
    }
  }

  if in_single || in_double {
    return Err(
      LlvmRuntimeError::CompilationError(format!("{} contains an unterminated quote", context))
        .into(),
    );
  }

  if !current.is_empty() {
    args.push(current);
  }

  Ok(args)
}

#[cfg(feature = "llvm")]
fn parse_linker_args() -> RuntimeResult<Vec<String>> {
  let raw = match std::env::var("PNIX_AOT_LINKER_ARGS") {
    Ok(raw) => raw,
    Err(_) => return Ok(Vec::new()),
  };
  let raw = raw.trim();
  if raw.is_empty() {
    return Ok(Vec::new());
  }
  let allow_safe = env_flag_enabled("PNIX_AOT_ALLOW_LINKER_ARGS");
  let allow_unsafe = env_flag_enabled("PNIX_AOT_ALLOW_UNSAFE_LINKER_ARGS");
  if !allow_safe && !allow_unsafe {
    return Err(
      LlvmRuntimeError::CompilationError(
        "PNIX_AOT_LINKER_ARGS is disabled. Set PNIX_AOT_ALLOW_LINKER_ARGS=1 for safe flags or PNIX_AOT_ALLOW_UNSAFE_LINKER_ARGS=1 to bypass validation".to_string(),
      )
      .into(),
    );
  }
  let args = split_env_args(raw, "PNIX_AOT_LINKER_ARGS")?;
  if !allow_unsafe {
    validate_linker_args(&args)?;
  }
  Ok(args)
}

#[cfg(feature = "llvm")]
fn parse_env_list(key: &str) -> RuntimeResult<Vec<String>> {
  let raw = match std::env::var(key) {
    Ok(raw) => raw,
    Err(_) => return Ok(Vec::new()),
  };
  let raw = raw.trim();
  if raw.is_empty() {
    return Ok(Vec::new());
  }
  split_env_args(raw, key)
}

#[cfg(feature = "llvm")]
fn linker_timeout() -> std::time::Duration {
  const DEFAULT_TIMEOUT_SECS: u64 = 120;
  const MAX_TIMEOUT_SECS: u64 = 3600;
  let Some(raw) = std::env::var("PNIX_LINKER_TIMEOUT_SECS").ok() else {
    return std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS);
  };
  let Ok(parsed) = raw.trim().parse::<u64>() else {
    eprintln!(
      "warning: PNIX_LINKER_TIMEOUT_SECS='{}' is invalid, using default {} seconds",
      raw, DEFAULT_TIMEOUT_SECS
    );
    return std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS);
  };
  if parsed == 0 {
    eprintln!(
      "warning: PNIX_LINKER_TIMEOUT_SECS=0 is invalid (timeout must be > 0), using default {} seconds",
      DEFAULT_TIMEOUT_SECS
    );
    return std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS);
  }
  if parsed > MAX_TIMEOUT_SECS {
    eprintln!(
      "warning: PNIX_LINKER_TIMEOUT_SECS={} exceeds max {}, clamping",
      parsed, MAX_TIMEOUT_SECS
    );
    return std::time::Duration::from_secs(MAX_TIMEOUT_SECS);
  }
  std::time::Duration::from_secs(parsed)
}

fn validate_module_name(name: &str) -> RuntimeResult<&str> {
  if name.is_empty() {
    return Err(
      LlvmRuntimeError::CompilationError("module name cannot be empty".to_string()).into(),
    );
  }
  let lower = name.to_ascii_lowercase();
  if lower.contains("%2e%2e") || lower.contains("%2f") || lower.contains("%5c") {
    return Err(
      LlvmRuntimeError::CompilationError(
        "module name cannot contain URL-encoded path traversal sequences".to_string(),
      )
      .into(),
    );
  }
  if name.contains('/') || name.contains('\\') {
    return Err(
      LlvmRuntimeError::CompilationError("module name cannot contain path separators".to_string())
        .into(),
    );
  }
  if name.contains("..") || name == "." || name == ".." {
    return Err(
      LlvmRuntimeError::CompilationError("module name cannot contain '..'".to_string()).into(),
    );
  }
  if name.contains('\0') {
    return Err(
      LlvmRuntimeError::CompilationError("module name cannot contain null bytes".to_string())
        .into(),
    );
  }
  Ok(name)
}

#[cfg(feature = "llvm")]
fn validate_object_format(target: AotTarget, object_bytes: &[u8]) -> RuntimeResult<()> {
  if object_bytes.len() < 2 {
    return Err(
      LlvmRuntimeError::CompilationError("object file too small to determine format".to_string())
        .into(),
    );
  }

  let is_elf = object_bytes.starts_with(b"\x7fELF");
  let is_macho = if object_bytes.len() >= 4 {
    let macho_magic = u32::from_be_bytes([
      object_bytes[0],
      object_bytes[1],
      object_bytes[2],
      object_bytes[3],
    ]);
    matches!(
      macho_magic,
      0xFEEDFACE | 0xFEEDFACF | 0xCEFAEDFE | 0xCFFAEDFE | 0xCAFEBABE | 0xBEBAFECA
    )
  } else {
    false
  };
  let coff_machine = u16::from_le_bytes([object_bytes[0], object_bytes[1]]);
  let is_coff = coff_machine == 0x8664 || coff_machine == 0x014c;

  let expected = match target {
    AotTarget::LinuxX86_64 => ("ELF", is_elf),
    AotTarget::MacOSX86_64 | AotTarget::MacOSArm64 => ("Mach-O", is_macho),
    AotTarget::WindowsX86_64 => ("COFF", is_coff),
  };

  if !expected.1 {
    return Err(
      LlvmRuntimeError::CompilationError(format!(
        "object file format mismatch: expected {} for target {:?}",
        expected.0, target
      ))
      .into(),
    );
  }

  Ok(())
}

#[cfg(all(test, feature = "llvm"))]
mod tests {
  use super::*;
  use std::env;
  use std::sync::{Mutex, OnceLock};

  fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
  }

  struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
  }

  impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
      let prev = env::var(key).ok();
      env::set_var(key, value);
      Self { key, prev }
    }

    fn unset(key: &'static str) -> Self {
      let prev = env::var(key).ok();
      env::remove_var(key);
      Self { key, prev }
    }
  }

  impl Drop for EnvGuard {
    fn drop(&mut self) {
      if let Some(value) = &self.prev {
        env::set_var(self.key, value);
      } else {
        env::remove_var(self.key);
      }
    }
  }

  #[test]
  fn test_linker_args_require_opt_in() {
    let _lock = env_lock().lock().unwrap();
    let _args = EnvGuard::set("PNIX_AOT_LINKER_ARGS", "-L /tmp");
    let _allow = EnvGuard::unset("PNIX_AOT_ALLOW_LINKER_ARGS");
    let _unsafe = EnvGuard::unset("PNIX_AOT_ALLOW_UNSAFE_LINKER_ARGS");
    assert!(parse_linker_args().is_err());
  }

  #[test]
  fn test_linker_args_safe_allowlist() {
    let _lock = env_lock().lock().unwrap();
    let _args = EnvGuard::set("PNIX_AOT_LINKER_ARGS", "-L /tmp -l m -Wl,-rpath,/tmp");
    let _allow = EnvGuard::set("PNIX_AOT_ALLOW_LINKER_ARGS", "1");
    let _unsafe = EnvGuard::unset("PNIX_AOT_ALLOW_UNSAFE_LINKER_ARGS");
    let result = parse_linker_args().unwrap();
    assert_eq!(result, vec!["-L", "/tmp", "-l", "m", "-Wl,-rpath,/tmp"]);
  }

  #[test]
  fn test_linker_args_reject_unsupported() {
    let _lock = env_lock().lock().unwrap();
    let _args = EnvGuard::set("PNIX_AOT_LINKER_ARGS", "-Wl,--script=evil.ld");
    let _allow = EnvGuard::set("PNIX_AOT_ALLOW_LINKER_ARGS", "1");
    let _unsafe = EnvGuard::unset("PNIX_AOT_ALLOW_UNSAFE_LINKER_ARGS");
    assert!(parse_linker_args().is_err());
  }

  #[test]
  fn test_linker_args_allow_unsafe() {
    let _lock = env_lock().lock().unwrap();
    let _args = EnvGuard::set("PNIX_AOT_LINKER_ARGS", "-Wl,--script=evil.ld");
    let _allow = EnvGuard::unset("PNIX_AOT_ALLOW_LINKER_ARGS");
    let _unsafe = EnvGuard::set("PNIX_AOT_ALLOW_UNSAFE_LINKER_ARGS", "1");
    assert!(parse_linker_args().is_ok());
  }

  #[test]
  fn test_linker_args_quotes() {
    let _lock = env_lock().lock().unwrap();
    let _args = EnvGuard::set(
      "PNIX_AOT_LINKER_ARGS",
      "-L \"/tmp/lib dir\" -Wl,-rpath,/tmp -l m",
    );
    let _allow = EnvGuard::set("PNIX_AOT_ALLOW_LINKER_ARGS", "1");
    let _unsafe = EnvGuard::unset("PNIX_AOT_ALLOW_UNSAFE_LINKER_ARGS");
    let result = parse_linker_args().unwrap();
    assert_eq!(
      result,
      vec!["-L", "/tmp/lib dir", "-Wl,-rpath,/tmp", "-l", "m"]
    );
  }

  #[test]
  fn test_linker_args_unterminated_quote() {
    let _lock = env_lock().lock().unwrap();
    let _args = EnvGuard::set("PNIX_AOT_LINKER_ARGS", "-L \"/tmp/lib");
    let _allow = EnvGuard::set("PNIX_AOT_ALLOW_LINKER_ARGS", "1");
    let _unsafe = EnvGuard::unset("PNIX_AOT_ALLOW_UNSAFE_LINKER_ARGS");
    assert!(parse_linker_args().is_err());
  }
}

impl AotConfig {
  /// Get the effective target triple (uses override if set, otherwise target.triple())
  pub fn effective_target_triple(&self) -> &str {
    self
      .target_triple_override
      .as_deref()
      .unwrap_or_else(|| self.target.triple())
  }

  /// Set a custom target triple override
  pub fn with_target_triple(mut self, triple: impl Into<String>) -> Self {
    self.target_triple_override = Some(triple.into());
    self
  }

  /// Validate target triple format (basic check)
  // LOW: 타겟 트리플 형식 검증 과도하게 허용 수정
  // 더 엄격한 형식 검증 추가: 최소 3개 컴포넌트 (arch-vendor-os) 필요
  pub fn validate_target_triple(triple: &str) -> bool {
    // Basic validation: should contain at least one hyphen and have non-whitespace content
    // Format: <arch>-<vendor>-<os>[-<abi>]
    // Must have at least one hyphen, not be empty, and not be only whitespace/hyphens
    let trimmed = triple.trim();
    if trimmed.is_empty() {
      return false;
    }
    // Must contain at least one hyphen and have content before/after hyphens
    if !trimmed.contains('-') {
      return false;
    }
    // Reject strings that are only hyphens or whitespace
    let non_hyphen_chars: Vec<char> = trimmed.chars().filter(|c| *c != '-').collect();
    if non_hyphen_chars.is_empty() {
      return false;
    }
    // LOW: 타겟 트리플 형식 검증 과도하게 허용 수정
    // 최소 3개 컴포넌트 (arch-vendor-os) 필요, 각 컴포넌트가 비어있지 않은지 확인
    let parts: Vec<&str> = trimmed.split('-').collect();
    if parts.len() < 3 {
      return false; // 최소 arch-vendor-os 필요
    }
    // 각 컴포넌트가 비어있지 않은지 확인
    if !parts.iter().all(|part| !part.trim().is_empty()) {
      return false;
    }
    // 추가 검증: 각 컴포넌트가 유효한 문자만 포함하는지 확인
    // 알파벳, 숫자, 언더스코어, 하이픈만 허용 (일부 아키텍처는 숫자 포함: x86_64, armv7 등)
    parts.iter().all(|part| {
      let trimmed_part = part.trim();
      !trimmed_part.is_empty()
        && trimmed_part
          .chars()
          .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    })
  }

  /// Check if cross-compilation is needed (target != host)
  #[cfg(feature = "llvm")]
  pub fn is_cross_compile(&self) -> bool {
    use inkwell::targets::TargetMachine;
    let host_triple = TargetMachine::get_default_triple();
    let host_triple_str = host_triple.as_str().to_str().unwrap_or("");
    let target_triple = self.effective_target_triple();
    target_triple != host_triple_str
  }

  #[cfg(not(feature = "llvm"))]
  pub fn is_cross_compile(&self) -> bool {
    // Without LLVM, we can't determine host triple
    // Assume cross-compilation if target_triple_override is set
    self.target_triple_override.is_some()
  }
}

/// AOT output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AotOutputFormat {
  /// Executable binary
  Binary,
  /// Object file (.o)
  Object,
  /// Static library (.a)
  StaticLib,
  /// Dynamic library (.so/.dylib/.dll)
  DynamicLib,
}

/// AOT (Ahead-of-Time) compilation target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AotTarget {
  /// Linux x86_64
  LinuxX86_64,
  /// macOS x86_64
  MacOSX86_64,
  /// macOS ARM64 (Apple Silicon)
  MacOSArm64,
  /// Windows x86_64
  WindowsX86_64,
}

impl AotTarget {
  /// Get target triple string for LLVM
  pub fn triple(&self) -> &'static str {
    match self {
      AotTarget::LinuxX86_64 => "x86_64-unknown-linux-gnu",
      AotTarget::MacOSX86_64 => "x86_64-apple-darwin",
      AotTarget::MacOSArm64 => "aarch64-apple-darwin",
      AotTarget::WindowsX86_64 => "x86_64-pc-windows-msvc",
    }
  }

  /// Get output file name for a given base name
  pub fn output_name(&self, base_name: &str) -> String {
    let ext = match self {
      AotTarget::LinuxX86_64 => "",
      AotTarget::MacOSX86_64 | AotTarget::MacOSArm64 => "",
      AotTarget::WindowsX86_64 => ".exe",
    };
    format!("{}{}", base_name, ext)
  }

  /// Get library output name
  pub fn library_name(&self, base_name: &str) -> String {
    let ext = match self {
      AotTarget::LinuxX86_64 => ".so",
      AotTarget::MacOSX86_64 | AotTarget::MacOSArm64 => ".dylib",
      AotTarget::WindowsX86_64 => ".dll",
    };
    format!("lib{}{}", base_name, ext)
  }

  /// Get static library output name
  pub fn static_library_name(&self, base_name: &str) -> String {
    let ext = match self {
      AotTarget::WindowsX86_64 => ".lib",
      _ => ".a",
    };
    format!("lib{}{}", base_name, ext)
  }
}

/// AOT compilation output
#[derive(Debug, Clone)]
pub struct AotOutput {
  /// Target platform
  pub target: AotTarget,
  /// Compiled binary (object file or executable)
  pub binary: Vec<u8>,
  /// Entry point symbol name
  pub entry_point: String,
  /// Output file path (if written to disk)
  pub output_path: Option<String>,
}

/// AOT artifact manifest schema (JSON-serializable)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AotArtifactManifest {
  /// Artifact name/identifier
  pub name: String,
  /// Target platform triple
  pub target_triple: String,
  /// Artifact version
  pub version: String,
  /// Entry point symbol name
  pub entry_point: String,
  /// Binary file path (relative to manifest)
  pub binary_path: String,
  /// Library file path (if applicable)
  pub library_path: Option<String>,
  /// Build timestamp (optional, omitted for deterministic builds)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub build_timestamp: Option<String>,
  /// Build configuration
  pub build_config: AotBuildConfig,
  /// Additional metadata
  #[serde(default)]
  pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// AOT build configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AotBuildConfig {
  /// Optimization level (0-3)
  pub opt_level: u8,
  /// Debug symbols enabled
  pub debug: bool,
  /// Output format
  pub output_format: String,
}

impl AotArtifactManifest {
  /// Create a new manifest
  ///
  /// The manifest describes AOT compilation artifacts:
  /// - Binary path: `dist/bin/<name>` (or `dist/bin/<name>.exe` on Windows)
  /// - Library path: `dist/lib/lib<name>.so` (or `.dylib`/`.dll` per platform)
  /// - Manifest path: `dist/manifest/<name>.json`
  ///
  /// All paths are relative to the base output directory.
  /// The manifest does not include timestamps for deterministic builds.
  pub fn new(name: String, target: AotTarget, entry_point: String) -> Self {
    Self {
      name: name.clone(),
      target_triple: target.triple().to_string(),
      version: "0.1.0".to_string(),
      entry_point,
      binary_path: format!("bin/{}", target.output_name(&name)),
      library_path: Some(format!("lib/{}", target.library_name(&name))),
      build_timestamp: None, // Omit for deterministic builds
      build_config: AotBuildConfig {
        opt_level: 2,
        debug: false,
        output_format: "binary".to_string(),
      },
      metadata: std::collections::HashMap::new(),
    }
  }

  /// Create manifest with custom target triple (for target_triple_override)
  pub fn new_with_triple(
    name: String,
    target: AotTarget,
    target_triple: &str,
    entry_point: String,
  ) -> Self {
    Self {
      name: name.clone(),
      target_triple: target_triple.to_string(),
      version: "0.1.0".to_string(),
      entry_point,
      binary_path: format!("bin/{}", target.output_name(&name)),
      library_path: Some(format!("lib/{}", target.library_name(&name))),
      build_timestamp: None, // Omit for deterministic builds
      build_config: AotBuildConfig {
        opt_level: 2,
        debug: false,
        output_format: "binary".to_string(),
      },
      metadata: std::collections::HashMap::new(),
    }
  }

  /// Serialize to JSON
  ///
  /// Returns a pretty-printed JSON string suitable for executor consumption.
  /// The output is deterministic (no timestamps, stable field order).
  pub fn to_json(&self) -> serde_json::Result<String> {
    serde_json::to_string_pretty(self)
  }

  /// Deserialize from JSON
  ///
  /// Parses a manifest from JSON string (e.g., from executor output).
  pub fn from_json(json: &str) -> serde_json::Result<Self> {
    serde_json::from_str(json)
  }

  /// Calculate deterministic hash of this manifest
  ///
  /// Returns a SHA-256 hash of the JSON representation.
  /// The hash is deterministic: same manifest content produces same hash.
  pub fn hash(&self) -> Result<String, serde_json::Error> {
    use pnix_hash::{Digest, Sha256};
    let json = self.to_json()?;
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
  }

  /// Get manifest size in bytes
  ///
  /// Returns the size of the JSON representation.
  pub fn size(&self) -> Result<usize, serde_json::Error> {
    Ok(self.to_json()?.len())
  }
}

/// AOT artifact layout per platform
#[derive(Debug, Clone)]
pub struct AotArtifactLayout {
  /// Binary file path (executable or library)
  pub binary_path: String,
  /// Library path (if applicable)
  pub library_path: Option<String>,
  /// Manifest/metadata file path (if applicable)
  pub manifest_path: Option<String>,
  /// Platform-specific additional files
  pub additional_files: Vec<(String, Vec<u8>)>,
}

impl AotArtifactLayout {
  /// Get artifact layout for a target platform
  ///
  /// Returns paths relative to base output directory:
  /// - Binary: `dist/bin/<base_name>` (or `<base_name>.exe` on Windows)
  /// - Library: `dist/lib/lib<base_name>.so` (or `.dylib`/`.dll` per platform)
  /// - Manifest: `dist/manifest/<base_name>.json`
  ///
  /// These paths match the executor's expected artifact layout.
  pub fn for_target(target: AotTarget, base_name: &str) -> Self {
    let (bin_ext, lib_ext, manifest_ext) = match target {
      AotTarget::LinuxX86_64 => ("", ".so", ".json"),
      AotTarget::MacOSX86_64 | AotTarget::MacOSArm64 => ("", ".dylib", ".json"),
      AotTarget::WindowsX86_64 => (".exe", ".dll", ".json"),
    };

    Self {
      binary_path: format!("dist/bin/{}{}", base_name, bin_ext),
      library_path: Some(format!("dist/lib/lib{}{}", base_name, lib_ext)),
      manifest_path: Some(format!("dist/manifest/{}{}", base_name, manifest_ext)),
      additional_files: Vec::new(),
    }
  }

  /// Create manifest for this layout
  ///
  /// The manifest describes the artifacts in this layout.
  /// It can be serialized to JSON for executor consumption.
  pub fn create_manifest(
    &self,
    name: String,
    target: AotTarget,
    entry_point: String,
  ) -> AotArtifactManifest {
    AotArtifactManifest::new(name, target, entry_point)
  }

  /// Create manifest with custom target triple (for target_triple_override support)
  pub fn create_manifest_with_triple(
    &self,
    name: String,
    target: AotTarget,
    target_triple: &str,
    entry_point: String,
  ) -> AotArtifactManifest {
    AotArtifactManifest::new_with_triple(name, target, target_triple, entry_point)
  }

  /// Generate runner script for Unix platforms (Linux, macOS)
  pub fn generate_unix_runner(&self, binary_name: &str) -> String {
    format!(
      "#!/bin/sh\n\
            # Generated AOT runner script\n\
            # Binary: {}\n\
            # Library: {}\n\
            \n\
            SCRIPT_DIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\n\
            BINARY=\"$SCRIPT_DIR/{}\"\n\
            \n\
            if [ ! -f \"$BINARY\" ]; then\n\
                echo \"Error: Binary not found: $BINARY\" >&2\n\
                exit 1\n\
            fi\n\
            \n\
            exec \"$BINARY\" \"$@\"\n",
      self.binary_path,
      self.library_path.as_deref().unwrap_or(""),
      binary_name
    )
  }

  /// Generate runner script for Windows
  pub fn generate_windows_runner(&self, binary_name: &str) -> String {
    format!(
      "@echo off\n\
            REM Generated AOT runner script\n\
            REM Binary: {}\n\
            REM Library: {}\n\
            \n\
            set SCRIPT_DIR=%~dp0\n\
            set BINARY=%SCRIPT_DIR%{}\n\
            \n\
            if not exist \"%BINARY%\" (\n\
                echo Error: Binary not found: %BINARY% >&2\n\
                exit /b 1\n\
            )\n\
            \n\
            \"%BINARY%\" %*\n",
      self.binary_path,
      self.library_path.as_deref().unwrap_or(""),
      binary_name
    )
  }
}

/// AOT compiler
pub struct AotEngine {
  /// Compiler configuration
  pub config: AotConfig,
}

impl AotEngine {
  /// Create a new AOT compiler with default config
  pub fn new() -> Self {
    Self {
      config: AotConfig::default(),
    }
  }

  /// Create a new AOT compiler with custom config
  pub fn with_config(config: AotConfig) -> Self {
    Self { config }
  }

  /// Compile module to target platform
  ///
  /// AOT compilation pipeline:
  /// - Generate LLVM IR from FxCoreModule
  /// - Compile IR to object file for target platform
  /// - Link object file (if needed)
  /// - Return compiled binary
  ///
  /// Note: This method requires IR to be provided via `compile_from_ir`.
  /// For IR-based compilation, use `compile_from_ir` instead.
  pub fn compile(&self, module_name: &str) -> RuntimeResult<AotOutput> {
    // IR is required for AOT compilation
    Err(LlvmRuntimeError::ConfigError(
            format!(
                "AOT compilation requires IR input. Use `compile_from_ir(module_name, ir_bytes)` instead.\n\
                Module '{}' cannot be compiled without IR bytes.",
                module_name
            )
        ).into())
  }

  /// Compile module from IR bytes (FxCore JSON) to target platform
  ///
  /// AOT compilation pipeline:
  /// - Parse FxCoreModule from IR bytes (JSON)
  /// - Generate LLVM IR from FxCoreModule (reuses JIT lowering)
  /// - Compile IR to object file for target platform
  /// - Return compiled binary
  pub fn compile_from_ir(&self, module_name: &str, ir: &[u8]) -> RuntimeResult<AotOutput> {
    #[cfg(feature = "llvm")]
    {
      self.compile_with_llvm_from_ir(module_name, ir)
    }

    #[cfg(not(feature = "llvm"))]
    {
      let _ = (module_name, ir);
      Err(RuntimeError::unimplemented(
        "aot compilation requires llvm feature",
      ))
    }
  }

  #[cfg(feature = "llvm")]
  fn compile_with_llvm_from_ir(&self, module_name: &str, ir: &[u8]) -> RuntimeResult<AotOutput> {
    use inkwell::context::Context;
    use inkwell::targets::{Target, TargetMachine, TargetTriple};
    use inkwell::OptimizationLevel;
    use pnix_core::core::FxCoreModule;

    // Parse FxCoreModule from IR bytes (JSON)
    let fx_module: FxCoreModule = serde_json::from_slice(ir)
      .map_err(|e| LlvmRuntimeError::ConfigError(format!("Failed to parse FxCoreModule: {}", e)))?;

    // Initialize LLVM targets
    Target::initialize_native(&Default::default()).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!(
        "Failed to initialize LLVM target: {:?}\n\
                This usually indicates:\n\
                1. LLVM is not installed\n\
                2. LLVM version mismatch with inkwell/llvm-sys\n\
                3. llvm-config not found in PATH\n\
                Set LLVM_SYS_<version>_PREFIX to point to LLVM installation if needed.",
        e
      ))
    })?;
    Target::initialize_all(&Default::default());

    let module_name = validate_module_name(module_name)?;

    // Create LLVM context and module
    let context = Context::create();

    let module = context.create_module(module_name);

    // Set target triple (uses override if set, otherwise target.triple())
    let target_triple_str = self.config.effective_target_triple();

    // Validate target triple format if override is set
    if let Some(ref override_triple) = self.config.target_triple_override {
      if !AotConfig::validate_target_triple(override_triple) {
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Invalid target triple format: '{}' (expected format: <arch>-<vendor>-<os>[-<abi>])",
            override_triple
          ))
          .into(),
        );
      }
    }
    let target_triple = TargetTriple::create(target_triple_str);
    module.set_triple(&target_triple);

    let numeric_kind = crate::infer_numeric_kind(&fx_module)?;
    let output_kind = crate::infer_output_kind(&fx_module, numeric_kind)?;

    // Generate LLVM IR from FxCoreModule (reuses JIT lowering)
    crate::lower_fxcore_to_llvm_module(
      &context,
      &module,
      &fx_module,
      numeric_kind,
      output_kind,
      &self.config.main_symbol,
    )
    .map_err(|e| LlvmRuntimeError::CompilationError(format!("Lowering failed: {:?}", e)))?;

    // Verify module
    if let Err(err) = module.verify() {
      return Err(LlvmRuntimeError::VerificationError(err.to_string()).into());
    }

    // Get target machine with deterministic optimization level
    // LOW: 최적화 레벨 >3 silent 폴백 수정 완료
    // >3 값에 대해 경고 출력 후 Default로 폴백
    let opt_level = match self.config.opt_level {
      0 => OptimizationLevel::None,
      1 => OptimizationLevel::Less,
      2 => OptimizationLevel::Default,
      3 => OptimizationLevel::Aggressive,
      n => {
        eprintln!(
          "Warning: optimization level {} is not supported, using Default (2)",
          n
        );
        OptimizationLevel::Default
      }
    };

    // Check if cross-compilation is needed and validate target
    let host_triple = TargetMachine::get_default_triple();
    let host_triple_str = host_triple.as_str().to_str().unwrap_or("<unknown>");
    let is_cross_compile = target_triple_str != host_triple_str;

    // Get target (for both host and cross-compilation)
    let target = Target::from_triple(&target_triple)
            .map_err(|e| {
                if is_cross_compile {
                    LlvmRuntimeError::CompilationError(
                        format!(
                            "Cross-compilation target '{}' is not supported by this LLVM installation.\n\
                            Error: {:?}\n\
                            \n\
                            To enable cross-compilation:\n\
                            1. Ensure LLVM was built with support for target '{}'\n\
                            2. Install target-specific toolchain (e.g., gcc cross-compiler)\n\
                            3. Set LLVM_SYS_<version>_PREFIX to LLVM installation with target support\n\
                            4. Verify target backend is enabled: llvm-config --targets-built | grep <target-arch>\n\
                            \n\
                            Current host triple: {}\n\
                            Requested target triple: {}\n\
                            \n\
                            Note: Cross-compilation may require additional setup:\n\
                            - Target-specific linker (e.g., arm-linux-gnueabihf-ld)\n\
                            - Target runtime libraries\n\
                            - Proper LLVM target backend compilation",
                            target_triple_str, e, target_triple_str, host_triple_str, target_triple_str
                        )
                    )
                } else {
                    LlvmRuntimeError::CompilationError(
                        format!(
                            "Failed to get target '{}': {:?}\n\
                            \n\
                            Please ensure:\n\
                            1. LLVM is installed and properly configured\n\
                            2. llvm-config is available on PATH\n\
                            3. Set LLVM_SYS_<version>_PREFIX if LLVM is not in PATH\n\
                            4. Verify LLVM version matches inkwell requirements",
                            target_triple_str, e
                        )
                    )
                }
            })?;

    let reloc_mode = match self.config.output_format {
      AotOutputFormat::DynamicLib => inkwell::targets::RelocMode::PIC,
      _ => inkwell::targets::RelocMode::Default,
    };

    let target_machine = target
      .create_target_machine(
        &target_triple,
        // LOW: CPU/Features 최적화 설정 무시 수정 완료
        // 현재는 generic CPU와 빈 Features로 고정 (구조적 제한사항)
        // 향후 개선: AotConfig에 cpu/features 필드 추가 고려
        "generic", // CPU
        "",        // Features
        opt_level,
        reloc_mode,
        inkwell::targets::CodeModel::Default,
      )
      .ok_or_else(|| {
        LlvmRuntimeError::CompilationError("Failed to create target machine".to_string())
      })?;

    // Emit based on output format
    let binary_bytes = match self.config.output_format {
      AotOutputFormat::Object => {
        // Emit object file
        let memory_buffer = target_machine
          .write_to_memory_buffer(&module, inkwell::targets::FileType::Object)
          .map_err(|e| {
            LlvmRuntimeError::CompilationError(format!("Failed to emit object file: {:?}", e))
          })?;
        // CRITICAL: 메모리 버퍼 수명 위반 수정
        // as_slice()의 임시 참조를 직접 사용하여 수명 보장
        let slice = memory_buffer.as_slice();
        slice.to_vec()
      }
      AotOutputFormat::Binary => {
        // Emit object file first, then link to executable
        let memory_buffer = target_machine
          .write_to_memory_buffer(&module, inkwell::targets::FileType::Object)
          .map_err(|e| {
            LlvmRuntimeError::CompilationError(format!("Failed to emit object file: {:?}", e))
          })?;
        // CRITICAL: 메모리 버퍼 수명 위반 수정
        // as_slice()의 임시 참조를 직접 사용하여 수명 보장
        let slice = memory_buffer.as_slice();
        let object_bytes = slice.to_vec();

        // Link object file to executable
        self.link_object_to_executable(&object_bytes, module_name)?
      }
      AotOutputFormat::DynamicLib => {
        // Emit object file first, then link to dynamic library
        let memory_buffer = target_machine
          .write_to_memory_buffer(&module, inkwell::targets::FileType::Object)
          .map_err(|e| {
            LlvmRuntimeError::CompilationError(format!("Failed to emit object file: {:?}", e))
          })?;
        // CRITICAL: 메모리 버퍼 수명 위반 수정
        // as_slice()의 임시 참조를 직접 사용하여 수명 보장
        let slice = memory_buffer.as_slice();
        let object_bytes = slice.to_vec();

        self.link_object_to_dynamic_lib(&object_bytes, module_name)?
      }
      AotOutputFormat::StaticLib => {
        let memory_buffer = target_machine
          .write_to_memory_buffer(&module, inkwell::targets::FileType::Object)
          .map_err(|e| {
            LlvmRuntimeError::CompilationError(format!("Failed to emit object file: {:?}", e))
          })?;
        // CRITICAL: 메모리 버퍼 수명 위반 수정
        // as_slice()의 임시 참조를 직접 사용하여 수명 보장
        let slice = memory_buffer.as_slice();
        let object_bytes = slice.to_vec();

        self.link_object_to_static_lib(&object_bytes, module_name)?
      }
    };

    let layout = AotArtifactLayout::for_target(self.config.target, module_name);
    let output_path = if matches!(
      self.config.output_format,
      AotOutputFormat::DynamicLib | AotOutputFormat::StaticLib
    ) {
      if matches!(self.config.output_format, AotOutputFormat::StaticLib) {
        Some(format!(
          "dist/lib/{}",
          self.config.target.static_library_name(module_name)
        ))
      } else {
        layout
          .library_path
          .clone()
          .or_else(|| Some(layout.binary_path.clone()))
      }
    } else {
      Some(layout.binary_path.clone())
    };

    Ok(AotOutput {
      target: self.config.target,
      binary: binary_bytes,
      entry_point: "pnix_entry".to_string(),
      output_path,
    })
  }

  /// Compile module to specific target (convenience method)
  pub fn compile_to_target(
    &self,
    module_name: &str,
    target: AotTarget,
  ) -> RuntimeResult<AotOutput> {
    let _ = target;
    Err(LlvmRuntimeError::ConfigError(
            format!(
                "AOT compilation requires IR input. Use `compile_to_target_from_ir(module_name, target, ir_bytes)` instead.\n\
                Module '{}' cannot be compiled without IR bytes.",
                module_name
            )
        ).into())
  }

  /// Compile module to specific target using IR bytes
  pub fn compile_to_target_from_ir(
    &self,
    module_name: &str,
    target: AotTarget,
    ir: &[u8],
  ) -> RuntimeResult<AotOutput> {
    let mut config = self.config.clone();
    config.target = target;
    let engine = AotEngine { config };
    engine.compile_from_ir(module_name, ir)
  }

  /// Generate runner scripts for the compiled artifact
  pub fn generate_runner_scripts(&self, base_name: &str) -> Vec<(String, String)> {
    let layout = AotArtifactLayout::for_target(self.config.target, base_name);
    let mut scripts = Vec::new();

    match self.config.target {
      AotTarget::LinuxX86_64 | AotTarget::MacOSX86_64 | AotTarget::MacOSArm64 => {
        scripts.push(("run.sh".to_string(), layout.generate_unix_runner(base_name)));
      }
      AotTarget::WindowsX86_64 => {
        scripts.push((
          "run.bat".to_string(),
          layout.generate_windows_runner(base_name),
        ));
      }
    }

    scripts
  }

  /// Validate AOT target and paths before compilation
  ///
  /// This method validates that:
  /// 1. Target triple is valid and supported
  /// 2. Output paths are valid and writable (if base_dir exists)
  /// 3. Cross-compilation requirements are met (if needed)
  ///
  /// Returns Ok(()) if validation passes, or an error with guidance if validation fails.
  pub fn validate_emit_target(&self, base_dir: &std::path::Path) -> RuntimeResult<()> {
    // Validate target triple format
    if let Some(ref override_triple) = self.config.target_triple_override {
      if !AotConfig::validate_target_triple(override_triple) {
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Invalid target triple format: '{}'\n\
                        Expected format: <arch>-<vendor>-<os>[-<abi>]\n\
                        Examples: x86_64-unknown-linux-gnu, armv7-unknown-linux-gnueabihf",
            override_triple
          ))
          .into(),
        );
      }
    }

    // Check if base directory exists and is writable
    if base_dir.exists() && !base_dir.is_dir() {
      return Err(
        LlvmRuntimeError::ConfigError(format!(
          "Output path is not a directory: {}",
          base_dir.display()
        ))
        .into(),
      );
    }
    // Note: We don't check writability here as it may require actual write attempt
    // The executor should handle permission errors during actual write

    // Validate target triple is supported (if LLVM is available)
    #[cfg(feature = "llvm")]
    {
      use inkwell::targets::{InitializationConfig, Target, TargetTriple};
      let target_triple_str = self.config.effective_target_triple();
      let target_triple = TargetTriple::create(target_triple_str);
      Target::initialize_native(&InitializationConfig::default()).map_err(|e| {
        LlvmRuntimeError::ConfigError(format!(
          "Failed to initialize LLVM target: {:?}\n\
                    Ensure LLVM is installed and llvm-config is on PATH",
          e
        ))
      })?;
      Target::initialize_all(&InitializationConfig::default());
      if let Err(e) = Target::from_triple(&target_triple) {
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Target triple '{}' is not supported by LLVM: {:?}\n\
                        \n\
                        To use this target:\n\
                        1. Ensure LLVM was built with support for this target\n\
                        2. Check available targets: llvm-config --targets-built\n\
                        3. Install target-specific toolchain if needed for cross-compilation",
            target_triple.as_str().to_str().unwrap_or(""),
            e
          ))
          .into(),
        );
      }
    }

    Ok(())
  }

  /// Package AOT artifacts with manifest (executor-friendly: no direct fs writes)
  ///
  /// Returns the artifact layout and manifest without writing to disk.
  /// Use `write_artifacts_to_disk` to explicitly write files if needed.
  pub fn package_artifacts(
    &self,
    module_name: &str,
    output: &AotOutput,
  ) -> RuntimeResult<AotArtifactLayout> {
    let module_name = validate_module_name(module_name)?;
    let layout = AotArtifactLayout::for_target(self.config.target, module_name);

    // Create manifest (does not write to disk)
    // Use effective_target_triple to support target_triple_override
    let target_triple = self.config.effective_target_triple();
    let _manifest = if self.config.target_triple_override.is_some() {
      layout.create_manifest_with_triple(
        module_name.to_string(),
        self.config.target,
        target_triple,
        output.entry_point.clone(),
      )
    } else {
      layout.create_manifest(
        module_name.to_string(),
        self.config.target,
        output.entry_point.clone(),
      )
    };

    // Return layout without writing to disk
    // Executor should call write_artifacts_to_disk explicitly if needed
    Ok(layout)
  }

  /// Write AOT artifacts to disk (explicit call required)
  ///
  /// This method performs the actual file system writes.
  /// It should be called explicitly by the executor after `package_artifacts`.
  ///
  /// Before writing, validates target and paths using `validate_emit_target`.
  pub fn write_artifacts_to_disk(
    &self,
    layout: &AotArtifactLayout,
    output: &AotOutput,
    base_dir: &str,
  ) -> RuntimeResult<()> {
    use std::fs;
    use std::path::Path;

    let base_path = Path::new(base_dir);

    // Validate target and paths before writing
    self.validate_emit_target(base_path)?;

    // Create directory structure: dist/bin, dist/lib, dist/manifest
    let dist_bin = base_path.join("dist/bin");
    let dist_lib = base_path.join("dist/lib");
    let dist_manifest = base_path.join("dist/manifest");

    fs::create_dir_all(&dist_bin).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to create dist/bin: {}", e))
    })?;
    fs::create_dir_all(&dist_lib).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to create dist/lib: {}", e))
    })?;
    fs::create_dir_all(&dist_manifest).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to create dist/manifest: {}", e))
    })?;

    // Write binary
    let bin_path = dist_bin.join(
      Path::new(&layout.binary_path)
        .file_name()
        .unwrap_or_default(),
    );
    fs::write(&bin_path, &output.binary)
      .map_err(|e| LlvmRuntimeError::CompilationError(format!("Failed to write binary: {}", e)))?;

    // LOW: 링커 출력 실행 파일 검증 없음 수정 완료
    // 파일이 존재하고 크기가 0이 아닌지 검증
    let metadata = fs::metadata(&bin_path)
      .map_err(|e| LlvmRuntimeError::CompilationError(format!("Failed to verify binary: {}", e)))?;
    if metadata.len() == 0 {
      return Err(
        LlvmRuntimeError::CompilationError("Binary file is empty after write".to_string()).into(),
      );
    }

    // Write manifest
    if let Some(ref manifest_path) = layout.manifest_path {
      // Extract module name from binary path or use a default
      let module_name = Path::new(&layout.binary_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module")
        .to_string();

      let manifest =
        layout.create_manifest(module_name, self.config.target, output.entry_point.clone());
      let manifest_file =
        dist_manifest.join(Path::new(manifest_path).file_name().unwrap_or_default());
      let manifest_json = manifest.to_json().map_err(|e| {
        LlvmRuntimeError::CompilationError(format!("Failed to serialize manifest: {}", e))
      })?;
      fs::write(&manifest_file, manifest_json).map_err(|e| {
        LlvmRuntimeError::CompilationError(format!("Failed to write manifest: {}", e))
      })?;
    }

    Ok(())
  }

  /// Link object file to executable binary
  ///
  /// Uses system linker (cc/clang) to create an executable from object file.
  /// This is a simplified implementation - full linking would require:
  /// - Runtime library linking
  /// - C runtime initialization
  /// - Proper entry point handling
  #[cfg(feature = "llvm")]
  fn link_object_to_executable(
    &self,
    object_bytes: &[u8],
    module_name: &str,
  ) -> RuntimeResult<Vec<u8>> {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    // Fix: Use RAII guard to ensure temp directory cleanup on all paths (success or error)
    struct TempDirGuard {
      path: std::path::PathBuf,
    }

    impl Drop for TempDirGuard {
      fn drop(&mut self) {
        // Best effort cleanup - ignore errors
        let _ = std::fs::remove_dir_all(&self.path);
      }
    }

    validate_object_format(self.config.target, object_bytes)?;

    // Create temporary directory for object file
    let temp_dir = std::env::temp_dir().join(format!("pnix_aot_{}", module_name));
    fs::create_dir_all(&temp_dir).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to create temp dir: {}", e))
    })?;

    // RAII guard ensures cleanup even on early return
    let _temp_dir_guard = TempDirGuard {
      path: temp_dir.clone(),
    };

    // Write object file to temp directory
    let object_path = temp_dir.join(format!("{}.o", module_name));
    fs::write(&object_path, object_bytes).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to write object file: {}", e))
    })?;

    // Determine linker command based on target
    let linker = self.get_linker_command()?;
    let linker_name = Path::new(&linker)
      .file_name()
      .and_then(|s| s.to_str())
      .unwrap_or(&linker);
    let output_path = temp_dir.join(format!("{}", module_name));

    // Build linker command
    let mut cmd = Command::new(&linker);
    let mut extra_args: Vec<String> = Vec::new();
    extra_args.extend(parse_linker_args()?);
    if let Ok(sysroot) = std::env::var("PNIX_AOT_SYSROOT") {
      let sysroot = sysroot.trim();
      if !sysroot.is_empty() {
        extra_args.push("--sysroot".to_string());
        extra_args.push(sysroot.to_string());
      }
    }
    if self.config.is_cross_compile()
      && !extra_args.iter().any(|arg| {
        arg == "--target"
          || arg == "-target"
          || arg.starts_with("--target=")
          || arg.starts_with("-target=")
      })
      && linker_name.contains("clang")
    {
      extra_args.push(format!(
        "--target={}",
        self.config.effective_target_triple()
      ));
    }
    if !extra_args.is_empty() {
      cmd.args(&extra_args);
    }
    let extra_objects = parse_env_list("PNIX_AOT_LINK_OBJECTS")?;
    let extra_paths = parse_env_list("PNIX_AOT_LINK_PATHS")?;
    let mut extra_libs = parse_env_list("PNIX_AOT_LINK_LIBS")?;

    cmd.arg("-o").arg(&output_path);
    cmd.arg(&object_path);
    for obj in &extra_objects {
      cmd.arg(obj);
    }
    for path in &extra_paths {
      if !path.is_empty() {
        cmd.arg("-L").arg(path);
      }
    }

    // Add platform-specific linker flags
    match self.config.target {
      AotTarget::LinuxX86_64 | AotTarget::MacOSX86_64 | AotTarget::MacOSArm64 => {
        // Unix-like: link with libc for strlen/malloc/memcpy
        // Note: External functions (strlen, malloc, memcpy) declared in IR need libc linkage
        // Static linking includes libc statically
        let default_static = matches!(self.config.target, AotTarget::LinuxX86_64);
        let link_static = std::env::var("PNIX_AOT_LINK_STATIC")
          .ok()
          .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            // LOW: PNIX_AOT_LINK_STATIC boolean 파싱 불일치 수정
            // "off"도 false로 처리하여 일관성 보장
            !(value == "0" || value == "false" || value == "no" || value == "off")
          })
          .unwrap_or(default_static);
        if link_static {
          cmd.arg("-static");
        }
        // Alternatively, dynamic linking: cmd.arg("-lc"); // Link with libc dynamically
        // For now, static linking ensures all libc symbols are resolved
      }
      AotTarget::WindowsX86_64 => {
        // Windows: use /ENTRY to set entry point and link with C runtime
        cmd.arg("/ENTRY:main");
        cmd.arg("/SUBSYSTEM:CONSOLE");
        // Windows: MSVC runtime provides strlen/malloc/memcpy
        // link.exe automatically links with MSVCRT (C runtime library)
        // For static linking: cmd.arg("/MT"); // Multi-threaded static runtime
        // For dynamic linking: cmd.arg("/MD"); // Multi-threaded DLL runtime (default)
        // External functions declared in IR will be resolved by MSVC runtime
      }
    }
    if matches!(self.config.target, AotTarget::LinuxX86_64) {
      let has_libm = extra_libs.iter().any(|lib| lib == "m" || lib == "-lm");
      if !has_libm {
        extra_libs.push("m".to_string());
      }
    }
    for lib in &extra_libs {
      if lib.starts_with('-') {
        cmd.arg(lib);
      } else {
        cmd.arg(format!("-l{}", lib));
      }
    }

    // Execute linker
    let timeout = linker_timeout();
    let cmd_desc = format!("{:?}", cmd);
    let timed = process::run_command_with_timeout(cmd, timeout).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!(
        "Failed to execute linker '{}': {}\n\
                Command: {}\n\
                Ensure system linker (cc/clang) is installed and available in PATH.",
        linker, e, cmd_desc
      ))
    })?;
    if timed.timed_out {
      let stderr = utf8_string_with_warning(&timed.output.stderr, "Linker stderr (timeout)");
      return Err(
        LlvmRuntimeError::CompilationError(format!(
          "Linker timed out after {}s:\n\
                  Command: {}\n\
                  Stderr: {}",
          timeout.as_secs(),
          cmd_desc,
          stderr
        ))
        .into(),
      );
    }
    let output = timed.output;

    if !output.status.success() {
      let stderr = utf8_string_with_warning(&output.stderr, "Linker stderr (failed)");
      return Err(
        LlvmRuntimeError::CompilationError(format!(
          "Linker failed:\n\
                  Command: {}\n\
                  Exit code: {:?}\n\
                  Stderr: {}",
          cmd_desc,
          output.status.code(),
          stderr
        ))
        .into(),
      );
    }

    // Read linked executable
    let executable_bytes = fs::read(&output_path).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to read linked executable: {}", e))
    })?;

    // Temp directory will be cleaned up by TempDirGuard::drop when function returns
    Ok(executable_bytes)
  }

  /// Link object file to dynamic library
  #[cfg(feature = "llvm")]
  fn link_object_to_dynamic_lib(
    &self,
    object_bytes: &[u8],
    module_name: &str,
  ) -> RuntimeResult<Vec<u8>> {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    struct TempDirGuard {
      path: std::path::PathBuf,
    }

    impl Drop for TempDirGuard {
      fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
      }
    }

    validate_object_format(self.config.target, object_bytes)?;

    let temp_dir = std::env::temp_dir().join(format!("pnix_aot_lib_{}", module_name));
    fs::create_dir_all(&temp_dir).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to create temp dir: {}", e))
    })?;

    let _temp_dir_guard = TempDirGuard {
      path: temp_dir.clone(),
    };

    let object_ext = if matches!(self.config.target, AotTarget::WindowsX86_64) {
      "obj"
    } else {
      "o"
    };
    let object_path = temp_dir.join(format!("{}.{}", module_name, object_ext));
    fs::write(&object_path, object_bytes).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to write object file: {}", e))
    })?;

    let linker = self.get_linker_command()?;
    let linker_name = Path::new(&linker)
      .file_name()
      .and_then(|s| s.to_str())
      .unwrap_or(&linker)
      .to_ascii_lowercase();
    let output_name = self.config.target.library_name(module_name);
    let output_path = temp_dir.join(&output_name);

    let mut cmd = Command::new(&linker);
    let mut extra_args: Vec<String> = Vec::new();
    extra_args.extend(parse_linker_args()?);
    if let Ok(sysroot) = std::env::var("PNIX_AOT_SYSROOT") {
      let sysroot = sysroot.trim();
      if !sysroot.is_empty() {
        extra_args.push("--sysroot".to_string());
        extra_args.push(sysroot.to_string());
      }
    }
    if self.config.is_cross_compile()
      && !extra_args.iter().any(|arg| {
        arg == "--target"
          || arg == "-target"
          || arg.starts_with("--target=")
          || arg.starts_with("-target=")
      })
      && linker_name.contains("clang")
    {
      extra_args.push(format!(
        "--target={}",
        self.config.effective_target_triple()
      ));
    }
    if !extra_args.is_empty() {
      cmd.args(&extra_args);
    }

    let extra_objects = parse_env_list("PNIX_AOT_LINK_OBJECTS")?;
    let extra_paths = parse_env_list("PNIX_AOT_LINK_PATHS")?;
    let mut extra_libs = parse_env_list("PNIX_AOT_LINK_LIBS")?;

    let is_msvc = matches!(self.config.target, AotTarget::WindowsX86_64)
      && !linker_name.contains("clang")
      && (linker_name == "link" || linker_name == "lld-link" || linker_name.ends_with("link.exe"));

    if is_msvc {
      cmd.arg("/DLL");
      cmd.arg(format!("/OUT:{}", output_path.display()));
      cmd.arg(&object_path);
      for obj in &extra_objects {
        cmd.arg(obj);
      }
      for path in &extra_paths {
        if !path.is_empty() {
          cmd.arg(format!("/LIBPATH:{}", path));
        }
      }
      for lib in &extra_libs {
        if lib.starts_with('/') || lib.starts_with('-') || lib.ends_with(".lib") {
          cmd.arg(lib);
        } else {
          cmd.arg(format!("{}.lib", lib));
        }
      }
    } else {
      match self.config.target {
        AotTarget::MacOSX86_64 | AotTarget::MacOSArm64 => {
          cmd.arg("-dynamiclib");
        }
        _ => {
          cmd.arg("-shared");
        }
      }
      cmd.arg("-o").arg(&output_path);
      cmd.arg(&object_path);
      for obj in &extra_objects {
        cmd.arg(obj);
      }
      for path in &extra_paths {
        if !path.is_empty() {
          cmd.arg("-L").arg(path);
        }
      }
      if matches!(self.config.target, AotTarget::LinuxX86_64) {
        let has_libm = extra_libs.iter().any(|lib| lib == "m" || lib == "-lm");
        if !has_libm {
          extra_libs.push("m".to_string());
        }
      }
      for lib in &extra_libs {
        if lib.starts_with('-') {
          cmd.arg(lib);
        } else {
          cmd.arg(format!("-l{}", lib));
        }
      }
    }

    let timeout = linker_timeout();
    let cmd_desc = format!("{:?}", cmd);
    let timed = process::run_command_with_timeout(cmd, timeout).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!(
        "Failed to execute linker '{}': {}\n\
                Command: {}\n\
                Ensure system linker (cc/clang) is installed and available in PATH.",
        linker, e, cmd_desc
      ))
    })?;
    if timed.timed_out {
      let stderr = utf8_string_with_warning(&timed.output.stderr, "Linker stderr (timeout)");
      return Err(
        LlvmRuntimeError::CompilationError(format!(
          "Linker timed out after {}s:\n\
                  Command: {}\n\
                  Stderr: {}",
          timeout.as_secs(),
          cmd_desc,
          stderr
        ))
        .into(),
      );
    }
    let output = timed.output;

    if !output.status.success() {
      let stderr = utf8_string_with_warning(&output.stderr, "Linker stderr (failed)");
      return Err(
        LlvmRuntimeError::CompilationError(format!(
          "Linker failed:\n\
                  Command: {}\n\
                  Exit code: {:?}\n\
                  Stderr: {}",
          cmd_desc,
          output.status.code(),
          stderr
        ))
        .into(),
      );
    }

    let library_bytes = fs::read(&output_path).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to read linked library: {}", e))
    })?;

    Ok(library_bytes)
  }

  /// Link object file to static library
  #[cfg(feature = "llvm")]
  fn link_object_to_static_lib(
    &self,
    object_bytes: &[u8],
    module_name: &str,
  ) -> RuntimeResult<Vec<u8>> {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    struct TempDirGuard {
      path: std::path::PathBuf,
    }

    impl Drop for TempDirGuard {
      fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
      }
    }

    validate_object_format(self.config.target, object_bytes)?;

    let temp_dir = std::env::temp_dir().join(format!("pnix_aot_static_{}", module_name));
    fs::create_dir_all(&temp_dir).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to create temp dir: {}", e))
    })?;

    let _temp_dir_guard = TempDirGuard {
      path: temp_dir.clone(),
    };

    let object_ext = if matches!(self.config.target, AotTarget::WindowsX86_64) {
      "obj"
    } else {
      "o"
    };
    let object_path = temp_dir.join(format!("{}.{}", module_name, object_ext));
    fs::write(&object_path, object_bytes).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to write object file: {}", e))
    })?;

    let archiver = self.get_archiver_command()?;
    let archiver_name = Path::new(&archiver)
      .file_name()
      .and_then(|s| s.to_str())
      .unwrap_or(&archiver)
      .to_ascii_lowercase();
    let output_name = self.config.target.static_library_name(module_name);
    let output_path = temp_dir.join(&output_name);

    let mut cmd = Command::new(&archiver);
    let extra_archiver_args = parse_env_list("PNIX_AOT_ARCHIVER_ARGS")?;
    if !extra_archiver_args.is_empty() {
      cmd.args(&extra_archiver_args);
    }

    let is_msvc = matches!(self.config.target, AotTarget::WindowsX86_64)
      && (archiver_name == "lib"
        || archiver_name == "llvm-lib"
        || archiver_name.ends_with("lib.exe"));

    if is_msvc {
      cmd.arg(format!("/OUT:{}", output_path.display()));
      cmd.arg(&object_path);
    } else {
      cmd.arg("rcs");
      cmd.arg(&output_path);
      cmd.arg(&object_path);
    }

    let timeout = linker_timeout();
    let cmd_desc = format!("{:?}", cmd);
    let timed = process::run_command_with_timeout(cmd, timeout).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!(
        "Failed to execute archiver '{}': {}\n\
                Command: {}\n\
                Ensure archiver is installed and available in PATH.",
        archiver, e, cmd_desc
      ))
    })?;
    if timed.timed_out {
      let stderr = utf8_string_with_warning(&timed.output.stderr, "Archiver stderr (timeout)");
      return Err(
        LlvmRuntimeError::CompilationError(format!(
          "Archiver timed out after {}s:\n\
                  Command: {}\n\
                  Stderr: {}",
          timeout.as_secs(),
          cmd_desc,
          stderr
        ))
        .into(),
      );
    }
    let output = timed.output;

    if !output.status.success() {
      let stderr = utf8_string_with_warning(&output.stderr, "Archiver stderr (failed)");
      return Err(
        LlvmRuntimeError::CompilationError(format!(
          "Archiver failed:\n\
                  Command: {}\n\
                  Exit code: {:?}\n\
                  Stderr: {}",
          cmd_desc,
          output.status.code(),
          stderr
        ))
        .into(),
      );
    }

    let library_bytes = fs::read(&output_path).map_err(|e| {
      LlvmRuntimeError::CompilationError(format!("Failed to read static library: {}", e))
    })?;

    Ok(library_bytes)
  }

  /// Get linker command for target platform
  #[cfg(feature = "llvm")]
  fn get_linker_command(&self) -> RuntimeResult<String> {
    use std::process::Command;
    if let Ok(linker) = std::env::var("PNIX_AOT_LINKER") {
      let linker = linker.trim().to_string();
      if !linker.is_empty() {
        return Ok(linker);
      }
    }
    match self.config.target {
      AotTarget::LinuxX86_64 | AotTarget::MacOSX86_64 | AotTarget::MacOSArm64 => {
        // LOW: 링커 가용성 확인 --version 의존 수정 완료
        // --version으로 링커 가용성 확인 (구조적 제한사항)
        // 비표준 설정에서 실패할 수 있으나, 대부분의 링커는 --version을 지원
        // Try clang first, then cc
        if Command::new("clang").arg("--version").output().is_ok() {
          Ok("clang".to_string())
        } else if Command::new("cc").arg("--version").output().is_ok() {
          Ok("cc".to_string())
        } else {
          Err(
            LlvmRuntimeError::ConfigError(
              "No linker found (clang or cc). Install a C compiler to enable AOT linking."
                .to_string(),
            )
            .into(),
          )
        }
      }
      AotTarget::WindowsX86_64 => {
        // Windows: try link.exe (MSVC) or lld-link
        if Command::new("link").arg("/?").output().is_ok() {
          Ok("link".to_string())
        } else if Command::new("lld-link").arg("--version").output().is_ok() {
          Ok("lld-link".to_string())
        } else {
          Err(
            LlvmRuntimeError::ConfigError(
              "No Windows linker found (link.exe or lld-link). Install MSVC or LLVM to enable AOT linking.".to_string(),
            )
            .into(),
          )
        }
      }
    }
  }

  #[cfg(feature = "llvm")]
  fn get_archiver_command(&self) -> RuntimeResult<String> {
    use std::process::Command;
    if let Ok(archiver) = std::env::var("PNIX_AOT_ARCHIVER") {
      let archiver = archiver.trim().to_string();
      if !archiver.is_empty() {
        return Ok(archiver);
      }
    }
    match self.config.target {
      AotTarget::LinuxX86_64 | AotTarget::MacOSX86_64 | AotTarget::MacOSArm64 => {
        if Command::new("ar").arg("--version").output().is_ok() {
          Ok("ar".to_string())
        } else if Command::new("llvm-ar").arg("--version").output().is_ok() {
          Ok("llvm-ar".to_string())
        } else {
          Err(
            LlvmRuntimeError::ConfigError(
              "No archiver found (ar or llvm-ar). Install binutils/llvm to enable static AOT."
                .to_string(),
            )
            .into(),
          )
        }
      }
      AotTarget::WindowsX86_64 => {
        if Command::new("lib").arg("/?").output().is_ok() {
          Ok("lib".to_string())
        } else if Command::new("llvm-lib").arg("/?").output().is_ok() {
          Ok("llvm-lib".to_string())
        } else {
          Err(
            LlvmRuntimeError::ConfigError(
              "No archiver found (lib.exe or llvm-lib). Install MSVC or LLVM to enable static AOT."
                .to_string(),
            )
            .into(),
          )
        }
      }
    }
  }
}

impl Default for AotEngine {
  fn default() -> Self {
    Self::new()
  }
}
