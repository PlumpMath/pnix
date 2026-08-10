# 2026-07-03 — tower 시대 표면 adversarial 감사 (#2)

대상: 1차 감사(2026-07-02, 사다리 닫힘) 이후 추가된 표면 — tower m2~m6d,
P6 v1a/v2, floats(c01), toJSON(c06), 동적 키(c10), 깊은 `==`, 정렬 attrs
(proposal 0002). 방법: 새 표면 조합의 edge 프로브 + 게이트 불변 확인.

## 발견 및 조치 (3건 수정, 1건 경계 기록)

**F1 (HIGH, 실버그)**: float 리터럴이 **함수 인자로 파스 불가** —
`builtins.toJSON 1.5` → "trailing tokens". 원인: juxtaposition apply의
인자-시작 판정에 `PxTok::Float` 누락(c01은 float을 인자 위치에 안 써서
게이트가 침묵). **수정**: 판정에 Float 추가. **가드**: px-check 인라인
`(x: x * 2.0) 3.5 == 7.0`.

**F2 (MED)**: `builtins.toJSON`에 Float arm 부재 — 유한 float도 에러.
**수정**: 유한 → `{:?}`(pnix-hy repr 동형), 비유한 → 명시 에러(JSON 무효).
**가드**: px-check 인라인 toJSON float leaf.

**F3 (LOW)**: rust-mirror 값 축이 비유한 float leaf에서 컴파일 불가 Rust
리터럴 생성 가능 → **held로 수정**(유한만 사영).

**경계 기록 (수정 아님)**: `1.0 / 0.0` → `inf` — 비유한 float 값의 canonical
print("inf")는 유효 px 소스가 아니어서 **P1 "print는 곧 소스" 성질이 비유한
float에 한해 깨짐**. pnix-hy도 repr(inf)='inf'로 동일한 성질 edge(자매 lane
공통). SCOPE_LOCK에 기재; 수요 발생 시 pnix-hy emit_float_source(inf →
1.0e309) 방식 검토.

## 확인된 무결(발견 없음)
- 클로저 포함 attrs 깊은 `==` → 에러 아닌 false(Nix 정합).
- 정렬 attrs 불변식: 관측 표면(print/attrNames/toJSON 정렬 출력) 불변,
  first-wins 누적기 버그는 구현 중 px-check가 즉시 검출(선형 검사로 수정).
- m6d frames 레지스트리: 미등록 gid 도달 경로 없음(recclo는 등록 let에서만).

## 재검증
px-check 19/19(가드 +2), substrate-check 3-way PASS, check aggregate
15 reports all_ready.
