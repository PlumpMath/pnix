# hy-meta 상태 (peer host-meta floor)

최종 검증: 2026-08-07.

## Peer-floor 성명

**hy-meta**는 `pnix-hy`의 Hy/Python host-meta bootstrap이다. host meta 중
가장 깊은 명시 stage ladder를 소유한다(stage1 → stage7 컴파일러 체인,
self-host kernel fixed point, stage 8–15/N product/organism seed).

| Peer | Peer floor | hy-meta 대응 |
|---|---|---|
| clj-meta | stage7 stock + bytecode selfhost | stage7-check + bootstrap-fixedpoint-check |
| rs-meta | TV + stage chain toward 15-N | stage ladder + parity ledger (kernel vs native) |
| cljs-meta | fixed-point compiler | bootstrap-fixedpoint-check (B==C kernel artifacts) |
| clr-meta | eval gen0–2 + C0–C3 | stage chain + evaluator/kernel path |

**정직한 분류:** self-hosting **back-end**(직접 kernel → Python AST),
reader의 완전 meta-circular 소유가 아님. `hy.reader`와 name mangling은
위임된 호스트 substrate로 남는다. Full upstream `hy.compiler` 패리티는
post-stage7 트랙이며 닫힌 주장이 아니다.

Python proof 타겟: **3.11**과 Homebrew **3.14**만 (3.12/3.13 거부).

## 닫힌 주장

이 세션 라이브 검증(2026-08-07) `./hy-meta/bin/hy-meta-gate primary` 경유:

```text
self-check                         PASS (stage1=6, stage2=42, stage2_self_check=True)
stage7-check                       PASS
  stage_count=7, all_stage_self_checks=True
  compiler/kernel AST+Python+value stage7 mirrors=True
  isolation (modules/macros/globals) ok
  kernel_factorial=120, kernel_loop=120, kernel_features=449.0
```

bootstrap 명령으로 문서상 닫힘(이 세션 재실행 안 함):

```text
chain-check / kernel-check / prime-check / stage3-check / mirror-check
self-host-check / bootstrap-fixedpoint-check / no-fallback-check
parity-ledger-check / stage8..stage15 / stagen seeds
reader-boundary-check / kernel-import-check / native-subset-test
```

## 열린 주장 (주장하지 말 것)

```text
full_reader_ownership = false
complete_upstream_hy_compiler_parity = false
full_REPL/hyc/hy2py/zipimport_product_surface = false
Python_3.12_or_3.13_support = false
trusting-trust_defense = false
pnix_language_semantics_ownership = false
```

Stage15/N 체크는 **로컬 product/organism seed**이며 Hy/CPython 대체가 아니다.

## Trusting-Trust 방어 로드맵 (Diverse Double-Compiling)

**hy-meta는 이 축에서 다섯 호스트 중 가장 앞서 있다.** 위의
`trusting-trust_defense = false` 줄은 *완전한* 바에 대한 것이며 진전 0이
아니다 — 실제 DDC 스타일 게이트가 이미 닫혀 있다. `todo.md` 2026-06-29
Deep-Research Audit에서:

```text
diverse-double-compile-check (CLOSED, wired into smoke)
    builds kernel.hy two independent ways:
      kernel_upstream = via upstream hy.compiler (stage1 seed)
      kernel_direct   = via the direct kernel (stage2 bridge, confirmed by a
                         nonzero direct-kernel hit count, i.e. it actually ran
                         through the new path, not a silent passthrough)
    both then compile kernel.hy and compiler.hy. Outputs agree at all four
    levels: normalized AST, canonical code, raw marshal, timestamped pyc.
    A backdoor present in the direct build path but absent from upstream would
    have produced a divergence here; none was found (green on 3.11 + 3.14).
```

**아직 "full" Trusting-Trust 방어가 아닌 이유:** 두 빌드 경로 중 하나
(`kernel_upstream`)는 여전히 신뢰된 실제 upstream `hy.compiler`를 seed로
거친다 — 독립 작성 제3 구현이 아니다. 이 검사는 *direct-kernel* 경로에만
도입된 백도어를 잡는다; upstream `hy.compiler` 자체에 이미 있는 백도어는
잡지 못한다 — 비교 대상 둘 중 하나가 그 동일 upstream 컴파일러이기
때문이다. Wheeler 바는 두 번째 컴파일러가 첫 번째와 공유 저작/계보가
없기를 요구한다.

