# pnix-rs — 열린 작업

목적: 지금 당장 누가 집어 들 수 있는, 아직 안 끝난 작업만. 착수 여부조차
안 정해진 방향은 [`docs/PLANS.md`](PLANS.md), 의도적으로 안 고치는
제한은 [`docs/BUGS.md`](BUGS.md), 이미 끝난 일의 역사는
[`docs/IMPLEMENTATION.md`](IMPLEMENTATION.md) §4를 볼 것.

## 현재 상태 (2026-08-20)

**이 lane에 지금 당장 열려 있는 작업 항목이 없다.** 옛 `todo.md`(795줄)를
줄 단위로 검토한 결과, P0~P13 사다리(runtime 기판부터 tower/Futamura
사영/action/cross-host까지)는 전부 `[x]` DONE이었고, 유일한 미완료
체크박스(P13의 "자매 lane TSV 파일-대-파일 비교")도 "지금 하지 않기로
결정한, 외부 의존성 대기 중인 항목"이라 여기보다는
[`docs/BUGS.md`](BUGS.md) §4에 정리했다(의도적 보류이지 누가 지금 바로
착수할 수 있는 일이 아니라서).

나머지 미착수 방향(트레이트 solving, 3차 Futamura 사영 완주, Nix builtin
표면 완전 수렴, 경로/string-context 값 등)은 전부 proposal로 등록돼
있거나 명시적으로 "수요 발생 시" 조건이 붙은 로드맵이라
[`docs/PLANS.md`](PLANS.md)에 있다.

## 다음 작업을 찾으려면

1. `pnix-rs check` (release 빌드 후) — all_ready 여부와 각 게이트 카운트로
   현재 상태 확인.
2. `docs/PLANS.md` §1의 로드맵 순위표 — proposal이 이미 있는 항목부터
   본다.
3. `docs/BUGS.md` — 사용자/다른 호스트와의 교차 테스트 중 발견되는 실제
   버그(의도적 held가 아닌 것)는 여기부터 채워나가는 게 이 프로젝트의
   실제 패턴이었다(§4 역사의 2026-08-19 참고 — cross-host 교차 배터리
   테스트가 "이름은 있는데 동작이 다른" 버그를 제일 잘 찾아냈다).
4. 새 기능을 만들기 전엔 `docs/IMPLEMENTATION.md` §5(게이트 레지스트리)를
   먼저 grep — 이미 구현됐는지부터 확인.

이 절이 다시 채워지기 시작하면(즉 실제로 진행 중인 작업이 생기면),
pnix-hy/pnix-clj의 TODO.md처럼 날짜·수용 기준·증거(게이트 카운트)를 갖춘
슬라이스 목록으로 기록할 것 — 옛 `todo.md`가 썼던 형식 그대로 재사용하면
된다(2026-08-20에 삭제됐지만 git 이력에는 남아있다).
