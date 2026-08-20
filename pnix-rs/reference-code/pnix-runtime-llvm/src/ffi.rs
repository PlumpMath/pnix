//! FFI (Foreign Function Interface) support for dynamic library loading
//!
//! This module provides functionality to load shared libraries (.so/.dylib/.dll)
//! and call functions from them at runtime.

#[cfg(feature = "llvm")]
use super::LlvmRuntimeError;
#[cfg(feature = "llvm")]
use libloading::{Library, Symbol};
#[cfg(feature = "llvm")]
use pnix_runtime_api::RuntimeResult;
#[cfg(feature = "llvm")]
use std::ffi::{CStr, CString, OsStr};
#[cfg(feature = "llvm")]
use std::os::raw::c_char;
#[cfg(feature = "llvm")]
use std::path::{Component, Path, PathBuf};

#[cfg(feature = "llvm")]
fn allow_system_libs() -> bool {
  matches!(std::env::var("PNIX_FFI_ALLOW_SYSTEM").as_deref(), Ok("1"))
}

#[cfg(feature = "llvm")]
const FFI_SIGNATURE_SUFFIX: &[u8] = b"__pnix_sig";

#[cfg(feature = "llvm")]
fn library_roots() -> RuntimeResult<Vec<PathBuf>> {
  let mut roots = Vec::new();
  if let Some(raw) = std::env::var_os("PNIX_FFI_LIBRARY_ROOTS") {
    for root in std::env::split_paths(&raw) {
      if root.as_os_str().is_empty() {
        continue;
      }
      let resolved = if root.is_absolute() {
        root
      } else {
        let cwd = std::env::current_dir().map_err(|err| {
          LlvmRuntimeError::IoError(format!("Failed to read current dir: {}", err))
        })?;
        cwd.join(root)
      };
      roots.push(resolved);
    }
  }
  Ok(roots)
}

#[cfg(feature = "llvm")]
fn validate_library_path(path: &Path) -> RuntimeResult<()> {
  if path.as_os_str().is_empty() {
    return Err(
      LlvmRuntimeError::ExecutionError("Dynamic library path is empty".to_string()).into(),
    );
  }
  if path
    .components()
    .any(|component| matches!(component, Component::ParentDir))
  {
    return Err(
      LlvmRuntimeError::ExecutionError("Dynamic library path contains '..'".to_string()).into(),
    );
  }

  let is_bare_name = path.components().count() == 1;
  if is_bare_name {
    if allow_system_libs() {
      return Ok(());
    }
    return Err(
      LlvmRuntimeError::ExecutionError(
        "Dynamic library name requires PNIX_FFI_ALLOW_SYSTEM=1".to_string(),
      )
      .into(),
    );
  }

  let roots = library_roots()?;
  if path.is_absolute() && roots.is_empty() {
    return Err(
      LlvmRuntimeError::ExecutionError(
        "Dynamic library absolute paths require PNIX_FFI_LIBRARY_ROOTS".to_string(),
      )
      .into(),
    );
  }

  let candidate = if path.is_absolute() {
    path.to_path_buf()
  } else {
    let cwd = std::env::current_dir()
      .map_err(|err| LlvmRuntimeError::IoError(format!("Failed to read current dir: {}", err)))?;
    cwd.join(path)
  };

  if !roots.is_empty() && !roots.iter().any(|root| candidate.starts_with(root)) {
    return Err(
      LlvmRuntimeError::ExecutionError(format!(
        "Dynamic library path '{}' is not in allowed roots",
        path.display()
      ))
      .into(),
    );
  }

  Ok(())
}

#[cfg(feature = "llvm")]
fn signature_symbol_name(symbol: &[u8]) -> Vec<u8> {
  let mut base = symbol.to_vec();
  if matches!(base.last(), Some(0)) {
    base.pop();
  }
  base.extend_from_slice(FFI_SIGNATURE_SUFFIX);
  base
}

#[cfg(feature = "llvm")]
fn display_symbol(symbol: &[u8]) -> String {
  let mut end = symbol.len();
  if end > 0 && symbol[end - 1] == 0 {
    end -= 1;
  }
  String::from_utf8_lossy(&symbol[..end]).to_string()
}

/// Y14b: Dynamic library loader
///
/// Provides platform-agnostic interface for loading shared libraries
/// and resolving function symbols.
#[cfg(feature = "llvm")]
pub struct DynamicLibrary {
  library: Library,
}

#[cfg(feature = "llvm")]
impl DynamicLibrary {
  /// Load a shared library from the given path
  ///
  /// # Platform-specific behavior
  /// - Linux: loads `.so` files
  /// - macOS: loads `.dylib` files
  /// - Windows: loads `.dll` files
  ///
  /// # Errors
  /// Returns an error if the library cannot be loaded (file not found, wrong architecture, etc.)
  ///
  /// # Safety
  /// Loading a dynamic library is inherently unsafe because:
  /// - The library may contain malicious code
  /// - The library may have undefined behavior
  /// - The library may conflict with other loaded libraries
  pub unsafe fn load<P: AsRef<OsStr>>(path: P) -> RuntimeResult<Self> {
    let path_ref = Path::new(path.as_ref());
    validate_library_path(path_ref)?;
    let library = Library::new(path_ref).map_err(|e| {
      LlvmRuntimeError::ExecutionError(format!("Failed to load dynamic library: {}", e))
    })?;
    Ok(Self { library })
  }

