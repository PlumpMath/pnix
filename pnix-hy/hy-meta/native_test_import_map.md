# Native Test Import Map

이 맵은 `hy-meta/native_subset_test.py` 및 관련 proof 명령에 표현된 upstream Hy
native-test 표면을 기록한다.

## 현재 커널 레인에 포팅됨

| Upstream file | Imported coverage |
| --- | --- |
| `tests/native_tests/comprehensions.hy` | Comprehension 종류, empty comprehensions, `for []`, `:do`/`:setv`, `:if`, `#*`/`#**`, `setx`, global/nonlocal 동작, 다차원 `for` break/continue, async comprehensions, `else` 있는 async `for`, 최종 `#*`용 generator send protocol. |
| `tests/native_tests/functions.hy` | Lambda list, 구조분해, annotation, named/anonymous async 함수 및 async generator, yield/yield-from, final generator return 값, return 동작, name/doc 메타데이터, 현재 커널 레인의 syntax error 커버. |
| `tests/native_tests/let.hy` | 순차/재바인딩/구조분해 let 바인딩, f-string, comprehensions, quasiquote, 예외, with, mutation, break/continue/yield/return, import, 중첩 함수/클래스, dotted target, default-argument 평가 횟수, nonlocal/global, 매크로 정의, body-binding 누수/은닉 커버. |
| `tests/native_tests/defclass.hy` | Bare class, 상속, attribute, function attribute, dynamic base, function leakage 없음, docstring, macro-expanded class body, body 부작용, metaclass `__prepare__`, `__init_subclass__` keyword 동작. |
| `tests/native_tests/decorators.hy` | Function/class decorator, stacked decorator, decorator/default/body 평가 순서, async function decorator. |
| `tests/native_tests/operators.hy` | 산술, bitwise, matrix multiply, 비교, identity, membership, boolean operator, operator shadowing, arity error, unpacked operand, augmented assignment grouping. |
| `tests/native_tests/dots.hy` | Attribute chain, method shortcut, keyword receiver, call/index chain, dot chain 통한 대입, multidot macro/function expansion, malformed dot syntax. |
| `tests/native_tests/setv.hy`, `setx.hy`, `del.hy`, `unpack.hy` | Assignment return, unpacking, `setx` scope, `setv :chain`, `del`, invalid target, target 평가 순서, call/literal unpacking, dfor mapping unpacking. |
| `tests/native_tests/match.hy` | Literal, capture, wildcard, value, OR, AS, sequence, mapping, class, keyword, dotted constructor, guard, failure, side-effect-order case. |
| `tests/native_tests/model_patterns.hy` | `do-until`, `loop`, whole/skip parser용 macro model-pattern 파싱. |
| `tests/native_tests/strings.hy` | Docstring, module docstring, Unicode string 길이, f-string conversion/spec/debug/평가 순서, bracket f-string, delayed quoted f-string, string model bracket 메타데이터, f-string repr 왕복. |
| `tests/native_tests/hy_repr.hy` | `hy.repr` 왕복, non-round-trip model promotion, dict view, datetime/date, collections, model constructor, self reference, custom repr registration, placeholder, fallback repr. |
| `tests/native_tests/mangling.hy` | Hyphen/underscore 동작, special form, operator, keyword, Unicode, non-ASCII mangle/unmangle, PEP 3131/NFKC 정규화 case. |
| `tests/test_models.py` | 커널 레인 관련 symbol/keyword/model 구성 동작, model equality, list 연산, invalid bracket model 구성, recursive model 탐지, repr/eval 동작. |
| `tests/native_tests/hy_eval.hy` | `hy.eval` core 값, globals/locals/module 조합, eval 중 매크로, extra macro table, 명시 filename, failure cleanup, namespace cleanup. |
| `tests/native_tests/hy_misc.hy` | `hy.gensym`, `hy.read`, `hy.read-many`, `hy.I`, `hy.R`, macroexpand, macroexpand-1 동작. |
| `tests/native_tests/hy_inspect.hy` | 현재 source-location 입도에 적용 가능한 module source, loader source, comment, docstring, source file, pydoc rendering. |
| `tests/compilers/test_ast.py` | Direct-kernel 레인 집중 compile-success/failure 커버: control form, declaration, class/function syntax, imports/requires, dot chain, dot unpacking 거부, placeholder special-form 거부, `pragma` 검증, reader-timed `:bracketed-templates`, inline `py`/`pys`, module prelude opt-out, f-string conversion, future-import 순서. |
| `tests/compilers/test_compiler.py` | Bare-name branch 보존, generator final-return AST 형태, compact short-circuit lowering 및 nested logic 보존용 boolop generated-Python 형태 패리티. |
| `tests/test_positions.py` | Direct-kernel top-level/generated statement root 집중 source-position 커버, macro-expanded statement form 포함. |
| Supported direct-kernel syntax errors | Direct-kernel 소유 syntax 경계의 대표 compile-error 메시지 텍스트를 집중 검사로 고정. |

## 부분 포팅

| Upstream file | Current status |
| --- | --- |
| `tests/native_tests/macros.hy`, `macros_first_class.hy`, `macros_local.hy`, `reader_macros.hy` | 매크로 정의, phase 동작, first-class 매크로, local/global macro table, reader macro 위임, failure cleanup, core-shadow 경고 커버. 광범위 upstream 매크로 툴링은 upstream Hy에 위임. |
| `tests/native_tests/import.hy` | Direct import, require, relative import/require, `hy.I`, `hy.R`, import hook module cache 동작, failed import cleanup, star export, runpy, reload 커버. Full CLI import 동작은 이 레인 밖. |
| `tests/native_tests/with.hy`, `try.hy`, `conditional.hy`, `break_continue.hy`, `nonlocal.hy`, `do.hy` | 커널 레인이 쓰는 native statement/expression lowering case 커버. 일반 upstream 컴파일러 동작만 행사하는 나머지 upstream 테스트는 여기서 blocker로 추적하지 않음. |

## 버전으로 스킵

| Upstream file | Reason |
| --- | --- |
| `tests/native_tests/tstrings.hy` | Python 3.14+ template string은 `TemplateStr`/`Interpolation` lowering 구현 전까지 현재 direct-kernel 레인 범위 밖. |
| `functions.hy`, `defclass.hy`, `deftype.hy`의 Python 3.12+ `:tp` case | Type-parameter syntax는 커널 `SyntaxError`로 명시 gate; 3.12/3.13 지원은 활성 3.11/3.14 proof 타겟 밖. |

## Upstream 툴링으로 스킵

| Upstream file | Reason |
| --- | --- |
| `tests/native_tests/repl.hy` | REPL 동작은 direct-kernel 소유 밖. |
| `tests/test_bin.py` | Full `hy`, stdin, `-c`, interactive mode, shebang, `hyc`, `hy2py`, startup file, output buffering, full CLI traceback rendering은 direct-kernel 소유 밖. 소유 `hy-meta/bootstrap.py run`, `py`, `kernel-run`, `kernel-py` 명령은 smoke로 커버. |

## 여전히 실패

현재 native-test import-map 항목 중 의도적으로 실패로 추적하는 것은 없다.
새 실패는 exact upstream file/function과 아직 포팅되지 않은 이유와 함께 여기
추가해야 한다.
