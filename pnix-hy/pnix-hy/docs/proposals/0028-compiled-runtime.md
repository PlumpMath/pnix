# 0028 — pnix compiled runtime (cogen 실행 성능벽의 진짜 해결 경로)

- 상태: **P1 SHIPPED 2026-07-02** ("해결해봐~"); P2는 아래 finding으로 재정의. 대형 신규 능력이라
  SCOPE_LOCK §7대로 proposal로 착수, 단계화.
- Scope: **추가 모듈** `pnix_hy/compiled/` (또는 `pnix_hy/nrun.py`). `pnix_runtime.py`/4-lane **SACRED
  무접촉**, `stage7` 무접촉. 기존 parser/AST/IR/tower 재사용.
- 동기: 0026 M6에서 cogen 아티팩트가 **specializer로 정확히 실행**됨을 검증했으나, cogen을 큰 객체
  (77KB 인터프리터 AST)에 실행해 **풀 컴파일러를 재도출**하는 것은 host tree-walker 성능으로 비현실적.

## 왜 필요한가 — 실측 증거 (두 손쉬운 해결책이 불충분함을 확인)
0026 M6 이후 두 가지를 실제로 시도(2026-07-02):
1. **naive compiled runtime** (pnix core→Python thunk 평가기 프로브): cogen→풀 컴파일러 **>6분**.
   원인: per-apply `{**env}` 전체 복사(O(env)/call), `==` 비교의 재-force.
2. **알고리즘 O(1) memo** (POLY의 `sig`를 문자열 키 + spec memo를 attrset 해시맵으로): **직접 2차사영
   1.9s→0.4s(~5배)**, spec-point 17→2 dedup 성공. 그러나 **cogen 실행은 여전히 >300s** — memo는
   복잡도를 줄였지만 tree-walker의 ~50배 해석 오버헤드와 pnix `//`의 O(n) 복사 삽입은 그대로.
   (이 memo 변경은 ladder report를 RecursionError로 불안정화시켜 되돌림 — 커밋 안 함.)

결론: cogen→풀 컴파일러는 **정확성이 아니라 근본 실행 성능**이며, 제대로 된 **컴파일 실행 런타임**이
있어야 한다. tower.py 한 파일 확장으로는 불가.

## 설계 (핵심 = tree-walker의 ~50배 상수 + 순수-언어 자료구조 오버헤드 제거)
1. **환경**: per-call 전체 복사 금지 — **linked frame 체인**(O(1) 확장, O(depth) 조회) 또는 persistent map.
2. **컴파일 타깃**: pnix core-subset(int/bool/string/null/var/binary/if/let(재귀-지연)/lambda(커리드)/
   apply/select/attrset/list/builtins) → **Python 소스/코드객체**로 lowering(`compile()`), 지연은
   memoized thunk, builtins는 고정 dispatch. `pnix_runtime`은 안 건드리고 별도 백엔드.
3. **자료구조**: 특화기의 spec-memo/attrset 누적을 O(1) 삽입 자료구조로(persistent dict 또는 mutable
   with 명시적 스코프) — pnix `//` O(n) 복사 회피.
4. **정합성 게이트**: 컴파일 런타임의 결과가 기존 `pnix_runtime.eval_source`와 **코퍼스 전수 동등**
   (신규 `compiled_runtime_report`, `--check` +1). SACRED 4-lane은 정본으로 유지, 컴파일 런타임은
   그것과 대조되는 추가 lane.

## 수용 기준 (구현 시)
- core-subset 코퍼스에서 `compiled_eval(src) == rt.stable_data(rt.eval_source(src))` 전수 동등.
- `run_cogen(mix ⊕ INT)` = **풀 컴파일러**가 실용 예산(예: <30s) 내 완주 → 3차 사영 end-to-end 성립
  (0026 M6의 성능 프론티어 해소), T2h 하니스로 수용.
- `pnix_runtime.py`/stage7/4-lane 무변경, `--check`/`--gate` 회귀 0.

## Phase
- **P1 — SHIPPED**: `pnix_hy/compiled.py` — core-subset AST를 **Python 클로저로 컴파일**(linked-env
  프레임, memoized thunk, native 연산/`==`, 고정 builtins). tree-walker 우회. `compiled_runtime_report`
  = 코퍼스 27/27이 `pnix_runtime.eval_source`와 동등(`--check` +1). 대조 lane(정본은 sacred pnix_runtime).
- **P2 — 재정의(연구급, 미해결)**: **cogen→풀 컴파일러 <30s는 compiled runtime으로도 달성 안 됨.**
  세 번째 실험(이 컴파일 클로저 런타임)에서도 >6분. **근본 원인 확정: naive cogen 자체가 알고리즘적으로
  비효율**(self-application이 비대한 생성확장을 만듦 — 직접 specializer 0.4~1.9s 대비 cogen은 ~100-1000배).
  런타임 상수배가 아니라 **효율적/최적(optimal) cogen**이 필요 — Jones-optimal specialization 등 PE 연구
  주제. compiled runtime은 그와 독립적으로 유효(빠른 core eval).
- **P3 — SHIPPED**: 실사용 fast-path. `subset_supported(src)`(core-subset 판별) + `evaluate(src)`
  (subset이면 compiled, 아니면 canonical fallback — 항상 정본 값) + CLI `--ceval SRC` + `evaluate`
  리포트(`--check` +1). 사용자가 안전하게(총체적) 속도 이득을 얻는다. (최적 cogen 결실 시 cogen 잔여도
  `compiled_eval`로 실행 가능 — 그건 P2 연구 대기.)

## 결정적 finding (네 실험 — 확정)
1. tree-walker: cogen→풀 컴파일러 >300s. 2. naive Python-thunk 런타임(cc.py): >6분.
3. Python-클로저 compiled 런타임(P1): >6분.
4. **스케일 스윕(compiled runtime × 축소 인터프리터)**: 4-branch뿐 아니라 **num-only 1-branch
   초소형 객체조차 >150s**. → 벽은 규모도 런타임도 아니라 **naive cogen 아티팩트 자체가 근본적으로
   비대**(self-application이 병리적 생성확장을 만들어, 어떤 입력에도 실행 자체가 intractable).
결론: 이는 **효율적/최적 cogen을 요구하는 진정한 PE 연구 문제**이며, 런타임/스케일 엔지니어링으로는
해결되지 않음이 4실험으로 입증됨. 더 이상의 런타임 시도는 무의미(중단).
실용 컴파일러 생성은 **직접 specializer(0026 M5b, poly_mix_in_pnix, 0.4~1.9s)로 이미 제공됨** —
cogen(3차)은 "생성·실행(specializer로 동작)"까지 검증(0026 M6)됐고, "cogen으로 풀 컴파일러 재도출"만
optimal-cogen 연구를 기다린다.

## Forbidden (지킴)
- `pnix_runtime.py`/stage7/4-lane 수정. 두 번째 "정본" evaluator 주장(컴파일 런타임은 **대조 lane**,
  정본은 여전히 sacred pnix_runtime). 공유 ABI/witness envelope 변경.
