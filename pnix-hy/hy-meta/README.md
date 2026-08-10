# Hy Meta Bootstrap

이 디렉터리는 Hy 1.3.0에 대한 로컬 메타서큘러(meta-circular) 실험을 구동한다.

- `stage1/` — Python으로 쓴 seed 컴파일러. Hy의 기존 reader/컴파일러를 작은 프로토콜로 감싼다:
  read, Python AST로 compile, Python emit, execute, evaluate, 그리고 .hy 파일을 Python 모듈로 load.
- `stage2/` — 같은 프로토콜을 Hy로 작성. stage1이 컴파일한다.
- `hy-meta/` — 부트스트랩 표면. `stage2/compiler.hy`를 로드한 뒤 Hy로 작성된 컴파일러 API로 코드를 돌린다.

**Python 3.11 또는 Homebrew 최신 Python 3.14 proof 타겟만** 명시적으로 사용한다.
이 레인에서 Python 3.12/3.13은 쓰지 않는다.

```sh
/usr/local/bin/python3.11 -m venv /tmp/pnix-hy-py311-venv
/tmp/pnix-hy-py311-venv/bin/python -m pip install 'funcparserlib ~= 1.0' pytest
/usr/local/opt/python@3.14/bin/python3.14 -m venv /tmp/pnix-hy-py314-venv
/tmp/pnix-hy-py314-venv/bin/python -m pip install -e . pytest
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py self-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py chain-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py kernel-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py direct-kernel-bridge-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py kernel-import-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py prime-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage3-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py mirror-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage7-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py self-host-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py bootstrap-fixedpoint-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py no-fallback-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py parity-ledger-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py parity-ledger-check --debug-dir work/parity-ledger-debug
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage8-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage8-check --debug-dir work/stage8-debug
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage9-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage9-check --debug-dir work/stage9-debug
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage10-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage10-check --debug-dir work/stage10-debug
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage11-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage11-check --debug-dir work/stage11-debug
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage12-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage12-check --debug-dir work/stage12-debug
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage13-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage13-check --debug-dir work/stage13-debug
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage14-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage14-check --debug-dir work/stage14-debug
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage14-export
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage14-import-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage14-import-check --debug-dir work/stage14-import-debug
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage15-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage15-check --debug-dir work/stage15-debug
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage15-export
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stagen-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stagen-check --debug-dir work/stageN-debug
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py reader-boundary-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py cli-io-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py hyc-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py repl-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py startup-output-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py native-subset-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/native_subset_test.py
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py run "(+ 40 2)"
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py run -c "(+ 40 2)"
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py run -f hy-meta/examples/shebang.hy
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py py -f hy-meta/examples/factorial.hy
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py hy2py -c "(+ 20 22)"
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py hyc -c "(setv answer 42)" -o /tmp/hy-meta-answer.pyc
printf '(+ 20 22)\n' | /tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py repl
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py kernel-run -f hy-meta/examples/factorial.hy
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py kernel-py -f hy-meta/examples/factorial.hy
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py kernel-run -f hy-meta/examples/kernel_features.hy
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py kernel-run -f hy-meta/examples/kernel_loop.hy
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage7-kernel-run "(+ 20 22)"
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage7-kernel-py "(+ 20 22)"
/tmp/pnix-hy-py314-venv/bin/python hy-meta/bootstrap.py stage7-check
/tmp/pnix-hy-py314-venv/bin/python hy-meta/bootstrap.py self-host-check
/tmp/pnix-hy-py314-venv/bin/python hy-meta/bootstrap.py bootstrap-fixedpoint-check
/tmp/pnix-hy-py314-venv/bin/python hy-meta/bootstrap.py no-fallback-check
/tmp/pnix-hy-py314-venv/bin/python hy-meta/bootstrap.py parity-ledger-check
/tmp/pnix-hy-py314-venv/bin/python hy-meta/smoke_test.py
```

`stage1`과 `hy-meta/bootstrap.py`는 Python 3.11 또는 Homebrew Python 3.14가 아닌 버전에서는
의도적으로 중단한다.

