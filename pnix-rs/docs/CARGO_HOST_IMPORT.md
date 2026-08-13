# Cargo host-main import of `pnix-rs-library`

**Doctrine:** [`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md) · [`../../HOST_IMPORT.md`](../../HOST_IMPORT.md)

`pnix-rs` is `publish = false` and has **zero crates.io deps**. Host crates do
not take a crates.io coordinate today. Two supported patterns:

---

## A. System library (nix / HM) — recommended for day-to-day

After `pnix-rs-rs` (bare `cargo`/`rustc`) or `pnix-rs-refs`:

```bash
pnix-rs-refs
# PNIX_RS_LIB_DIR=…/lib
# PNIX_RS_INCLUDE_DIR=…/include
```

### `build.rs` (optional) — print rustc flags

```rust
fn main() {
    if let Ok(dir) = std::env::var("PNIX_RS_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rustc-link-lib=static=pnix_rs"); // or dylib
    }
    if let Ok(inc) = std::env::var("PNIX_RS_INCLUDE_DIR") {
        println!("cargo:rerun-if-env-changed=PNIX_RS_INCLUDE_DIR");
        println!("cargo:INCLUDE={inc}");
    }
}
```

### C FFI

```c
#include "pnix_rs.h"
/* compile with: -I$PNIX_RS_INCLUDE_DIR -L$PNIX_RS_LIB_DIR -lpnix_rs */
```

### Pure Rust (same crate tree)

Prefer **path dependency** on the monorepo when you develop inside `~/pnix`
(pattern B). The installed rlib is for embedding / C / multi-lang hosts.

---

## Local export (personal feed, not crates.io)

```bash
cd pnix-rs/pnix-rs
./bin/export-pnix-rs-library          # → target/pnix-rs-library/{lib,include}
./bin/pnix-rs-library-smoke
set -a; source target/pnix-rs-library/refs.env; set +a
```

`publish = false` remains; use path dep or the exported `lib/` + `include/`.

## B. Path dependency (checkout)

In-tree mini demo:

```bash
cd examples/host-import/rs/pnix-rs-smoke
cargo run -q -- ../../hello.px   # => 3
```

```toml
# Cargo.toml
[dependencies]
pnix-rs = { path = "../../../../pnix-rs/pnix-rs", package = "pnix-rs" }
```

```rust
fn main() {
    println!("{}", pnix_rs::eval("1 + 2").unwrap());
    println!("{}", pnix_rs::eval_file("prog.px").unwrap());
}
```

Note: `package = "pnix-rs"` because the crate name is `pnix-rs` while the lib
name is `pnix_rs` (`[lib] name = "pnix_rs"`).

---

## C. `nix build` artifact only

```bash
cd pnix-rs
nix build .#pnix-rs-library
ls result/lib result/include
export PNIX_RS_LIB_DIR=$PWD/result/lib
export PNIX_RS_INCLUDE_DIR=$PWD/result/include
```

---

## Do not

- Mix full `pnix-rs` package (ships dylib) and `pnix-rs-library` in one
  home-manager `buildEnv` (file clash).
- Expect crates.io `pnix-rs` — not published (`publish = false`).
- Claim a portable multi-host `.px` package; this is **Rust/rs host-bound**.
