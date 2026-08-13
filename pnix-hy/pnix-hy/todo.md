# pnix-hy todo

> ⛔ **SCOPE LOCK** (see `/SCOPE_LOCK.md`): 의도적 placeholder를 미구현으로 재해석해 구현하지 말 것.
> 새 기능은 `docs/proposals/NNNN-*.md`로 시작. "complete w.r.t. the stated scope"라고만 말한다.
> 능력 인덱스(중복개발 방지 조회): `pnix-hy-project --capabilities` (= 생성물 `docs/CAPABILITIES.md`).
> 과거 이력: `docs/archive/todo-history.md`. 이 파일은 **활성 작업만** 담는다.

---

## ⚙️ 작업 규칙 (코덱스 필독 — 모든 항목 공통)

1. **환경**: 작업 디렉터리 = `pnix-hy/`(패키지 루트). 검증 파이썬 = Hy 1.3.0이 있는 proof python
   (예: `/tmp/pnix-hy-py311-venv/bin/python`; nix면 `nix build .#proofPython` 산출의 `bin/python`).
   실행 형식: `PYTHONPATH=. PNIX_HY_PYTHON=<proof python> <proof python> -m pnix_hy.cli --check`.
2. **금지**: `pnix_hy/pnix_runtime.py` 수정 금지(sacred — 이번 작업에 이 파일 버그 없음).
   두 번째 evaluator/mirror/gate 생성 금지. hy-meta 복제 금지. 의도적 placeholder(§SCOPE_LOCK.md §3)
   를 "고치지" 말 것. 공유 witness 스키마/opaque-ref shape(§6) 무단 변경 금지.
3. **버그별 절차(A 단계)**: (1) 원장(`docs/audits/2026-07-02-deep-research-audit.md`)의 Repro를
   **먼저 그대로 재현**해 실패 확인 → (2) 수정 → (3) 같은 Repro가 통과함을 확인 → (4) 가능하면
   그 Repro를 해당 모듈의 `*_report()` 케이스로 추가(회귀 고정) → (5) `--check` all_ready 확인.
4. **커밋 단위**: 파일/주제 묶음별 1커밋(아래 A-그룹이 커밋 단위). 메시지에 원장 항목 번호(A4 등)
   명시. push 후 `git branch -f main HEAD` + `git push origin HEAD:main`(main FF 유지).
5. **최종 검증(각 단계 끝)**: `--check` all_ready → `--gate` PASS(1111/1260/545×4/closure) →
   `--capabilities > docs/CAPABILITIES.md` 재생성 후 diff 확인(docs_drift 게이트).
6. **리포트 카운트**: A 단계는 57 유지(케이스 추가는 기존 리포트 내부). B 단계에서
   0014(+1), 0017(+1), 0018(+1), 0019(+1) → 최종 **61**; 0015/0016은 기존 리포트 확장(카운트 불변).

---

## ▶ PHASE A — 확정 버그 26건 수정 (우선순위 1; proposal 불요)

전체 상세·Repro는 `docs/audits/2026-07-02-deep-research-audit.md` §A(A1–A26)에 있음. 아래는 수정
지침. **A-1 그룹(HIGH)부터 순서대로.**

### A-1그룹: specialize_pnix 의미 오류 — `pnix_hy/pnix_mirror.py` `_pe` (A4, A5, A15) [커밋 1]

- [x] **A4 (high) `_pe` let이 재귀 아닌 순차 처리** (`pnix_mirror.py:~2414`)
  - 원인: pnix `let`은 재귀 스코프(형제 바인딩이 서로 참조, 뒤 바인딩이 앞을 shadow)인데 `_pe`는
    텍스트 순서로 env를 누적 → 뒤에서 바인딩될 이름이 **바깥 static env로 잘못 해석**됨.
  - 수정: 2-패스로. ① 이 let의 바인딩 이름 집합 N을 먼저 수집. ② let 본문/바인딩 안에서 N에 속한
    이름은 **바깥 env로 절대 해석 금지**(형제 결과로만). ③ 고정점 반복: "참조하는 N-이름이 전부
    이미 fold됨"인 바인딩만 fold, 더 못 줄이면 중단. ④ 하나라도 fold 못 하면 **let 전체를
    gap(`let-recursive-not-static`)으로 기록하고 dynamic residual로**(부분 fold로 Hy `let`(순차)을
    방출하지 말 것 — 그 자체가 의미 불일치). 건전성 우선, 특화력 손실 허용.
  - 시험: `specialize_pnix('let x = 5; in let y = x + 1; x = 10; in y')` → fully_static **11**
    (eval과 동일). `specialize_pnix('let b = a + 1; a = 2; in b')` → **3**. 자유변수 남는 residual
    이 나오면 실패. 동적 형제 케이스(`let b = a + d; a = 2; in b`, d는 dynamic var) → gaps 비어있지
    않음. 마지막에 `specialize_report()` 통과 + 위 3케이스를 report에 추가.
- [x] **A5 (high) `_pe` attrset 다중경로에서 break → 바인딩 통째 드랍** (`:~2437`)
  - 수정(권장): `a.b = 1`을 중첩 dict로 실제 구현 — path 세그먼트를 걸어가며 병합(충돌 시 gap).
    간단히 가려면: 다중경로 발견 시 **attrset 전체를 gap+dynamic residual**로(부분 결과 `{}` 절대
    금지). break로 이후 바인딩을 버리는 현재 동작은 어느 쪽으로든 제거.
  - 시험: `specialize_pnix('{ a.b = 1; c = 2; }')` → fully_static이면 값이
    `{'a': {'b': 1}, 'c': 2}`, 아니면 gaps 비어있지 않음. **잔여 `{}`+gaps 없음 조합이 나오면 실패.**
- [x] **A15 (med) `_pe` if가 Python truthiness로 프루닝** (`:~2400`)
  - 원인: pnix는 if 조건이 bool 아니면 오류인데 `_pe`는 `if cond[1]`로 1/문자열도 통과.
  - 수정: 조건이 static이면 `isinstance(value, bool)` 검사 — bool 아니면 gap(`if-non-bool-cond`)
    + dynamic residual(런타임이 오류를 내도록). bool이면 기존 프루닝 유지.
  - 시험: `specialize_pnix('if 1 then 2 else 3')`이 **fully_static value 2를 반환하지 않음**
    (gaps 기록). `rt.eval_source` 오류와 모순되는 성공 판정 금지. 정상 bool 케이스 회귀 없음.