같은 proof 레인이 `.github/workflows/hy-meta-proof.yml`에 Python 3.11 / 3.14 매트릭스로 연결되어
있고, `hy==1.3.1`을 설치해 syntax/whitespace 검사와 `hy-meta/smoke_test.py`를 실행한다.

## Status / primary gate

See [STATUS.md](STATUS.md). Primary gate: `./hy-meta/bin/hy-meta-gate` (self-check + stage7-check).

## 부트스트랩·mirror 체크

`chain-check` — 핵심 부트스트랩 체크:
1. Python으로 쓴 stage1이 `stage2/compiler.hy`를 로드.
2. Hy로 쓴 stage2가 `stage2/compiler.hy`를 두 번째 모듈로 다시 로드.
3. 두 stage2 모듈이 `hy-meta/examples/factorial.hy`를 평가.
4. 생성된 Python 출력을 비교.

`kernel-check` — stage2를 통해 `stage2/kernel.hy`를 로드하고, Hy로 작성된 작은 컴파일러 커널을
검증한다. 이 커널은 대상 프로그램에 대해 `hy.compiler.hy_compile`을 호출하지 않고, 의도적으로 작은
Hy 서브셋을 **직접 Python AST로** 컴파일한다.

### 커널이 직접 지원하는 Hy 서브셋 (요약; 폼 이름은 코드 식별자 그대로)

정밀 스펙이므로 아래 형태 이름은 원문 그대로 둔다. 커널이 직접 낮추는(direct-kernel) 범위:

- **리터럴/기본**: 리터럴, f-string 리터럴, 심볼, `...`/Ellipsis, 호출, `defn`, `setv`, `if`,
  `do`, `fn`(문장 바디 포함), `let`, `quote`, 단순 `import`, 속성/메서드 접근, `while`, `break`,
  `continue`, 산술/비교 연산자, 기본 컬렉션 리터럴.
- **파라미터**: 필수/기본/vararg 위치 파라미터, `#**` 키워드 캡처, 호출부·리터럴 iterable/mapping
  언패킹, 키워드 인자, positional-only `/`, keyword-only `*`, keyword-only 기본값, keyword-only
  튜플-패턴 구조분해, `#(...)` 파라미터 구조분해 + 그 위 annotation.
- **매크로/staging(컴파일타임)**: top-level `defmacro`(+ `quasiquote`/`unquote`/시퀀스
  `unquote-splice`), 컴파일타임 `require`(module-prefix/선택/`*`/`:as`/per-name alias/재귀 `*`
  import/no-leak macro-table/상대 require), one-shot `hy.R...` 매크로 호출. 중첩 quasiquote는
  unquote 깊이를 보존.
- **구조분해/let**: starred list/tuple 대입 구조분해, `let`의 list/tuple 구조분해, starred·중첩
  starred `let` 구조분해, annotated `let` 바인딩, 순차 `let` 재바인딩, native 스타일 `let` 바디
  바인딩 누수(let-bound 이름 은닉), 연산자 인자 튜플-패턴 iterable 언패킹.
- **제어흐름**: 식/문 수준 `when`/`cond`; `do` 식은 pending-statement 결과변수 낮춤으로 문장 폼을
  담을 수 있고, `if`/`when`/`cond` 식은 브랜치-로컬 pending 문장을 보존; 하나 이상의 target/iterable
  쌍에 대한 `for`; `for`/`while`의 후행 `(else ...)`.
- **예외**: 문장 `raise`, `try`(+`except`/`else`/`finally`), body/`except`/`else` 값을 돌려주는
  `try` 식(sync/async, 결과변수 인라인 낮춤), `except`/`except*` spec의 catch-all `[]`·타입 리스트·
  native name-first 바인딩·구형 type-first 형태, Python 3.11 `except*`(TryStar).
