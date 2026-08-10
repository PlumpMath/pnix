# 0003 — px 재귀 let의 call-by-name → call-by-need (thunk-memo)

상태: **구현 완료 + 실증(2026-07-03)** — 2차 사영이 6라운드 미종결(20분~1h40m)에서 **~0.1초 완주**로. 전 게이트 green(substrate 3-way 포함).
held 표면 "thunk-memo laziness"의 수요 도달로 개방.

## 동기 (m6f 계측 실측)
fuel 계측: 2차 사영 poly 실행이 fuel=1에서 14ms, fuel=100에서 33.9s —
게스트 스텝당 ~340ms. 원인: px 재귀 let이 **call-by-name**(Rec 프레임
바인딩을 참조할 때마다 재평가). 상태-스레딩 스타일
`let l = f x st; r = g y l.st; in ... l.n ... r.n ...`에서 `l` 참조 4~5회
→ f가 4~5회 재실행 → 트리 전체에서 지수 재평가. 1R~6R의 모든 미종결의
최상위 원인(memo/lid/gid/정렬은 각자 실재했으나 이것에 가려짐).
pnix-hy가 같은 사다리를 완주한 근본 이유 = CPython 위 lazy **call-by-need**.

## 변경
`PxFrame::Rec`에 바인딩별 memo 슬롯(`Rc<RefCell<Vec<Option<PxVal>>>>`)을
추가하고 `px_lookup`이 첫 평가 결과를 기록·재사용.

## 관측 동등 논증
px는 순수(부작용 없음)·결정적. 같은 프레임의 같은 바인딩은 같은 prefix
env에서 평가되므로(프레임은 let 진입마다 새 Rc로 생성되고, env 복제는
prefix를 통째로 복제 — Rc-공유 프레임의 prefix는 생성 시점에 고정) 재평가
결과는 항상 동일. 따라서 name↔need는 값/에러/발산 관측에서 동등하고
비용만 다르다. 전 게이트(corpus/mirror/stage/ir/tower/substrate 3-way)가
동등성의 실증.

## subset 근거
`Rc<RefCell<Vec<T>>>`는 rs-meta interp 자신의 Val 패턴(자기호스트 증명).
