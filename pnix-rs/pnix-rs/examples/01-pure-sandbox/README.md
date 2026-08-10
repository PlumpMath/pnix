# 01-pure-sandbox — 신뢰 가능한 평가 — 순수성/효과 게이트

## 쉽게 말하면 (비유)
낯선 코드를 실행하는 건 낯선 사람에게 집 열쇠를 통째로 주는 것이다.
pnix-rs의 gate는 **실행 전에** "이 코드가 순수한가, 무슨 효과가 필요한가"를 판정하고,
모르는 것은 **fail-closed**(의심스러우면 거부)한다.

## 무엇을
신뢰할 수 없는 px 코드를 **부작용 분류 + 효과 클래스 admission + 미지 builtin fail-closed**로 게이트한다.

## plain Rust의 한계 (`limit_rust.rs`)
Rust에는 임의 코드를 '실행 전에 순수한지' 정적으로 판정하는 표준 수단이 없다.
매크로/문자열-eval도 없고, `unsafe`/FFI/`std::process`는 문법상 언제나 허용된다 —
즉 *plain Rust는 신뢰 경계 밖 로직을 안전하게 가둘 내장 게이트가 없다.*

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs gate -c '<px>'` — 순수/효과-클래스/미지 builtin을 **실행 전** 판정(witness 포함)
- 순수식은 `pure true / allowed true`, 미지 builtin은 `uncertain [...]`로 **fail-closed**

## 어디에 쓰나
사용자 제출 로직 / 설정 DSL / 규칙식의 **안전한 평가**, 감사 가능한 계산 레이어.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/01-pure-sandbox/limit_rust.rs -o /tmp/limit_01-pure-sandbox && /tmp/limit_01-pure-sandbox

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/01-pure-sandbox/pnix_rs_way.sh
```