- **클래스/import**: `defclass`(optional base vector, `:metaclass` 등 class kwarg, 문장 바디,
  데코레이터 리스트), `defn`/`defclass` 데코레이터, 단순/dotted/상대 import, Hy 스타일 module/name
  mangling, `:as` alias, per-name alias가 있는 `from` import, `(import math *)` → `from math
  import *`, 선행 `__future__` import(주입된 `import hy` 앞, docstring 뒤).
- **f-string/quote 메타데이터**: f-string → `JoinedStr`/`FormattedValue`(conversion/format-spec/
  중첩 spec/debug/문장-생성 치환식), quoted 문자열·f-string 모델은 bracket/컴포넌트 메타데이터를
  보존해 지연 `hy.eval` + `repr`/`eval` 왕복 유지.
- **대입/문장-식 위치**: 심볼/구조분해/속성/native chained `setv :chain`; `setv`는 식 위치에서 `None`
  반환; `defn`/`defclass`/`for`/`assert`/`pass`/`del`이 식 위치에서 `None`; `and`/`or`는 pending
  문장에도 short-circuit 보존.
- **컬렉션/subscript/dot chain**: list/tuple/set/dict 리터럴 평가순서 보존, `(get ...)` 다중/중첩
  subscript, 속성/subscript/메서드 세그먼트 dot chain, method shortcut(선행 kwarg 뒤 receiver 배치,
  빈 dot chain은 receiver 그대로, `(. defn)` 같은 dotted statement-special root).
- **연산/증강대입**: `//`/`%`/`**`/matrix `@`/shift/bitwise and·or·xor/`bnot`/`invert` + 대응
  증강대입, unary `+`/`-`/reciprocal `/`, right-assoc `**`, `|` 빈 identity, iterable 언패킹 arity;
  multi-RHS 증강대입의 native 그룹핑(`**=`/`@=`).
- **annotation/setx/raise :from**: `annotate`·`#^`(변수/standalone/파라미터/반환), `setx`(대입식;
  comprehension 스코프가 helper-local이 아닌 포함 스코프 갱신), `:from` cause를 가진 `raise`.
- **비교/comprehension**: `in`/`not-in`/`is`/`is-not` 등(native arity 보존: unary `=`/`<`/… 는 True,
  `!=`/`in`/… 은 2개 이상 필요), `chainc` 혼합 chained 비교, `lfor`/`sfor`/`dfor`/`gfor`(+`:if`/
  sync·async `:do`/`:setv`, 최종 `#*`/`#**` 언패킹).
- **context manager/generator/scope**: sync `with`(named/`_`/다중/`:async` 쌍), sync·async `with`
  식의 outer-scope 효과(인라인 결과변수), `AsyncWith`; `return`/`yield`(문·식)/`(yield :from ...)`;
  module/function/class docstring; `pass`/`assert`; `global`/`nonlocal`(module로 resolve되는
  `nonlocal`은 `global`로 승격).
- **match/async**: native flat clause·wildcard·capture·literal·keyword·dotted-value·sequence·
  mapping·class 패턴, `#*`/`#**` 캡처, `(| ...)` OR, `(as ...)`/`:as` AS, `:if` guard(문장형 `do`
  guard 포함); `defn :async`/`fn :async`/`await`, `for [:async ...]`, `:async` comprehension.

`do`/`if`/`when`/`cond` 식의 문장 포함은 pending-statement 결과변수 낮춤으로 처리한다.

`hy-meta/native_subset_test.py`는 커널 경로에 대한 집중 native-style 검사를 담는다(lambda-list
edge, 문장-바디 `fn`, class-body 베어 `#^`, 복합 annotation, 잘못된 바인딩 거부, quasiquote/중첩
quasiquote 깊이 + 임시 `sys.modules` 복원 하의 반복 `hy.eval`, docstring edge, f-string 포맷/변환/
spec/debug/평가순서, quoted 모델 메타데이터, keyword 비교·정렬·pickling·mangling, 연산자 edge,
break/continue, defclass dynamic-base/leak/side-effect, 데코레이터 순서/스태킹/async, `with` 예외
억제, comprehension side-effect, match 패턴, native match 구문, `let` 바디 leak, `defclass`
shorthand + class kwarg, `and`/`or`·`if`/`when`/`cond`의 문장-생성 피연산자, `do`/`del` edge,
`nonlocal` 승격·missing 거부, dotted method shortcut, native 인자 순서, `hy.eval`/`hy.read`/
`hy.I`/`hy.R`/macroexpand/`hy.repr`/모델 구성/재귀 모델 탐지/Unicode mangling/PEP 3131/kernel import
hook를 통한 모듈 inspect 메타데이터 등).

