# clj-meta 상태 (peer host-meta 하한)

최종 검증: 2026-08-07.

## Peer-floor 선언

**clj-meta**는 `pnix-clj`용 JVM/Clojure host-meta 기판입니다. 다른 meta 대비
실무 peer 하한:

| Peer | Peer 하한 | clj-meta 대응 |
|---|---|---|
| hy-meta | stage ladder / fixed-point 검사 | `:gate` 바이트코드 selfhost + stock stage7 재빌드 |
| rs-meta | TV + multi-stage selfhost | 컴파일러 conformance + self-emit fixed point |
| cljs-meta | fixed-point 컴파일러 (stage2==stage3) | backend self-emit determinism / stage1→7 selfhost 체인 |
| clr-meta | eval gen0–2 + C0–C3 Stage1/2 | kernel + full-eval tower + 컴파일러 레인 |

**서로 다른** 두 레인 (혼동 금지):

1. **바이트코드 meta 컴파일러** (`src/pnix/clj_meta/{compiler,selfhost,gate}.clj`) —
   analyzer/ASM emit + 결정적 self-host 검사. 주 제품 하한.
2. **Stock stage7 재빌드** (`stage7-gate.sh`) — Maven/Ant를 통한 Clojure 1.12.5
   호스팅 결정적 재빌드. 재현 가능 빌드 증거이며, 메타순환 컴파일러 증명이 아님.

어느 레인도 JVM 없는 Clojure self-hosting을 주장하지 않습니다. Reader,
`clojure.core`, JVM은 영구 기판으로 남습니다.

## 닫힌 주장

이 세션에서 라이브 검증됨 (2026-08-07):

```text
./bin/clj-meta-gate selfhost              PASS  ready=true
./bin/clj-meta-gate stage7                PASS  stage7-check OK (Maven 3.9.12)
./bin/clj-meta-gate primary (:gate)       READY ✅
  stage11 multisurface                    OK
  stage12 quarantine                      OK
  stage13 long-horizon                    OK
  stage14 crosshost                       OK  (missing external transcripts held)
  stage15 openworld                       OK
  stageN recursive                        OK
  full-source stage1                      OK  (M12 fallback-free accepted)
  lowering admission                      OK
  diverse double compile                  OK
  reproducible DDC lane                   OK
  (plus prior OK witnesses: bytecode/TV/verifier/language-surface/…)
```

### stage11–N을 닫은 수정 (이번 웨이브)

1. **stage9/10 child classpath** — 자식이 pnix-clj 루트에서
   `clojure -M:audit-self-source`를 호출했음(루트 `deps.edn` 없음). 이제
   primary 게이트와 같은 형태로 절대 `clj-meta/src`를 쓰는 `-Sdeps`를 사용.
2. **stage10 sandbox cwd** — `source-path`가 이제 `CLJ_META_ROOT` 아래에서
   해석됨(`clj-meta/src/...` 우선). sandbox 재배치 시 compiler.clj를 찾음.
3. **lowering-admission** — m6aj `checked-fallback` 허용 행
   (`promotion/allowed?=false`)을 raw-bytecode 허가가 아니라 held 경계로 매핑.
4. **stage14** — 누락된 외부 transcript와 synthetic drift sentinel을
   `:unavailable`/`:rejected`가 아니라 `:held`로 처리(docstring + invariant 정합).

설계상 닫힌 것으로 문서화:

```text
boundary policy: direct emit uses host Compiler 0 times; fallback explicit
M12 fallback-free genuine stage1 boundary: ACCEPTED (host-Compiler-fallback-forms=0)
```

## 열린 / 주장하지 않는 것

```text
full-language-correctness                 false
trusting-trust / Wheeler independent DDC  false
JVM-free self-hosting                     false
external stage14 transcripts (hy/pnix-hy/pnix-clj files)  optional held evidence
```

Stage11–N은 정직한 held 경계(누락된 cross-host transcript, checked-fallback
lowering)를 가진 **로컬 제품/organism 클로저 seed**이며, Clojure
언어-런타임 대체가 아닙니다.

## Trusting-Trust 방어 로드맵 (Diverse Double-Compiling)

**현재 상태는 "false" 한 단어가 암시하는 것보다 강합니다.** 위의 "diverse double
compile OK" / "reproducible DDC lane OK" 행은 실제 통과 게이트이며 희망 사항이
아닙니다. 이미 닫힌 구체적 조각이 세 개 있습니다(`todo.md` U5/U6/U8/U10 전체
상세와 2026-06-29 receipt 참고):

```text
U5  independent-kernel-evaluator-supported-corpus
    kernel.clj (tree-walking value-semantics evaluator) cross-checked against
    compiler.clj on the full 112-case conformance corpus: host≡compiler≡kernel,
    0 unsupported. Honest scope: shares host clojure.core, not a second
    bytecode compiler, no independent deftype/defrecord typegen.
U6  frontend-selfhost
    a self-authored tiny reader + tiny analyzer + direct ASM emitter, sharing
    no recognizer/range-engine/emit-helper code with compiler.clj. Compiles
    212 fixtures (fn/if/do/let/loop-recur/`deftype` implementing
    protocols/interfaces with method bodies -- confirmed via `javap
    -p -c` that a declared field is read INSIDE a method body via
    exactly the same `this.fieldName` shape closure/`reify` captures
    already use, just unconditionally present (no free-variable
    analysis needed: `deftype` has no enclosing lexical scope to
    capture from at all), reusing `analyze-reify-method` unchanged
    with the field list standing in for an enclosing scope/`defprotocol`+protocol method
    dispatch -- a leading top-level form (alongside `deftype`) that
    generates a plain abstract interface (one method per protocol
    method, all Object-typed -- no pre-existing type to reflect
    against, unlike `reify`); a protocol method call `(methodName
    instance args...)` compiles as a fixed special form to the exact
    FAST-PATH shape real host itself uses when it can prove direct
    interface implementation (`checkcast Interface; invokeinterface`),
    not a first-class Var-bound dispatch function -- real host's full
    `MethodImplCache`/`-cache-protocol-fn` extend-type fallback
    machinery deliberately not reproduced; `reify`'s interface
    resolution extended to check protocols defined earlier in the same
    program before falling back to `Class/forName`, so `(reify
    ProtocolName ...)` implements a protocol directly/`deftype` (fields only, no
    protocol/interface method implementations) via a NEW top-level
    program shape `(do (deftype Name [field...])... (fn ...))` --
    the only way `deftype` can appear at all, since it defines a
    NAMED class tied to the whole compile unit rather than an inline
    expression; a dynamic var threads the leading `deftype`s'
    `{name Class}` registry into the trailing `fn`'s analysis so
    `(Name. args...)` there constructs directly (`NEW`/`DUP`/
    `INVOKESPECIAL`, matching real host's own compile-time-known-type
    shape exactly, not the Reflector-based general-construction path);
    field access already worked for free via the existing general
    `.-fieldName` reflection mechanism; confirmed real host itself
    can't compile this exact do-wrapped single-string shape either
    (same "class not yet defined" issue), so these stay U6-only, not
    wired into the DDC row/`reify` -- one fully-qualified
    interface, resolved via `Class/forName` at analyze time to reflect
    its methods and match each `reify` method form by name+arity;
    reference-typed params only (a primitive param would need
    auto-boxing on entry, not attempted), but a primitive RETURN type
    (e.g. `Comparator/compare`'s `int`) IS coerced via unbox-at-return
    (deliberately not reproducing real host's always-added
    `clojure.lang.IObj` meta boilerplate, orthogonal to the reified
    interface's own behavior); reuses the closure capture mechanism
    (fields+constructor) built for nested closures, `this` bound the
    same way a named `fn`'s self-reference already is/`letfn`
    (non-mutually-recursive
    bindings only -- desugars entirely to nested `let`+self-named-`fn`,
    confirmed via `javap -c` to be real host's own shape for this case,
    reusing existing closure/self-recursion machinery with zero new
    bytecode; sibling mutual recursion rejected with a clear
    macro-expansion-time error, not attempted here)/true nested closures -- a `fn`
    literal appearing inside another `fn`'s body, capturing free
    variables from the enclosing scope as instance fields on a
    recursively-emitted inner class (one nesting level, single arity
    clause; both scoped narrower on purpose) -- plus, found while
    testing closures against real `clojure.core/filter`, a real
    pre-existing bug FIXED across every boolean-producing op in this
    file: `GeneratorAdapter/box` on a boolean built a non-singleton
    `new Boolean(z)`, invisible to this witness's own lenient `if`
    (`RT.booleanCast`) but silently always-truthy to real host's own
    identity-based compiled `if`, now `GeneratorAdapter/valueOf`
    everywhere/`catch` of any fully-qualified
    Throwable subclass, not just the small original allowlist (resolved
    via `Class/forName` at analyze time -- no runtime bytecode call
    needed, only the internal-name string), incl. `clojure.lang.ExceptionInfo`
    (what `ex-info` actually throws)/`recur` at a `fn` body's own
    tail position with no enclosing `loop` (real stack-safe tail
    recursion, distinct from named self-recursion which does a real
    `IFn.invoke`; targets the method's own argument slots via the same
    GOTO-loop mechanism `loop`/`recur` already implements)/computed call heads (`((f x)
    99)`, cast-and-invoke whatever the head expression itself produces,
    no Var/local lookup) incl. keyword-as-fn (`(:a m)`) for free/named
    self-recursive `fn`
    (`(fn name [x] ...)`, a same-shape `this`-load self-reference, real
    host bytecode exactly)/variadic `+`/`-`/`*` (0/1/2/N
    args, left-folded exactly as real host does)/unary `-`/chained `<`/
    `=`/`>`/`>=`/`<=` for arities other than 2 (falls back to the same
    general Var-call mechanism real host itself uses there, confirmed
    via `javap -c` -- not a folded/desugared chain)/3-arg `get` with a
    default value (`RT.get(Object,Object,Object)`)/arithmetic/compare/
    data-literals
    incl. `N`/`M` bignum literals (real `clojure.lang.BigInt`/
    `java.math.BigDecimal`, composing with the existing `+`/`-`/`*`/`=`
    ops), regex literals (`#"..."` -> `java.util.regex.Pattern`), and
    ratio literals (`1/3` -> `clojure.lang.Ratio`, reduced/collapsed via
    `Numbers/divide` at parse time)/`binding`+dynamic-var deref (real
    `push-thread-bindings`/`pop-thread-bindings`, exact host bytecode
    shape)/quote/14 macros incl. `case` with a real no-default throw/vector-
    destructuring/fixed multi-arity fn mixed with one variadic `&`
    rest-args ceiling (real `RestFn` `invoke(N)`+`doInvoke`+
    `getRequiredArity` overrides, exact host bytecode shape)/any
    `clojure.core` fn as a general call head or first-class value (`map`/
    `filter`/`reduce`/`apply`/`conj`/`assoc`/`vec`/`into`/... -- the `str`
    mechanism generalized, gated by a real `ns-resolve` existence check so
    an unknown symbol still fails at analyze time like real host)/a local
    (fn param or `let` binding) called as a fn, e.g. higher-order params
    passed into `map`/count/
    `try` with any combination of (single- or multi-)`catch` and
    `finally`/`throw`/general class construction and general static
    interop (allowlisted classes via direct bytecode, any other
    fully-qualified class via `RT.classForName`+`Reflector`)/`locking`/
    `.methodName` instance interop/`.-fieldName` field access and
    `set!` (all via `clojure.lang.Reflector`)/
    `str` (via `clojure.lang.Var` + `IFn.invoke`, the same mechanism real
    host uses for any ordinary function call))
    with ZERO calls into tools.analyzer.jvm or the host reader.
U8  fuzz-conformance
    10,000 random-program comparisons (250 programs x 40 inputs), host≡compiler,
    0 divergences found.
