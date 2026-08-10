# 0002 — px 값 attrset의 정렬 표현 + 이진 탐색 조회

상태: 제안 → 구현(2026-07-03). sacred runtime(src/px.rs) 변경이므로 절차 준수.

## 동기 (실측)
tower m6c/m6d에서 2차 Futamura 사영의 알고리즘 벽(지수 unfold, per-apply
재계산)은 전부 해소했으나 실행이 1h40m+ 미종결(4R). 최종 진단: px 값
attrset 연산이 **선형 스캔 Vec** — polyvariant specializer의 env(~40 엔트리)
에 대한 hasAttr/getAttr가 guest 스텝마다 수십 회 × 수백만 스텝 = 10^9급
문자열 비교. (pnix-hy가 같은 사다리를 완주한 것은 CPython dict의 O(1) 덕.)

## 변경
- **표현 불변식**: `PxVal::Attrs`의 필드는 이름 기준 **정렬 상태**(px_str_lt,
  중복 없음)로 유지. 유일한 값 생성자 `px_attrs`(m3a 캐노니컬)가 삽입 정렬로
  불변식을 수립 — 모든 소비자는 자동 획득.
- 조회(select/getAttr/hasAttr/px_attrs_has): 이진 탐색 O(log n).
- `//` 병합: 정렬 병합 O(n+m) (기존 O(n·m)).
- `==`: 정렬 zip 비교 O(n) (기존 O(n²)).
- attrNames: 불변식 재사용(정렬 생략 가능하나 방어적 정렬 유지).

## 안전 근거 (관측 불가능성)
값 필드 순서를 관측하는 표면 없음: px_print/toJSON/attrNames는 출력 시
정렬(불변식과 일치), 나머지는 이름 기반 접근. 평가 순서 의미(literal 필드의
좌→우 평가, 에러 타이밍)는 평가 후 정렬이므로 보존. AST(PxExpr::Attrs)는
소스 순서 유지 — emit byte-왕복 불변. 중복 키는 표현 진입 전에 이미 제거됨
(파서 거부·listToAttrs first-wins·`//` 병합).

## 게이트
px-check/mirror/stage/ir/oracle canonical 불변(정렬 출력은 원래부터),
substrate-check(subset 준수 — 인덱스 루프+문자열 비교만 사용), tower 패리티.