`hy-meta/native_test_import_map.md`는 어떤 upstream native-test 파일이 포팅/부분포팅/버전스킵/직접
커널 레인 밖(upstream Hy 툴링)으로 스킵됐는지 기록한다.

`hy-meta/failure_minimizer.py`는 커널/native parity probe용 결정적 줄삭제 축소기다. 지정한 예외
클래스와 선택적 오류 substring을 보존하며 소스 파일/stdin probe를 줄인다:

```sh
/tmp/pnix-hy-py311-venv/bin/python hy-meta/failure_minimizer.py \
  --expect SyntaxError --contains "kernel" -f failing_probe.hy
```

생성된 Python/source diff 스냅샷은 의도적으로 커밋하지 않는다. mirror 명령들은 AST 덤프와 생성
소스를 메모리에서 비교해, 잡음 파일을 추가하지 않고 proof 표면을 결정적으로 유지한다.

`hy-meta/examples/kernel_features.hy`는 최신 커널 경로를 종합 예시로 돌린다(quasiquote 매크로,
quoted/splicing 매크로, closure, `let`, docstring, keyword/callable/kwargs, import, method
shortcut, `#*`/`#**`, starred 구조분해, `dfor #**`, `let` 구조분해, `defclass` shorthand + class
kwarg, dynamic base, f-string, tuple-pattern 파라미터, positional/keyword-only, 문장-바디 `fn`,
comprehension `:do`/`:setv`, match, `when`/`cond`, `for`, 예외, 데코레이터, 확장 import, 속성/
subscript/dot-chain, del/증강대입, annotation, `setx`, boolean short-circuit, slice, containment,
comprehension, context manager(sync/async), `pass`/`assert`, `global`/`nonlocal`, loop `else`,
`except*`, `raise :from`, match, return/yield, async 함수·generator·`await`·async `for`·async
comprehension 등).

## stage 체인 / mirror / self-host 체크

`prime-check` — 현재 메타서큘러 proof 레인: Python stage1 → stage2 → stage2-prime → `hy-meta-check`
(Hy로 작성, stage2-prime에 상주) → `stage2/kernel.hy` 로드 → factorial/feature/loop 예제 평가.

`stage3-check` — 체인을 한 세대 더 확장(stage1 → stage2 → stage2-prime → stage3; stage3가
`hy-meta-check` 실행 + 커널 로드 + Hy 커널로 예제 평가).

`mirror-check` — 메타서큘러 mirror 표면 비교: stage2/2-prime/3가 `stage2/compiler.hy`를 동일한
소스위치-무관 AST + 동일 Python 소스로 컴파일; factorial 결과 동일; 각 스테이지 로드 커널이
factorial/loop/feature/stability-stress 예제를 동일 AST/소스/값으로.

`stage7-check` — mirror 비교를 stage7까지 확장 + 모듈 오염 검사: stage2~7 모두 `self-check` 통과;
컴파일러 AST/생성 Python/factorial 결과 동일; 스테이지 모듈명 유일 + `sys.modules` 정합; probe
모듈의 macro/reader-macro 테이블·globals가 서로 구별(우발적 공유 상태 포착); stability-stress
예제로 helper 낮춤·local macro·comprehension·`try`/`with`/`let`/`match`/언패킹·반복 compile 검증.