**이 세션에 추가된 독립 mini backend (2026-08-11):**
`independent_mini_backend.py`는 from-scratch Hy-subset-to-Python-AST
컴파일러 — 자체 hand-written tokenizer/reader + 직접 `ast` 노드 구성,
`hy.reader`, `hy.compiler`, `stage1/compiler.py`, `stage2/kernel.hy`와
코드 공유 제로. Python `ast` 모듈과 `compile()` builtin은 신뢰 호스트
substrate로 남는다(clj-meta 유사 `frontend_selfhost.clj`에 대한 JVM
classfile 형식과 같은 정직한 역할). *별도* 체크(`independent-mini-backend-check`,
`smoke_test.py`에서 `diverse-double-compile-check` 직후)로 연결, 그 검사의
literal 세 번째 다리가 아님: 기존 DDC 검사는 전체 파일
`kernel.hy`/`compiler.hy` bytecode-artifact 해시를 비교하며, 제한된 tiny
backend는 의미 있게 참여할 수 없다 — clj-meta tiny frontend가 whole-file
DDC 비교에 병합되지 않고 자체 `independent-mini-backend-subset` 행을 갖는
것과 같은 이유.

8 fixtures 커버(산술, 비교, `if`, `defn`, factorial 경유 recursion,
boolean/`None`-equality branching), 각각 실제 upstream Hy
(`stage1.compiler.eval_source`)를 호스트 참조로 검사. 지원 인터프리터 둘
모두에서 라이브 검증: `independent-mini-backend-check` → Python 3.11.15와
3.14에서 8/8 accepted, 양쪽 `stage7-check` 재실행 영향 없음(새 import 회귀
없음).

**진짜 3-way per-fixture 비교로 확대 (2026-08-11, 이 세션 후반):** 8
fixture 각각을 *세* 독립 계보 평가기에 대해 검사 — `host_result`(upstream Hy
1.3.0, `stage1.compiler.eval_source`, `diverse-double-compile-check`가
`kernel_upstream`이라 부르는 다리), `kernel_direct_result`(kernel.hy를
direct-kernel bridge로 컴파일·실행, `stage2.load_hy_file(KERNEL_PATH, ...)`,
그 검사가 `kernel_direct`라 부르는 다리), `mini_backend_result`(독립 mini
backend). `diverse-double-compile-check`는 이미 전체 파일
kernel.hy/compiler.hy bytecode-artifact 수준에서 upstream vs direct-kernel을
비교; 여기에 작은 fixture *행동* 입도의 세 번째, 코드 독립 다리를 추가해
이전에 문서가 지적한 "아직 `kernel_direct`를 formal third leg로 교차 검증하지
않음" 갭을 닫음. 라이브 검증: `independent-mini-backend-check` → 세 다리
모두 일치 8/8 accepted, `diverse-double-compile-check` 여전히
`ddc_status: reproduced`, full `hy-meta-gate full` ladder
(self-check, chain-check, stage7-check, self-host-check,
bootstrap-fixedpoint-check) 여전히 green — `kernel_direct` 다리 추가 회귀
없음.

**이것으로 닫히는 것과 아직 아닌 것:** upstream Hy *와* direct kernel *둘
다*에 있는 백도어(예: 이전 bootstrap 단계에서 direct-kernel 빌드에 상속)도
mini backend가 어느 쪽과도 코드·툴링·bootstrap 계보를 공유하지 않으므로
잡힌다. 여전히 14 fixtures일 뿐 conformance corpus가 아니며 — clj-meta·
cljs-meta가 합의한 동일 정직한 바 — behavior equivalence이지 bit-identical
artifact가 아니다. **다음 구체 단계:** fixture 세트 계속 성장(더 많은
seq/dict ops, 매크로 커버)해 clj-meta 측 `frontend_selfhost.clj` ~50-fixture
범위에 접근.

**당일 추가 확대 (2026-08-12):** string 리터럴, list 리터럴 반환값,
`setv`/`while` mutation 추가(8→12 fixtures). `independent_mini_backend.py`의
`_emit_defn`은 이전에는 *마지막* body form만 emit(return으로 감쌈)하고 그 전
폼을 조용히 버림 — 기존 pure-expression fixture에는 괜찮았으나
`setv`/`while`(최종 식이 아니라 부작용 문으로만 의미 있음)은 전혀 실행
불가. 새 `_emit_stmt`가 `(setv name value)`를 실제 `ast.Assign`으로,
`(while test body...)`를 실제 `ast.While`로 바꾸고, `_emit_defn`은 마지막
제외 모든 body form을 거치게 함. fixture 추가 전 양쪽 실제 다리 대비 검증
(가정 아님): `bootstrap.py run -c`(upstream)과 `bootstrap.py kernel-run -c`
(direct kernel) 모두 0..9 summing while-loop(45)와 setv-then-arithmetic(41),
bare string·list 리터럴에서 mini backend와 일치. 라이브 검증:
`independent-mini-backend-check` → 세 다리 일치 12/12 accepted,
`diverse-double-compile-check` 여전히 `reproduced`, full `hy-meta-gate full`
ladder 여전히 green — 회귀 없음.

