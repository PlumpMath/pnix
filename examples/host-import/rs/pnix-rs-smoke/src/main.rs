//! Minimal host-main import of the pnix-rs library (path dependency).
//!
//! ```bash
//! cd examples/host-import/rs/pnix-rs-smoke
//! cargo run -q -- ../../hello.px
//! # => 3
//! ```

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--call") {
        if args.len() != 4 {
            eprintln!("usage: pnix-rs-smoke --call FILE.px ENTRY ARGS_JSON");
            std::process::exit(2);
        }
        match pnix_rs::call_file_json(&args[1], &args[2], &args[3]) {
            Ok(value) => println!("{value}"),
            Err(err) => {
                eprintln!("pnix-rs-smoke: {err}");
                std::process::exit(1);
            }
        }
        return;
    }
    let path = args
        .first()
        .cloned()
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
