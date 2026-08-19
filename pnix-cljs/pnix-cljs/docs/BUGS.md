# pnix-cljs BUGS

목적: 알려진 버그·한계, 그리고 **의도적으로 안 고치는** 항목을 적는다.
의도적으로 안 고치는 항목은 아래처럼 "이건 버그 아니라 의도된 제한"이라고
명시해서, 나중에 누가 실수로 "고치려고" 손대지 않게 한다. (구
`SCOPE_LOCK.md`의 "이 seed에서 제외" 목록이 정확히 이런 종류의 내용이라
2026-08-20에 여기로 옮겨왔다.)

## 의도된 제한 (버그 아님)

`pnix-cljs`는 ClojureScript/JavaScript 프로젝션 메커니즘만 소유한다(제품
소유 범위 정의는 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §2 참고). 아래
항목들은 "아직 구현 안 됨"이 아니라 **애초에 이 seed의 스코프 밖으로
못박아둔 것**이다 — 이건 버그 아니라 의도된 제한:

- service policy 및 admission status
- evaluator fallback
- proof-receipt-gated execution
- JVM/Java/ASM 구현 코드
- retained effects 및 filesystem execution
- automatic application code generation
- authoritative string-encoded types
- 복사된 `stdlib`, `pnixc-pnix`, `pnix-mirror-runtime`, 또는 domain-content roots

같은 맥락에서: 이 저장소에는 이식 가능한 언어 의미를 소유하는 별도
저장소-수준 트리가 없다 — 이 호스트는 복사된 Clojure/JVM 런타임 트리를
유지하지 않고 자체 네이티브 seed로 파싱/평가한다. 네이티브 seed는 공유
적합 코퍼스가 연결되고 all-host gate로 비교되기 전까지 정규 크로스호스트
패리티를 주장할 수 없다 — 이것도 버그가 아니라 지금 단계에서 그렇게
정해둔 상태다.

## 알려진 한계 (구조적, 버그로 취급하지 말 것)

경로가 일반 값이 아님, `import`/`scopedImport`가 파서 단계에서 리터럴
경로 토큰만 받고 동적으로 계산된 경로 식은 못 받음 — 이런 구조적 차이는
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §5 "다른 호스트와 알려진
차이점"에 상세히 정리돼 있다(여기서 중복하지 않음). 언젠가 일반 Path 값
타입을 도입하려는 시도가 2026-08-19에 한 번 있었다가 절반만 고치면 더
헷갈리는 상태가 된다는 걸 확인하고 되돌린 적 있다 — 자세한 경위는 위
문서 참고.

`builtins.unsafeGetAttrPos`는 이름은 등록돼 있지만 실제 구현이 없어
호출하면 "not-callable" 에러가 난다 — 이름만 있고 죽어있는 등록이다.
다른 호스트와 통일할 방향은 [`PLANS.md`](PLANS.md)의 `unsafeGetAttrPos`
항목에 정리해뒀다.