  /// Get a function symbol from the loaded library
  ///
  /// # Safety
  /// This function is unsafe because:
  /// - The function signature must match exactly
  /// - The function must be thread-safe if called from multiple threads
  /// - The function must not have been unloaded
  ///
  /// # Type parameters
  /// - `T`: Function signature type (e.g., `extern "C" fn(i32, i32) -> i32`)
  ///
  /// # HIGH: FFI 심볼 시그니처 미검증
  /// 호출하려는 심볼은 `<symbol>__pnix_sig` 형태의 NUL 종료 UTF-8 시그니처 문자열을 함께
  /// 노출해야 합니다. 시그니처는 `type_name::<T>()`와 일치해야 하며, 불일치 시 오류를 반환합니다.
  pub unsafe fn get<T>(&self, symbol: &[u8]) -> RuntimeResult<Symbol<'_, T>> {
    let expected = std::any::type_name::<T>();
    let declared = self.signature_for(symbol)?;
    if declared != expected {
      return Err(
        LlvmRuntimeError::ExecutionError(format!(
          "FFI signature mismatch for '{}': expected '{}', got '{}'",
          display_symbol(symbol),
          expected,
          declared
        ))
        .into(),
      );
    }
    self.get_unchecked(symbol)
  }

  /// Get a function symbol without signature validation.
  ///
  /// # Safety
  /// The caller must ensure the symbol signature is correct.
  pub unsafe fn get_unchecked<T>(&self, symbol: &[u8]) -> RuntimeResult<Symbol<'_, T>> {
    self.library.get(symbol).map_err(|e| {
      LlvmRuntimeError::ExecutionError(format!(
        "Failed to get symbol '{}': {}",
        display_symbol(symbol),
        e
      ))
      .into()
    })
  }

  fn signature_for(&self, symbol: &[u8]) -> RuntimeResult<String> {
    let sig_symbol = signature_symbol_name(symbol);
    let sig_ptr: Symbol<'_, *const c_char> = unsafe { self.library.get(sig_symbol.as_slice()) }
      .map_err(|e| {
        LlvmRuntimeError::ExecutionError(format!(
          "Missing signature symbol '{}' for '{}': {}",
          String::from_utf8_lossy(&sig_symbol),
          display_symbol(symbol),
          e
        ))
      })?;
    let ptr = *sig_ptr;
    if ptr.is_null() {
      return Err(
        LlvmRuntimeError::ExecutionError(format!(
          "Signature pointer for '{}' is null",
          display_symbol(symbol)
        ))
        .into(),
      );
    }
    let sig = unsafe { CStr::from_ptr(ptr) }
      .to_str()
      .map_err(|e| {
        LlvmRuntimeError::ExecutionError(format!(
          "Signature for '{}' is not valid UTF-8: {}",
          display_symbol(symbol),
          e
        ))
      })?
      .to_string();
    Ok(sig)
  }
}

#[cfg(feature = "llvm")]
impl Drop for DynamicLibrary {
  fn drop(&mut self) {
    // Library is automatically unloaded when dropped
  }
}

/// Free a C string allocated by Rust
///
/// This function is used to free strings that were allocated by Rust code
/// and passed to C/LLVM code. The pointer must have been allocated using
/// `CString::into_raw()` or equivalent.
///
/// # Safety
/// - The pointer must be valid and point to a C string allocated by Rust
/// - The pointer must not be null
/// - The string must not be freed more than once
/// - The string must not be accessed after being freed
#[cfg(feature = "llvm")]
#[no_mangle]
pub extern "C" fn pnix_free_string(ptr: *mut c_char) {
  if !ptr.is_null() {
    unsafe {
      // Take ownership of the CString and drop it
      let _ = CString::from_raw(ptr);
    }
  }
}

/// Free a Rust-allocated list (array of pointers)
///
/// This function frees a list that was allocated by Rust code and passed to
/// C/LLVM code. The list is represented as an array of C string pointers.
///
/// # Safety
/// - `ptr` must point to a valid array allocated by Rust via `Vec::into_raw_parts`
/// - `len` must be the actual number of elements in the array
/// - Each element pointer must have been allocated using `CString::into_raw()`
/// - The array must not be freed more than once
/// - The array must not be accessed after being freed
#[cfg(feature = "llvm")]
#[no_mangle]
pub extern "C" fn pnix_free_list(ptr: *mut *mut c_char, len: usize, cap: usize) {
  if !ptr.is_null() {
    unsafe {
      // First free each string element
      let slice = std::slice::from_raw_parts(ptr, len);
      for &elem in slice.iter() {
        if !elem.is_null() {
          let _ = CString::from_raw(elem);
        }
      }
      // Then free the array itself
      let _ = Vec::from_raw_parts(ptr, len, cap);
    }
  }
}

