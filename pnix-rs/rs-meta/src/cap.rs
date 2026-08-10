//! Capability gate for host contact (Plan Phase E3). The interpreter floor
//! needs NO capabilities; the native tier (rustc compile / artifact run),
//! generic subprocess spawns, and proof-file writes are explicit gates.
//!
//! Grant model: rs-meta is a local tool, so the DEFAULT is all-granted
//! (RSMETA_CAPS unset). Setting RSMETA_CAPS to a comma list restricts to
//! exactly that set — including the empty set — and denial is fail-closed
//! with a stable, machine-matchable message. cap-check proves both
//! directions in clean processes, and proves `run` (the trusted floor)
//! works with ZERO capabilities.
//!
//! Bootstrap source/corpus reads remain the trusted tool substrate. Reusable
//! host I/O exposed to dependent runtimes is separate and explicitly gated by
//! `file-read` through `io.rs`.

pub const CAP_NATIVE_COMPILE: &str = "native-compile";
pub const CAP_NATIVE_RUN: &str = "native-run";
pub const CAP_SUBPROCESS: &str = "subprocess";
pub const CAP_FS_WRITE: &str = "fs-write";
pub const CAP_FS_READ: &str = "file-read";

pub fn cap_granted(name: &str) -> bool {
    match std::env::var("RSMETA_CAPS") {
        Err(_) => true,
        Ok(csv) => {
            for part in csv.split(",") {
                if part.trim() == name {
                    return true;
                }
            }
            false
        }
    }
}

pub fn require_cap(name: &str) -> Result<(), String> {
    if cap_granted(name) {
        Ok(())
    } else {
        Err(format!(
            "gate: capability {} not granted (RSMETA_CAPS)",
            name
        ))
    }
}
