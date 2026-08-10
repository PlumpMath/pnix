//! Build-time host stack sizing for the bootstrap binary.
//!
//! The stage2/stage3 chain gates run rs-meta evaluators inside rs-meta
//! evaluators. That intentionally nests evaluator call stacks. Keep this outside
//! `src/*.rs` so the self-host source surface does not need host-thread APIs.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-stack_size,0x8000000");
    }
}
