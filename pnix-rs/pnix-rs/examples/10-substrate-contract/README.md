# 10-substrate-contract — 기판 계약 — rs-meta ↔ pnix-rs 3-way 의존 증명

## 쉽게 말하면 (비유)
pnix-rs의 px 엔진(src/px.rs)은 **rs-meta의 평가 subset 안에서** 쓰여 있다.
substrate-check는 그 소스를 **rs-meta 인터프리터 == rustc == 네이티브** 세 방향으로 돌려
'pnix-rs가 rs-meta에 의존한다'를 반증 가능하게 증명한다.

## 무엇을
src/px.rs + 하니스를 rs-meta bootstrap으로 interp/native 실행하고 네이티브와 대조 — 3-way 동등.

## plain Rust의 한계 (`limit_rust.rs`)
Rust에서 '인터프리터 구현 == 컴파일 결과'를 프로그램마다 강제하는 translation-validation은
표준으로 없다. plain Rust는 두 실행 경로의 합치를 스스로 증명하지 않는다.

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs substrate-check` — rs-meta interp == rustc == native (의존 증명)
  (`RS_META_BOOTSTRAP`이 rs-meta bootstrap을 가리켜야 함 — flake devShell이 자동 설정)

## 어디에 쓰나
메타-circular 계약, 컴파일러 검증, 신뢰 기반(TCB) 축소.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/10-substrate-contract/limit_rust.rs -o /tmp/limit_10-substrate-contract && /tmp/limit_10-substrate-contract

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/10-substrate-contract/pnix_rs_way.sh
```