`self-host-check` — 컴파일러 축 자기적용: stage7 컴파일러로 `stage2/kernel.hy`를 로드해 kernelA를
얻고, kernelA로 전체 `stage2/kernel.hy`를 컴파일해 kernelB로 실행, kernelB가 `self-check` + factorial
probe를 통과해야 한다. `stage2/compiler.hy` shim도 kernelB로 컴파일해 정규화 artifact 해시 보고.
(아직 B == C 고정점이나 reader 소유는 증명하지 않음 — 별도 체크리스트.)

`bootstrap-fixedpoint-check` — B == C 컴파일러 축 고정점 증명: stage7-로드 kernelA로 kernelB를,
kernelB로 kernelC를 컴파일·로드하고, B와 C가 낸 커널 + `stage2/compiler.hy` shim artifact가 정규화
AST/Python·canonical instruction payload·raw marshal·timestamp `.pyc` 해시에서 일치해야 한다.
kernelC는 `self-check` + factorial probe 통과 필요.

`no-fallback-check` — `stage2/compiler.hy`에 `DIRECT-KERNEL-STRICT`를 켜고 소유 코퍼스
(`stage2/compiler.hy`, `stage2/kernel.hy`, 커널 proof 예제)를 컴파일. direct-kernel 실패 시 upstream
`hy.compiler`로 폴백하지 않고 raise; 모든 항목이 direct-kernel hit이고 fallback 0일 때만 통과.

`parity-ledger-check` — 소유 코퍼스 + upstream `tests/native_tests/*.hy`에 대한 direct-kernel
hit/fallback 경계 측정. 파일/폼 단위 합계, native fallback 파일, direct 비율을 보고하고 `--debug-dir`에
per-file 레코드(`parity-ledger.json`)를 쓸 수 있다. upstream native test의 fallback은 측정만 하며
소유 컴파일러 커버리지로 인정하지 않고, 소유 코퍼스 fallback은 실패로 본다.

`stage8-check` — 한 번 더 fresh 메타서큘러 재로드 후 컴파일러/런타임 artifact 비교. (Hy에선 JVM
jar/class 비교가 아니라) artifact 표면 = 소스 → 소스위치-무관 AST → 생성 Python 소스 → canonical
instruction/code-object payload → marshal된 code object → timestamp `.pyc` 바이트. stage7 번들과
fresh stage8 번들을 비교한다. raw marshal/`.pyc` 해시는 진단용(3.11이 같은 필드를 다른 marshal
reference 바이트로 인코딩 가능)이고, `code_artifacts_match`가 instruction 수준 수용 게이트다.
`--debug-dir`로 `stage7/`·`stage8-fresh/`·`diff/` manifest를 남긴다.

`stage9-check` — 제품용 컴파일러 entrypoint를 고정 `PYTHONHASHSEED`·locale·timezone의 clean
subprocess에서 replay. manifest는 Python·설치된 Hy·route 정책·feature gate·repo root·소스 해시·
결정적 env를 바인딩. 각 subprocess가 stage2 `run`/`py`/`hy2py`/`hyc`, kernel `run`/`py`,
stage7-kernel `run`/`py`, expected-error 경계 fixture를 돌려 canonical JSON probe 반환. fresh probe
2회 + 대체-cwd replay를 비교하고 probe별 elapsed ms 보고. `--debug-dir`로 manifest·drift report·
각 probe stdout/stderr 기록.

`stage10-check` — 같은 기본 제품 요청을 직접 stage2 실행, versioned 로컬 서버 핸들러, 실제 HTTP
loopback 서버, 대체 작업디렉터리의 clean CLI subprocess, 격리·동시 세션 모듈, kernel import-hook
sandbox witness로 돌린다. 표현 텍스트가 아닌 canonical 결과 필드를 비교하고, 세션 macro/reader
테이블 구별·protocol downgrade·sandbox denial fixture(outside-root/zip/bytecode)·생성 Python/AST +
`.pyc` witness 해시(`--debug-dir`)를 기록.

