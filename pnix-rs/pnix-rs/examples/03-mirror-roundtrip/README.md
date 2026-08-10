# 03-mirror-roundtrip — 정본 AST + 손실 없는 왕복(roundtrip)

## 쉽게 말하면 (비유)
코드를 파싱→다시 문자열로 찍었을 때 원래와 '의미가 같은가'를 plain Rust는 스스로 증명하지 않는다.
pnix-rs의 mirror는 한 소스의 **모든 단면**(토큰/값/emit/reparse/재평가)을 한 번에 물화하고
**emit 고정점 + 재평가 일치**를 lossless로 판정한다.

## 무엇을
한 px 소스를 **singleton mirror_run**으로 통과시켜 source/tokens/value/emit/reparse_ok/
revalue_match/emit_fixed_point/status 를 한 레코드로 낸다.

## plain Rust의 한계 (`limit_rust.rs`)
Rust에는 '파싱→재출력이 의미를 보존하는가'를 판정하는 상태 어휘(lossless/held/rejected)나
정본 재출력이 표준으로 없다. `{:?}`(Debug)는 안정성 보증이 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs mirror -c '<px>'` — 모든 단면 + `emit_fixed_point` + `status lossless`
- emit을 다시 파싱·평가해 원래 값과 일치함(`revalue_match true`)을 증명

## 어디에 쓰나
번역/리팩터링의 의미보존 검증, 정본 직렬화.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/03-mirror-roundtrip/limit_rust.rs -o /tmp/limit_03-mirror-roundtrip && /tmp/limit_03-mirror-roundtrip

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/03-mirror-roundtrip/pnix_rs_way.sh
```
