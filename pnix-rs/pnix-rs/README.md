# pnix-rs

**rs-meta 기반 pnix 런타임 프론트엔드** — pnix 런타임 경로의 Rust 호스트 레인.
`pnix-clj`(clj-meta 기반), `pnix-hy`(hy-meta 기반)의 형제.

```text
pnix-rs    = pnix 런타임 경로용 Rust bootstrap/프론트엔드 (이 레인)
../rs-meta = Rust 메타원형 stage15-N 컴파일러/평가기 substrate (의존)
runtime/   = 저장소 소유 .px 런타임 아티팩트
```

rs-meta 의존은 명목적이 아니라 반증 가능하다: px 엔진(`src/px.rs`)은
rs-meta가 평가하는 Rust 부분집합 안에서 작성되고, `substrate-check`는
rs-meta bootstrap이 그 소스를 해석한 뒤, 출력이 rustc 컴파일 실행 및
이 바이너리 자체 네이티브 동작과 같은 `.px` 프로브에서 일치(3-way equality)를
요구한다.

## 명령

```sh
export CARGO_TARGET_DIR=/tmp/pnix-rs-target
cargo build --release
P=/tmp/pnix-rs-target/release/pnix-rs

$P px-check          # seed .px 코퍼스 -> 기대 정규 출력
$P substrate-check   # rs-meta interp == rs-meta rustc == pnix-rs native
$P px-eval -c 'let a = 1; b = a + 2; in a + b'
$P px-eval -f runtime/corpus/c05_recurse.px
```

## Seed .px 표면

정수, 불리언, `+ - * /`, 비교, `if/then/else`, 람다 `param: body`,
병치 적용 (`f x y`), **재귀** `let ... in` (형제·자기 참조 해석 — pnix let 의미),
attrset 리터럴 `{ k = v; }` (정렬된 정규 출력), `#` 주석.
seed 밖 (float, 문자열, 리스트, `//` merge, selection, builtins)은 정직하게
거부되며(대부분은 이후 확장됨 — 현재 지원 현황은
`docs/IMPLEMENTATION.md`, 남은 진짜 갭은 `docs/BUGS.md`에 추적된다).

`runtime/corpus/`의 코퍼스 파일은 이후 크로스호스트 비교를 위해 pnix-clj
`rust_grounded` 불변 코퍼스에서 벤더한 두 케이스(`c05_recurse`, `c09_lambda`)와
seed 소유 케이스(재귀 let 회귀 가드 `seed_let_rec.px` 포함)를 포함한다.

**이중 축 / Cargo 임포트:** [`docs/IMPLEMENTATION.md`](docs/IMPLEMENTATION.md) §7,
[`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md).
