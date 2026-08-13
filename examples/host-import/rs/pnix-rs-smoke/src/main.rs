//! Minimal host-main import of the pnix-rs library (path dependency).
//!
//! ```bash
//! cd examples/host-import/rs/pnix-rs-smoke
//! cargo run -q -- ../../hello.px
//! # => 3
//! ```

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../../hello.px".to_string());
    match pnix_rs::eval_file(&path) {
        Ok(value) => {
            println!("{value}");
        }
        Err(err) => {
            eprintln!("pnix-rs-smoke: {err}");
            std::process::exit(1);
        }
    }
}
