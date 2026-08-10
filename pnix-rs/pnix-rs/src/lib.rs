//! Embeddable PNIX runtime library.
//!
//! All evaluation delegates to the same `px` module used by the `pnix-rs`
//! executable. Platform packages are projections of this library, not new
//! semantic implementations.

pub mod px;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

pub const PNIX_RS_ABI_VERSION: u32 = 1;

/// Native Rust entry point used by Rust applications and tests.
pub fn eval(source: &str) -> Result<String, String> {
    px::px_run(source)
}

unsafe fn store_ffi_text(out: *mut *mut c_char, text: String) -> c_int {
    if out.is_null() {
        return -1;
    }
    match CString::new(text) {
        Ok(value) => {
            unsafe { *out = value.into_raw() };
            0
        }
        Err(_) => {
            unsafe { *out = ptr::null_mut() };
            -3
        }
    }
}

/// C ABI shared by desktop, Android JNI wrappers, iOS Swift bridges, and WASM.
///
/// Returns 0 on success, 1 on a structured PNIX evaluation failure, and a
/// negative value for an ABI/input failure. `*out` is always owned by the
/// caller after a 0 or 1 result and must be released with
/// `pnix_rs_string_free`.
#[no_mangle]
pub unsafe extern "C" fn pnix_rs_eval(
    source: *const c_char,
    out: *mut *mut c_char,
) -> c_int {
    if source.is_null() || out.is_null() {
        return -1;
    }
    unsafe { *out = ptr::null_mut() };
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(value) => value,
        Err(_) => return -2,
    };
    match eval(source) {
        Ok(value) => unsafe { store_ffi_text(out, value) },
        Err(error) => {
            let status = unsafe { store_ffi_text(out, error) };
            if status == 0 { 1 } else { status }
        }
    }
}

#[no_mangle]
pub extern "C" fn pnix_rs_abi_version() -> u32 {
    PNIX_RS_ABI_VERSION
}

#[no_mangle]
pub unsafe extern "C" fn pnix_rs_string_free(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) });
    }
}