**다시 확대, 2026-08-13:** dict 리터럴 추가(string 키만 — Hy keyword 리터럴은
실제 호스트에서 `hy.models.Keyword` 객체로 읽히며, 이 from-scratch backend가
의도적으로 재현하지 않는 reader-model 정체성이라 keyword-keyed dict는 범위
밖) 및 multi-`defn`-composition fixture(한 top-level `defn`이 다른 것을 호출;
기존 `compile_and_eval` 루프가 이미 지원, 코드 변경 없음 — 각 `defn`이 실제
module-level `FunctionDef`가 되어 이후 것이 공유 `exec()` namespace로 이전
것을 이름으로 호출; 가정 아닌 검증). 새 tokenizer/reader 지원: `{`/`}`
tokenize, `_parse_one`이 `("__dict__", pairs)` marker form 구축, 일반
call-form dispatch 앞 새 `_is_dict` 검사로 emit(`ast.Constant` string 키
`ast.Dict`). fixture 추가 전 양쪽 실제 다리 대비 검증(8→12→14 total). 라이브
검증: `independent-mini-backend-check` → 세 다리 일치 14/14 accepted,
`diverse-double-compile-check` 여전히 `reproduced`, full `hy-meta-gate full`
ladder 여전히 green — 회귀 없음.

**이 세션 수정: 누락 native-corpus 의존성.** fresh checkout/venv는 예전에
`diverse-double-compile-check` 및 기타 native-corpus 의존 체크를
`hy.errors.HyRequireError: No module named 'tests'`로 실패시킴(수정 없는
`bootstrap.py`에서도 확인 — 회귀가 아니라 진짜 갭). 원인: `tests/`(upstream
Hy 자체 `tests/native_tests/*.hy` + `tests/resources/tlib.hy`, native-Hy
oracle)가 이 checkout에 물질화되지 않음 — 경로로만 참조. 수정:
`hy-meta/bin/fetch-native-tests`가 이미 핀된 `hy-src` flake input
(`github:hylang/hy` 태그 `1.3.1`, `flake.lock` 해시 검증)을 Nix로 resolve해
`tests/` subtree를 `pnix-hy/tests/`로 복사(gitignored, ~528K, 95 files —
커밋 안 함). `nix develop` shellHook이 이제 자동 수행. `tests/` 존재 후 더
작은 갭: `tests/resources/__init__.py`가 모듈 로드 시 `pytest` import
(upstream Hy 자체 test-resource 파일, hy-meta가 호출하는 것이 아님) →
`pytest`를 `flake.nix` `proofPython`에 추가, manual venv에도 필요. 이 세션
라이브 검증: `diverse-double-compile-check` → `ddc_status: reproduced`(이전엔
실행 자체가 불가), `native-subset-check` → `ok`,
`parity-ledger-check` → 100% direct (45/45 files, 1487/1487 forms, 0
fallbacks), `hy-meta-gate full` → PASS.

## 기본 게이트

```sh
# From pnix-hy/
./hy-meta/bin/hy-meta-gate              # self-check + stage7-check
./hy-meta/bin/hy-meta-gate self-check
./hy-meta/bin/hy-meta-gate full         # + self-host + fixedpoint subset
```

이 세션 사용 env:

```sh
/usr/local/bin/python3.11 -m venv /tmp/pnix-hy-py311-venv
/tmp/pnix-hy-py311-venv/bin/python -m pip install 'funcparserlib ~= 1.0' 'hy == 1.3.1'
export HY_META_PYTHON=/tmp/pnix-hy-py311-venv/bin/python

# Only needed for diverse-double-compile-check / parity-ledger-check /
# native-subset-test (the native-Hy-corpus checks):
./hy-meta/bin/fetch-native-tests                          # materializes tests/
/tmp/pnix-hy-py311-venv/bin/python -m pip install pytest   # tests/resources/__init__.py needs it
```

## 마지막 실행 (이 머신, 2026-08-07)

| Gate | Result | Notes |
|---|---|---|
| `hy-meta-gate primary` | **PASS** | Python 3.11.15 + hy 1.3.0 + funcparserlib |
| full ladder stage8–stagen | not default-run | available via bootstrap.py |
