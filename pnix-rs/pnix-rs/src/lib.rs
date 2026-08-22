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

fn import_targets(expr: &px::PxExpr, out: &mut Vec<String>) {
    match expr {
        px::PxExpr::DeferredError(_) => {}
        px::PxExpr::Isolated { with_scope, body } => {
            if let Some(scope) = with_scope {
                import_targets(scope, out);
            }
            import_targets(body, out);
        }
        px::PxExpr::Apply { func, arg } => {
            if matches!(func.as_ref(), px::PxExpr::Var(name) if name == "import") {
                if let px::PxExpr::Var(marked) = arg.as_ref() {
                    if marked.starts_with(":path:") {
                        out.push(marked.chars().skip(6).collect());
                        return;
                    }
                }
            }
            if let px::PxExpr::Apply { func: inner, arg: scope } = func.as_ref() {
                if matches!(inner.as_ref(), px::PxExpr::Var(name) if name == "scopedImport") {
                    if let px::PxExpr::Var(marked) = arg.as_ref() {
                        if marked.starts_with(":path:") {
                            out.push(marked.chars().skip(6).collect());
                            import_targets(scope, out);
                            return;
                        }
                    }
                }
            }
            import_targets(func, out);
            import_targets(arg, out);
        }
        px::PxExpr::Str(parts) => {
            for part in parts {
                if let px::PxStrPart::Sub(value) = part {
                    import_targets(value, out);
                }
            }
        }
        px::PxExpr::List(items) => {
            for item in items {
                import_targets(item, out);
            }
        }
        px::PxExpr::Select { base, .. } => import_targets(base, out),
        px::PxExpr::Lambda { body, .. } => import_targets(body, out),
        px::PxExpr::If { cond, then_e, else_e } => {
            import_targets(cond, out);
            import_targets(then_e, out);
            import_targets(else_e, out);
        }
        px::PxExpr::Binary { lhs, rhs, .. } => {
            import_targets(lhs, out);
            import_targets(rhs, out);
        }
        px::PxExpr::LetIn { bindings, body } => {
            for (_, value) in bindings {
                import_targets(value, out);
            }
            import_targets(body, out);
        }
        px::PxExpr::With { scope, body } => {
            import_targets(scope, out);
            import_targets(body, out);
        }
        px::PxExpr::Attrs(fields) => {
            for (_, value) in fields {
                import_targets(value, out);
            }
        }
        px::PxExpr::Int(_)
        | px::PxExpr::Float(_)
        | px::PxExpr::Bool(_)
        | px::PxExpr::Null
        | px::PxExpr::Var(_) => {}
    }
}

fn normalize_host_path(path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("eval_file: current directory: {error}"))?
            .join(path)
    };
    let mut normalized = std::path::PathBuf::new();
    for part in absolute.components() {
        match part {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

fn module_key(path: &std::path::Path) -> String {
    format!(".{}", path.to_string_lossy())
}

fn load_module(
    path: &std::path::Path,
    modules: &mut Vec<(String, String)>,
) -> Result<String, String> {
    let path = normalize_host_path(path)?;
    let key = module_key(&path);
    if modules.iter().any(|(known, _)| *known == key) {
        return Ok(key);
    }
    let source = std::fs::read_to_string(&path)
        .map_err(|error| format!("eval_file: read {}: {error}", path.display()))?;
    let ast = px::px_parse(&source)?;
    modules.push((key.clone(), source));

    let mut targets = Vec::new();
    import_targets(&ast, &mut targets);
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("/"));
    for target in targets {
        // Keep lazy import behavior: a missing module in dead code is lowered
        // to the evaluator's deferred import error by px_expand_imports.
        let _ = load_module(&parent.join(target), modules);
    }
    Ok(key)
}

/// Evaluate a `.px` file with the same transitive relative-import closure as
/// the CLI `px-eval -f` path, returning the raw PNIX value for host embedding.
pub fn eval_file_value(path: &str) -> Result<px::PxVal, String> {
    let mut modules = Vec::new();
    let key = load_module(std::path::Path::new(path), &mut modules)?;
    let source = modules
        .iter()
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, source)| source.clone())
        .ok_or_else(|| String::from("eval_file: entry module missing"))?;
    px::px_run_value_with_modules(&source, &modules, &key)
}

/// Host-language import of a `.px` file (transitive read + eval).
/// Host-bound (Rust/rs); not a portable multi-host bytecode package.
pub fn eval_file(path: &str) -> Result<String, String> {
    eval_file_value(path).map(|value| px::px_print(&value))
}

/// Call one exported, curried function from a `.px` attrset module.
/// Arguments/results are PNIX values so Rust callers can keep exact integers.
pub fn call_file(
    path: &str,
    entry: &str,
    arguments: &[px::PxVal],
) -> Result<px::PxVal, String> {
    let module = px::px_force(&eval_file_value(path)?)?;
    let fields = match module {
        px::PxVal::Attrs(fields) => fields,
        _ => return Err(String::from("call_file: module must evaluate to an attrset")),
    };
    let selected = fields
        .iter()
        .find(|(name, _)| name == entry)
        .map(|(_, value)| value.clone())
        .ok_or_else(|| format!("call_file: missing entry {entry}"))?;
    let mut current = px::px_force(&selected)?;
    for argument in arguments {
        current = px::px_apply(&current, argument.clone())?;
    }
    px::px_force_deep(&current)
}

/// JSON-safe common host ABI for pure `.px` library entrypoints.
/// `arguments_json` must be an array; the result is canonical JSON.
pub fn call_file_json(
    path: &str,
    entry: &str,
    arguments_json: &str,
) -> Result<String, String> {
    let arguments = match px::px_from_json(arguments_json)? {
        px::PxVal::List(values) => values,
        _ => return Err(String::from("call_file_json: arguments must be a JSON array")),
    };
    let result = call_file(path, entry, arguments.as_ref())?;
    px::px_to_json(&result)
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

#[cfg(test)]
mod tests {
    use super::{call_file_json, eval_file};

    fn example(name: &str) -> String {
        format!(
            "{}/examples/production-readiness/{name}",
            env!("CARGO_MANIFEST_DIR")
        )
    }

    #[test]
    fn eval_file_resolves_relative_imports() {
        let value = eval_file(&example("consumer.px")).expect("consumer");
        assert!(value.contains("answer = 42"), "{value}");
    }

    #[test]
    fn calls_exported_library_functions_with_json_data() {
        let library = example("library.px");
        assert_eq!(call_file_json(&library, "double", "[21]").unwrap(), "42");
        assert_eq!(
            call_file_json(&library, "mapDouble", "[[1,2,3]]").unwrap(),
            "[2,4,6]"
        );
        assert_eq!(
            call_file_json(&library, "summarize", "[[1,2,3,4]]").unwrap(),
            "{\"count\":4,\"total\":10}"
        );
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
