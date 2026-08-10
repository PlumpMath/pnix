//! Pnix-agnostic, capability-gated read-only host I/O substrate.
//!
//! This file is source-shared by pnix-rs's interop boundary. It deliberately
//! contains no pnix request/value semantics.

use std::path::Path;

pub const FILE_READ_CAPABILITY: &str = "file-read";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetaIoError {
    pub error_class: String,
    pub message: String,
}

fn error(error_class: &str, message: &str) -> MetaIoError {
    MetaIoError {
        error_class: String::from(error_class),
        message: String::from(message),
    }
}

fn require_file_read(granted: &[String]) -> Result<(), MetaIoError> {
    if granted.iter().any(|item| item == FILE_READ_CAPABILITY) {
        Ok(())
    } else {
        Err(error("capability-denied", "file-read capability denied"))
    }
}

fn classify(path: &Path) -> Result<String, MetaIoError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| error("not-found", "file type target not found"))?;
    let kind = metadata.file_type();
    if kind.is_symlink() {
        Ok(String::from("symlink"))
    } else if kind.is_dir() {
        Ok(String::from("directory"))
    } else if kind.is_file() {
        Ok(String::from("regular"))
    } else {
        Ok(String::from("unknown"))
    }
}

pub fn path_exists(path: &str, granted: &[String]) -> Result<bool, MetaIoError> {
    require_file_read(granted)?;
    Ok(Path::new(path).exists())
}

pub fn file_type(path: &str, granted: &[String]) -> Result<String, MetaIoError> {
    require_file_read(granted)?;
    classify(Path::new(path))
}

pub fn read_utf8(path: &str, granted: &[String]) -> Result<String, MetaIoError> {
    require_file_read(granted)?;
    let bytes = std::fs::read(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            error("not-found", "read target not found")
        } else {
            error("io-error", "read failed")
        }
    })?;
    String::from_utf8(bytes).map_err(|_| error("invalid-utf8", "read target is not UTF-8"))
}

pub fn read_dir(path: &str, granted: &[String]) -> Result<Vec<(String, String)>, MetaIoError> {
    require_file_read(granted)?;
    let entries = std::fs::read_dir(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            error("not-found", "directory not found")
        } else if e.kind() == std::io::ErrorKind::NotADirectory {
            error("not-directory", "target is not a directory")
        } else {
            error("io-error", "directory read failed")
        }
    })?;
    let mut out = Vec::new();
    for item in entries {
        let entry = item.map_err(|_| error("io-error", "directory read failed"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = classify(&entry.path())?;
        out.push((name, kind));
    }
    out.sort();
    Ok(out)
}

#[allow(dead_code)]
pub fn io_check() -> bool {
    println!("[io-check (capability-gated read-only host I/O)]");
    let none: Vec<String> = Vec::new();
    let granted = vec![String::from(FILE_READ_CAPABILITY)];
    let denied = matches!(
        path_exists("Cargo.toml", &none),
        Err(MetaIoError { ref error_class, .. }) if error_class == "capability-denied"
    );
    let ready = denied
        && path_exists("Cargo.toml", &granted) == Ok(true)
        && file_type("Cargo.toml", &granted) == Ok(String::from("regular"))
        && read_utf8("Cargo.toml", &granted)
            .map(|text| text.contains("name = \"rs-meta\""))
            == Ok(true)
        && read_dir("src", &granted)
            .map(|entries| entries.iter().any(|(name, kind)| name == "main.rs" && kind == "regular"))
            == Ok(true);
    if ready {
        println!("  => PASS (file-read denied by default; four read effects ready)");
    } else {
        println!("  => FAIL");
    }
    ready
}
