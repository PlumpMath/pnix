# 11-self-hosting-tower — 자기호스팅 타워 — px가 px를 평가(S=L)

## 쉽게 말하면 (비유)
메타-circular의 정수: **px로 쓴 self-interpreter**가 물화된 px 프로그램을 평가하고,
그 결과가 네이티브 평가와 **일치**한다(S=L 씨앗). reify/reflect로 코드↔데이터를 오간다.

## 무엇을
reify(px AST -> px 데이터) / reflect(역) + `runtime/tower/self_interp.px`가 물화 프로그램 평가,
그리고 재귀 let·전 표면·고차 builtins·call-by-need까지 == 네이티브.

## plain Rust의 한계 (`limit_rust.rs`)
Rust에는 '자기 언어로 쓴 self-interpreter가 물화된 자기 프로그램을 평가해 네이티브와 수렴'
하는 S=L 구조가 없다(코드=데이터가 아니다).

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs tower-check` — reify/reflect 왕복 + px 자기해석기 == 네이티브(전 표면)

## 어디에 쓰나
메타프로그래밍, 스테이징, 부분평가의 기반.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/11-self-hosting-tower/limit_rust.rs -o /tmp/limit_11-self-hosting-tower && /tmp/limit_11-self-hosting-tower

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/11-self-hosting-tower/pnix_rs_way.sh
```
