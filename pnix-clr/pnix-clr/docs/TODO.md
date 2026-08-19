# pnix-clr TODO

지금 당장 누가 집어서 끝낼 수 있는, 방향이 이미 정해진 개별 작업만 여기
적는다. 이미 끝난 일이나 방향이 아직 안 정해진 아이디어는 여기 넣지 않는다
— 지난 작업 이력은 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §4(역사),
확정 안 된 미래 방향은 [`PLANS.md`](PLANS.md), 의도적으로 안 하는
것/알려진 제한은 [`BUGS.md`](BUGS.md) 참고.

**2026-08-20 현재: 확정된 개별 작업 항목 없음.**

문서를 4개로 정리하면서(`SCOPE_LOCK.md`/`todo.md`/
`CLOJURE_CLR_ADMITTED_SURFACE.md`/`IN_PROCESS_EVAL.md`/`TFM_POLICY.md`
통합) 옛 `todo.md`에 있던 두 항목(pnixMounts/unsafeGetAttrPos 통일,
CAPABILITIES.md 자동 생성기 부재)을 다시 살펴봤는데, 둘 다 "기본 언어
기능이 production 수준으로 갖춰지기 전까지는 방향 확정 안 함" 또는
"패턴 참고용 구현은 있지만 아직 착수 안 한 방향 선택 문제"라서 실제로는
`PLANS.md`행이었다(§1, §2). in-process 평가기 스파이크의 남은 항목들도
"언제 승격할지 미정"이라 마찬가지로 `PLANS.md`(§3)로 갔다.

새로 이 파일에 넣을 만한, 방향이 이미 정해진 작업이 생기면 여기 추가할
것.
