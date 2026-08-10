# 06-specialization-futamura — 부분평가(specialize) + Futamura 사영

## 쉽게 말하면 (비유)
인터프리터에 '고정된 프로그램'을 부분평가하면, 해석 계층이 사라진 **잔여 프로그램**이 나온다.
pnix-rs는 px로 쓴 specializer로 **1차·2차 Futamura 사영**을 실제로 돌린다.

## 무엇을
닫힌 부분식을 sacred runtime으로 fold(A4-건전), 그리고 px-표현 specializer로
인터프리터를 고정 프로그램에 특화(1차) / mix를 인터프리터에 특화해 컴파일러 도출(2차).

## plain Rust의 한계 (`limit_rust.rs`)
Rust에는 부분평가/잔여 프로그램 생성이 표준으로 없다. const-eval은 컴파일 상수만 접고,
'인터프리터를 특정 프로그램에 특화해 해석 계층을 없앤 소스'를 만들지 못한다.

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs specialize -c '<px>'` — 잔여식 + gaps(A4-건전)
- `pnix-rs tower-check` — **1차 사영**(인터프리터 붕괴) + **2차 사영**(컴파일러 도출) 게이트

## 어디에 쓰나
DSL 컴파일, 인터프리터 최적화, 스테이징.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/06-specialization-futamura/limit_rust.rs -o /tmp/limit_06-specialization-futamura && /tmp/limit_06-specialization-futamura

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/06-specialization-futamura/pnix_rs_way.sh
```