```

**`independent-mini-backend-subset` DDC 행 (이 세션 검증, 2026-08-11):**
`diverse_double_compile.clj`의 이 행은 이미 실제 3-way 비교(host `eval` ≡
`compiler.clj` backend ≡ U6의 독립 mini backend)를 실행합니다 — 문서만 있는
것이 아니라 라이브 통과 게이트입니다. 처음에 U6의 51 fixture 중 14개만
연결되어 arithmetic/let/loop/threading-macros/cond/if-let/comparisons/quot-rem/
unary-ops/seq-ops/get/nested-destructure를 커버했습니다. 이 세션에서
`frontend_selfhost.clj`에 이미 존재하고 단독 통과했던 미연결 범주를 추가해
**43 fixture**로 키웠습니다: `do`, let-shadowing, boolean/nil/equality
branching, 네 가지 data literal(vector/string+keyword/map/set), 세 가지 quote
form, 남은 13개 macro
(`when`/`and`/`or`/`not`/`nil?`/`when-let`/`if-not`/`as->`/`cond->`/`cond->>`/
`some->`/`some->-nil`/`some->>`), plain 및 rest-position destructuring,
`zero?`/`neg?`. `-M:ddc` 재실행: `independent-mini-backend-subset -> accepted`
(43/43 일치), 전체 `diverse-double-compile: OK`, 다른 15행 회귀 없음. Receipt
digest: `4688b206f7cd9c22beb0f3bbc4ae5a69d61fcdb01d806726ef24125f3827838c`.

**다시 확장, 2026-08-13: fixed multi-arity `fn` 지원.** `analyze-fn`은 이제
`(fn [x] ...)`(단일 arity, 변경 없음)와 `(fn ([x] ...) ([x y] ...))`(한 함수에
여러 *fixed* arity)를 모두 처리합니다. AST를 arity 절 리스트로 일반화하고,
`emit-class`는 같은 `AFunction` 서브클래스에 절마다 ASM `invoke` 메서드를
방출합니다 — 실제 호스트 컴파일러와 같은 메커니즘입니다(각 `invoke0..invoke20`
override는 `IFn`의 일반 인자 개수 기반 호출 해석으로 독립 dispatch되므로 호출
측에 glue 코드가 필요 없음). Variadic `&` rest-args는 시도하지 않음 —
`clojure.lang.RestFn`이 필요하며, 자체 arity-dispatch/rest-collection 계약을
가진 실질적으로 다른 base class라 별도·더 큰 기능입니다. 추가 전 실제 호스트
`eval`에 대해 검증(2-arity·3-arity 케이스, 그리고 일치하지 않는 arity 호출이
양쪽 모두 `ArityException`을 던져 호스트 동작과 정확히 일치) — ASM 코드 읽기만
으로 가정하지 않음. U6: 51→55 fixture (`frontend_selfhost.clj` 단독 검사,
`-M:frontend-selfhost`: 55 전부 수락). DDC 행: 43→47 fixture (같은 4 multi-arity
케이스를 `diverse_double_compile.clj`의 `mini-backend-ddc-fixtures`에 추가, 라이브
3-way host≡compiler≡mini-backend 검사 — `independent-mini-backend-subset ->
accepted`, 47/47 일치). 전체 `-M:conformance`(116/116, 영향 없음 — 이 레인은
`compiler.clj`/`kernel.clj`를 건드리지 않음)와 전체 `bin/clj-meta-gate`
(`metacircular gate: READY`) 모두 녹색, 회귀 없음.

**같은 날 다시 확장 (2026-08-13): variadic `&` rest-args.** 위에서 의도적으로
미룬 "별도, 더 큰 슬라이스"를 이제 완료. `clojure.lang.RestFn`의 정확한 계약은
추측이 아니라 실제 호스트에서 역공학: 신뢰 호스트 컴파일러로 AOT 컴파일한
`(fn [a & r] r)`, `(fn [& r] r)`, `(fn [a b & r] r)`를 `javap -c`로 덤프.
발견: `RestFn`은 모든 public `invoke(...)` overload(arity 0–20 및 진 가변
20+ overload)를 *구체적으로* 이미 구현 — 인자 개수 매칭과 rest-sequence
수집은 전부 base class 책임. 서브클래스는 정확히 두 가지만 제공:
`getRequiredArity()`(고정 인자 개수)와 파라미터 개수가 `fixed-arg-count + 1`인
`doInvoke` overload 하나(마지막 슬롯이 수집된 rest sequence, `ISeq` 또는 추가
인자 없으면 `nil`). `emit-class`는 이제 어떤 arity 절에 `rest-param`이 있는지에
따라 분기: 있으면 클래스가 `AFunction` 대신 `RestFn`을 확장하고 그
`doInvoke`/`getRequiredArity` 쌍만 방출; 없으면 위 multi-arity 작업의 기존
per-clause `invoke` 경로 유지. 범위 의도적 축소: variadic 절을 같은 `fn`의
다른 fixed arity와 섞지 않음(실제 `RestFn` 서브클래스는 가능하나 위 두 조각
이상의 하위 arity `invoke` override가 필요 — 추가 슬라이스, 여기서 시도하지
않음; `analyze-fn`은 이 형태에 대해 조용한 오컴파일 대신 명확한 오류를 throw).
또한 새 unary op으로 `count` 추가(`RT.count`, `Integer/valueOf`로 박싱 —
같은 `javap -c` 역공학으로 실제 `count`가 이 파일의 다른 모든 numeric op처럼
`Long`이 아니라 `Integer`로 박싱됨을 확인, 가정하면 틀렸을 것). 추가 전 실제
호스트 `eval`에 대해 검증: 3 인자 및 정확히 1 인자(rest = `nil`)의
`(fn [a & r] r)`, `(fn [& r] (count r))`, `(fn [a b & r] [a b r])`, 일치하지
않는 0-arg 호출이 양쪽 `ArityException`, variadic+fixed 혼합이 올바르게 거부됨.
U6: 55→61 fixture, 전부 수락. DDC 행: 47→50 fixture (3개 새 variadic 케이스를
`independent-mini-backend-subset`에 연결, 여전히 수락). 전체 `-M:conformance`
(116/116, 영향 없음)와 전체 `bin/clj-meta-gate`(`metacircular gate: READY`) 모두
녹색.

**같은 날 세 번째 확장 (2026-08-13): `case`.** 기존 `cond`/`when`/threading
macro 옆의 macro 확장(`expand-case`)으로 추가 — `let` 바인딩 테스트 값 + 중첩
`if`/`=` 체인. 새 ASM/바이트코드 작업 불필요, 이미 있는 `if`/`=` 기계 재사용.
실제 `case`는 해시 기반 O(1) dispatch(JVM lookupswitch/tableswitch); 이
backend의 기준은 커버된 fixture의 동작 동등성이며 바이트코드 형태 동등성이
아님. 순차 `=` 체인은 여기의 모든 int/keyword/string-literal fixture에 동일한
결과를 줍니다. **fixture 추가 전에 실제 정확성 공백을 찾아 수정:** 첫 초안은
trailing default 없이 매칭 절이 없는 `case`가 조용히 `nil`로 떨어지게 했는데,
실제 호스트 테스트는 `IllegalArgumentException`을 throw. 이 tiny 언어에 아예
없는 별도·더 큰 기능인 `throw`를 특수 처리하는 대신, `expand-case`는 이제
trailing default 절을 요구하고 default 없는 `case`를  outright 거부 — 조용한
오컴파일 대신 정직한 fail-closed 범위 한도. 추가 전 실제 호스트 `eval`에 대해
검증(일치하는 int/keyword/string dispatch, default-fallthrough, no-default 형태
거부). U6: 61→65. DDC 행: 50→52 (2개 새 `case` 케이스). 전체 `-M:conformance`
(116/116)와 `bin/clj-meta-gate` 녹색.

**같은 날 네 번째 확장 (2026-08-14): 단일 절 `try`/`catch`.** 실제 ASM
exception-table 방출(`GeneratorAdapter.visitTryCatchBlock` + 보호 영역과
handler 주변 `mark`/`goTo` 라벨, 이 파일의 `if`와 같은 label/branch 관용구) —
macro 확장이 아님. `case`의 범위 메모가 제대로 된 no-default `case`에 실제
`throw`가 필요하다고 했고, `try`/`catch`는 그 방향의 실제 독립 단계(아직
닫지 않음: 이 슬라이스는 이 backend가 지원하는 op에서 이미 생기는 예외만
*잡음*, 가장 유용한 것은 0 나눗셈의 `quot`/`rem` `ArithmeticException`; `case`
no-default 공백을 완전히 닫는 임의 거부를 허용하는 사용자 대면 `throw` special
form은 여전히 열림). 이 파일의 확립된 패턴에 맞춘 의도적 좁은 범위: 본문
표현식 정확히 하나, `catch` 절 하나(`finally` 없음, multi-catch 없음,
multi-form try/catch 본문 없음), 잡힌 클래스는 작은 명시 allowlist
(`ArithmeticException`/`Exception`/`RuntimeException`/`Throwable`) — 이 tiny
언어는 임의 호스트 클래스를 이름 지을 방법이 없음. 추가 전 실제 호스트
`eval` 검증: 0 나눗셈 catch(`:divzero`), 비 throw 경로는 그대로 통과, 잡힌
예외 객체가 catch 이름에 올바르게 바인딩(`(nil? e)` → `false`), `try`/`catch`가
`let` 안 중첩 시 올바르게 합성. U6: 65→69. DDC 행: 52→54. 전체 conformance와
gate 녹색.

**같은 날 다섯 번째 확장 (2026-08-14): `throw`와 allowlist 예외 생성 —
`case` no-default 공백을 실제로 닫음.** 두 추가 모두 작성 전 실제 호스트
바이트코드를 `javap -c`로 역공학(AOT 컴파일 `(fn [x] (throw x))`와
`(fn [] (throw (IllegalArgumentException. "boom")))`, JVM 내부를 건드리는
것에 대한 이 파일의 확립된 규율):

- **`throw`**: `(throw expr)`이 `expr` 코드를 방출한 뒤 `CHECKCAST Throwable;
  ATHROW` — 근사가 아니라 *정확한* 실제 호스트 형태. 잡힌 예외 local을 포함한
  모든 표현식에서 동작하므로 re-throw(`catch` 본문의 `(throw e)`) 가능.
- **`ClassName.` 생성자 호출** (실제 호스트의 점-접미사 interop 구문, 발명한
  `new` 키워드 아님): `(IllegalArgumentException.)` 또는
  `(IllegalArgumentException. "msg")`가 `NEW; DUP; [args]; INVOKESPECIAL
  <init>` 방출, 다시 정확한 실제 형태. 의도적 좁음: `catch`가 이미 쓰는 같은
  작은 클래스 allowlist(이제 `IllegalArgumentException` 포함), 일반 Java
  클래스 해석 아님.

이전 확장에서 추가한 `expand-case`의 no-default 제한은 이제 해제: 매칭 없는
default 없는 `case`는 거부 대신 `(throw (IllegalArgumentException. "No matching
clause"))`로 확장. 정직한 주의: 실제 호스트 메시지는 동적(`"No matching clause:
<value>"`, `str`로 구성 — 이 tiny 언어에 없음)이라 여기 throw 메시지는 고정
문자열이며 exact match 아님 — 그러나 예외 *클래스*와 *throw 여부 자체*는 정확히
일치하며, 모든 `try`/`catch` 래핑 fixture가 실제로 관찰하는 것이 그것.
디버깅 메모: plain 예외 생성(`(IllegalArgumentException. "boom")`, `throw` 없음)
의 초기 수동 테스트가 호출 시 throw처럼 보였는데 실제 버그였을 수 있음(생성만
으로는 절대 throw하지 않아야 함) — 격리 프로세스에서 재테스트하니 같은
`clojure -e` 프로세스의 이전 실패 eval에서 온 REPL/세션 아티팩트였고 실제
결함 아님; 격리 재테스트, raw `javap -c` 덤프, 최종 fixture 실행 모두
construction-only 경로가 절대 throw하지 않음에 동의. 추가 전 실제 호스트
`eval` 검증: 메시지 있는 throw catch, no-arg 생성자 throw catch, 외부
`catch`로 re-throw된 잡힌 예외, 새로 열린 no-default `case`의 양쪽 분기.
U6: 69→74. DDC 행: 54→56. 전체 conformance와 gate 녹색.

**같은 날 여섯 번째 확장 (2026-08-14): `.methodName`을 통한 일반 인스턴스
interop — 좁은 allowlist가 아닌 실제 Java interop 해금.**
`(.methodName receiver args...)`, 실제 호스트의 점-접두 interop 구문이 이제
*untyped* receiver에 대해 실제 호스트 `eval`이 만드는 것과 정확히 같게
컴파일: `clojure.lang.Reflector.invokeInstanceMethod(Object, String, Object[])`,
런타임에 해석되는 동적 name+arg 기반 dispatch. 호스트 AOT 컴파일
`(.getMessage e)`(0-arg)와 `(.equals a b)`(1-arg)에 대해 코드 작성 *전*
`javap -c`로 라이브 확인 — 근사가 아니라 실제 Clojure의 모든 untyped interop
호출이 이미 타는 exact fallback 경로(type hint가 있으면 실제 컴파일러가 직접
`invokevirtual`을 방출할 수 있으나 이 tiny 언어에 type hint가 없으므로 여기의
모든 interop 호출도 실제 호스트에서 항상 이 경로). 구현은 args 배열에
`emit-object-array`를 그대로 재사용하는 얇은 래퍼(vector/map/set literal이 이미
쓰는 동일한 array-construction 바이트코드 패턴). 이 파일 다른 곳의
exception-class allowlist와 달리 진짜 일반적: 어떤 메서드 이름, 어떤 receiver,
어떤 인자 개수 — `Reflector.invokeInstanceMethod`가 런타임에 해석하는 범위로만
한정, 부분 집합 근사가 아니라 실제 호스트 동작과 정확히 일치. 이로써 이전
`throw`-확장 commit의 fixture가 `(nil? e)`로 우회해야 했던 이전 `.getMessage`
공백도 닫힘. 추가 전 실제 호스트 `eval` 검증: 잡힌 예외의 `.getMessage`,
문자열 `.length`/`.toUpperCase`(실제 conformance-corpus 행
`(fn [^String s] (.length s))` / `(fn [^String s] (.toUpperCase s))`와 직접
일치), `.equals` true/false 분기. U6: 74→79. DDC 행: 56→58. 전체 녹색.

**같은 날 일곱 번째 확장 (2026-08-14): static interop
(`ClassName/methodName`).** 실제 호스트는 여기서 대상 class+method를 *컴파일*
시간에 해석(인스턴스 interop과 달리 클래스 이름이 구문상 존재)하고 종종 직접
`invokestatic` 방출 — `javap -c`로 untyped `x`에 대한 `(Integer/toString x)`도
여전히 컴파일 시간 해석이며 Reflector 경로가 아님을 확인. 그 exact Java
overload 해석 메커니즘 일치는 범위 밖; 대신
`clojure.lang.Reflector.invokeStaticMethod(Class, String, Object[])`, Reflector가
static 호출용으로 노출하는 런타임-dispatch 프리미티브를 사용 — `case`의
`=`-체인이 이미 세운 behavior-equivalence-not-bytecode-shape 기준과 동일. 의도적
으로 작은 클래스 allowlist(`Math`, `Integer`, `Long`, `String` — 이 tiny 언어에
일반 클래스 이름 해석 없음), reader 자체의 namespace-qualified symbol 파싱 재사용:
`(symbol "Math/sqrt")`가 이미 namespace `"Math"` + name `"sqrt"`로 분리, reader
변경 0. 추가 전 실제 호스트 `eval` 검증, 흥미로운 부정 케이스 포함: 실제
`(Math/max 1 2.0)`은 호스트가 `IllegalArgumentException: "No matching method max
found taking 2 args"`로 거부(모호한 int/double overload —
`conformance.clj` 자체 부정-corpus 행 중 하나) — 이 backend의
`Reflector.invokeStaticMethod`도 *정확히 같은* 예외 클래스와 메시지로 동일
호출을 거부, 다른 결과나 조용한 수락이 아님. 거부 경로에서도 런타임-dispatch
대체가 행동적으로 충실함을 확인. U6: 79→83. DDC 행: 58→60. 전체 녹색.

**같은 날 여덟 번째 확장 (2026-08-14): `try`/`finally`, 그리고 이 확장이
`try`/`catch`에서도 잡은 실제 순서 버그.** 실제 호스트는 공유 서브루틴이
아니라 정상·예외 두 종료 경로 모두에 `finally` 블록을 복제 —
호스트 AOT `(try a (finally (.incrementAndGet side)))`에 대해 코드 작성 전
`javap -c`로 확인. `emit-try-finally`가 그 exact 형태를 맞춤: 정상 완료는
본문 결과를 저장, `finally`를 한 번 실행(값 버림 — `finally`는 절대 `try` 자체
결과에 기여하지 않음, 라이브 확인: `(try 42 (finally 99))` → `42`), 저장 결과
반환; catch-all handler(`visitTryCatchBlock` with `nil` type, 실제 호스트의
`type=any` exception-table entry와 일치)가 `finally`를 다시 실행한 뒤 원래
예외를 변경 없이 re-throw. 범위: 단일 본문 표현식, 단일 `finally` 절, 같은
`try`에 `catch`와 `finally` 결합 없음(추가 중첩 try-catch-block 형태 필요,
별도 슬라이스) — `try`는 이제 두 번째 form으로 `catch` 또는 `finally` 중
하나만 허용.

**깨끗하게 새 기능만 추가한 것이 아니라 실제 버그를 찾아 수정:**
`try`/`finally`의 첫 working 버전은 격리 fixture는 전부 통과했으나 바깥
`try`/`catch` 안에 중첩하면
(`(try (try (quot 10 x) (finally (.incrementAndGet a))) (catch ArithmeticException e :caught))`)
안쪽 `finally`의 부수 효과를 조용히 건너뜀 — 라이브 mutable counter로 실제
호스트 `eval`과 비교해 확인(호스트: counter 끝값 `1`, 이 backend: counter
`0` 유지), 검사만으로는 못 잡음. 원인, 실제 방출 클래스 exception table에 대한
`javap -c -v`로 발견: `emit-try`(및 `emit-try-finally` 첫 초안)가 `body` 방출
*전에* `visitTryCatchBlock`을 호출해 OUTER handler의 exception-table entry가
NESTED try 자신의 entry보다 먼저 등록됨. JVM은 메서드 exception table을 등록
순서로 검색하고 첫 매칭 entry를 사용 — outer entry가 먼저 나열되면 nested try
보호 영역에서 throw된 매칭 예외가 안쪽 `finally`(및 안쪽 `catch`도 마찬가지로)
를 완전히 우회하고 outer handler로 직행. 수정: `emit-try`와
`emit-try-finally` 모두에서 `visitTryCatchBlock`을 END로 이동(body 이후,
따라서 nested try의 이제 더 이른 등록 이후) — 유효한 ASM 사용, 호출은
`endMethod` 전에만 있으면 되고 Label 위치는 참조하는 `visitTryCatchBlock` 호출
시점과 무관하게 이미 `mark`로 고정됨. 수정 후 기존 try/catch fixture 세트와
새 중첩 케이스를 실제 호스트에 대해 재검증; 전부 일치. U6: 83→88(중첩 회귀
케이스를 영구 fixture로 포함, 정상·예외 분기 및 nested `try`를 통해 예외 경로의
finally 부수 효과를 관찰하는 버전). DDC 행: 60→62(순수 값 fixture만 — 행의 세
순차 leg(host/compiler/mini)에 공유되는 mutable AtomicInteger arg는 leg 간
mutation이 쌓여 비교를 무의미하게 하므로 U6 전용). 전체 `-M:conformance`
(116/116)와 `bin/clj-meta-gate` 녹색.

**같은 날 아홉 번째 확장 (2026-08-14): 하나의 `try`에 `catch`+`finally` 동시
지원.** 바로 앞에서 의도적으로 미룬 갭을 닫음. 실제 호스트는 `finally`를
정상 경로·`catch` 성공 경로·(catch-body 내부에서 던져진 것 포함) 둘 중
어느 쪽도 완전히 처리하지 못한 예외 경로까지 **세 개** 종료 경로에
복제 — 코드 작성 전 host-AOT `(try (quot 10 x) (catch ArithmeticException
e :divzero) (finally (.incrementAndGet a)))`를 `javap -c -v`로 확인.
exception table entry가 세 개 필요: `[try-start,try-end) -> catch-start`
(특정 `catch-class`), `[try-start,try-end) -> any-handler`(catch-all —
`catch-class`와 안 맞는 예외도 propagate 전에 `finally`가 돌게), 그리고
`[catch-start,catch-end) -> any-handler`(catch-all — catch-body 자체가
던진 예외도 `finally`가 돌게). 등록 순서가 두 겹으로 중요: 같은
`[try-start,try-end)` 범위에선 특정 `catch-class`가 any-handler보다
먼저 등록돼야 하고(바로 앞 슬라이스의 실제 버그와 같은 교훈), 이
파일의 모든 try* emitter처럼 세 등록 전부 `body`/`catch-body`/
`finally-body`를 다 방출한 뒤 맨 마지막에 이뤄져 — 이 구문 안에 중첩된
것이 먼저 등록되게. `try`의 arity 체크를 정확히 2에서 2~3으로 넓힘
(`catch`만, `finally`만, 또는 `catch` 다음 `finally`); 기존 catch-only·
finally-only 경로는 변경 없음(새 fixture 추가 전 digest 일치로 확인).
실제 host `eval` 대비 검증: 정상/catch 경로의 값+counter, 그리고 앞선
중첩-버그 시나리오를 이 새 결합 경로에 재사용한 regression 체크(안쪽
`catch-class`와도 안 맞고 잡히지도 않는 예외 타입이어도 바깥 `catch`에
닿기 전에 `finally`가 돎 — host·이 backend 둘 다 결과 `:outer-caught`,
counter `1`). U6: 88→94. DDC 행: 62→64(상수 `finally` fixture 두 개,
mutable arg 공유 문제는 앞과 동일 이유로 회피). 전체 `-M:conformance`
(116/116)와 `bin/clj-meta-gate`(`metacircular gate: READY`) 녹색.

**같은 날 열 번째 확장 (2026-08-14): `str`.** 실제 host에서 `str`은
컴파일러 특수형이 **전혀 아니다** — `javap -c`로 확인: `(str a b)`는
`RT.var("clojure.core", "str")`로 Var를 찾고, `getRawRoot()`로 실제 함수
값을 읽고, `IFn`으로 캐스트해서 `.invoke(...)`를 호출 — 일반 사용자 정의
함수 호출과 완전히 같은 메커니즘. 이 emitter도 그 형태를 그대로
따름(매 호출마다 인라인으로 Var 조회 — 실제 host는 `const__N` static
field에 캐싱하지만, 그건 성능 차이일 뿐 `Var.getRawRoot()`가 주는
값 자체는 동일하므로 행동 차이는 없음). 이 메커니즘은 원리상 `str` 외에
다른 `clojure.core` 함수로도 일반화되지만, 이번 슬라이스는 `str`만
fixture로 검증해 연결함. 실제 host `eval` 대비 검증(추가 전): 2-인자
문자열 연결, 숫자 인자, 1-인자, 0-인자(`""`), `nil` 인자는 `"null"`이
아니라 빈 문자열로 처리되는 Clojure 고유 동작(`(str nil "x")` → `"x"`),
문자열 리터럴과 섞어 쓰기. U6: 94→100. DDC 행: 64→66(2개 — mutable arg
공유 문제와 무관한 순수 값 fixture). 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 열한 번째 확장 (2026-08-14): field access(`.-fieldName`)와
`set!`.** host-AOT `(.-x p)`/`(set! (.-x p) v)`를 `javap -c`로 확인해
얻은 형태: field GET은 `Reflector.invokeNoArgInstanceMember(Object,
String, boolean)`을 `boolean=true`로 호출(`.methodName` 무인자 호출이
쓰는 `false`와 구분 — false는 필드/무인자 메서드 둘 다 시도, true는
필드로 확정). `set!`은 `Reflector.setInstanceField(Object, String,
Object)`로 가는데, 이 메서드 자체가 대입된 값을 반환해 Clojure의 `set!`
자체 의미(대입값으로 평가됨)와 정확히 일치. `.-fieldName`은 naive하게
"`.`으로 시작"만 검사하면 기존 `.methodName` 판정과 충돌하므로(예:
`.-x`가 `interop-method-name`에도 걸림), cond 분기 순서상
`field-access-name`을 먼저 검사하고, `interop-method-name` 쪽에도
`.-`로 시작하는 이름을 명시적으로 제외하는 이중 방어 추가. `set!`은
`(set! (.-field expr) value)` 형태만 허용(다른 대입 대상 — dynamic var
등 — 은 범위 밖, 별도 슬라이스). 실제 host `eval` 대비 검증(추가 전):
`java.awt.Point`에서 필드 읽기, `set!`이 대입값을 반환하는지, `set!`이
실제로 객체를 mutate하는지(대입 후 다시 읽어서 확인) 셋 다 host와 일치.
U6: 100→103. DDC 행: 66→67(읽기 전용 fixture만 — mutation 관찰 fixture는
공유 mutable arg 문제로 U6 전용). 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 열두 번째 확장 (2026-08-14): `locking`.** host-AOT `(locking sb
(.append sb "x"))`를 `javap -c -v`로 확인한 결과, `emit-try-finally`와
구조적으로 완전히 동일하다 — 다만 "finally에 해당하는 것"이 임의 표현식이
아니라 항상 lock 객체에 대한 `MONITOREXIT` 하나뿐이고, 보호 구역 시작 전에
`MONITORENTER`가 한 번 더 실행된다는 점만 다르다. 실제 host는
`MONITORENTER`/`MONITOREXIT` 뒤에 `ACONST_NULL`을 push했다 바로 pop하는
코드도 남기는데(Clojure의 일반 표현식-지향 컴파일러가 `monitor-enter`/
`monitor-exit`을 statement 위치에서 `nil`로 평가되는 표현식으로 다루는
데서 온 부수 효과), 이건 관찰 가능한 차이가 없는 화장용 잔재라 재현하지
않았다. 범위는 의도적으로 좁게: lock 표현식 + 단일 body 표현식만(다중
form body 없음), 이 파일의 기존 최소-범위 패턴과 동일. 실제 host `eval`
대비 검증(추가 전): 정상 경로에서 `StringBuilder`에 append, 그리고
`locking`이 감싼 body에서 던진 예외가 `try`/`catch`를 정상적으로
통과해 나가는지(host·이 backend 둘 다 `:caught`, 예외 클래스·메시지
일치) 확인. U6: 103→105. DDC 행: 67→68(예외 경로 fixture만 — 정상 경로
fixture는 `StringBuilder`가 leg마다 계속 mutate되어 세 leg에 걸쳐
누적되므로 공유 mutable arg 문제로 U6 전용; 예외 경로 fixture는 관찰되는
mutation이 없는 plain `Object`라 안전). 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 열세 번째 확장 (2026-08-14): multi-catch.** host-AOT `(try
(quot 10 x) (catch ArithmeticException e :divzero) (catch
IllegalArgumentException e :bad-arg))`를 `javap -c -v`로 확인. `finally`의
catch-all `nil`-type entry와 달리, 여기선 각 `catch` 절이 자기만의
handler와 자기만의 exception-table entry를 갖는다 — 전부 같은
`[try-start,try-end)` 범위를 커버하지만 (handler, 구체 클래스) 쌍이
서로 다르고, 소스 순서 그대로 등록된다(real host와 정확히 일치).
`finally`를 guarantee할 필요가 없으니 catch-all entry는 필요 없음.
기존 단일-catch/`finally`-only/`catch`+`finally` 경로는 전혀 안 건드리고
`:try-multi-catch`라는 새 AST 노드+emitter를 따로 추가(이미 검증된
코드 경로를 건드리는 리스크를 최소화). 김에 `(try body)`(clause 없는
bare try)도 사소하게 지원 — 그냥 `body`와 동일하게 처리. multi-catch에
`finally`를 같이 쓰는 것은 범위 밖, 별도 슬라이스. 실제 host `eval`
대비 검증(추가 전): 2개 catch 중 첫 번째가 매치, 2개 중 두 번째가
매치, 3개 catch 중 세 번째가 매치 — 다 host와 일치. U6: 105→111. DDC
행: 68→70. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 열네 번째 확장 (2026-08-14): multi-catch + `finally` 결합.**
바로 앞에서 범위 밖으로 미룬 것을 닫음. host-AOT `(try (quot 10 x) (catch
ArithmeticException e :divzero) (catch IllegalArgumentException e
:bad-arg) (finally (.incrementAndGet a)))`를 `javap -c -v`로 확인 —
`emit-try-catch-finally`의 N-catch 일반화. 각 catch 절은 여전히 자기만의
구체 클래스 exception-table entry를 try-body 범위에 갖고(소스 순서대로,
`emit-try-multi-catch`와 동일), 거기에 try-body 범위 전체를 덮는
공유 catch-all `finally` entry 하나가 추가되며, **각 catch-body 자신의
범위도** 독립적으로 같은 `finally` handler를 가리키는 catch-all entry를
갖는다(어느 catch-body 안에서 예외가 나도 `finally`가 돌게). 등록 순서:
같은 `[try-start,try-end)` 범위에서 각 catch의 구체 entry가 공유
any-handler entry보다 먼저(기존 순서 규칙 그대로), catch-body별
any-handler entry들은 서로/다른 것과 겹치지 않으므로 상호 순서는 무관.
실제 host `eval` 대비 검증(추가 전): 정상 경로 값+counter, 첫 번째
catch 매치 값+counter, 두 번째 catch 매치 값, 그리고 가장 까다로운
경우 — **catch-body 안에서 예외가 다시 던져져도** `finally`가 돌고
바깥 `try`/`catch`로 정상 전파되는지(host·이 backend 둘 다
`:outer-caught`, counter `1`) — 전부 일치. U6: 111→117. DDC 행: 70→72
(상수 `finally` fixture만 — mutable arg 공유 문제 회피). 전체
`-M:conformance`(116/116)와 `bin/clj-meta-gate`(`metacircular gate:
READY`) 녹색, 회귀 없음.

**같은 날 열다섯 번째 확장 (2026-08-14): 일반 클래스 생성(작은 예외
허용목록을 넘어서).** `java.awt.Point.`/`java.util.ArrayList.`를
`javap -c`로 확인해서 real host 자신의 정확한 메커니즘을 그대로 씀:
(클래스, arity) 쌍이 컴파일타임에 생성자를 유일하게 특정하면(예:
`Point(int,int)`는 하나뿐) real host는 직접 `NEW`/`INVOKESPECIAL`을
내지만, 특정 안 되면(예: `ArrayList(int)`는 `ArrayList(int)`와
`ArrayList(Collection)` 둘이 arity 1로 겹침) `RT.classForName(String)` +
`Reflector.invokeConstructor(Class, Object[])`로 **런타임** 반사
디스패치로 폴백한다 — `.methodName`/`ClassName/methodName`이 이미 쓰던
것과 같은 일반 메커니즘. 이 백엔드는 이 애매한-arity 경우의 폴백 경로만
사용(어떤 클래스든, 어떤 인자 개수든 일반적으로 처리) — 유일-arity
경우의 컴파일타임 직접 호출 최적화는 안 함(`case`/static interop에서
이미 정한 "바이트코드 모양이 아니라 행동이 같으면 됨" 기준과 동일).
기존 작은 예외 허용목록(`known-exception-classes`) 경로는 전혀 안
건드리고(먼저 체크되어 우선함), 새로운 `general-constructor-class-name`
검사를 폴백으로 추가. 정직한 caveat: 존재하지 않는 클래스 이름은 real
host처럼 analyze 시점이 아니라 `RT.classForName`이 실행되는 **런타임**에
실패함 — 이 백엔드의 fixture가 계산하는 어떤 값에도 영향 없는, 잘못된
소스에 대해서만 나는 차이. 실제 host `eval` 대비 검증(추가 전):
유일-arity 생성자(`Point`), 애매한-arity 생성자를 int 인자로(`.size`
`0`), 같은 애매한 생성자를 Collection 인자로(다른 오버로드로 정확히
디스패치되어 `.size` `3`), 무인자 생성자 — 전부 host와 일치, 그리고
필드 접근(`.-x`)·instance interop(`.size`)과도 자연스럽게 합성됨을
확인. U6: 117→121. DDC 행: 72→74. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 열여섯 번째 확장 (2026-08-14): 일반 static interop(작은
static-interop 허용목록을 넘어서) — 바로 앞 일반 클래스 생성과 같은
원리를 static method 호출에 그대로 적용.** `(Character/isDigit c)`를
`javap -c`로 확인: real host는 짧은 클래스 이름 `Character`도 자기
default-import 표(`java.lang.*`)로 `java.lang.Character`로 완전히
해석하고, `Character.isDigit`이 `(char)`/`(int)` 두 오버로드로 애매하니
일반 클래스 생성 때와 **똑같은** `RT.classForName(String)` +
`Reflector.invokeStaticMethod(Class, String, Object[])`로 폴백한다. 이
tiny 언어엔 자체 import 표가 없어서 real host와 달리 **완전히 정규화된
클래스 이름**을 요구함(`java.lang.Character/isDigit`, 짧은
`Character/isDigit`은 안 됨) — real host의 이름 해석 자체를 흉내내는 게
아니라 정직하게 더 좁은 범위. 기존 작은 `known-static-classes`
허용목록(`Math`/`Integer`/`Long`/`String`) 경로는 전혀 안 건드리고(먼저
체크), 새 `general-static-interop-target`을 폴백으로 추가. 실제 host
`eval` 대비 검증(추가 전): `isDigit` 참/거짓 둘 다, 무인자 static
호출(`Collections/emptyList`) — 전부 host와 일치. U6: 121→124. DDC 행:
74→75. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 열일곱 번째 확장 (2026-08-14): bignum 리터럴(`N`/`M` 접미사) —
그리고 이번에도 진짜 버그 하나를 값만이 아니라 **클래스까지** 대조해서
잡았다.** `5N`/`1.5M`을 `javap -c`로 확인했더니 뜻밖의 결과: real host는
이걸 `BigInteger.valueOf` 같은 걸로 직접 만드는 게 아니라, 리터럴의
소스 텍스트 자체를 문자열 상수로 저장해뒀다가 클래스 초기화 시점
(`<clinit>`)에 `clojure.lang.RT.readString`을 **한 번** 호출해서 만든
값을 static field에 캐싱한다. 이 메커니즘은 일부러 재현하지 않았다 —
그대로 따라하면 이 witness 자신의 상수 생성이 real reader를 거치게
되어, 독립 DDC witness라는 존재 이유 자체가 무너진다. 대신 표준
라이브러리 API로 직접 값을 만든다(같은 값, 완전히 다른 생성 경로).

**첫 시도에서 진짜 버그**: `N` 접미사 값을 `java.math.BigInteger`로 바로
만들었더니 `(= tiny host)`는 `true`였지만(Clojure `=`가 숫자 타입 간
교차 비교를 하므로) `(class tiny)` ≠ `(class host)`였다 — `(class 5N)`은
사실 `clojure.lang.BigInt`(Clojure 자체 wrapper 타입)이지 raw
`BigInteger`가 아니었다. **값만 비교하고 넘어갔으면 안 잡았을 버그** —
클래스까지 명시적으로 대조해서 발견하고, `BigInt.fromBigInteger(new
BigInteger(String))`로 고쳤다. `M` 접미사는 처음부터 진짜
`java.math.BigDecimal`이라 별도 wrapper 불필요(라이브 확인).

또 하나 실제로 고친 것: `analyze-expr`의 기존 `(integer? form)` 분기가
`(long form)`으로 캐스팅하는데, `BigInt`도 `integer?`를 만족해서 그
분기를 먼저 타면 `Long/MAX_VALUE`를 넘는 값이 조용히 잘렸을 것 —
`emit-const`/`analyze-expr` 둘 다에서 BigInt/BigDecimal 분기를 generic
`integer?` 분기보다 **먼저** 검사하도록 함.

이 리터럴들은 기존 `+`/`-`/`*`/`=` 연산자와 자연스럽게 합성된다(이미
있는 `Numbers.add` 등이 애초에 모든 숫자 타입에 다형적으로 동작하므로) —
그래서 그냥 리터럴 하나 추가한 게 아니라 임의 정밀도 산술이라는 진짜
새 능력이 기존 연산자를 통해 열림. 실제 host `eval` 대비 검증(추가
전): 단순 리터럴 값+클래스, `Long/MAX_VALUE`를 넘는 `+`(오버플로 없이
정확히 계산), `BigDecimal` 곱셈, `BigInt`와 `Long`의 `=` 비교 — 전부
host와 일치(값·클래스 둘 다). U6: 124→130. DDC 행: 75→78. 전체
`-M:conformance`(116/116)와 `bin/clj-meta-gate`(`metacircular gate:
READY`) 녹색, 회귀 없음.

**같은 날 열여덟 번째 확장 (2026-08-14, 배치 진행): regex 리터럴(`#"..."`)
+ ratio 리터럴(`1/3`).** `javap -c` 확인: regex는 real host도 그냥
`Pattern.compile(String)` 직접 호출(리더 의존 없음, 그대로 재현); ratio는
bignum과 같은 `RT.readString` 경로라 이번에도 일부러 안 씀 — 대신
`Numbers/divide`를 parse 시점에 호출(실제 reader와 동일 메커니즘, 라이브
확인: `1/3`→reduced `Ratio`, `4/2`→`Long 2`로 collapse)해서 이미
reduce된 numerator/denominator로 `new Ratio(BigInteger,BigInteger)`
직접 생성. `Pattern`은 `.equals`를 오버라이드 안 해서(`(= #"a+" #"a+")`
→ `false`, 라이브 확인) fixture는 `.pattern` 문자열로 비교. U6: 130→134.
DDC 행: 78→81. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 열아홉 번째 확장 (2026-08-14, 배치 진행): `binding` + dynamic var
deref.** `javap -c -v`로 `(binding [*x* 42] *x*)`를 확인하니 real host는
`push-thread-bindings`/`pop-thread-bindings`를 (이미 `str`에 쓴 것과
같은 `RT.var`+`IFn.invoke` 메커니즘으로) `emit-locking`과 구조적으로
동일한 try/finally로 감싸고, lock 획득/해제 자리에 push/pop을 넣는다 —
이번엔 우회 없이 그대로 재현(behavior뿐 아니라 bytecode shape까지 일치).
이 tiny 언어엔 `def`가 아예 없어서, `binding`이 참조할 수 있는 dynamic
var 하나를 `frontend_selfhost.clj` 자신에 `(def ^:dynamic
*tiny-dynamic-var* :tiny-dynamic-var-root)`로 미리 선언해두고 작은
allowlist로 연결(`known-exception-classes`와 같은 패턴). DDC 행에
연결하려면 real host `eval`/`compiler.clj`가 다른 `*ns*`에서도 같은
심볼을 풀 수 있어야 해서 정규화(`dynamic-var-target`)로 bare/qualified
심볼 둘 다 같은 항목에 매칭되게 함 — 세 다리 전부 실제로 대조 검증(정상
종료 후 root로 복귀/예외 종료 후에도 root로 복귀 포함). U6: 134→137.
DDC 행: 81→84. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

`letfn`은 조사만 하고 미룸: real host가 mutual recursion을 위해 binding당
별도 클래스 + 상호 참조 필드를 생성하는 걸 확인했는데(`javap -c`), 지금
U6는 클래스를 하나만 찍는 구조라 `deftype`/`reify`급 아키텍처 확장이
필요함 — 그 둘과 묶어서 나중에.

**같은 날 스무 번째 확장 (2026-08-14, 배치 진행): 고정 arity와 variadic
`&` ceiling의 혼합.** 그동안 U6는 "고정 multi-arity만" 또는 "variadic
단독 하나만"만 지원하고 둘을 섞으면 거부했는데, `javap -c`로
`(fn ([a] a) ([a b] (+ a b)) ([a b & r] ...))`를 확인하니 real host는
`RestFn`을 상속한 같은 클래스 안에 고정 arity마다 `invoke(N)` 오버라이드를
평소처럼 찍고, variadic 절 하나만 `doInvoke`+`getRequiredArity`로 감싸는
단순 조합이었다 — 별도 클래스 불필요, `letfn`/`deftype`급이 아니었음.
고정 절의 param 개수가 variadic 절과 같으면 그 arity는 고정 절의
`invoke(N)`이 상속된 `RestFn` 기본 동작(`doInvoke`로 라우팅)을 그냥
오버라이드로 이기는 것도 라이브로 확인(`(f 1 2)`가 `([a b] ...)`를 선택,
`([a b & r] ...)` 아님) — 별도 런타임 로직 없이 JVM 오버라이드 우선순위
그대로 성립. real host의 "고정 arity가 variadic보다 param이 많으면
거부"(`Can't have fixed arity function with more params than variadic
function`) 검증도 그대로 재현. U6: 137→141. DDC 행: 84→86. 전체
`-M:conformance`(116/116)와 `bin/clj-meta-gate`(`metacircular gate:
READY`) 녹색, 회귀 없음.

**같은 날 스물한 번째 확장 (2026-08-14, 배치 진행): 임의 `clojure.core`
함수를 일반 call head/first-class 값으로.** `str`은 처음부터 compiler
special이 아니라 `RT.var("clojure.core","str").getRawRoot()`를 `IFn`으로
캐스트해 호출하는 평범한 Var 룩업이었다(이전 슬라이스에서 확인) —
이번엔 `(map inc coll)`을 `javap -c`로 확인해서 `map`(call head)과
`inc`(그냥 값으로 전달, `.invoke` 없이 `getRawRoot()`만)가 **정확히 같은
메커니즘**임을 재확인하고, `str` 하나만 하드코딩했던 걸 일반화 —
`analyze-call`의 마지막 fallback과 `analyze-expr`의 심볼 fallback 둘 다
`clojure.core`에 실제로 존재하는 아무 함수 이름이면 받아들이게 함.
단순 allowlist가 아니라 `(ns-resolve (find-ns 'clojure.core) (symbol
name))`로 **진짜 존재 여부**를 analyze 시점에 검증 — real host가 컴파일
시점에 "Unable to resolve symbol"로 즉시 거부하는 것과 같은 fail-fast를
재현(안 그러면 `RT.var`가 존재하지 않는 이름도 그냥 새 unbound Var를
인턴해버려서, 오타가 컴파일은 통과하고 런타임에야 터짐). `map`/
`filter`/`reduce`/`apply`/`conj`/`assoc`/`vec`/`into` 등 클로저 표준
라이브러리 표면 전체가 새 바이트코드 하나 안 늘려도 열림. 실제 host
대비 전부 대조 검증. U6: 141→149. DDC 행: 86→91. 전체
`-M:conformance`(116/116)와 `bin/clj-meta-gate`(`metacircular gate:
READY`) 녹색, 회귀 없음.

**같은 날 스물두 번째 확장 (2026-08-14, 배치 진행): local을 함수로 호출
(고차 함수 파라미터).** `(fn [f x] (f x))`처럼 파라미터 자체가 함수인
경우가 이전까지 전부 "unsupported call"이었다 — `analyze-call`의
fallback이 `clojure.core` Var 존재 여부만 체크했지 local(env에 이미
있는 이름)은 아예 고려하지 않았기 때문. `javap -c`로 확인하니 real
host는 이 경우 Var 룩업이 전혀 없이 그냥 local 값을 `IFn`으로
`checkcast`해서 바로 `invokeinterface`— 이미 있던 `emit-local`을
재사용해서 새 바이트코드 메커니즘 추가 없이 구현. env에 있는 이름이
`core-var-exists?` 체크보다 먼저 매칭되게 해서 real host처럼 local이
같은 이름의 `clojure.core` 함수를 shadow하는 순서도 재현. 이전
슬라이스의 core-fn-value와 합쳐져서 `(map f coll)`처럼 local 함수를
`map`/`filter`/`reduce`에 넘기는 것도 가능해짐. 실제 host 대비 대조
검증. U6: 149→152. DDC 행: 91→93. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 스물세 번째 확장 (2026-08-14, 배치 진행): `+`/`-`/`*` variadic화
+ unary `-`.** 지금까지 `+`/`-`/`*`는 정확히 2-인자만 받았는데(`<`/`=`
같은 비교 연산과 같은 취급), `javap -c`로 `(+ a b c)`를 확인하니 real
host는 `Numbers.add(Numbers.add(a,b),c)`처럼 **왼쪽부터 fold**해서
컴파일했다 — 그래서 3개 이상 인자는 analyze 시점에 기존 `:binary` 노드로
왼쪽 fold desugar, emitter는 전혀 안 건드림. `(+)`→0, `(*)`→1, `(+ a)`/
`(* a)`→`a` 그대로(라이브 확인)도 반영. `(- a)`(단항 음수)는 `(- a b)`와
다른 오버로드(`Numbers.minus(Object)` 1-인자, 기존 2-인자
`Numbers.minus(Object,Object)`와 별개)임을 `javap -c`로 확인하고
`inc`/`dec`와 같은 `:unary` 부류에 새로 추가. `(-)` 0-인자는 real host도
`ArityException`으로 거부하는 것 확인, 그대로 재현. U6: 152→161. DDC 행:
93→97. 전체 `-M:conformance`(116/116)와 `bin/clj-meta-gate`
(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 스물네 번째 확장 (2026-08-14, 배치 진행): 연쇄 비교(`<`/`=`/
`>`/`>=`/`<=` 2-인자 외 arity) + `get` 3-인자 기본값.** `+`와 달리
`<`는 3개 인자에서 fold를 안 한다는 걸 `javap -c`로 확인 —
`(< a b c)`도 `(< a)`도 그냥 `RT.var("clojure.core","<").getRawRoot()`
+ `IFn.invoke`(기존 `core-fn-call` 메커니즘과 정확히 동일)였다. 그래서
정확히 2-인자일 때만 기존 `Numbers.lt` 직접 호출 fast path를 쓰고, 그
외 arity(0/1/3+)는 `core-fn-call`로 폴백 — 이건 "동작만 같은" 게
아니라 real host가 실제로 쓰는 바로 그 메커니즘. `get`의 3-인자
기본값 형태(`(get m k d)`)는 다른 패턴: `javap -c` 확인 결과 별개의
`RT.get(Object,Object,Object)` 오버로드 직접 호출이라 새 `:get3` 노드로
추가. 전부 실제 host 대비 대조 검증(참/거짓 양쪽, key 있음/없음 양쪽).
U6: 161→171. DDC 행: 97→100. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 스물다섯 번째 확장 (2026-08-14, 배치 진행): named 자기재귀
`fn`.** `(fn foo [n] ... (foo ...))`가 이전까지 "malformed fn clause"로
전부 거부됐다 — `analyze-fn`이 이름 있는 형태 자체를 파싱 못 했음.
`javap -c`로 real host의 named `(def f (fn foo [n] ...))`를 확인하니
`foo`라는 이름의 self-reference는 그냥 `this`를 로드(`aload_0`)해서
`IFn`으로 checkcast 후 invoke — 이전 슬라이스의 `emit-local-fn-call`과
완전히 같은 모양(당연함: 컴파일된 클래스가 이름과 무관하게 항상
`AFunction`/`RestFn`을 통해 `IFn`을 구현하므로). 새 바이트코드
메커니즘 없이 `emit-local`에 `:self` kind 하나(`this` 로드)만 추가하고,
`analyze-fn`이 이름을 파싱해서 각 arity 절의 env에 미리 넣어줌 — 파라미터가
같은 이름이면 그 파라미터가 self-reference를 shadow하는 것도 real host와
동일(라이브 확인: `(fn foo [foo] foo)` → 파라미터 값 그대로 반환).
고정+variadic 혼합 arity와 named self-recursion을 같이 쓰는 경우도 검증.
U6: 171→175. DDC 행: 100→102. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 스물여섯 번째 확장 (2026-08-14, 배치 진행): 계산된 call head
+ 덤으로 딸려온 keyword-as-fn.** `((constantly x) 99)`처럼 call head
자체가 심볼이 아니라 계산되는 표현식인 경우가 전부 "unsupported
call"이었음 — `analyze-call`의 fallback들이 죄다 `(symbol? op)`를
전제하고 있었기 때문. `javap -c` 확인: real host는 head 표현식이 만든
값을 그냥 `IFn`으로 checkcast해서 invoke — Var 룩업도 local 슬롯도
없음, 기존 `local-fn-call`/`core-fn-call`의 "cast하고 invoke"하는
꼬리 부분과 완전히 같은 모양이라 새 `:computed-fn-call` 노드 하나로
`(not (symbol? op))`일 때를 잡아냄. 구현하고 나서 발견한 덤:
keyword(`:a`)도 이미 `:const`로 analyze되고(`emit-const`가 처리)
`clojure.lang.Keyword`가 `IFn`을 구현하니 `(:a m)`(keyword-as-function
맵 조회)도 새 코드 없이 그냥 통과 — 라이브로 확인하고 fixture 추가.
U6: 175→178. DDC 행: 102→104. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 스물일곱 번째 확장 (2026-08-14, 배치 진행): `fn` body 자체
tail 위치의 `recur` (진짜 stack-safe 꼬리재귀).** 지금까지 `recur`는
명시적 `loop` 안에서만 됐고, `(fn [n] (if (= n 0) 0 (recur (- n
1))))`처럼 `loop` 없이 `fn` body 꼬리 위치에서 바로 쓰면 "recur outside
loop"였다. 이건 25번째 슬라이스의 named self-recursion(진짜
`IFn.invoke` 재호출이라 스택이 계속 쌓임)과는 **의미적으로 다른**
기능 — `javap -c` 확인: real host는 `astore`로 인자 슬롯에 재저장하고
메서드 맨 위로 `goto`, 스택이 안 쌓이는 진짜 루프. 기존 `loop`/`recur`의
GOTO 메커니즘을 일반화해서(`recur-target-key`의 slot을 `{:kind
:local}`/`{:kind :arg}`로 태깅) `analyze-fn-clause`가 각 arity 절
env에 자기 param들을 recur target으로 미리 깔아두고, `emit-class`가
각 `invoke(N)`/`doInvoke` 메서드 맨 앞에 label을 찍어둠. 중첩 `loop`가
같은 env key로 자연스럽게 shadow하는 것도 확인(라이브: `recur`가
`loop`를 타겟해야 하는 경우 실제로 `loop`를 타겟함). `(f 100000)`으로
스택 안 쌓이는 것 직접 확인(named self-recursion이었다면
StackOverflow 위험 구간), variadic arity에서도 동작 확인. U6: 178→182.
DDC 행: 104→106. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 스물여덟 번째 확장 (2026-08-14, 배치 진행): 임의
fully-qualified 예외 타입 `catch`.** 지금까지 `catch`는 5개짜리 작은
allowlist(`ArithmeticException`/`Exception`/`RuntimeException`/
`Throwable`/`IllegalArgumentException`)만 가능했음 —
`clojure.lang.ExceptionInfo`(`ex-info`가 실제로 던지는 타입!)조차
못 잡았다. 확인해보니 `:catch-class`를 쓰는 곳은 전부
`Type/getInternalName`만 호출 — JVM 예외 테이블(`visitTryCatchBlock`)은
컴파일타임 문자열 상수만 필요하고, general-static-interop/
general-constructor처럼 런타임 `RT.classForName` 바이트코드 호출이
전혀 필요없다는 뜻. 그래서 host 쪽에서 analyze 시점에 그냥
`Class/forName`으로 풀면 끝 — 새 바이트코드 메커니즘 자체가 없음.
fully-qualified 이름만 허용(이 tiny 언어엔 import 표가 없어서
`NullPointerException`처럼 짧은 이름은 여전히 안 됨, `java.lang.
NullPointerException`은 됨 — 기존 general-static-interop과 같은
정직한 범위 제한). `ExceptionInfo` 잡아서 `ex-data`/`.getMessage`
읽기, `NullPointerException` 잡기 전부 실제 host 대비 검증. U6:
182→185. DDC 행: 106→108. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 스물아홉 번째 확장 (2026-08-15): 진짜 중첩 closure — 그리고
그 과정에서 진짜 버그 하나를 잡았다.** 사용자가 "왜 진짜 컴파일러
코드를 재사용하지 않느냐"고 물어서 Trusting-Trust/DDC 독립성 원칙을
설명했고, 사용자가 동의하며 "그래야 백도어 같은 코드를 제대로
걸러낼 수 있다"고 확인 — U6를 계속 독립적으로 넓혀가는 방향을 재확인한
뒤, 남은 4개 큰 항목 중 사용자가 직접 "중첩 fn 리터럴(진짜 closure)"을
선택.

`javap -c`로 `(fn [x] (fn [y] (+ x y)))`를 확인하니 real host는 내부
`fn`마다 별도 클래스를 만들고, 캡처된 자유변수를 인스턴스 필드로,
생성자가 그 값들을 받아 필드에 저장(`NEW; DUP; <captured-values>;
INVOKESPECIAL`)하는 표준 closure 패턴이었다. 구현: (1) `fn`을
`analyze-call`의 진짜 special form으로 추가(`analyze-nested-fn`),
(2) 자유변수는 **analysis 이후에** 이미 분석된 AST를 재귀적으로 훑어서
`:local`/`:local-fn-call` 참조를 전부 모으고 이 closure 자신의
파라미터/self-name을 빼는 방식으로 계산(analyze 시점 env는 `:kind`
구분이 없어서 "이게 내 것인지 바깥 것인지"를 그 자리에서 구분 못 하기
때문), (3) `emit-class`를 일반화해서 `:captures`가 있으면 필드 +
다중인자 생성자를 만들고 각 clause env에 `:capture` kind를 심음,
(4) `emit-closure`가 정의 지점에서 재귀적으로 `emit-class`를 호출해
내부 클래스를 즉시 정의/로드하고 `NEW; DUP; <바깥 env에서
emit-local로 캡처값 읽기>; INVOKESPECIAL`을 방출. 일부러 좁힌 범위:
**중첩 1단계까지만**(closure 안의 closure는 거부), **closure는 단일
arity 절만**(multi-arity 중첩 closure 거부) — 둘 다 자유변수 계산을
단순하게 유지하기 위한 의도적 제한, 명확한 에러로 거부되는 것까지
확인.

**구현 후 filter로 검증하다가 진짜 버그를 하나 발견했다.**
`(filter (fn [x] (> x threshold)) coll)`이 아무것도 걸러내지 않고
전부 통과시켰다 — `map`/직접 호출/`apply`로는 같은 술어가 정확한
값을 냈는데 `filter`만 틀렸다. 추적해보니: `GeneratorAdapter/box`가
boolean에 대해 `new Boolean(z)`(deprecated 생성자)를 방출하지
`Boolean.valueOf(z)`를 안 씀 — 그래서 이 witness가 만드는 `<`/`>`/
`=`/`zero?`/`pos?`/`neg?`/literal `true`/`false` 전부 **매번 새
Boolean 인스턴스**를 만들었다. 이 witness 자신의 `if`는
`RT.booleanCast`(진짜 `instanceof`+`.booleanValue()` 변환)를 써서
non-singleton Boolean도 문제없이 처리하지만, `javap -c`로 real
host의 `(if x ..)`를 까보니 `RT.booleanCast` 호출이 **전혀 없고**
`dup; ifnull ..; getstatic Boolean.FALSE; if_acmpeq ..`처럼 캐시된
싱글턴에 대한 **순수 레퍼런스 identity 비교**였다. 그래서
non-singleton `false`는 `nil`도 아니고 `Boolean.FALSE`와
`identical?`도 아니니 real host 코드 입장에선 **항상 truthy** —
`clojure.core/filter`의 실제 컴파일된 `(if (pred f) ..)`가 정확히
이 경로를 타서 뭘 넣어도 다 통과시켰던 것. `map`은 술어의 진위를
`if`로 검사하지 않아서 증상이 안 보였을 뿐. 라이브로
`(identical? (Boolean/valueOf false) Boolean/FALSE)` → `true`,
`(identical? (Boolean. false) Boolean/FALSE)` → `false`,
`(if (Boolean. false) :truthy :falsy)` → `:truthy`로 재현·확인.
수정: `.box`를 쓰던 9곳 전부 `GeneratorAdapter/valueOf`로 교체(raw
ASM 프로브로 `invokestatic Boolean.valueOf:(Z)Ljava/lang/Boolean;`
방출 확인, real host와 동일 메커니즘) — 이 witness 자신의 fixture는
전부(자기 `if`가 관대해서) 이미 통과하고 있었기 때문에 이 버그는
**독립 witness가 real host와 실제로 상호작용할 때만** 드러났다는
점이 특히 중요 — U6를 계속 독립적으로 키우는 이유 그 자체를
증명하는 사례. 재발 방지 fixture(`identical?` 직접 대조)와 원인이 된
`filter`+closure fixture 둘 다 추가. compiler.clj(production
backend)는 closure와 boolean identity 둘 다 문제없음을 확인 후 DDC
행에도 연결. U6: 185→194. DDC 행: 108→112. 전체
`-M:conformance`(116/116)와 `bin/clj-meta-gate`(`metacircular gate:
READY`) 녹색, 회귀 없음(기존 185개 fixture 전부 이 fix 이후에도
그대로 통과 — 자기 `if`가 애초에 관대했으므로 예상대로).

**같은 날 서른 번째 확장 (2026-08-15): `letfn`(비-상호재귀만).**
`javap -c`로 single-binding, 상호재귀 아닌 `(letfn [(add-x [y] (+ x
y))] (add-x 10))`를 확인하니 real host의 바이트코드가 `(let [add-x
(fn add-x [y] (+ x y))] (add-x 10))`와 **완전히 같은 모양** — `x`를
진짜 closure 필드로 캡처, 생성된 인스턴스를 평범한 local slot에
저장, `local-fn-call`로 호출. 즉 non-mutual `letfn`은 이미 있는
`let`+named-`fn`+closure 기능의 순수한 desugar였다. 그래서
`letfn`을 매크로 확장 레이어에만 추가(`expand-letfn`) — 각
`(name [params] body)` 바인딩을 `(let [name (fn name [params]
body)] ...)`로 안쪽부터 감싸 nested `let`으로 변환, 새 바이트코드
전혀 불필요. 진짜 상호재귀(`even?`가 `odd?`를 부르고 그 반대도)는
real host가 각 바인딩 생성자를 (아직 null일 수도 있는) 다른
바인딩들의 현재 값으로 호출한 뒤, 전부 만들어지고 나서 아직 null이던
필드들을 `putfield`로 되짚어 채우는 2단계 생성-후-backpatch 메커니즘이
필요 — 이건 별도로 크게 다뤄야 할 확장이라 이번엔 시도하지 않고,
매크로 확장 시점에 각 바인딩의 원본 body에서 **다른** 형제 이름이
등장하는지 raw-form 심볼 스캔으로 검사해서 명확한 에러로 거부(분석
단계까지 미뤄서 애매한 "unknown local"이 나오는 것보다 나음) — 자기
자신 재귀는 named self-recursion으로 이미 잘 되므로 정상 허용. 단일
바인딩(외부 변수 캡처), 자기재귀 바인딩, 서로 독립된 두 바인딩,
상호재귀 거부까지 전부 실제 host 대비 검증. compiler.clj도 `letfn`
정상 지원 확인 후 DDC 행에 연결. U6: 194→197. DDC 행: 112→114. 전체
`-M:conformance`(116/116)와 `bin/clj-meta-gate`(`metacircular gate:
READY`) 녹색, 회귀 없음.

**같은 날 서른한 번째 확장 (2026-08-15): `reify`.** 남은 4개 큰 항목
중 사용자에게 `deftype`(하니스가 "fn 하나만 컴파일"하는 구조라
top-level multi-form 지원으로 아키텍처 변경 필요) vs `reify`(인터페이스
reflection + primitive 반환타입 매칭 필요)를 놓고 트레이드오프를
설명, 사용자가 `reify` 선택.

`javap -c`로 `(reify Comparator (compare [this a b] ...))` 확인: real
host는 새 클래스가 지정한 인터페이스를 직접 `implements`하고(`IFn`
상속 아님 — reify 결과는 일반 fn으로 호출 불가, 그 인터페이스로만
호출 가능), 자유변수는 closure와 완전히 같은 방식(인스턴스 필드)으로
캡처하며, 항상 `clojure.lang.IObj`(`meta`/`withMeta`) 보일러플레이트도
추가함. `IObj`는 일부러 재현 안 함 — 리파이된 인터페이스 자신의
동작과 무관하고(이 witness의 어떤 fixture도 `.meta`를 호출 안 함),
"바이트코드 모양이 아니라 동작 동등성" 기준 그대로 적용. 구현:
`InterfaceName`을 analyze 시점에 `Class/forName`으로 풀고(fully-qualified
하나만, 다중 인터페이스는 범위 밖), 각 `(method [this args...] body)`를
이름+arity로 real 인터페이스의 reflected method와 매칭. **파라미터가
primitive면 거부**(진입 시 auto-boxing이 필요해 이 witness의 균일한
`Object` local 처리와 안 맞음, 시도 안 함) — 하지만 **반환타입이
primitive인 건 지원**(`Comparator/compare`의 `int`처럼, body가 항상
만드는 boxed 값을 return 지점에서 unbox — 훨씬 흔하고 필요한 경우라
로컬 처리 메커니즘을 안 건드리고 return 지점 하나만 다루면 됨).
클로저 캡처(필드+생성자) 메커니즘을 그대로 재사용, `this`는 named
`fn`의 self-reference와 같은 방식(`:kind :self`, `this` 로드)으로
바인딩. `java.util.function.Function`(전부 Object, capture 있음/
없음), `java.util.Comparator`(primitive int 반환, 진짜 real
`sort`한테 넘겨서 정상 작동하는 것까지 확인), `java.lang.Runnable`
(void 반환, side-effect로 검증) 전부 실제 host 대비 대조 검증.
primitive 파라미터 거부, 다중 인터페이스 거부 둘 다 명확한 에러로
확인. compiler.clj도 `reify` 지원 확인 후 DDC 행에 연결. U6: 197→202.
DDC 행: 114→116. 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 서른두 번째 확장 (2026-08-15): `deftype`(필드만, protocol/
인터페이스 구현 없음).** `javap -p -c`로 protocol 구현 없는 최소
`(deftype Point [x y])`를 확인하니 그냥 public final 필드 2개 + 그
값을 필드에 저장하는 생성자뿐 — real host가 항상 추가하는
`clojure.lang.IType`(마커 인터페이스)와 `getBasis`(reflection 헬퍼)는
관찰 가능한 필드/생성 동작에 영향 없어 재현 안 함.

`deftype`은 (closure/reify와 달리) 표현식이 아니라 컴파일 단위 전체에
묶인 이름 있는 클래스를 정의하므로, `fn` body 안에 중첩될 수 없음 —
그래서 `compile-source`에 완전히 새로운 top-level 진입 형태
`(do (deftype Name [field...])... (fn ...))`를 추가(기존 "fn 하나만
컴파일" 경로는 100% 그대로 유지, 이 형태일 때만 새 경로를 탐).
앞쪽 `deftype`들을 먼저 analyze+emit해서 `{이름 Class}` registry를
만들고, dynamic var(`*known-deftype-classes*`, real host 자신도
`*ns*` 같은 컴파일 범위 dynamic state를 쓰는 것과 같은 방식)로 이
registry를 뒤쪽 `fn`의 analysis에 전달 — `fn` 안의 `(Name. args...)`가
이 registry를 찾아 컴파일타임에 알려진 타입으로 직접
`NEW/DUP/INVOKESPECIAL`(real host와 같은 모양, Reflector 안 씀,
컴파일타임에 Class를 이미 알기 때문). 필드 접근(`.-x`)은 이미 있는
일반 reflection 기반 `.-fieldName` 메커니즘으로 새 코드 없이 그냥
됨. 2필드/3필드/독립된 두 타입 조합 전부 실제 host 대비 검증(단,
real host 자신도 `(do (deftype ...) (fn ...))`를 **하나의 문자열로
eval**하면 "class not found"로 똑같이 실패함을 확인 — deftype은
실제로 별도 컴파일 단위여야 하는 게 real host의 진짜 제약이라, 내
witness의 접근(앞쪽 deftype을 먼저 별도로 emit)이 오히려 그 진짜
순서를 정확히 반영한 것. `compiler.clj`도 이 do-wrap 문자열 하나로는
같은 이유로 실패 확인 — 그래서 이 fixture들은 U6 전용으로 남기고 DDC
행에는 연결 안 함(기존에도 shared-mutable-arg 등으로 U6 전용 처리한
전례와 같은 패턴). U6: 202→206. DDC 행 변화 없음(114→114, deftype은
연결 불가능). 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**같은 날 서른세 번째 확장 (2026-08-15): `defprotocol` + protocol method
dispatch.** 남은 마지막 큰 항목. `javap -p`로 `(defprotocol Greet
(greet [this]))`를 확인하니 real host는 public 인터페이스(메서드
하나당 abstract 메서드 하나, `this` 제외하고 전부 Object 타입 — 매칭할
기존 타입이 없어서 `reify`처럼 reflection이 필요 없음)를 생성한다.
프로토콜 메서드 호출의 **fast path**(`javap -c` 확인: 인자가 생성된
인터페이스를 직접 구현하면 그냥 `checkcast Interface; invokeinterface`)
만 재현 — real host의 **full** 메커니즘은
`AFunction.__methodImplCache`/`clojure.core/-cache-protocol-fn`으로
`extend-protocol`이 등록한 임의 타입(예: `java.lang.String`)까지
디스패치하는데, 이건 Clojure 프로토콜 런타임의 상당 부분을 재구현하는
셈이라 이번엔 시도 안 함 — 이 witness의 프로토콜 메서드 호출은
`reify`(추후 확장되면 `deftype`)로 직접 구현한 값에서만 동작.
구현: `defprotocol`을 `deftype`과 같은 top-level 진입 형태(`(do
(deftype/defprotocol ...)... (fn ...))`)의 세 번째 leading-form
종류로 추가 — `deftype-program-form?`를 `top-level-program-form?`로
일반화해서 `deftype`/`defprotocol` 혼합도 허용. 프로토콜 메서드
이름은 `(methodName instance args...)` 호출 시점에 새 dynamic var
registry(`*known-protocol-methods*`)에서 찾아 고정된 special form으로
컴파일. `reify`의 인터페이스 해석도 확장해서 `*known-protocol-interfaces*`
registry를 `Class/forName`보다 먼저 확인 — `(reify ProtocolName ...)`가
같은 프로그램에서 먼저 정의된 프로토콜을 구현할 수 있게 함. 단일
메서드/인자 있는 메서드+캡처/2-메서드 프로토콜/deftype과 혼합 전부
실제 host 대비 검증(단, `deftype`과 마찬가지로 `do`-wrap 단일 문자열
eval은 real host에서도 안 되는 케이스가 있어 — 여기선 real host 자체
eval은 되지만 `compiler.clj`가 실패해서 — U6 전용, DDC 행 미연결).
U6: 206→210. DDC 행 변화 없음(114 유지). 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY`) 녹색, 회귀 없음.

**이 슬라이스 도중 인프라 사고 하나 발견·해결**: 전체 게이트의
`reproducible DDC lane`(clj-meta 자신과 무관한, **stock Clojure
1.12.5 자체**의 Maven 7단계 재현빌드 증거, `:promotion/allowed?
false`로 이미 명시된 선택적 레인)이 `clj-meta/proof/stage-chain.receipt.edn`
누락으로 실패해서 게이트 전체가 `NOT READY`로 떨어졌던 걸 발견. 이
파일을 만드는 코드는 clj-meta/pnix-clj 어디에도 없음을 git 이력
전체·삭제 이력까지 확인했고, `~/pnix-zero`라는 완전히 별도 리포에서
진짜 `stage7-gate.sh`(Maven으로 Clojure 1.12.5를 7번 재빌드하고 jar
내용을 서로 비교)를 찾아 **실제로 재실행**해서 새로 계산된 진짜
증거(다이제스트 `258fdb97...`, 이전 값과 다름 — 새로 계산됐다는 뜻)로
복구. 가짜로 채우는 대신 실제 빌드를 끝까지 기다림 — 이 프로젝트
자신의 "정직한 부재 증거, Held 금지 fabrication" 원칙을 그대로 적용한
사례.

**같은 날 서른네 번째 확장 (2026-08-15): `deftype` + protocol 결합.**
사용자가 남은 문제들을 계속 해결하라고 지시. `javap -p -c`로
`(deftype Rect [w h] Shape (area [this] (* w h)))`를 확인하니 `Rect
implements Shape, IType`이고, `area()` 안에서 `w`/`h`는 정확히
closure/`reify` 캡처와 **완전히 같은** `this.fieldName`(`aload_0;
getfield`) 방식으로 읽힌다 — 차이는 `deftype`의 필드가 자유변수
분석 없이 **항상 무조건** 존재한다는 것뿐(top-level 정의라 캡처할
바깥 렉시컬 스코프 자체가 없음). `analyze-reify-method`를 그대로
재사용(메서드/인터페이스 매칭, primitive 파라미터 거부, body 분석
전부 동일 로직) — 넘기는 env만 "바깥 스코프" 대신 "deftype 자신의
필드 목록"으로 바뀜. `emit-deftype-class`를 확장해서 `implements`
절과 메서드 바디를 추가로 방출(`emit-reify-class`와 같은 패턴,
필드가 캡처 자리를 대신함).

**구현 중 진짜 버그 하나 발견·수정**: `emit-leading-program-forms`가
앞쪽 top-level form들을 처리하는 동안 `*known-protocol-interfaces*`
등 dynamic var를 전혀 바인딩 안 하고 있었다 — 그래서 `deftype Rect
... Shape ...`처럼 **앞서 정의된 protocol을 뒤쪽 leading form이
바로 참조**해야 하는 경우("interface not found") 실패. 원래
`reduce` 기반이었는데, `binding`은 내부적으로 `try`/`finally`로
확장되고 `recur`는 `try` 경계를 못 건너뛰므로 `loop`+`recur`로는 이
"매 스텝마다 재바인딩" 패턴을 못 짬 — 일반 (꼬리재귀 아닌) 재귀
호출로 바꿔서 해결(앞쪽 선언 개수가 항상 적어서 스택 문제 없음).
단일 메서드 protocol 구현/2-메서드 구현/필드 전용 deftype 회귀 없음
전부 실제 host 대비 검증(U6 전용, DDC 행 미연결 — deftype과 같은
이유). U6: 210→212. DDC 행 변화 없음(114 유지). 전체
`-M:conformance`(116/116)와 `bin/clj-meta-gate`(`metacircular gate:
READY`, `reproducible DDC lane: OK`) 녹색, 회귀 없음.

**같은 날 서른다섯 번째 확장 (2026-08-15): closure 중첩 깊이 제한 제거
(1단계 → 무제한).** "남은 문제들 해결해봐" 지시에 따라 남은 4개 갭
중 첫 번째 착수. `javap -c`로 host-AOT-컴파일된 `(fn [x] (fn [y] (fn
[z] (+ x (+ y z)))))`를 확인하니 **전이적 캡처(transitive
capture)**가 필요함을 확인 — 중간 closure(`y`를 받는 fn)가 자기
body에서는 전혀 참조하지 않는 `x`를, 오직 안쪽 closure의 생성자에
넘기기 위해서만 캡처해야 함. `ast-referenced-names`에 `:op
:closure` 노드를 특별 취급하는 case 추가(자신의 `:captures`를
바깥 스코프가 "참조"하는 것으로 간주하되 `:body`는 재귀하지
않음) — 이 한 가지 변경만으로 기존 자유변수 계산 메커니즘이
임의 깊이에서 그대로 작동. `analyze-nested-fn`의 depth-1 초과 시
throw하던 가드를 제거. depth 2(`(((f 1) 2) 3)` → `6`)와 depth
4(`((((f 1) 2) 3) 4)` → `10`) 양쪽 모두 실제 host 대비 검증,
기존 단일-레벨 closure fixture 전부 회귀 없음(standalone U6 체크
전부 OK). `compiler.clj`(DDC 2번째 레그)도 double-nested 패턴을
동일하게 지원함을 먼저 확인한 뒤 DDC 행에 연결. U6:
212→214(`:tiny-closure-double-nested-transitive-capture`,
`:tiny-closure-quadruple-nested`). DDC 행: 114→115
(`:mini-backend-closure-double-nested-transitive-capture`). 전체
`-M:conformance`(116/116)와 `bin/clj-meta-gate`(`metacircular
gate: READY ✅`, `reproducible DDC lane: OK`) 녹색, 회귀 없음.

**같은 날 서른여섯 번째 확장 (2026-08-15): `letfn` 진짜 상호재귀
(mutual recursion).** 남은 4개 갭 중 두 번째. `javap -c`로
host-AOT-컴파일된 `(letfn [(my-even? [x] ...) (my-odd? [x]
...)] ...)`를 확인: 각 바인딩이 자기만의 closure 클래스로
컴파일되는 건 같지만, 상대를 가리키는 필드가 `final`이 아님 —
바깥 메서드가 먼저 모든 바인딩의 local 슬롯을 `null`로 초기화한
뒤, 작성 순서대로 각 closure를 생성자 호출로 만들면서(아직
안 만들어진 앞쪽 참조는 그 시점 슬롯 값 그대로, 즉 `null`을
넘김) 슬롯에 저장하고, **전부 다 만들어진 뒤에야** 형제를
참조하는 모든 필드를 진짜 인스턴스로 `putfield` 되짚어 채움
(backpatch) — 참조 그래프가 순방향/역방향/순환 어느 쪽이든
이 2단계(생성 → backpatch) 하나로 전부 처리되고, 위상 정렬이
전혀 필요 없음. 매크로 전개 단계의 `raw-form-contains-symbol?`
스캔으로 상호참조 여부를 미리 판정해서, 상호참조가 없으면
기존의 저비용 nested-`let` desugar(`expand-letfn`)를 그대로
타고, 상호참조가 있으면 desugar하지 않고 `letfn` 형태를 그대로
analyzer까지 보존(`analyze-letfn-mutual`)해서 진짜 closure-field
배선을 새로 함(`emit-letfn`) — 기존 단일/자기재귀 binding
fixture는 전혀 건드리지 않는 구조. 자기 자신을 부르는
바인딩은 기존 named-fn 메커니즘 그대로 `this`로 처리(캡처
필드가 아예 안 생김). `emit-class`에 `:final-captures?`
옵션을 추가해 letfn 전용 클래스만 필드를 non-final(+public
— 각 `emit-class` 호출이 독립된 `DynamicClassLoader`를 새로
띄우기 때문에, backpatch를 수행하는 제3의 클래스가 접근하려면
package-private로는 부족해서 `IllegalAccessError`로 실제
확인 후 `public`으로 조정)로 선언. 2-way/3-way 상호재귀, 자기재귀
+ 형제 혼합, 바깥 스코프 캡처 + 형제 혼합 4가지 조합 전부 실제
host 대비 검증(값 일치). U6: 213→216(`:tiny-letfn-mutual-recursion`,
`:tiny-letfn-mutual-recursion-three-way-cycle`,
`:tiny-letfn-self-recursion-plus-mutual-sibling`). `compiler.clj`는
이미 이 패턴을 지원하고 있었음(별도 `fixtures`의
`:letfn-mutual-recursion`이 기존에도 accepted였음)을 확인한 뒤
DDC 행에 U6측 fixture 연결(`:mini-backend-letfn-mutual-recursion`,
`mini-backend-ddc-fixtures` 120→121). 전체
`-M:conformance`(116/116)와 `bin/clj-meta-gate`(`metacircular
gate: READY ✅`, `reproducible DDC lane: OK`) 녹색, 회귀 없음.

**이 슬라이스 도중 발견한 사소한 기록 오차**: 위 숫자들을 이번엔
`(count specs)`/`(count (mini-backend-ddc-fixtures))`로 직접
재확인했는데, 서른다섯 번째 확장 문단이 적어둔 "U6 212→214·DDC
행 114→115"가 실제 커밋된 값(213→214는 맞지만 시작점 212가 아니라
211, DDC 행도 114가 아니라 119)과 어긋나 있었음 — 게이트/영수증
자체는 항상 실측값으로 돌아가서 전혀 영향 없고(READY/OK는 코드
실행 결과지 이 산문 숫자에서 나오는 게 아님), 순수히 산문 기록의
누적 오프-바이-원 오차. 과거 커밋 메시지까지 소급 정정하지는
않되, 지금부터는 매 슬라이스마다 실측 `count`로 재확인.

