# 26-stage-ladder — 런타임 스테이지 사다리(5단계 일치)

## 쉽게 말하면 (비유)
"같은 프로그램을 평가한다"에는 여러 경로가 있다: 그냥 평가, 정규화 후
평가, 내용주소 스토어를 거쳐 평가, AST 왕복 후 평가, replay로 재현.
stage-check는 코퍼스의 모든 프로그램이 이 **5개 경로 전부에서 같은
값 해시**를 내는지 본다 — 하나라도 다르면 어딘가의 경로가 의미를
바꾸고 있다는 뜻이다.

## 무엇을
코퍼스 전체(문자열/리스트/attrset/재귀/builtins/seed 프로그램 등)에
대해: 직접 평가, 정규화 평가, 내용주소(CAS) 평가, AST 왕복 평가, replay
가 **모두 같은 value_fnv 해시**를 낸다. `03-mirror-roundtrip`이 "한
프로그램의 여러 단면"을 본다면, 이건 "그 단면들을 지나는 다섯 경로가
사다리처럼 쌓여도 값이 안 흔들리는가"를 코퍼스 전체로 본다.

## plain Rust의 한계 (`limit_rust.rs`)
"AST로 파싱했다가 다시 실행", "정규화했다가 실행", "직렬화-역직렬화 후
재현" 같은 서로 다른 실행 경로가 정말 같은 결과를 내는지 코퍼스 규모로
검사하는 표준 도구가 Rust에는 없다 — 최적화 레벨이 달라져도 "의미가
같은지"를 게이트하지 않는다(단지 관례적으로 같기를 기대할 뿐).

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs stage-check` — 코퍼스 전체에서 direct/normalized/CAS/AST-
  roundtrip/replay 5경로 값 해시 일치

## 어디에 쓰나
런타임 파이프라인의 여러 실행 경로(캐시, 재현, 직렬화)가 서로 의미를
바꾸지 않는다는 회귀 방지.

## 실행
```sh
rustc -O examples/26-stage-ladder/limit_rust.rs -o /tmp/limit_26-stage-ladder && /tmp/limit_26-stage-ladder
bash examples/26-stage-ladder/pnix_rs_way.sh
```