`stage11-check` — 첫 multi-domain adapter 계약 고정. math/code/language/document/graphics/robot/
audio/open-problem adapter 레코드를 안정 스키마로 라우팅(code·graphics는 candidate, document는
evidence, robot은 human 확인 held, open problem은 proof-forbidden held, math/language/audio는 실제
route 생기기 전까지 unsupported held). incompatible candidate 충돌 처리, accepted/promotion/execution
강제 시도(악성 adapter), schema 마이그레이션/unsupported-schema, capability matrix 불변성도 기록.

`stage12-check` — stage11 held/gap 레코드를 self-improvement candidate로 만들고 격리 유지 증명.
candidate/quarantine-replay/owner-admission 레코드를 분리 기록, 어떤 candidate도 live truth를
변경하지 않음, 직접 promotion은 fail-closed, quarantine replay 후에도 stage11 상태 벡터 불변.

`stage13-check` — 장기 제품 organism 레인 시작. 과거 accepted 답을 현재 manifest/corpus/capability
바인딩에 replay, hard 바인딩 변경 시 stale 레코드를 held로 강등, user/session/project referent가
경계를 넘어 leak되지 않음, 무관한 제품 업데이트 후에도 stage12 quarantine candidate가 not-admitted
유지 확인.

`stage14-check` — Hy/Python용 host-neutral JSON export로 cross-host law 레인 시작. stage9 answer
plan·stage11 adapter 상태 벡터·stage12 quarantine replay 벡터·stage13 lineage/boundary 레코드를
export하고, primary와 fresh Hy/Python export를 answer-plan 해시로 비교. 아직 연결 안 된 외부
Clojure/제품 host는 drift가 아니라 held capability로 기록. `stage14-export`는 이후 `pnix-clj`/
`clj-meta` 비교용 JSON 레코드를 낸다.

`stage14-import` — 디스크의 peer JSON export를 읽어 로컬 Hy/Python export와 비교. `stage14-import-
check`는 현재·draft-v0 peer export를 생성하고, draft 형태를 현재 스키마로 마이그레이션한 뒤 둘이
answer-plan 해시로 일치(스키마 마이그레이션을 drift로 취급하지 않음)함을 검증. unsupported 스키마
입력은 replay 대신 `drift`로 fail-closed.

`stage15-check` — open-world evidence federation 레인 시작. 외부 proof artifact·solver 결과·code
patch·document claim·LLM 제안·user 파일·remote sandbox witness·graph backend 레코드를 evidence-only
입력으로 모델링. 외부 레코드가 직접 accepted truth가 될 수 없음, route/compiler/profile/rule 변경은
quarantine 경유, admission은 offline, stale-held revocation 기록, 모든 admission candidate를 gate
고려 전에 stage13 lineage 컨텍스트에 바인딩함을 증명.

`stage15-export` — 같은 open-world evidence 번들을 JSON으로 내보내, 향후 admission 서비스가 canonical
evidence/replay/admission/quarantine/revocation 레코드를 직접 소비하게 한다.

`stagen-check` — versioned stageN 레인 시작. stage15 evidence export에 앵커된 machine-readable
extension manifest 인덱스를 만들고, 각 post-stage15 extension이 closure target·artifact 표면·
바인딩·replay 전략·fail-closed 경계·마이그레이션 규칙·debug 계약·locality·timeout·cost note를
stage7/8/9/15를 약화시키지 않고 명시함을 증명.

`kernel-import-check` — Python `importlib`가 `.hy` 파일을 Hy 커널 경로로 로드하게 하는 임시 import
hook 설치. `kernel_import_probe.hy`를 직접, `kernel_import_consumer.hy`를 그 첫 모듈을 import하는
두 번째 모듈로(probe docstring 포함) import; `kernel_import_pkg/__init__.hy`(package `__path__`로
`child.hy`, 상대 import로 `sibling.hy`)도 import. loader 타입·모듈 캐시·package path·import 값·
`__file__`·모듈 globals 구별·macro/reader 테이블 격리·scoped import 후 hook 제거를 검증한다. 또한
일부러 실패하는 `.hy` 모듈을 import해 실패한 import/require/reader macro load/macro 확장이 stale
`sys.modules`·macro-table·reader-table 항목을 남기지 않음을 확인하고, 재로드가 같은 생성 출력을
내는지 검사.

