# 05-witness-and-gate — 증거(witness) + 권한(capability) 게이트

## 쉽게 말하면 (비유)
그냥 실행하면 '무엇을 했는지'의 증거가 남지 않는다.
pnix-rs는 실행에 **13-필드 witness**(방향/입출력 해시/효과/상태...)를 붙이고,
host 접촉은 **capability 게이트**로 허가 없으면 fail-closed한다.

## 무엇을
eval에 **내용해시 witness**(동결된 13-필드 스키마)를 부여하고, host-call은 grant 없으면 거부.

## plain Rust의 한계 (`limit_rust.rs`)
Rust에는 실행 결과에 '무엇을·어떤 효과로·같은 입력인지'의 기계가독 증거를 붙이는
표준이 없고, 함수가 요구하는 효과를 호출부에서 강제 허가하는 capability 모델도 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs witness -c '<px>'` — 13-필드 witness(in/out/env 해시, effect, status)
- `pnix-rs interop-check` — host-call은 grant 없이 **거부**(denial + witness)

## 어디에 쓰나
감사(auditable) 실행, 공급망 증명(in-toto/SLSA 유사), 최소권한.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/05-witness-and-gate/limit_rust.rs -o /tmp/limit_05-witness-and-gate && /tmp/limit_05-witness-and-gate

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/05-witness-and-gate/pnix_rs_way.sh
```
