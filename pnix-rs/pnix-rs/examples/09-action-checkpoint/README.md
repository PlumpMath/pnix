# 09-action-checkpoint — 액션 체크포인트 — 하나의 verdict

## 쉽게 말하면 (비유)
실행 결과가 '허용해도 되는지'를 판단하려면 순수성·왕복·해시·증거를 따로 조립해야 한다.
pnix-rs의 action은 그 모두를 모아 **하나의 verdict**(allowed/refused)로 준다 — 새 기계 없이.

## 무엇을
**ActionVerdict** = gate + mirror + ir + witness를 조합한 단일 판정(allowed = gate_allowed && mirror lossless).

## plain Rust의 한계 (`limit_rust.rs`)
Rust에는 '이 계산을 승인해도 되는가'를 순수성/의미보존/증거로 종합 판정하는 표준 체크포인트가 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs action -c '<px>'` — 단일 verdict(gate+mirror+ir+witness)
- `pnix-rs action-check` — admitted/refused/결정성

## 어디에 쓰나
정책 게이트, 배포 승인, 신뢰 경계 통과 판정.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/09-action-checkpoint/limit_rust.rs -o /tmp/limit_09-action-checkpoint && /tmp/limit_09-action-checkpoint

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/09-action-checkpoint/pnix_rs_way.sh
```