### A-2그룹: 투영 오류 — `pnix_hy/pnix_mirror.py` (A14) [커밋 2]

- [x] **A14 (med) select 투영이 `(. base attr)` 방출 — dict라 항상 AttributeError** (`:~1433`)
  - 수정: `(get base "attr")` 형태로 방출(Hy에서 dict 접근 정상 동작). 다운스트림
    (`compiler_emit_shape`, `projection_value_roundtrip`, `synthesize_pnix_from_hy` 역방향)이 새
    형태를 소화하는지 확인하고, 불가한 지점이 있으면 그 지점에서 gap 기록(침묵 clean 금지).
  - 시험: `pnix_to_hy_form('{ a = 1; }.a')`의 hy_source가 stage7/hy로 평가돼 **1**;
    `projection_value_roundtrip('{ a = 1; }.a')` meaning_preserved; `classify_drift` 판정이 실제
    평가 가능성과 일치. 관련 리포트(`pnix_to_hy_form`, `projection_value_roundtrip`,
    `interop_hy_macro_bridge`) 전부 통과.

### A-3그룹: stage7/투영 worker — `pnix_hy/hy_mirror.py` (A2, A10, A23) [커밋 3]

- [x] **A2 (high) worker가 stdout 출력에 desync — 이후 호출이 이전 값 반환** (`:~313`)
  - 수정: `_STAGE7_WORKER_SCRIPT`에서 평가를 `contextlib.redirect_stdout(io.StringIO())`로 감싸
    (투영 worker와 동일 패턴) 응답 JSON만 실제 stdout에 쓰기. 캡처된 출력은 응답 필드
    `stdout`으로 동봉(버리지 말 것). 추가 방어: `_stage7_worker_eval`에서 `json.loads` 실패 시
    `HyMirrorError`로 변환 **+ worker kill**(재동기화 — 오염된 파이프 재사용 금지).
  - 시험: `stage7_eval('(do (print "HELLO") 43)')` → `'43'`; 직후 `stage7_eval('(+ 1 2)')` → `'3'`;
    직후 `stage7_eval('(+ 10 10)')` → `'20'`. one-shot(`PNIX_HY_NO_WORKER=1`) 경로와 값 일치.
- [x] **A10 (med) worker readline에 timeout 없음 — 무한 대기** (`:309, :469`)
  - 수정: `select.select([proc.stdout], [], [], remaining)` 루프(또는 reader thread+join)로
    deadline 부여. 대상: `_stage7_worker_eval`/`_proj_worker_run`의 readline + 두
    `_ensure_worker`의 drain 루프(줄이 안 와도 deadline 체크). 타임아웃 상수는 env
    `PNIX_HY_WORKER_TIMEOUT`(기본 120)로 — 테스트에서 5초로 줄여 검증 가능하게.
  - 시험: `PNIX_HY_WORKER_TIMEOUT=5`로 `stage7_eval('(while True 1)')` → ~5초 내
    `HyMirrorError`(무한 대기 금지) + worker가 정리됨(다음 호출 정상).
- [x] **A23 (low) eval 오류가 healthy worker를 죽여 27초 재빌드 유발** (`:~337`)
  - 수정: 예외 분리 — worker가 **정상 응답으로 ok:False**를 보낸 경우(평가 오류)는 worker를 살려둔
    채 즉시 raise(예: `Stage7EvalError(HyMirrorError)` 서브클래스); 응답 자체가 없거나 깨진 경우
    (인프라 오류)만 kill + one-shot 폴백.
  - 시험: warm worker에서 `stage7_eval('(undefined-symbol)')`가 **~수초 내** 오류(27초 one-shot
    폴백 아님), 직후 `stage7_eval('(+ 1 2)')`가 **즉시**(재빌드 없음) `'3'`.

### A-4그룹: interop 경계 — `pnix_hy/interop.py` (A3, A11, A12, A13, A24, A25) [커밋 4]

- [x] **A3 (high) `_required_positional_count`가 `*args` 보면 필수 인자 폐기** (`:~568`)
  - 수정: VAR_POSITIONAL/VAR_KEYWORD 파라미터는 **건너뛰고**(카운트 중단 금지) POSITIONAL_ONLY/
    POSITIONAL_OR_KEYWORD 중 default 없는 것을 계속 센다. `lambda a, b, *rest` → 2.
    기존 nullary(0) 특례와 signature 실패 폴백(unary)은 유지.
  - 시험: 원장 repro — `f = host_callable_to_pnix(lambda a, b, *rest: a + b)`; pnix `f 1 2` → **3**.
    `max`/`os.path.join` 게이트 하에 동작. `host_callable_arity` 리포트에 케이스 추가.
- [x] **A11 (med) 함수 sentinel과 같은 값의 진짜 문자열을 opaque로 오분류** (`:~388`)
  - 수정: sentinel 분기는 `_is_pnix_function(raw)`가 True일 때만. 실제 문자열이면 문자열로 통과.
  - 시험: `to_host(rt.eval_source('"#<pnix-hy-closure>"'))` → 문자열 그대로 + loss `lossless`.
- [x] **A12 (med) 중첩 PnixPath 손실을 lossless로 허위 표기** (`:~402`)
  - 수정: realize 전 raw 구조를 재귀(깊이 제한) 스캔하는 `_contains_path_like(raw)` 추가 —
    발견 시 `loss_status='lossy'`, `loss_reason='nested-path-context'`. 값 변환은 불변(마킹만).
  - 시험: `to_host(eval_source_raw('{ p = ./foo; }', realize=False))` → loss **lossy**
    (top-level `./foo`와 일관). `roundtrip_report` 케이스 추가.
