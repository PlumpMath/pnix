# 07-incremental — 증분 평가 — 이름이 아니라 '의미'로 캐시

## 쉽게 말하면 (비유)
이름 기반 캐시는 변수 하나만 이름을 바꿔도(alpha-rename) 전부 무효화된다.
pnix-rs는 **의존성-치환 내용해시**(Unison식)로 정의별로 캐시해, 이름 변경에 면역이다.

## 무엇을
top-level 정의별 **의존성-치환 content hash**(형제 참조를 그 해시로 치환) + SCC 그룹 +
realisation store **조기 컷오프**(ir 해시 히트면 평가 생략).

## plain Rust의 한계 (`limit_rust.rs`)
Rust의 빌드/캐시는 파일·심볼 이름 단위라 alpha-rename/포맷 변경에 취약하고,
'정의별 의미 해시로 부분 재사용 + 알파 불변'을 표준으로 제공하지 않는다.

## pnix-rs의 방식 (`pnix_rs_way.sh`) — Rust ↔ pnix
- `pnix-rs incremental -c '<px>'` — 정의별 해시(형제 이름 무관) + SCC
- `pnix-rs incremental-check` — **알파 불변** + realisation 조기 컷오프

## 어디에 쓰나
증분 빌드, 콘텐츠 주소 캐시, 대규모 코드베이스 재평가.

## 실행
```sh
# plain Rust의 한계 (직접 컴파일·실행)
rustc -O examples/07-incremental/limit_rust.rs -o /tmp/limit_07-incremental && /tmp/limit_07-incremental

# pnix-rs 방식 (flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
#              아니면 PNIX_RS=/path/to/pnix-rs 로 지정)
bash examples/07-incremental/pnix_rs_way.sh
```
