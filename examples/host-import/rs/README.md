# rs host-import 예제

요리책: [`../../pnix-rs/docs/CARGO_HOST_IMPORT.md`](../../pnix-rs/docs/CARGO_HOST_IMPORT.md)

## 미니 crate (path dep) — 시작점

```bash
cd pnix-rs-smoke
cargo run -q -- ../../hello.px
# => 3
```

```toml
# Cargo.toml (발췌)
pnix-rs = { path = "../../../../pnix-rs/pnix-rs", package = "pnix-rs" }
```

```rust
pnix_rs::eval_file("../../hello.px")
```

## crate 없이 가장 빠른 스모크

```bash
pnix-rs px-eval -f ../hello.px
# 또는
pnix-rs px-eval -c '1 + 2'
```

## 시스템 라이브러리 (HM / nix build)

```bash
pnix-rs-refs   # 또는: nix run ./pnix-rs#pnix-rs-refs
# 그다음 -L $PNIX_RS_LIB_DIR 로 링크 (C / embed);
# 순수 Rust 는 위 path dep 를 선호
```
