# 12-bta-analysis — 이진시간 분석(BTA) — specializer와 교차검증

## 쉽게 말하면 (비유)
부분평가가 '무엇을 접을 수 있는가'를 실행 전에 예측하는 오프라인 분석이 BTA다.
pnix-rs의 BTA는 static/dynamic을 분류하고, 그 예측을 **실제 specializer(mix)와 교차검증**한다.

## 무엇을
px 식을 static/dynamic으로 monovariant 분류(미지=Dynamic 보수적), if-조건 BTA를
mix 폴딩과 교차검증(정적 if ⟺ mix 폴딩). BTA는 폴딩의 상한(upper bound)임을 명시.

## plain Rust의 한계 (`limit_rust.rs`)
Rust에는 binding-time analysis가 없다. '어떤 부분이 컴파일-타임에 접히는가'를 예측하고
그 예측을 실제 특화 거동과 대조하는 표준 분석기가 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs bta-check` — static/dynamic 분류 + specializer 교차검증 + 상한 경계

## 어디에 쓰나
부분평가 준비, 스테이징 분석, 종결성 진단.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/12-bta-analysis/limit_rust.rs -o /tmp/limit_12-bta-analysis && /tmp/limit_12-bta-analysis

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/12-bta-analysis/pnix_rs_way.sh
```
