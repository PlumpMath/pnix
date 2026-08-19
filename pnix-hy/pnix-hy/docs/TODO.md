# pnix-hy TODO — 진행 중인 작업만

> 과거 완료 이력: `docs/archive/todo-history.md`(~2026-07-01까지) +
> `docs/IMPLEMENTATION.md` §8(그 이후 Phase A/B/C 등, 2026-07-02~).
> 새 기능은 `docs/proposals/NNNN-*.md`로 시작(요약 인덱스: `docs/PLANS.md`)
> — 이 파일에 `[ ]`로 바로 추가하지 않는다. 의도적으로 고치지 않는
> placeholder/한계는 `docs/BUGS.md`. 범위 선언·wording 규칙은
> `docs/IMPLEMENTATION.md` §4 — "complete w.r.t. the stated scope"라고만
> 말한다("전체 완성"이라고 하지 말 것).
>
> 능력 인덱스(중복개발 방지 조회): `pnix-hy-project --capabilities`
> (= 생성물 `docs/CAPABILITIES.md`).

---

## ⚙️ 작업 규칙 (공통)

1. **환경**: 작업 디렉터리 = `pnix-hy/`(패키지 루트). 검증 파이썬 = Hy 1.3.0이 있는 proof python
   (예: `/tmp/pnix-hy-py311-venv/bin/python`; nix면 `nix build .#proofPython` 산출의 `bin/python`).
   실행 형식: `PYTHONPATH=. PNIX_HY_PYTHON=<proof python> <proof python> -m pnix_hy.cli --check`.
2. **금지**: `pnix_hy/pnix_runtime.py` 수정 금지(sacred). 두 번째 evaluator/mirror/gate 생성 금지.
   hy-meta 복제 금지. 의도적 placeholder(`docs/BUGS.md` §1/§2/§3)를 "고치지" 말 것. 공유 witness
   스키마/opaque-ref shape(`docs/IMPLEMENTATION.md` §4.6) 무단 변경 금지.
3. **버그 수정 절차**: (1) repro를 먼저 그대로 재현해 실패 확인 → (2) 수정 → (3) 같은 repro가
   통과함을 확인 → (4) 가능하면 그 repro를 해당 모듈의 `*_report()` 케이스로 추가(회귀 고정) →
   (5) `--check` all_ready 확인.
4. **최종 검증(각 작업 끝)**: `--check` all_ready → `--gate` PASS →
   `--capabilities > docs/CAPABILITIES.md` 재생성 후 diff 확인(docs_drift 게이트).
5. **커밋/push 관례**: 파일/주제 묶음별 1커밋. push 후 `git branch -f main HEAD` +
   `git push origin HEAD:main`(main FF 유지).

---

## ▶ 열린 작업

### Host 환경 / 패키징 (dot-nix, 2026-08-13~)

- [ ] **단일 인터프리터 스토리 정리**: proofPython(Hy pin) vs kimchi
  python-with-packages가 여전히 두 갈래다 — 순수 pnix 평가용 "the" host와
  과학 스택(numpy 등)용 host를 어느 쪽으로 문서화할지 정리 필요. 정본:
  `docs/IMPLEMENTATION.md` §7.1 / monorepo `HOST_DEV_ENV.md`.
- [ ] **(optional) `packages.pnix-hy-host`를 flake에서 직접 노출**: 지금은
  각 home-manager 트리가 매번 `symlinkJoin`을 재구현해야 한다 — flake
  output으로 한 번만 만들어두면 소비자가 재구현할 필요가 없어진다.

---

## 참고 — 최근 종료된 단계 (상세는 다른 문서)

Phase A(딥리서치 감사가 찾은 버그 26건 수정) / Phase B(proposals
0014-0019) / Phase C(proposals 0020-0029) 및 specializer/BTA 연구 백로그
대부분은 전부 SHIPPED 또는 결정 완료(CLOSED/WON'T-DO)됐다 —

- 무엇이 언제 SHIPPED됐는지: `docs/IMPLEMENTATION.md` §8.2.
- "안 하기로 결정"한 항목(stage-polymorphic maybe-lift 등, 다시 열지 말 것): `docs/BUGS.md` §3.
- 아직 착수 안 한 연구 후보(Q1-3 CPS specializer, R5 scheduler×rebuilder 분류,
  0028 P2 optimal cogen): `docs/PLANS.md` §1.2.
- proposal 0001-0030 전체 상태 인덱스: `docs/PLANS.md` §2.