- [x] **A13 (med) `wrap_pnix_callable`이 force 시점 PnixError를 그대로 누출(D1 위반)** (`:~720`)
  - 수정: `rt.force_value(closure_raw)`를 D1 try/except 안으로 이동 → `InteropError`로 변환
    (`_interop_error` 재사용).
  - 시험: `pnix_callable('throw "boom"')` → **InteropError**(raw PnixError 금지).
    `interop_error_contract` 리포트에 케이스 추가.
- [x] **A24 (low) 혼합 타입 set의 from_host 순서가 해시시드 의존(비결정)** (`:~431`)
  - 수정: 정렬 불가 set은 `sorted(value, key=lambda x: (type(x).__name__, repr(x)))`류의
    **결정적 키**로 정렬해 리스트화(현재 lossy 마킹은 유지).
  - 시험: `PYTHONHASHSEED=1`과 `=7` 서브프로세스에서 `from_host({1,'a','b','c'})[0]` 동일.
- [x] **A25 (low) 예약키 `__pnix_opaque__` 든 평범한 dict가 자기모순 roundtrip 리포트** (`:~509`)
  - 수정: ① `from_host`가 예약키(`__pnix_opaque__`/`__hy_meta_opaque__`) 포함 dict를 데이터로
    변환할 때 `loss='lossy'`(`reserved-key`) 표기. ② `roundtrip_host_value`의 by-ref 판정은
    `is_opaque_ref(pv)`가 아니라 **from_host record가 실제 opaque를 만들었는지**로.
  - 시험: `roundtrip_host_value({'__pnix_opaque__': 5})` → `by-value`/lossy, `equal` 판정과 모순 없음.

### A-5그룹: action 층 — `pnix_hy/action.py` (A7, A19) [커밋 5]

- [x] **A7 (med) check_action이 매 호출 ~30초 Hy 서브프로세스(roundtrip_status)** (`:~170`)
  - 수정: `check_action(..., include_roundtrip=False)` 파라미터 추가, **기본 skip**
    (`record['roundtrip'] = None`). verdict 스키마(0009)의 필수 필드에 roundtrip 없음 — 스키마
    불변. `include_roundtrip=True`일 때만 기존 동작. docstring의 '얇음' 주장과 일치화.
  - 시험: Hy 없는 plain python에서 `check_action('let a=1; in a+2')`가 **2초 이내** accepted;
    `action_report()`도 수 초 내 ready(현재 수 분). `--action-check` CLI 즉답.
- [x] **A19 (low) `granted='file-read'`(str)가 9개 한 글자 권한으로 분해** (`:~131`)
  - 수정: `check_action`/`verify_action`/`gate.gate_check` 입구에서
    `if isinstance(granted, str): granted = (granted,)`.
  - 시험: `check_action('builtins.pathExists "/etc/passwd"', granted='file-read')` → **accepted**
    (튜플형과 동일). `action_report`에 케이스 추가.

### A-6그룹: 배포/발견 — `pnix_hy/deploy.py`, `hy_mirror.py`, `pyproject.toml` (A1, A17) [커밋 6]

- [x] **A1 (high) deployment_info가 hy 없는 python도 projection ready로 보고** (`deploy.py:~26`)
  - 수정: 존재 검사에 더해 **실제 probe**: `subprocess.run([py, '-c', 'import hy'],
    cwd=str(hm.HY_ROOT), timeout=15)` — cwd=HY_ROOT라 벤더드 hy도 인정(레포 안 시나리오),
    설치본 hy도 인정. returncode==0일 때만 `proof_python_found=True`. 결과는 모듈 수준 캐시.
  - 시험: `PNIX_HY_PYTHON=/usr/bin/python3`(hy 없음) → `projection=False` + hint 표시;
    proof python → True. 원장 repro 역전 확인.
- [x] **A17 (med) pip extras로 hy 설치해도 자기 인터프리터를 후보로 안 봄** (`hy_mirror.py:~55`)
  - 수정: `_candidate_pythons()`에 `sys.executable`을 **PNIX_HY_PYTHON 다음, 하드코딩 경로 앞**
    순위로 추가(단 `importlib.util.find_spec('hy')`이 그 인터프리터에서 성공할 때만 — 자기
    자신이므로 in-process 검사 가능). deploy hint 문구에 'PNIX_HY_PYTHON 설정 또는 현재 python에
    hy 설치' 명시.
  - 시험: hy가 설치된 인터프리터에서 PNIX_HY_PYTHON unset → `hy_python() == sys.executable`;
    `pip install '.[projection]'` 후 hint대로만 따라 하면 projection 동작(원장 시나리오 역전).

### A-7그룹: 관리 도구 — `pnix_hy/capabilities.py` (A8, A9, A20) [커밋 7]

- [x] **A8 (med) capabilities가 PNIX_HY_HOME 무시 — off-tree에서 proposals=0 + 인덱스 truncate**
  - 수정: `_PKG_ROOT`/`_DOCS_DIR` 산출에 `PNIX_HY_HOME` 계층 적용(`hy_mirror`/`interop._hy_meta_dir`
    와 동일 패턴): env 있으면 `<HOME>/pnix-hy/docs`, 없으면 기존 `parents[1]/docs`.
  - 시험: 트리 밖 복사본 + `PNIX_HY_HOME=~/pnix-hy` → `capability_index()['counts']
    ['proposals'] == 20`(0000–0019); env 없이 in-repo 결과 불변.
- [x] **A9 (med) 4자리 위키링크 무조건 통과 — 깨진 proposal 링크 미검출**
  - 수정: `re.fullmatch(r"[0-9]{4}", target)` 화이트리스트 분기 **삭제**(실존 id는 이미
    `_known_names`의 proposals에서 해소됨).
  - 시험: 임시 문서에 존재하지 않는 4자리 이중대괄호 링크 주입 → `wikilinks_unresolved`에 잡힘;
    실존 id(0013 등) 링크는 통과. 임시 문서 제거 후 `docs_drift` ready.