## Delegation Boundary (위임 경계)

현재 stage7 레인은 `self-host-check`로 명시적 커널 자기적용을, `bootstrap-fixedpoint-check`로 B == C
artifact 고정점을 갖지만, 아직 reader까지 소유한 self-host는 아니다. 따라서 공개 분류는 "완전
메타서큘러"가 아니라 **self-hosting back-end**이며, reader와 name mangling은 검증된-순수 host
substrate로서 upstream `hy.reader`에 공식적으로 위임한다. `front-end-boundary-check`가 그 결정을
기록하고 `mangle`/`unmangle`이 결정적·왕복가능·부작용 없음을 증명한다. 경계는 명시적이다:

- `stage1/compiler.py` — Python seed. Hy로 쓴 stage2 프로토콜을 로드하기 위해 read와 일반 Hy
  컴파일을 upstream Hy에 위임.
- `stage2/compiler.hy` — Hy로 쓴 부트스트랩 프로토콜. 소스 컴파일러 경로는 지원 소스에 대해
  `stage2/kernel.hy`를 먼저 시도(자기 자신 `stage2/compiler.hy` 재로드 포함)하고, 커널이 폼을
  거부하거나 `import-stdlib`가 꺼졌을 때만 upstream `hy.compiler.hy_compile`로 폴백. `eval-source`는
  upstream `hy_eval` 대신 생성 AST를 재작성해 직접 실행.
- `stage2/kernel.hy` — 직접 Hy 컴파일러 커널. 위의 집중 서브셋을 소유하고, 그 대상 프로그램에 대해
  `hy.compiler.hy_compile` 없이 Python `ast`를 낸다.
- `stage7-kernel-run`/`stage7-kernel-py` — 지원 서브셋에 대해 최종 stage7 컴파일러 체인으로 그 직접
  커널을 로드한 뒤 평가/Python 출력.
- `hy.reader` — stage7의 reader 경계로 유지. `reader-boundary-check`가 stage7-로드 커널이 upstream
  `hy.reader.read_many`를 씀을 증명하며, reader macro는 격리·실패정리를 증명한 채 upstream reader/
  runtime에 의도적으로 위임.
- Python 런타임 서비스는 host 서비스로 유지: `ast`, `compile`, `exec`, `importlib`, `runpy`,
  `sys.modules`, Python 예외, descriptor, context-manager 프로토콜, async 프로토콜, 일반 Python
  객체 모델.
- kernel import hook은 scoped proof 인프라. 파일시스템 `.hy` 모듈을 `stage2/kernel.hy`로 로드하며,
  bytecode/autocompile·zipimport·전체 CLI·REPL·`hyc`·`hy2py`는 명시적으로 옮기기 전까지 이 직접-커널
  import 레인 밖.

현재 stage7 레인 미지원:

- 전체 upstream `hy.compiler.py`, `hy/core/result_macros.py`, `tests/compilers/test_ast.py`,
  `tests/compilers/test_compiler.py` parity. stage7 레인은 `native_subset_test.py`의 direct-kernel
  서브셋을 증명하며, 전체 컴파일러 parity는 별도 post-stage7 트랙.
- REPL 프롬프트, 대화형 `-i`, startup 파일, stdin 스트리밍, shebang 명령 동작, 출력 버퍼링.
- `hyc`, `hy2py`, bytecode/autocompile, zipimport, 디렉터리/zip 스크립트 직접 실행, 전체 `hy` 명령
  인자 의미.
- 부트스트랩이 Python 3.11에 고정된 동안 Python 3.12+/3.13+/3.14+ 동작.

stage 경계는 지금 의도적으로 얇다. post-stage7 작업은 `stage7-check`·`mirror-check`·`stage3-check`를
green으로 유지하면서 더 넓은 Hy native suite 호환성을 추구하고 Python `hy.compiler` 의존을 줄일 수 있다.
