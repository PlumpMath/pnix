# rs host-import example (lightweight)

Prefer the monorepo path-dep cookbook:

→ [`../../pnix-rs/docs/CARGO_HOST_IMPORT.md`](../../pnix-rs/docs/CARGO_HOST_IMPORT.md)

## Fastest smoke (no crate scaffolding)

```bash
# pnix-main style (already on PATH with HM)
pnix-rs px-eval -f ../hello.px
# or
pnix-rs px-eval -c '1 + 2'
```

## Host-main path dep (when you add a real crate)

```toml
[dependencies]
pnix_rs = { path = "../../../pnix-rs/pnix-rs", package = "pnix-rs" }
```

```rust
fn main() {
    println!("{}", pnix_rs::eval_file("../hello.px").unwrap());
}
```

A full `Cargo.toml` demo crate is **P2.2 hard follow-up** (optional); not started
to avoid another workspace lockfile in the monorepo.