- [x] **A20 (low) 상수 심볼이 'Built-in immutable sequence.' + 소유 `pnix_hy`로 오표기**
  - 수정: kind=='value'일 때 ① 소유 모듈: `pnix_hy` 서브모듈들을 identity 스캔(`getattr(mod, name,
    None) is obj`)해 실제 정의 모듈 기재, ② summary: `inspect.getdoc(obj)`가
    `type(obj).__doc__`와 동일하면 버리고 `''`(타입 docstring 오염 차단).
  - 시험: 재생성한 `docs/CAPABILITIES.md`에서 `ROUNDTRIP_STATUS_VOCAB` 행이
    `pnix_hy.pnix_mirror` 소유 + 'Built-in immutable sequence.' 아님.

### A-8그룹: CLI — `pnix_hy/cli.py` (A21, A22) [커밋 8]

- [x] **A21 (low) --specialize/--hy-trace가 잘못된 입력에 raw traceback**
  - 수정: 두 cmd를 try/except로 감싸 `--diagnose` 스타일 구조화 오류(1줄 + `--json`시 dict) 출력,
    exit 1. 예외 삼키기 금지 — 분류해 메시지만 정리.
  - 시험: `--specialize '(('` / `--hy-trace '(bad'` → 짧은 진단 + exit 1, traceback 없음.
- [x] **A22 (low) `;;` 분리가 문자열 리터럴 내부까지 자름**
  - 수정: `_split_capability_spec`를 따옴표 인지 스캐너로(이중따옴표/이스케이프 상태 추적, 문자열
    밖의 첫 `;;`에서만 분리). `cmd_gate_check`의 인라인 중복 구현을 이 헬퍼로 통일.
  - 시험: `--explain '"a;;b"'` → 값 `a;;b`(safe-eval과 동일); `--explain '1+2 ;; file-read'` →
    기존 분리 유지. 4개 사용처(--explain/--action-check/--action-explain/--specialize/--gate-check)
    전부 확인.

### A-9그룹: REPL — `pnix_hy/repl.py` (A16, A26) [커밋 9]

- [x] **A16 (med) `foldl'`/`my-var` 같은 유효 pnix 이름 바인딩 불가**
  - 수정: `_BIND_RE`의 이름 문자군을 런타임과 일치(`[A-Za-z_][A-Za-z0-9_'-]*` — `pnix_runtime.
    _is_ident_char` 기준). `:let`도 동일.
  - 시험: REPL 세션(io.StringIO)에서 `foldl' = 2` → bound; `foldl' + 1` → 3; `:env`에 표시.
- [x] **A26 (low) realize 실패 라인이 `_`를 오염**
  - 수정: `realize_value(raw)` **성공 후에만** `env['_'] = raw` 대입(순서 교체).
  - 시험: `41 + 1` → 42; `{ a = 1 / 0; }` → error; `_` → **42**(division-by-zero 아님).

### A-10그룹: 빌드 글루 — `scripts/ci-local.sh`, `Makefile` (A6, A18) [커밋 10]

- [x] **A6 (high) find_hy_python이 정상 proof python을 오거부(cwd 문제) + PNIX_HY_PYTHON 무시**
  - 수정: ① probe를 `(cd "$HERE/.." && "$c" -c 'import hy')`로(HY_ROOT cwd — 벤더드 hy 인정).
    ② `PNIX_HY_PYTHON`이 명시돼 있으면 **그것만** 검사: 통과 → 사용, 실패 → 즉시 exit 2(다른
    후보로 조용히 대체 금지). ③ 버전 확인 `import hy; print(hy.__version__)` == 1.3.0 경고.
  - 시험: `PNIX_HY_PYTHON=/tmp/pnix-hy-py311-venv/bin/python bash scripts/ci-local.sh` →
    proof python 채택되어 4단계 진행(LOCAL CI: PASS까지).
- [x] **A18 (low) `PY ?= python`(맥에 없음) + 실패 시 CAPABILITIES.md 0바이트 truncate**
  - 수정: ① `PY ?= python3`. ② `PYTHONPATH := $(CURDIR)$(if $(PYTHONPATH),:$(PYTHONPATH),)`
    (덮어쓰기 → 앞에 붙이기). ③ `capabilities`/`capabilities-check`: 임시파일에 쓰고 성공 시에만
    `mv`(실패해도 커밋본 보존).
  - 시험: PY를 존재하지 않는 명령으로 강제(`make capabilities PY=nope`) → 실패해도
    `docs/CAPABILITIES.md` 원본 유지(`git diff` 없음).

### PHASE A 종료 조건
- [x] A1–A26 전부 닫힘, 각 그룹 커밋 + 원장 항목 번호 명시.
- [x] `--check` 57/57 all_ready(추가한 회귀 케이스 포함), `--gate` PASS, `bash scripts/ci-local.sh`
  = LOCAL CI: PASS, examples 18개 섹션 스팟 실행(01, 03, 18) 통과.
- [x] `docs/CAPABILITIES.md` 재생성 diff 없음. main == HEAD.

---

## ▶ PHASE B — 승격 proposal 구현 (우선순위 2; ACCEPTED 0014–0019)

각 항목의 **전체 스펙·수용 기준·시험은 해당 proposal 문서가 정본** — 먼저 읽을 것.
순서는 의존성 없음(단 0018의 CLI `--ir-diff`는 A22의 헬퍼 수정 후가 편함). 각 1커밋.

- [x] **0014 Jones-optimality 게이트** (`docs/proposals/0014-jones-optimality-gate.md`)
  - `pnix_mirror.jones_optimality_report()` 신설: 코퍼스 전수 `ir_of(p) == ir_of(emit(parse(p)))`
    해시 동등 + emit∘parse 2회 고정점. 레지스트리 `"jones_optimality"` 등록(**57→58**).
  - 시험: report ready; emit를 임시 변형하면 FAIL로 떨어지는지 확인 후 원복; `--gate` PASS.
- [x] **0015 수치 무손실 술어** (`0015-interop-numeric-losslessness.md`)
  - `interop.numeric_fits()` + from_host/to_host 수치 경로 lossy 마킹(값 불변) +
    `roundtrip_host_value` probe에 `2**53+1`, `10**30`, `0.1`, inf, nan 추가. 카운트 불변.
  - 시험: `roundtrip_report` ready + numeric 섹션 판정 일치; 기존 loss 마킹 회귀 0.
