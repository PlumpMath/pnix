# 02-canonical-hash-and-drift — 코드의 정본(canonical) 내용주소 해시

## 쉽게 말하면 (비유)
같은 뜻의 코드를 두 가지 형식으로 써도, plain 해시는 다른 값을 준다.
pnix-rs의 IR은 **의미의 정본**을 해시하므로, 형식이 달라도 같은 뜻이면 같은 주소가 된다.

## 무엇을
px 코드를 **정본 IR(직접 평가 가능)** 로 정규화하고 그 **sha256 내용주소**를 준다.
바인딩 순서만 다른 변형은 **같은 IR 해시를 공유**(identity sharing)한다.

## plain Rust의 한계 (`limit_rust.rs`)
Rust에는 '소스의 의미에 대한 안정적 내용주소 해시'가 없다. `DefaultHasher`는 실행마다
시드가 바뀌고(재현 불가), 소스 문자열 해시는 공백/포맷/주석에 민감하다 — 같은 뜻의
두 프로그램이 다른 해시를 갖는다.

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs ir -c '<px>'` — 정본 IR + `ir_sha256` 내용주소
- 바인딩 순서 변형이 **같은 ir_sha256**을 공유함을 확인(realisation 조기 컷오프의 기반)
- `pnix-rs ir-check` — 같은 보장(sha256 FIPS self-test + 직접 평가 가능 + identity
  sharing)을 위 두 손 실험이 아니라 **전체 코퍼스**로 게이트

## 어디에 쓰나
빌드 캐시 / 증분 평가 / 재현 가능한 아티팩트 주소.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/02-canonical-hash-and-drift/limit_rust.rs -o /tmp/limit_02-canonical-hash-and-drift && /tmp/limit_02-canonical-hash-and-drift

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/02-canonical-hash-and-drift/pnix_rs_way.sh
```
