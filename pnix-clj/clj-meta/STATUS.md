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
    88 fixtures (fn/if/do/let/loop-recur/arithmetic/compare/data-literals/
    quote/14 macros incl. `case` with a real no-default throw/vector-
    destructuring/fixed multi-arity fn/variadic `&` rest-args/count/
    single-clause `try`/`catch`, single-clause `try`/`finally`/`throw`/
    allowlisted exception construction/`.methodName` instance interop and
    `ClassName/methodName` static interop, both via `clojure.lang.Reflector`)
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