- [x] **0016 own/borrow 수명 규율** (`0016-opaque-own-borrow-lifecycle.md`)
  - `_OPAQUE_META`에 owned/num_lends; `lend_opaque()` contextmanager; 대여 중 `release_opaque` →
    InteropError; `opaque_lifecycle_report`에 위반 케이스 추가. **ref shape 무변경.** 카운트 불변.
  - 시험: 정상/위반 시퀀스 판정 + 리포트 30회 연속 멱등(기존 회귀 유지).
- [x] **0017 hygiene self-check** (`0017-hygiene-self-check.md`)
  - `pnix_mirror.hygiene_report()`: 포획 시도/gensym/심볼 diff 3계열 케이스. 레지스트리
    `"hygiene"` 등록(**+1**). Hy 부재 시 available:False.
  - 시험: 심은 포획 검출 + gensym 무포획; report ready.
- [x] **0018 IR 구조 diff + 패스 물화** (`0018-ir-diff-and-pass-reification.md`)
  - `ir.ir_diff(a,b)`(결정적 노드-경로 diff) + `ir.ir_pipeline()`(패스 델타 물화+불변식) +
    `ir_diff_report()` 등록(**+1**). 선택: CLI `--ir-diff 'A ;; B'`.
  - 시험: 동일 소스 equal / 리터럴 1개 차이의 divergence 경로 정확 지목; report ready.
- [x] **0019 해시-키 검사 캐시** (`0019-hash-keyed-check-cache.md`)
  - `pnix_hy/check_cache.py`(verifying-trace 캐시, FAIL 비캐시, 보수적 키) + `--check --cached`
    플래그(**기본 경로 불변**) + `check_cache_report` 등록(**+1**).
  - 시험: 2회째 대부분 cached & all_ready 동일; 소스 1바이트 변경 → 재실행; `--gate` 비캐시 PASS.

### PHASE B 종료 조건
- [x] `--check` **61/61** all_ready, `--gate` PASS, `--capabilities` 재생성(proposals 20, reports 61)
  diff 없음, 각 proposal 문서 상태를 SHIPPED로 갱신, main == HEAD. (2026-07-02 구현·검증 완료)

---

## ▶ ACTIVE PHASE C — 0013 잔여 후보 전량 승격 구현 (proposals 0020–0027, ACCEPTED)

각 proposal 문서가 스펙·수용기준의 정본. 목표: `--check` 61 → **68**(신규 리포트 7). 전부 additive,
sacred 무접촉, 공유 ABI envelope 무변경(0024는 payload-레벨 설계로 회피), 0027만 hy-meta 레인
(host_exec/clean_replay, bootstrap 본체 무접촉). GitHub Actions·D2/D3 cross-repo ABI는 범위 외 유지.

- [x] **0020 interop 하드닝** — I1 `grant_capability` 핸들(attenuate/suspend/revoke), I3
  `interop_context()`(닫힘 후 접근 거부), I4 `InteropError.blame`, I5 `harden_opaque`(표면 witness
  재검증), I6 `declare_opaque_invariants`(기질 강제). → `interop_hardening_report` (+1)
- [x] **0021 Compartment** — 구획별 env+모듈 테이블, builtins 공유, 완전 격리. → `compartment_report` (+1)
- [x] **0022 phase 게이트** — P2 phase ±정수 대수 + P4 lowering 무부작용·관측 무관성. → `phase_separation_report` (+1)
- [x] **0023 증분 평가** — R1 정의-단위 의존성-치환 해시+부분 재계산, R3 realisation(ir_hash→value_hash)
  조기중단, α-rename 히트. → `incremental_eval_report` (+1)
- [x] **0024 typed witness** — payload-레벨 predicate URI + deprecate 마이그레이션(envelope 불변 검증
  포함). → `typed_witness_report` (+1)
- [x] **0025 PE 어노테이션+재특화** — assumptions/boundaries(기본 경로 byte-동일), deopt 판정,
  `respecialize_if_drifted`. → `pe_annotations_report` (+1)
- [x] **0026 타워 마일스톤-1~7 — 허용 scope 내 CLOSED** (Futamura 사다리 1·2·3차 사영을 pnix로 표현·생성·실행 검증 + `--futamura` 통합)
- [x] **0029 efficient cogen (cogen approach) P1+P2 SHIPPED** — P2: `compiler_source`/`compile_with` = 생성확장을 standalone pnix 소스로(poly_mix_in_pnix 재사용).  `pnix_hy/cogen.py`: hand-written 생성확장(`generating_extension`/`compiler_from_interpreter`), self-application 없이. 인터프리터→컴파일러 **0.003s**(naive run_cogen >150s), `cogen` 리포트(--check 71→72). 0028 P2 해소. 근거=`docs/audits/2026-07-02-cogen-stagepoly-research.md`
- [~] **0028 pnix compiled runtime — P1+P3 SHIPPED** (P3: `subset_supported`/`evaluate`/`--ceval` fast-path 자동 fallback + 활용: `--compiled-bench`·examples/19·`compiled_differential` 오라클. P2(optimal cogen)만 연구 대기) — cogen→풀 컴파일러 성능벽의 진짜 해결(별도 백엔드, SACRED 무접촉). 두 손쉬운 시도(compiled cc.py>6min, O(1) memo)가 불충분함을 실측 확인. P1=`pnix_hy/compiled.py`+`compiled_runtime` 리포트(코퍼스 27/27, --check 68→69). **P2(cogen<30s): 4실험(tree-walker/thunk/closure/스케일스윕)으로 근본원인=naive cogen 아티팩트의 병리적 bloat 확정 — num-only 초소형조차 >150s. 런타임/스케일로 해결 불가, optimal-cogen 연구 필요(중단)**. 스펙: `docs/proposals/0028-compiled-runtime.md` (M7: `futamura_ladder`/`--futamura` 통합 capstone — 1·2·3차 사영 한 산출물. 남은 둘=cogen 풀컴파일러(성능/compiled-runtime), stage7 stage-poly 재작성(SACRED/경계결정) — 정확성 아님) (M6: `run_cogen` — **cogen 아티팩트가 실행되어 specializer로 정확히 동작**(2*3+4→10, a*b|a=6→(6*b); run_cogen==poly_mix_in_pnix) — milestone:6. 풀 컴파일러 재도출은 호스트 성능벽. M7=stage7 stage-poly 재작성은 SACRED, SCOPE_LOCK 경계결정 필요) (M5c: closure conversion으로 **cogen self-application 완주**(21-point 아티팩트 생성; 3차 사영 종료 성립). 벽 이동: 생성됨→실행 비현실적(호스트 깊이). milestone:5. M6 = cogen 아티팩트 실행가능화(trampolined force)+stage7 stage-poly 재작성) (M5b: **pnix 단독 2차 Futamura 사영**(POLY 객체언어 폐포+deep-eval 워커; 컴파일러를 pnix가 단독 생성) — milestone:5. M5c cogen=self-application은 호스트 재귀깊이로 blocked(정직 기록, T2h 수용장치 대기). M6=stage7 stage-poly 재작성) (M5a: **polyvariant mix를 pnix로**(state-passing memo, 외부 S=L core 성립) — milestone:5; 남은 사다리 = M5b/c 객체언어 폐포→pnix 단독 2차→cogen, M6 stage7 재작성) (M4: polyvariant specialization + **실제 2차 Futamura 사영**(컴파일러 생성·검증) — milestone:4; M5 = pnix-표현 polyvariant mix→자기적용→cogen, stage7 재작성) (M3: pnix-안 1차 사영·MIX 객체언어 S=L 폐포 성장·offline BTA v1 — milestone:3; M4 = polyvariant→실 2차사영→cogen, stage7 재작성) (M2: 인터프리터 붕괴·mix 비교연산·CEK 중단/재개·평가-중 EM — tower_ladder_report milestone:2) — T5m stage-poly 미니 평가기, T3m mix-in-pnix(서브셋 S=L), T2h cogen
  수용 하니스, T6 reify/reflect v0(권한 게이트+parity), T8 EM v0(이중 모드). → `tower_ladder_report` (+1)
