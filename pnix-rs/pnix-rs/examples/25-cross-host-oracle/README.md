# 25-cross-host-oracle — 다섯 호스트가 공유하는 오라클 export의 drift 게이트

## 쉽게 말하면 (비유)
pnix-rs 혼자 옳다고 주장하는 건 의미가 약하다. 이 게이트는 pnix-rs가
내보내는 **오라클 파일**(다른 host들이 같은 기대값을 교차검증하는 데
쓰는 표)이 재생성해도 항상 같은지, witness 스키마가 얼어붙어 있는지,
corpus 기대값과 실제 export가 어긋나지 않는지를 본다.

## 무엇을
(1) `proof/oracles-rs.tsv`가 재생성 결과와 **일치**(drift 없음),
(2) witness의 **13-필드 스키마**가 렌더 순서까지 고정(다른 host가
파싱할 때 순서에 의존해도 깨지지 않음), (3) corpus에 박아둔 기대값과
실제 export 행이 어긋나지 않음.

## plain Rust의 한계 (`limit_rust.rs`)
Rust에는 "이 프로젝트가 내보내는 참조 데이터가 다른 언어/프로젝트와
스키마를 공유하며, 그 스키마가 고정되어 있고 재생성해도 안 변한다"를
게이트하는 표준 관행이 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs cross-host-check` — 오라클 export 재생성 일치 + witness
  스키마 고정 + corpus-export 일치

## 어디에 쓰나
다섯 pnix host(clj/hy/rs/cljs/clr)가 서로 다른 언어로 구현되면서도
"같은 걸 같다고 판정하는지" 비교할 수 있는 공통 참조 표.

## 실행
```sh
rustc -O examples/25-cross-host-oracle/limit_rust.rs -o /tmp/limit_25-cross-host-oracle && /tmp/limit_25-cross-host-oracle
bash examples/25-cross-host-oracle/pnix_rs_way.sh
```
