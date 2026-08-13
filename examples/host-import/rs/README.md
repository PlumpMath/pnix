# rs host-import example

Cookbook: [`../../pnix-rs/docs/CARGO_HOST_IMPORT.md`](../../pnix-rs/docs/CARGO_HOST_IMPORT.md)

## Mini crate (path dep) — started

```bash
cd pnix-rs-smoke
cargo run -q -- ../../hello.px
# => 3
```

```toml
# Cargo.toml (excerpt)
pnix-rs = { path = "../../../../pnix-rs/pnix-rs", package = "pnix-rs" }
```

```rust
pnix_rs::eval_file("../../hello.px")
```

## Fastest smoke without a crate

```bash
pnix-rs px-eval -f ../hello.px
# or
pnix-rs px-eval -c '1 + 2'
```

## System library (HM / nix build)

```bash
pnix-rs-refs   # or: nix run ./pnix-rs#pnix-rs-refs
# then link with -L $PNIX_RS_LIB_DIR (C / embed); pure Rust prefers path dep above
```