- [x] **0027 host artifact gap** — G2 form_sha256, G5 변수-단위 env_diff (hy-meta 레인, additive).
- [x] 종료: `--check` **68/68**, `--gate` PASS, CAPABILITIES 재생성 diff 0, proposal들 SHIPPED, main FF. (2026-07-02 완료)

## SHIPPED (요약; 상세 `docs/proposals/`, 이력 `docs/archive/todo-history.md`)

- **0001–0007** interop 언어기능(--check 44→54) · **0008** REPL 5모드 · **0009** action VM(→56) ·
  **0010** 모듈 배포 티어 · **0011** docs-as-code 인덱스+drift 게이트(→57) · **0012** CI 게이트
  (GitHub Actions는 현재 disabled; 로컬 = `make ci`) · **0013** deep-research 후보 카탈로그(문서).

## 연구 백로그 — 딥리서치 #1/#2 도출 (전부 미착수 · 근거: `docs/audits/2026-07-02-cogen-stagepoly-research.md`, `docs/audits/2026-07-03-laziness-stagepoly-research.md`)

### Q1 — laziness × 부분평가 (specializer/BTA lane · 추가적 · SACRED 4-lane 무위험)
> 핵심 발견: 해악은 laziness가 아니라 **SHARING**. call-by-need는 CBV의 문제(naive unfold가 공유 작업 중복 → 잔여가 원본보다 느림, CBV 최대 33×)를 상속. 무제한 정규화는 비공유 normal-order에서만 Jones-optimal. thunk-as-dynamic은 맞지만 보수적(dynamic이 아래로 전파해 정적 내부식을 재구성=bloat). 회복은 "더 많은 정적계산"이 아니라 **sharing-safe 축약 + BTI**. 순수-lazy 컴파일러 PE 생성은 실증됨(BAWL, Jørgensen POPL'92) — 단 BTI 필수.

- [x] **Q1-1 sharing-safe unfolding (affine/right-linear 가드) — SHIPPED** — `tower._occurs` + `_ps` let 경로: dynamic 바인딩이 body에서 2회↑면 공유 `let`으로 잔여화(인라인 복제 방지). `cogen_report.sharing_safe_unfold`. 회귀 0.
  - 알아낸 것: 공유 변수 unfold가 중복 계산을 낳아 잔여가 느려짐(Brown&Palsberg POPL'18 Table3: CBV 0.03× = 33× 느림; Fischer/Silva/Tamarit/Vidal LOPSTR'07). 방어적 "unfold 전면 금지"는 과도(프로그램 그대로 반환) → 필요한 건 **공유를 잃지 않는 redex만 축약**하는 specialization-safe reducer.
  - 구현: `_ps`의 apply/let unfold 지점에서, 인자가 dynamic이고 파라미터가 body에서 2회↑ 참조(non-affine)면 unfold 대신 residual `let`으로 공유 보존. affine 판정 = body에서 해당 var 발생 횟수 카운트 헬퍼.
  - 테스트: `let y = <dyn 무거운식>; in y + y` 잔여가 y를 1회만 계산(중복 0); M4/M5a/M5b + `cogen_report` 회귀 0; parity 유지.
  - 위험: specializer lane 한정, `pnix_runtime`/stage7/4-lane 무접촉.
- [x] **Q1-2 eta-expansion "The Trick" (product/sum/list) SHIPPED** — `_ps` if 핸들러가 dynamic if의 구조적 브랜치(attrset/list)를 구조로 **분배**(`_as_attrs`/`_as_list`), 다운스트림 select/index가 정적 필드 회복. **syntactic + let-bound 둘 다**: `(if b then{v=1}else{v=2}).v`→`(if b then 1 else 2)`; `let r=if b then{a=10;c=20}else{...}; in r.a+r.c`→`((if b then 10 else 30)+(if b then 20 else 40))`(attrset 소거). `cogen_report.eta_expansion_trick`. 잔여(소규모): function-type eta(부분정적 함수) + BTA 리포트 통합
  - 알아낸 것: dynamic 분류가 아래로 전파해 정적 부분을 재구성(bloat). eta-expansion = 함수/합/곱 타입 경계의 uniform binding-time coercion으로 정적계산 회복 + 합 타입에서 "The Trick" 자동화 + "padding"으로 dynamic 오염 차단; **extended BTA가 자동 삽입 가능**(소스 수정 불필요). (Danvy/Malmkjær/Palsberg TOPLAS'96, LASC'95)
  - 구현: BTA에 타입-경계 eta-expansion 삽입 패스 추가 → `poly_specialize`/`cogen`이 개선된 주석 사용.
  - 테스트: 부분-정적 구조에서 정적 부분 폴딩되어 잔여 축소; 의미 동일(parity); 회귀 0.
- [ ] **Q1-3 CPS로 작성한 specializer (Bondorf BTI)** — 큰 작업(별도 proposal 권장)
  - 알아낸 것: specializer를 CPS로 쓰면 source-CPS의 BTI 효과를 얻되 **출력 안 부풀림**(잔여는 CPS 아님, 생성확장에 closure 조작 오버헤드 없음; Bondorf LFP'92). CBN-CPS는 deforestation 전부 달성(Nielsen&Sørensen SAS'95). 주의: BTI는 non-Jones-optimal specializer를 보상하지 못함(refuted 1-2) → 건전한 reducer 전제.
  - 구현: `poly_specialize`를 CPS 스타일로 재작성 또는 신규 변형(대규모).
  - 테스트: 잔여 크기/속도 벤치 개선 + parity.
- [x] **Q1-4 측정 SHIPPED** — `cogen.pe_size_report`(`--check` +1): 공유 부분식이 사용 k=2/4/8/16 전부 **1회**(잔여 33/45/69/117B, y참조만 증가) vs naive 복제 k회 → Q1-1 공유로 잔여가 k에 대해 flat. eta 폴딩 동반 측정. (CBV/CBN/CBNeed 교차비교는 pnix가 call-by-need 단일이라 불가 — 대신 공유효과를 정량화.)

### Q2 — stage-polymorphic maybe-lift (SACRED 관련 · **CLOSED: 미추진 결정**)
> 딥리서치 #3 결론(`docs/audits/2026-07-03-stagepoly-decision-research.md`): maybe-lift/별도 평가기 **미추진**. 이유: (a) in-place=미러 소스해시 깨짐(배제), (b) 별도 hand-maintained 병행 평가기=문헌(RPython)이 명시적 반대(drift; anti-drift는 mechanized generation 필요) + 검증 방법(Q2.3) 근거 0, Truffle/RPython 기법은 host-specific이라 Hy/CPython 미전이. **대신 이미 shipped된 derive 경로(0029 `compiler_from_interpreter`/`poly_mix_in_pnix`)가 '인터프리터 1개→컴파일러 파생'을 실현** — RPython 권고(파생, hand-maintain 금지) 형태와 일치, sacred 미러 무접촉. 즉 stage-poly 목표는 Futamura/cogen 경로로 충족됨.

- [x] **Q2-0 (제약, 지킴) sacred in-place 재작성 금지** — 확정(소스해시 lane 깸). RULED OUT.
- [x] **Q2-1 host staging 층 — WON'T-DO** — maybe-lift 미추진 결정에 따라 불필요. (Truffle/RPython staging은 host-specific이라 Hy/CPython 미전이 확인.)
- [x] **Q2-2 별도 maybe-lift 평가기 — WON'T-DO (derive 경로가 대체)** — 문헌이 hand-maintained 병행을 반대(drift), 검증 방법(Q2.3) 근거 0. '인터프리터 1개→컴파일러 파생' 목표는 0029 cogen 경로(`compiler_from_interpreter`)로 이미 충족(RPython 권고 형태), sacred 무접촉. → maybe-lift 재구현 불필요(중복).

### 3차 딥리서치 (Q2 미해결분 — 검증 통과 0이었음)
- [x] **R3-1 (Q2.2) 답변** — Truffle=PE(1차 사영)+JVM 관용구(@Child/@CompilationFinal/@ExplodeLoop/@TruffleBoundary/@Specialization), RPython=메타트레이싱(JitDriver green/red + jit_merge_point/can_enter_jit, RPython+PyPy 툴체인 필수). **둘 다 host-specific → Hy/CPython 미전이**. green/red=binding-time(=LMS Rep). SOM 실측 Truffle~2.3×/RPython~3×(실용).
- [x] **R3-2 (Q2.3) 답변=미해결(문헌 근거 0)** — translation validation/refinement/metamorphic/differential/bisimulation 검증 통과 0. 별도 평가기 게이트 방법 부재 → 별도 평가기 미추진의 또 다른 근거. (pnix는 실무상 4-lane 미러+`compiled_differential`로 차등게이트 이미 사용.)
- [x] **R3-3 (Q2.5) 답변=derive-not-hand-maintain** — RPython 권고=single-source auto-regeneration(파생, 병행 hand-maintain 금지). pnix는 0029 derive 경로가 그 형태 → **별도 maybe-lift 평가기 미추진**, sacred 무접촉으로 목표 충족. (Q2.2(c) LMS는 여전히 문헌-open이나 결정에 무관.)

## 연구-도출 구현 기회 (미착수 종합 · 딥리서치 #1–#3) — 우선순위대로 구현
> Q1-1/Q1-2/Q1-4 SHIPPED, Q2 stage-poly CLOSED(derive 경로가 대체). 아래는 문헌이 시사한 **남은 실행 가능 BTI/최적화**. 전부 specializer/BTA lane · 추가적 · SACRED 무접촉 · 순수 pnix라 의미 불변(크기/공유만 개선).

- [x] **I1 bounded static variation — SHIPPED** — `_ETA_DIST_BUDGET`(=200): `_ps` if-distribution이 `len(cc)*(nfields-1)<=예산`일 때만 분배, 초과면 단일 if(cond 1회). 작은 cond는 분배 유지, 큰-cond·다필드는 폭증 방지. `pe_size_report.bounded_static_variation`. 회귀 0.
  - 알아낸 것: Q1-2 if-distribution은 cond 코드를 **각 필드에 복제**한다(`{a:(if cc ..),c:(if cc ..)}`). cond가 크고 필드가 많으면 크기 폭증 — DMP/JGS의 "bounded static variation"이 이걸 막는 BTI.
  - 구현: `_ps` if-distribution에서 `len(cc) * (nfields-1) <= 예산(≈200)`일 때만 분배; 초과면 단일 `if`로 잔여화(현행 안전 동작). attrs/list 둘 다.
  - 테스트: 작은 cond(`b`, `x.tag==..`)는 여전히 분배(기존 게이트 유지); 큰-cond·다필드 합성 케이스는 단일 if(cond 1회). parity·회귀 0.
- [~] **I2 eta at SUM type — 계측결과 SKIP(실익 없음)** — 계측(2026-07-03): dynamic tag-분기(`(if b then "a" else "z")=="a"`)는 **우리 실제 워크로드에 미등장** — 인터프리터는 static prog.tag로 분기 → 컴파일러 잔여에 dynamic tag-비교 0개. 개선 대상 없어 speculative 구현 보류. (필요한 프로그램이 생기면: binary/`==`를 dynamic if 브랜치로 push + `if b then true else false`→`b` peephole.)
- [~] **I3 eta at FUNCTION type — SKIP(polyvariance가 이미 처리)** — 부분-정적 함수는 현행 spec-point polyvariance가 static 시그니처별로 특화 중. arity raising의 추가 실익 불확실 + 우리 워크로드 미검증 → 보류.
- [x] **I4 let-insertion — SHIPPED** — `_PSState.hoist`(코드 dedup) + `poly_specialize`가 hoist 바인딩을 최상위 공유 let으로 방출. eta-분배의 non-trivial cond를 1회 hoist(`__h`), trivial은 인라인. `(if (x*x+x)>5 then{a=1;c=2}else{...}).{a,c}`→`let __h1=((x*x+x)>5); in ((if __h1 then 1 else 3)+(if __h1 then 2 else 4))`. `pe_size_report.let_insertion`. (정적-구조 일반 중복 hoist는 후속 여지.)
  - 알아낸 것: 잔여에서 **정적 구조**가 여러 곳에 복제될 때 let으로 공유(Q1-1은 dynamic만 다룸). let-insertion BTI.
  - 구현: 잔여 방출 시 동일 부분트리 ≥2회면 `let`으로 hoist(구조적 해시로 탐지).
  - 테스트: 중복 정적 구조가 1회로; parity.
- [~] **Q1-3 → 0030 context propagation P1 SHIPPED** — CPS 전면 재작성 대신 Bondorf CPS **효과**(문맥 전파)를 commuting conversion으로 realize: `_commute_binary_if`가 `(if c then a else b) op R`(R=static scalar)를 브랜치로 push→폴딩. `pe_size_report.commuting_conversion`. 스펙 `docs/proposals/0030-context-propagation.md`. P2(apply-through-if/let-bound/전면 CPS)는 실익 우리 워크로드 미검증이라 보류.
- [ ] **(연구-open, 현재 비착수)** Q2.2(c) LMS `Rep[T]` 타입-스테이징 심화 · Q2.3 등가-보존 검증 방법론(translation validation/refinement/bisimulation) — 별도 평가기 미추진 결정으로 **결정엔 무관**. 필요 시 4차 리서치.

## Host PATH (dot-nix, 2026-08-13)

dot-nix exposes `pnix-hy-python` / `pnix-hy-hy` and joins them as
`python`/`python3`/`hy` via `pnix-hy-host`. Global override of
`pkgs.python311` is intentionally **not** done (breaks nixpkgs builders).

### Open (product / packaging)

1. Single interpreter story: proofPython (Hy pin) vs kimchi python-with-packages
   still dual — document which is “the” host for pure pnix vs science stacks.
2. Optional: ship a `packages.pnix-hy-host` from the flake so consumers need
   not re-implement the symlinkJoin in every HM tree.


## Host-language import of pnix product library (user intent, 2026-08-13)

**Canonical doctrine:** [`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md)  
**HM mirror:** `~/dot-nix/dev/PNIX-HOSTS.md`

Context from home-manager (`dot-nix`) integration:

- `pnix-<host>-pnix` = pnix-language surface (REPL/eval of `.px`) on this host.
- `pnix-<host>-<lang>` = host-language interpreter/compiler used for day-to-day
  host development.
- Libraries produced by the **pnix product half** of this host are **host-
  language libraries**: they must load in *this* host language. They are **not**
  assumed to be portable common bytecode for other hosts.
- A future **common portable `.px` library** track (historical pnix-meta style)
  is deferred; do not block host-local import work on that.

dot-nix can only set PATH/env (classpath, PYTHONPATH, link paths, NODE_PATH,
DLL HintPath). Anything that requires a real packaging format is product work
below.


### hy — status (2026-08-14)

**Landed:**

1. Dual-axis docs: `HOST_DEV_ENV.md`, host `CLAUDE.md` / `README.md`.
2. Host-main: `PYTHONPATH` + `pnix_hy` via HM `pnix-hy-host` (`python`/`hy`).
3. Host-language `.px` import: `pnix_hy.eval_file` (= `run_px`); package install.
4. Env: `PNIX_HY_HOME`, `PNIX_HY_LIBRARY`, `PNIX_HY_PYTHON`.

**Still open:**

1. Public `import pnix_hy` API stability (which submodules beyond `__all__` are
   host-library API) — `__all__` + `HOST_IMPORT.md` is the current contract.
2. ~~Optional: py.typed~~ — `pnix_hy/py.typed` + setuptools package-data (2026-08-14).
   Richer stubs still optional if external consumers need them.
3. ~~import hook documented~~ — see monorepo `HOST_IMPORT.md` § hy
   (`install_pnix_import_hook`); host-only, not common-meta.

## Post host-env plan (2026-08-14) — plan only

Host dual-axis + library import for this host is **closed** for day-to-day.
Optional P2/P3 and residual product work: monorepo `HOST_ENV_P2_P3.md`.
Do not reopen host-env packaging as a primary gate unless env contracts break.

### Local library export (2026-08-14)
- [x] `bin/export-pnix-hy-library` → `target/pnix-hy-library/site` + `py.typed`
- [x] `bin/pnix-hy-library-smoke` (eval_file → 3)
- Not PyPI — personal/local PYTHONPATH feed only.
