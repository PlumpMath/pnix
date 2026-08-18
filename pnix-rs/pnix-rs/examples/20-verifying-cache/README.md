# 20-verifying-cache — 감사하는 캐시(신뢰 대신 재검증)

## 쉽게 말하면 (비유)
`07-incremental`의 realisation store는 캐시 hit을 **신뢰**한다(내용해시가
같으면 저장된 값을 그대로 씀). verifying cache는 한 걸음 더 나가서, hit이
나도 값을 **다시 유도해서 store와 일치하는지 확인**한다 — 캐시를 믿지 않고
감사한다.

## 무엇을
(1) 실제 eval로 채운 store는 hit 시 재검증에 통과, (2) **오염된 store
엔트리**(값을 몰래 바꿔치기)는 재검증에서 즉시 탐지(이빨), (3) 알려지지
않은 소스는 애초에 엔트리가 없음. `pnix-hy`의 30-verifying-cache와 같은
계열이다.

## plain Rust의 한계 (`limit_rust.rs`)
`cargo`/`rustc`의 증분 빌드 캐시는 신뢰 기반이다 — 캐시 항목이 손상되거나
변조되어도 재검증 없이 그대로 재사용한다(`cargo clean`이 유일한 방어).

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs verifying-cache-check` — 캐시 hit 재검증 + 변조 탐지(이빨)

## 어디에 쓰나
신뢰할 수 없는 캐시 소스(분산 빌드, 공유 스토어)에서 캐시 무결성을
스스로 보증해야 하는 경우.

## 실행
```sh
rustc -O examples/20-verifying-cache/limit_rust.rs -o /tmp/limit_20-verifying-cache && /tmp/limit_20-verifying-cache
bash examples/20-verifying-cache/pnix_rs_way.sh
```