/// Free a Rust-allocated attrset (key-value pairs as flat array)
///
/// The attrset is represented as a flat array of C string pointers:
/// [key0, val0, key1, val1, ...] where each key and value is a C string.
///
/// # Safety
/// - `ptr` must point to a valid array allocated by Rust
/// - `num_pairs` is the number of key-value pairs (array length = num_pairs * 2)
/// - Each string pointer must have been allocated using `CString::into_raw()`
/// - The attrset must not be freed more than once
/// - The attrset must not be accessed after being freed
#[cfg(feature = "llvm")]
#[no_mangle]
pub extern "C" fn pnix_free_attrset(ptr: *mut *mut c_char, num_pairs: usize, cap: usize) {
  if !ptr.is_null() {
    unsafe {
      let total = num_pairs * 2;
      // First free each string (keys and values)
      let slice = std::slice::from_raw_parts(ptr, total);
      for &elem in slice.iter() {
        if !elem.is_null() {
          let _ = CString::from_raw(elem);
        }
      }
      // Then free the array itself
      let _ = Vec::from_raw_parts(ptr, total, cap);
    }
  }
}

/// Free a Rust-allocated tuple represented as an array of C string pointers.
///
/// This is a convenience helper for callers that flatten tuple outputs
/// into `*mut *mut c_char` (pointer-like fields only).
///
/// # Safety
/// - `ptr` must point to a valid array allocated by Rust via `Vec::into_raw_parts`
/// - `len` must be the number of elements in the array
/// - Each element pointer must have been allocated using `CString::into_raw()`
/// - The array must not be freed more than once
/// - The array must not be accessed after being freed
#[cfg(feature = "llvm")]
#[no_mangle]
pub extern "C" fn pnix_free_tuple(ptr: *mut *mut c_char, len: usize, cap: usize) {
  if !ptr.is_null() {
    unsafe {
      let slice = std::slice::from_raw_parts(ptr, len);
      for &elem in slice.iter() {
        if !elem.is_null() {
          let _ = CString::from_raw(elem);
        }
      }
      let _ = Vec::from_raw_parts(ptr, len, cap);
    }
  }
}

#[cfg(test)]
mod tests {
  #[cfg(feature = "llvm")]
  use super::*;
  #[cfg(feature = "llvm")]
  use std::env;

  #[cfg(feature = "llvm")]
  struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
  }

  #[cfg(feature = "llvm")]
  impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
      let prev = env::var(key).ok();
      env::set_var(key, value);
      Self { key, prev }
    }
  }

  #[cfg(feature = "llvm")]
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
  #[cfg(feature = "llvm")]
  fn test_dynamic_library_loading() {
    let _guard = EnvGuard::set("PNIX_FFI_ALLOW_SYSTEM", "1");
    // Test loading a system library (libc)
    // Note: This test may fail on some systems if libc is not available
    // or has a different name

    #[cfg(unix)]
    {
      // Try loading libc (common on Unix systems)
      // On Linux: libc.so.6, on macOS: libc.dylib
      #[cfg(target_os = "linux")]
      let lib_path = "libc.so.6";
      #[cfg(target_os = "macos")]
      let lib_path = "libc.dylib";

      unsafe {
        if let Ok(lib) = DynamicLibrary::load(lib_path) {
          // Try to get a common symbol (abs function)
          type AbsFn = extern "C" fn(i32) -> i32;
          if let Ok(abs_symbol) = lib.get_unchecked::<AbsFn>(b"abs") {
            // Test calling the function
            let result = abs_symbol(-42);
            assert_eq!(result, 42);

            let result2 = abs_symbol(42);
            assert_eq!(result2, 42);

            let result3 = abs_symbol(0);
            assert_eq!(result3, 0);
          }
        }
      }
    }

    #[cfg(windows)]
    {
      // On Windows, try loading MSVCRT.dll (C runtime library)
      unsafe {
        if let Ok(lib) = DynamicLibrary::load("msvcrt.dll") {
          type AbsFn = extern "C" fn(i32) -> i32;
          if let Ok(abs_symbol) = lib.get_unchecked::<AbsFn>(b"abs") {
            // Test calling the function
            let result = abs_symbol(-42);
            assert_eq!(result, 42);

            let result2 = abs_symbol(42);
            assert_eq!(result2, 42);

            let result3 = abs_symbol(0);
            assert_eq!(result3, 0);
          }
        }
      }
    }
  }

  #[test]
  #[cfg(feature = "llvm")]
  fn test_dynamic_library_rejects_parent_dir() {
    let result = unsafe { DynamicLibrary::load("../libevil.so") };
    assert!(result.is_err(), "parent dir should be rejected");
    let err = result.err().unwrap();
    let msg = format!("{:?}", err);
    assert!(
      msg.contains(".."),
      "Error should mention parent dir rejection: {}",
      msg
    );
  }
}