**같은 날 서른일곱 번째 확장 (2026-08-15): `reify`/`deftype` 메서드의
PRIMITIVE 파라미터 지원.** 남은 4개 갭 중 세 번째. `javap -c`로
host-AOT-컴파일된 `(reify java.util.function.IntUnaryOperator
(applyAsInt [this x] (+ x n)))`를 확인: `applyAsInt(int)`는 실제로
`int` 그대로 받되(`iload_1`), 리턴 쪽의 `coerce-reify-return!`가
이미 하고 있던 것과 정반대 방향의 처리를 진입부에서 함 — 즉
파라미터를 그대로 두는 게 아니라, 이 위트니스의 "본문은 전부
Object" 가정과 맞추려면 메서드 진입 시점에 그 자리에서 박싱해서
평범한 이미 박싱된 local처럼 취급하면 충분함(리턴 쪽 `unbox`와
정확히 대칭). `emit-reify-class`/`emit-deftype-class`에 완전히
동일하게 중복돼 있던 "메서드별 GeneratorAdapter 구성 + arg-env
구성" 루프를 `emit-reified-methods!` 하나로 통합하면서, PRIMITIVE
타입 파라미터만 새 local(`{:kind :let :slot ...}`)에 박싱해서 담고
REFERENCE 타입 파라미터는 기존 `{:kind :arg ...}` 그대로 유지하는
분기를 추가. `analyze-reify-method`의 프리미티브 파라미터 거부
가드를 제거. `IntUnaryOperator`(파라미터 1개)/`IntBinaryOperator`
(2개) 양쪽, 그리고 `deftype`이 실제 Java 인터페이스를 구현하며
프리미티브 파라미터 메서드를 갖는 조합까지 전부 실제 host 대비
검증. U6: 216→219(`:tiny-reify-primitive-int-param`,
`:tiny-reify-two-primitive-int-params`,
`:tiny-deftype-implements-java-interface-primitive-param`).
`compiler.clj`도 지원함을 먼저 확인한 뒤 reify(단독, deftype
아님) fixture 하나를 DDC 행에 연결(`mini-backend-ddc-fixtures`
121→122, 실측 확인). 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY ✅`, `reproducible
DDC lane: OK`) 녹색, 회귀 없음.

**같은 날 서른여덟 번째 확장 (2026-08-15): multi-interface `reify`.**
남은 4개 갭 중 마지막으로 다룬 것(protocol의 진짜
`extend-protocol` 디스패치는 여전히 별도 대형 프로젝트로 held —
아래 참고). `javap -p -c`로 host-AOT-컴파일된 `(reify Runnable
(run [this] ...) Callable (call [this] ...))`를 확인: 클래스가
`implements Callable, Runnable, IObj`로 두 인터페이스를 그냥
나란히 선언 — `deftype`이 여러 protocol/interface를 구현할 때
이미 하고 있던 것과 완전히 같은 모양. 그래서 `deftype`의
`parse-deftype-impl-groups`(교대로 나오는 심볼/메서드-폼 그룹
파싱)를 그대로 재사용해서 `reify`도 `(reify Iface1 (m1 ...)
Iface2 (m2 ...) ...)` 형태를 받도록 확장 — `analyze-reify`가
단일 `iface-sym`+평평한 method-forms 대신 그룹 목록을 분석하고,
모든 그룹의 메서드를 합쳐서 캡처 계산(자유변수 분석)을 한 번에
수행. 인터페이스 해석 로직이 `reify`/`deftype` 양쪽에 거의
동일하게 중복돼 있던 것도 `resolve-reify-interface`로 통합.
`emit-reify-class`는 `:interface`+`methods` 대신 `:impls`를 받아
`implements` 절에 여러 인터페이스를 나열(`emit-deftype-class`와
같은 패턴). 2개 인터페이스, 각 인터페이스 메서드가 서로 독립적으로
동작하는지, 그리고 multi-interface + primitive 파라미터 조합까지
전부 실제 host 대비 검증. U6: 219→222(`:tiny-reify-two-interfaces`,
`:tiny-reify-two-interfaces-both-methods-independent`,
`:tiny-reify-two-interfaces-with-primitive-param`). `compiler.clj`도
지원 확인 후 DDC 행 연결(`mini-backend-ddc-fixtures` 122→123,
실측 확인). 전체 `-M:conformance`(116/116)와
`bin/clj-meta-gate`(`metacircular gate: READY ✅`, `reproducible
DDC lane: OK`) 녹색, 회귀 없음.

**같은 날 서른아홉 번째 확장 (2026-08-15): `extend-protocol`(진짜
protocol 디스패치).** 사용자가 "그래~ 계속 해결해봐"로 명시적으로
지시. 이전까지 held로 분류하던 마지막 큰 항목. `javap -c`로
host-AOT-컴파일된 `(extend-protocol Shape Long (area [this] ...))
... (area n)` 호출부를 확인: 실제로는 두 부분 — (1) call-site
자체는 단순 fast-path(`instanceof Interface; checkcast;
invokeinterface`)가 fall-through로 그친 뒤, interface를 직접
구현하지 않는 값이면 protocol method의 Var 자체를 평범한 IFn처럼
호출, (2) 그 Var의 root function 안에서 진짜
`MethodImplCache`/`extend`-등록 룩업이 일어남 — call site 자체는
간단하지만 (2)는 정말 Clojure protocol runtime의 상당 부분 재구현이
필요함을 재확인. **다만 `extend-protocol`의 대상 클래스들은 소스
안에 심볼로 나열돼 COMPILE TIME에 이미 다 알려져 있다**는 점을
이용해서, 런타임 캐시 대신 컴파일타임에 확정된 `instanceof`
체인(인터페이스 fast path → 각 확장 클래스를 선언 순서대로 검사 →
실패 시 명확한 에러)으로 **관찰 가능한 동작을 그대로** 재현 —
이 파일 전체에서 일관되게 써온 "행동 동등성, 바이트코드-모양
동등성 아님" 기준을 여기에도 그대로 적용한 것. `deftype`이 이미
가진 `parse-deftype-impl-groups`(교대 심볼/메서드-폼 그룹 파싱)를
`extend-protocol`에도 재사용. `analyze-extend-protocol-form`이
새 leading top-level form으로 추가(`deftype`/`defprotocol`과
같은 자리, 새 클래스는 전혀 만들지 않고 `*known-protocol-extensions*`
레지스트리만 채움). `emit-protocol-call`을 재작성: `instance`/각
`arg`를 딱 한 번만 평가해 local에 저장(여러 `instanceof` 검사와
실제 실행 양쪽에서 재사용해야 하므로) → interface fast path 검사
→ 확장된 각 클래스 검사(맞으면 그 클래스의 컴파일된 메서드
본문을 `emit-expr`로 그 자리에서 직접 실행, `this`/인자는
`emit-let`이 이미 쓰는 것과 같은 평범한 `:let` local로 해석) →
전부 실패하면 `IllegalArgumentException`을 명확한 메시지로 throw
(예전엔 무조건 `checkcast`라서 미확장 타입에 대해 그냥
`ClassCastException`이 났음 — 이것도 관찰 가능한 동작의 진짜
개선). 단일 확장 클래스, 두 확장 클래스, 확장된 클래스와 `reify`
fast path가 같은 프로그램에서 섞여 쓰이는 경우, 미확장 타입에
대한 에러(타입까지 실제 host와 정확히 `IllegalArgumentException`
일치)까지 전부 실제 host 대비 검증. `compiler.clj`는 do-wrapped
멀티폼 프로그램 모양 자체를 못 받아들임을 재확인(`deftype`/
`defprotocol` 조합 때와 정확히 같은 이유) — 그래서 U6 전용,
DDC 행 미연결. U6: 222→225(`:tiny-extend-protocol-single-class`,
`:tiny-extend-protocol-two-classes`,
`:tiny-extend-protocol-and-reify-mixed-dispatch`). 전체
`-M:conformance`(116/116), `-M:diverse-double-compile`(영향
없음, `extend-protocol`은 DDC 행에 없으므로 당연)과
`bin/clj-meta-gate`(`metacircular gate: READY ✅`, `reproducible
DDC lane: OK`) 녹색, 회귀 없음.

**진짜로 남은 것(narrow, 문서화된 채로):** `extend-protocol` 대상
클래스는 `java.lang.Long`처럼 완전정규명이어야 함(real host처럼
`Long` 같은 `java.lang.*` 축약명 자동 해석은 없음 — 이 파일
전체에서 이미 일관된 "정직하게 더 좁은 범위" 원칙). 여러
`extend-protocol` 확장이 서로 겹치는 클래스 계층(부모/자식
클래스 둘 다 확장)일 때의 우선순위는 선언 순서 first-match일 뿐,
real host의 더 정교한 최적-매치 규칙과 다를 수 있음 — 현재
fixture들은 이 애매한 경우를 만들지 않음.

**같은 날 마흔 번째 확장 (2026-08-15): 중첩 closure의 multi-arity
지원.** 사용자가 "문제점있는것을 찾아서 해결해봐"로 지시. 먼저
이번 세션에 새로 만든 `letfn`/`reify`/`deftype`/`extend-protocol`
코드 자체에 실제 버그가 있는지 실제 host 대비로 교차 조합
테스트(nested closure 안에서 extend-protocol 디스패치, reify/
deftype 메서드 안에서 protocol 호출, side-effect가 있는 인자가
정확히 한 번만 평가되는지 등)부터 먼저 확인 — 전부 실제 host와
일치, 새 버그는 못 찾음. 그래서 남은 좁은 gap 중 실제로 액션
가능한 것(중첩 closure의 multi-arity 제한)을 다음으로 착수.
`javap -c`로 host-AOT-컴파일된 `(let [g (fn ([] x) ([y] (+ x
y)))] ...)`를 확인: 클래스 하나, 캡처 필드 하나(`x`, 양쪽 arity가
공유), `invoke()`/`invoke(Object)` 두 override — 이미 top-level
`fn`이 쓰고 있던 `emit-class`의 multi-arity 메커니즘과 완전히
동일. `analyze-fn`과 `analyze-nested-fn`에 각각 따로 있던 arity
검증 로직(최소 1개, 고정 arity 중복 없음, variadic 최대 1개,
고정 arity가 variadic보다 파라미터 많으면 안 됨)을
`validate-arities!`로 통합. **구현 중 발견한 진짜 주의점**: 캡처
계산을 여러 clause를 합친 하나의 "own names" 집합으로 하면 안 됨
— clause A의 파라미터가 outer 스코프의 어떤 이름을 가리는데
(shadow) clause B(그 파라미터가 없는)가 바로 그 이름을 outer에서
캡처해야 하는 경우, 합쳐서 계산하면 clause B의 진짜 캡처가
빠짐. 각 clause의 캡처를 독립적으로 계산한 뒤 합집합을 취하는
방식으로 수정 — `(fn ([y] (+ y 1)) ([] (+ y 100)))`(바깥 `y`=7)
케이스로 실제 host 대비 검증해서 이 수정이 꼭 필요함을 확인
(`[2 107]` 일치). `emit-closure`는 `arities`를 그대로
`emit-class`에 넘기도록 단순화(기존 multi-arity 메커니즘 재사용,
새 바이트코드 없음). 공유 캡처, variadic+고정 혼합, self-recursion
(clause 간 재귀 호출), transitive capture와의 조합, shadowing
엣지 케이스까지 전부 실제 host 대비 검증. U6:
225→229(`:tiny-closure-multi-arity-shared-capture`,
`:tiny-closure-multi-arity-variadic`,
`:tiny-closure-multi-arity-self-recursive`,
`:tiny-closure-multi-arity-per-clause-capture-shadowing`).
`compiler.clj`도 지원 확인 후 DDC 행 연결(123→124, 실측 확인).
전체 `-M:conformance`(116/116)와 `bin/clj-meta-gate`(`metacircular
gate: READY ✅`, `reproducible DDC lane: OK`) 녹색, 회귀 없음.

**같은 날 마흔한 번째 확장 (2026-08-15): `extend-protocol`/`reify`의
남은 좁은 제약 두 가지 해결.** 사용자가 "남은것빨리 해결해봐"로
지시 — 직전 슬라이스에서 "narrow, 문서화된 채로 유지"라고 적어둔
바로 그 두 항목. **(1) 축약 클래스명**: `resolve-class-name`을
새로 만들어 `Class/forName`이 실패하고 이름에 점이 없으면
`java.lang.` 접두어를 붙여 재시도(real host의 `java.lang.*`
기본 import를 흉내 — 다른 패키지까지 일반화한 게 아니라
`general-static-interop-target`이 이미 쓰던 것과 같은 좁은
스코프). `resolve-reify-interface`와 `analyze-extend-protocol-form`
양쪽의 클래스 해석을 이걸로 통합. **(2) 클래스 계층 우선순위**:
실제 host 대비로 먼저 버그를 재현 확인 — `(extend-protocol Shape
Number (area [this] :number) Long (area [this] :long))`에서 real
host는 `Long` 값에 대해 선언 순서와 무관하게 `:long`(더
구체적인 클래스)을 고르는데, 이 witness는 선언 순서 그대로
`:number`를 골라 실제로 값이 달랐음(수정 전 재현: `:number`,
버그 확정). `sort-extensions-by-specificity`를 추가해서
`known-protocol-method`가 `:extensions`를 반환할 때
`.isAssignableFrom` 기반으로 서브타입을 슈퍼타입보다 먼저
오도록 정렬(Clojure `sort`는 안정 정렬이라 무관한 클래스끼리는
원래 선언 순서 유지). 슈퍼클래스 먼저/서브클래스 먼저 두 선언
순서 모두, 그리고 확장 안 된 타입이 여전히 슈퍼클래스로 올바르게
fallback하는 경우까지 전부 실제 host 대비 검증(수정 후 전부
일치). U6: 229→234(`:tiny-extend-protocol-bare-java-lang-names`,
`:tiny-reify-bare-java-lang-interface-name`,
`:tiny-extend-protocol-hierarchy-most-specific-wins`,
`:tiny-extend-protocol-hierarchy-declaration-order-independent`,
`:tiny-extend-protocol-hierarchy-falls-to-superclass`). `reify`
축약명 fixture는 단일 폼이라(do-wrap 불필요) `compiler.clj`도
지원 확인 후 DDC 행에 연결(124→125, 실측 확인) — extend-protocol
쪽은 여전히 do-wrapped 멀티폼이라 U6 전용 유지. 전체
`-M:conformance`(116/116), `-M:diverse-double-compile`과
`bin/clj-meta-gate`(`metacircular gate: READY ✅`, `reproducible
DDC lane: OK`) 녹색, 회귀 없음. 이걸로 이번 세션에서 찾은
좁은/실제-액션-가능 gap 전부 닫힘.

**같은 날 마흔두 번째 확장 (2026-08-15): U6를 전체 conformance
corpus(116개)와 처음으로 교차 검증 — 진짜 pre-existing 버그
하나 발견·수정.** 사용자가 "그래 잘 적어놓고 해야할일들해봐"로
지시. U6(`frontend_selfhost.clj`)는 지금까지 자기 자신의
hand-maintained `specs`(243개)만으로 검증돼 왔고, `conformance.clj`의
116-case corpus(host/compiler.clj/kernel.clj 3-way)에는 **한
번도 연결된 적이 없었다** — 이번에 처음으로 U6의 `compile-source`를
그 116개 케이스 전체에 직접 돌려서 교차 검증(스크래치 스크립트,
아직 영구 게이트로 연결 안 함). 결과: 85/116 일치, 나머지는
대부분 정직하게 스코프 밖인 기능(`def`/top-level 부작용, `instance?`,
`var`, `defrecord`, `StringBuilder` 등 미등록 클래스,
`clojure.core.protocols/coll-reduce`, `(set! (. p x) v)` 형태의
interop set!, 콤마 있는 map 리터럴 등) — 그런데 **그 중 정확히
2개는 기능 부재가 아니라 진짜 `VerifyError` 크래시**였다.

**근본 원인**: JVM 예외 핸들러는 항상 "빈 operand stack + 잡힌
예외" 하나로만 시작한다(JVM 스펙 자체의 불변조건) — 그런데 이
witness의 여러 emit 함수가 "값 하나를 스택에 남겨둔 채로 또 다른
`emit-expr`를 중첩 호출"하는 패턴을 쓰고 있었다(`emit-object-array`가
배열 참조+인덱스를 스택에 남긴 채 각 원소를 emit, `emit-binary`가
lhs를 스택에 남긴 채 rhs를 emit, 등). 그 중첩된 서브식이 `try`를
포함하면: 정상 경로는 "먼저 남겨둔 값 + try 결과"가 스택에 같이
있지만, 예외 경로(핸들러 진입)는 JVM이 스택을 통째로 비워버려서
"먼저 남겨둔 값"이 사라진 채로 합류 — 두 경로의 스택 모양이
실제로 달라져서 verifier가 정당하게 거부(`(fn [] [(try 1 (catch
Exception e 2)) 99])`로 최소 재현, 실제 host는 `[1 99]`를 정상
반환). 이번 세션에서 새로 만든 코드가 원인이 아니라, 원래부터
있던 잠재 버그 — `try`가 비-tail 위치의 raw 서브식으로 쓰이는
조합을 그동안 어떤 fixture도 우연히 안 건드렸을 뿐. **일반적이고
견고한 수정**: 어떤 값도 중첩 `emit-expr` 호출을 가로질러 raw
operand stack에 남겨두지 않는다 — 계산 직후 즉시 local에 저장하고,
그룹의 모든 값이 안전하게 계산된 뒤에만 local에서 다시 로드.
`stack-safe!`/`emit-exprs-stack-safe!` 공용 헬퍼를 추가하고
아래 14개 emit 함수 전부에 이 원칙을 기계적으로 적용:
`emit-binary`, `emit-get3`, `emit-object-array`(vector/map/set
리터럴이 전부 이걸 통해 나감), `emit-interop-call`,
`emit-static-interop-call`, `emit-general-static-interop-call`,
`emit-general-new`, `emit-new`, `emit-core-fn-call`,
`emit-local-fn-call`, `emit-computed-fn-call`, `emit-field-set`,
`emit-list`, `emit-binding`, `emit-deftype-new`. (`emit-let`/
`emit-loop`/`emit-recur`/`emit-do`/`emit-locking`/`emit-try`
계열/`emit-protocol-call`은 감사 결과 이미 안전한 패턴이었음 —
값을 계산 직후 바로 local에 저장하거나, 여러 서브식이 겹치지
않게 순차 소비.) `try`를 vector/list/binary-op-rhs/interop-call
인자/core-fn-call 인자/local-fn-call 인자/deftype 생성자 인자
안에 raw로 넣는 9가지 조합 전부 실제 host 대비로 검증(수정 전
2개는 크래시 재현, 9개 전부 수정 후 host와 정확히 일치).
회귀 없음(기존 243개 fixture 전부 그대로 통과 — 대부분 이
14개 함수를 거치는데도 관찰 가능한 동작은 100% 동일, 순수
안전성 리팩터). U6:
234→243(`:tiny-try-inside-vector-literal`,
`:tiny-try-inside-vector-literal-no-finally`,
`:tiny-try-inside-vector-literal-exceptional-path-with-finally-counter`,
`:tiny-try-inside-binary-op-rhs`, `:tiny-try-inside-list-literal`,
`:tiny-try-inside-interop-call-arg`,
`:tiny-try-inside-core-fn-call-arg`,
`:tiny-try-inside-local-fn-call-arg`,
`:tiny-try-inside-deftype-new-arg`). `compiler.clj`는 애초에
이 버그가 없었음(진짜 tools.analyzer.jvm/ASM 경로라 서브식
평가 순서/스택 관리가 다름) — 대표 fixture 하나를 DDC 행에
연결(125→126, 실측 확인). 전체 `-M:conformance`(116/116),
`-M:diverse-double-compile`과 `bin/clj-meta-gate`(`metacircular
gate: READY ✅`, `reproducible DDC lane: OK`) 녹색, 회귀 없음.

**진짜로 남은 것**: U6-vs-conformance-corpus 교차검증은
`scratch/u6_vs_conformance_corpus.clj`로 리포에 보관(수동으로
`clojure -M -e '(load-file "scratch/u6_vs_conformance_corpus.clj")'`
실행 — `bin/clj-meta-gate`/`-M:conformance`/`-M:diverse-double-compile`
어느 것도 이 파일을 안 건드림, 의도적으로 영구 게이트 아님). `via-u6`를
`conformance.clj`에 정식으로 추가할지, 아니면 이대로 수동 도구로
둘지는 다음 결정 사항. 나머지 29개 미매치 케이스는 개별적으로 스코프
판단(기능 추가 대상 vs 의도적 held)이 필요하며 아직 안 함.

**아직 진정으로 열린 것:** full Wheeler DDC는 독립 backend 커버리지가 43-fixture
부분 집합이 아니라 *production* corpus와 맞아야 하고, (더 어렵게)
behavior-identical이 아니라 bit-identical 출력이 필요합니다 — 우연히 같은
바이트코드 형식을 겨냥한 두 다른 컴파일러 backend는 정직한 기준이 아니며,
행동 동등성이 기준입니다. U5의 kernel도 여전히 인터프리터이지 두 번째
컴파일러가 아니므로 이 주장에 독립적으로 기여하지 않습니다.

**다음 구체 단계:** `frontend_selfhost.clj`의 아직 미연결 남은 ~8 fixture
(이미 커버된 범주와 겹치는 것, 예: 단독 `>`/`>=`/`<=`, 단독 `inc`/`dec`/`pos?`)
로 `mini-backend-ddc-fixtures`를 더 넓혀 완결성을 채운 뒤, U6 자체 fixture
세트를 51을 넘어 U5가 이미 도달한 전체 112-case conformance corpus 쪽으로
키우기 — 그 시점에서 주장이 "43-fixture 부분 집합"에서 "full
conformance-corpus 독립 2nd 컴파일러 교차 검증"으로 올라가며, 이 corpus에 대한
실제 Wheeler 기준이 됩니다. Full 컴파일러-바이너리 DDC(bit-identical
바이트코드, 행동만이 아님)는 그 위의 더 어려운 기준으로 명시적으로 held; 대조
검증한 CakeML/CompCert/Octagon 연구 트레일은 `todo.md` U10 참고.

## 주 게이트

```sh
# From pnix-clj/clj-meta/
./bin/clj-meta-gate              # full :gate integrated receipt
./bin/clj-meta-gate selfhost     # practical peer floor (bytecode selfhost)
./bin/clj-meta-gate stage7       # stock rebuild (needs mvn on PATH)
```

## 마지막 실행 (이 머신, 2026-08-07)

| 게이트 | 결과 | 메모 |
|---|---|---|
| `./bin/clj-meta-gate selfhost` | **PASS** | ready=true |
| `./bin/clj-meta-gate stage7` | **PASS** | Maven 3.9.12 |
| `./bin/clj-meta-gate primary` | **READY ✅** | stage11–N + DDC + full-source closed |
| env | JDK 21, Clojure 1.12.5 CLI, Maven 3.9.12 | OK |
