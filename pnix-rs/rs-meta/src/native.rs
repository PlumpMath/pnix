//! Native tier: run the *same Rust source* through `rustc` and read its stdout.
//!
//! Because rs-meta's target language IS Rust, the native tier is direct — there
//! is nothing to "lower". This is the Evcxr mechanism (Rust evaluated via rustc)
//! done in-house with zero crates.io dependencies: `std::process` invokes the
//! `rustc` toolchain. The interpreter (`interp.rs`) is the oracle; this path must
//! agree with it on every program (translation validation).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Compile `src` (a complete Rust program with `fn main`) with rustc and return
/// its captured stdout.
pub fn native_run(src: &str, workdir: &Path) -> Result<String, String> {
    crate::cap::require_cap(crate::cap::CAP_NATIVE_RUN)?;
    let artifact = compile_native(src, workdir, true)?;
    let run = Command::new(&artifact.bin_path)
        .output()
        .map_err(|e| format!("native: failed to run binary: {}", e))?;
    if !run.status.success() {
        return Err(format!(
            "native: program exited non-zero:\n{}",
            String::from_utf8_lossy(&run.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&run.stdout).to_string())
}

pub fn native_artifact_hash(src: &str, workdir: &Path) -> Result<u64, String> {
    let artifact = compile_native(src, workdir, false)?;
    let bytes =
        fs::read(&artifact.bin_path).map_err(|e| format!("native: read artifact: {}", e))?;
    Ok(crate::hash::fnv1a_bytes(&bytes))
}

pub fn native_artifact_receipt(src: &str, workdir: &Path) -> Result<String, String> {
    let artifact_hash = native_artifact_hash(src, workdir)?;
    let rustc = rustc_version()?;
    Ok(format!(
        "stage=stage8-repro-seed\n\
         source_fnv={:016x}\n\
         rustc={}\n\
         flags={}\n\
         artifact_fnv={:016x}\n",
        crate::hash::fnv1a_text(src),
        rustc.trim(),
        deterministic_flags_record(),
        artifact_hash
    ))
}

pub fn native_cache_probe(src: &str, workdir: &Path) -> Result<bool, String> {
    let _first = compile_native(src, workdir, true)?;
    let second = compile_native(src, workdir, true)?;
    Ok(second.cache_hit)
}

struct NativeArtifact {
    bin_path: PathBuf,
    cache_hit: bool,
}

fn compile_native(src: &str, workdir: &Path, use_cache: bool) -> Result<NativeArtifact, String> {
    crate::cap::require_cap(crate::cap::CAP_NATIVE_COMPILE)?;
    crate::cap::require_cap(crate::cap::CAP_FS_WRITE)?;
    fs::create_dir_all(workdir).map_err(|e| format!("native: mkdir {:?}: {}", workdir, e))?;
    let hash = crate::hash::fnv1a_text(&cache_key(src));
    let src_path = workdir.join(format!("prog_{:016x}.rs", hash));
    let bin_path = workdir.join(format!("prog_{:016x}{}", hash, bin_suffix()));
    if use_cache && bin_path.exists() {
        if let Ok(existing) = fs::read_to_string(&src_path) {
            if existing == src {
                return Ok(NativeArtifact {
                    bin_path,
                    cache_hit: true,
                });
            }
        }
    }
    fs::write(&src_path, src).map_err(|e| format!("native: write src: {}", e))?;

    let compile = Command::new("rustc")
        .env("SOURCE_DATE_EPOCH", "0")
        .arg("--edition")
        .arg("2021")
        .arg("-O")
        .arg("-A")
        .arg("warnings")
        .arg("-C")
        .arg("debuginfo=0")
        .arg("-C")
        .arg("metadata=rsmeta")
        .arg("-C")
        .arg("codegen-units=1")
        .arg("--remap-path-prefix")
        .arg(format!("{}=.", workdir.display()))
        .arg(&src_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .map_err(|e| format!("native: failed to invoke rustc: {}", e))?;
    if !compile.status.success() {
        return Err(format!(
            "native: rustc rejected the program:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        ));
    }
    Ok(NativeArtifact {
        bin_path,
        cache_hit: false,
    })
}

fn rustc_version() -> Result<String, String> {
    crate::cap::require_cap(crate::cap::CAP_SUBPROCESS)?;
    let version = Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|e| format!("native: failed to invoke rustc --version: {}", e))?;
    if !version.status.success() {
        return Err(format!(
            "native: rustc --version failed:\n{}",
            String::from_utf8_lossy(&version.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&version.stdout).trim().to_string())
}

fn deterministic_flags_record() -> &'static str {
    "SOURCE_DATE_EPOCH=0|--edition=2021|-O|-A=warnings|-C=debuginfo=0|-C=metadata=rsmeta|-C=codegen-units=1|--remap-path-prefix=<workdir>=."
}

fn cache_key(src: &str) -> String {
    format!("{}\n{}", deterministic_flags_record(), src)
}

fn bin_suffix() -> &'static str {
    if cfg!(windows) {
        ".exe"
    } else {
        ""
    }
}

/// Deterministic content hash (FNV-1a) — stable file names without Date/rand,
/// so the native tier is cacheable and reproducible.


pub fn default_workdir() -> PathBuf {
    PathBuf::from("work/native")
}
