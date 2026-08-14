# Cargo host-main에서 `pnix-rs-library` 임포트

**교리:** [`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md) · [`../../HOST_IMPORT.md`](../../HOST_IMPORT.md)

`pnix-rs`는 `publish = false`이며 **crates.io 의존 0개**. 호스트 crate는
오늘 crates.io 좌표를 쓰지 않는다. 지원 패턴 둘:

---

## A. 시스템 라이브러리 (nix / HM) — 일상용 권장

`pnix-rs-rs` (bare `cargo`/`rustc`) 또는 `pnix-rs-refs` 이후:

```bash
pnix-rs-refs
# PNIX_RS_LIB_DIR=…/lib
# PNIX_RS_INCLUDE_DIR=…/include
```

### `build.rs` (선택) — rustc 플래그 출력

```rust
fn main() {
    if let Ok(dir) = std::env::var("PNIX_RS_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
        println!("cargo:rustc-link-lib=static=pnix_rs"); // 또는 dylib
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

### 순수 Rust (같은 crate 트리)

`~/pnix` 안에서 개발할 때는 monorepo **path dependency** 우선 (패턴 B).
설치된 rlib은 임베딩 / C / 다언어 호스트용.

---

## 로컬 export (개인 피드, crates.io 아님)

```bash
cd pnix-rs/pnix-rs
./bin/export-pnix-rs-library          # → target/pnix-rs-library/{lib,include}
./bin/pnix-rs-library-smoke
set -a; source target/pnix-rs-library/refs.env; set +a
```

`publish = false` 유지; path dep 또는 export된 `lib/` + `include/` 사용.

## B. Path dependency (체크아웃)

인트리 미니 데모:

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

참고: crate 이름은 `pnix-rs`이고 lib 이름은 `pnix_rs`
(`[lib] name = "pnix_rs"`)이므로 `package = "pnix-rs"`가 필요.

---

## C. `nix build` 아티팩트만

```bash
cd pnix-rs
nix build .#pnix-rs-library
ls result/lib result/include
export PNIX_RS_LIB_DIR=$PWD/result/lib
export PNIX_RS_INCLUDE_DIR=$PWD/result/include
```

---

## 하지 말 것

- 전체 `pnix-rs` 패키지 (dylib 포함)와 `pnix-rs-library`를 하나의
  home-manager `buildEnv`에 혼합 (파일 충돌).
- crates.io `pnix-rs` 기대 — 미게시 (`publish = false`).
- 이식 가능한 멀티호스트 `.px` 패키지 주장; 이것은 **Rust/rs 호스트 바인딩**.
