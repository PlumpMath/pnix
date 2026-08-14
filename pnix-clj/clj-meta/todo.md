# clj-meta TODO / 연속성 노트 (continuation note)

기준일: 2026-06-28 KST

이 문서는 `~/pnix-hy/hy-meta/todo.md`와 같은 역할이다. 재개 시 여기부터
읽고, 검증 명령으로 현재 상태를 확인한 뒤 다음 슬라이스를 잇는다.

---

## Current Remaining Work (verified 2026-08-11)

Audit pass over this entire file + `STATUS.md` + source (`compiler.clj`,
`diverse_double_compile.clj`, `frontend_selfhost.clj`, `language_surface.clj`,
`kernel.clj`) to separate genuinely-open work from stale/superseded TODO
placeholders. **Headline: almost everything "fixable" is closed.** Of 3223
lines and hundreds of `- [x]` entries, only 3 raw `- [ ]` checkboxes exist in
the whole file (§18.1-18.3, "perf/bug/production-API agent triage") — and
those are stale: the "★★ codex 구현 인계" note near the top of this file
(same 2026-06-29 date) already reports that exact triage done and closed
("현재 todo 큐 남은 구현 항목 없음"). Treat §18.1-18.3's empty checkboxes as
dead placeholders, not open work. No `SCOPE_LOCK.md` exists inside
`clj-meta/` itself (the sibling `pnix-clj/SCOPE_LOCK.md` governs the pnix
*runtime* layer, not this host-proof lane, and doesn't add constraints beyond
what's already reflected below).

What's actually left, by axis (map into the detailed sections below for
receipts/digests — this is a pointer, not a duplicate):

### 1. Trusting-Trust / Diverse-Double-Compiling depth — MEDIUM, in progress
STATUS.md `trusting-trust/Wheeler-independent-DDC = false` is the honest
top-line claim, but the substance is much closer than "false" implies:
- U5 (`kernel.clj` interpreter) cross-checks 112/112 conformance corpus,
  U6 (`frontend_selfhost.clj`) is a genuinely independent tiny reader +
  analyzer + ASM emitter (0 calls into `tools.analyzer.jvm`/host reader),
  U8 fuzzes 10,000 random-program comparisons — all closed, all live gates.
- `independent-mini-backend-subset` (the real 3-way host≡compiler≡U6-backend
  DDC row) was widened this session in two passes: 14→43/51, then 43→**46/51**
  of U6's fixtures (verified by direct grep count against source, matches
  STATUS.md exactly).
- **"Wire U6's other ~8 fixtures" item — DONE, closed as "nothing left to
  wire" (2026-08-12, re-verified).** A prior pass here estimated ~8 remaining
  fixtures by NAME-matching U6's `:tiny-*` ids against the DDC row's
  `:mini-backend-*` ids. Re-ran that comparison carefully this pass (fixed a
  colon-prefix bug in the id-normalization that was inflating the unmatched
  count) and got 17 nominally-unmatched names, then checked EACH ONE's
  actual `:source`/`:args`/`:expected` against every existing
  `mini-backend-*` fixture rather than trusting the name mismatch. All 17
  turned out to be either byte-identical to an existing fixture under a
  different name (`tiny-destructure-nested` ≡ `mini-backend-destructure`,
  `tiny-op-get` ≡ `mini-backend-get`, `tiny-let-shadowing-branch` ≡
  `mini-backend-let-shadowing`, etc.) or the same *operation* already
  exercised inside an existing combined fixture (`tiny-op-gt`/`gte`/`lte` →
  `mini-backend-op-comparisons`; `tiny-op-inc`/`dec`/`pos?` →
  `mini-backend-op-unary`; `tiny-op-first`/`next-first` →
  `mini-backend-seq-ops`; `tiny-op-quot`/`rem` →
  `mini-backend-op-quot-rem`; `tiny-one-arg`'s single-arg `+` → already
  exercised by `mini-backend-arithmetic`; `tiny-const-arithmetic`'s
  zero-arg literal arithmetic → already exercised by
  `mini-backend-do-body`). There is genuinely nothing left to add at the
  *current* 46-fixture scope — do not re-flag this as "~8 remaining" again;
  any future gap here would have to come from item below (growing U6's own
  fixture set past 51), not from wiring existing U6 fixtures into the DDC row.
- **Remaining, size large**: grow U6's *own* fixture set past 51 toward the
  full conformance corpus U5 already reaches (116 cases as of this pass, up
  from 112 — grows over time via `examples/*.clj`) — this is what upgrades
  the claim from "51-fixture subset" to "full-corpus independent 2nd
  compiler." Each new fixture needs its own tiny-frontend support (new
  syntax/op coverage in `frontend_selfhost.clj`), so this is proportional to
  how much of Clojure's surface remains unimplemented there — plausibly
  several more sessions at the pace of the 14→43 slice.

  **One real slice landed, 2026-08-13: fixed multi-arity `fn`** (51→55).
  `(fn ([x] ...) ([x y] ...))` — multiple *fixed*-arity clauses on one
  function, each compiled to its own ASM `invoke` method on the same
  `AFunction` subclass (exactly how the real host compiler does it; `IFn`'s
  normal argument-count dispatch on the call side needs no glue). Verified
  against real host `eval` (2-arity, 3-arity, and the arity-mismatch
  `ArityException` case) before adding. Also wired the same 4 cases into the
  live `independent-mini-backend-subset` DDC row (43→47). Deliberately did
  NOT attempt variadic `&` rest-args in the same pass — that needs
  `clojure.lang.RestFn`, a different base class with its own
  arity-dispatch/rest-collection contract (multiple `invoke` overloads
  forwarding to `doInvoke` with the tail packaged as a seq), materially
  bigger and riskier to get right without the host compiler's source open
  side-by-side — a separate slice, not bundled into this one to keep this
  DDC witness's fixtures high-confidence rather than "probably right."
  **Second slice landed same day: variadic `&` rest-args** (55→61), closing
  the item deferred above. Reverse-engineered `clojure.lang.RestFn`'s exact
  contract from the real host (AOT-compiled 3 variadic fixtures, `javap -c`'d
  the class files) rather than guessing: `RestFn` implements every public
  `invoke(...)` overload concretely already, so a subclass only needs
  `getRequiredArity()` + one `doInvoke` overload (fixed-arg-count + 1
  params, last slot = rest `ISeq` or `nil`). `emit-class` now branches to a
  `RestFn`-extending path when any arity clause carries a `rest-param`.
  Mixing a variadic clause with other fixed arities in the same `fn` is
  explicitly rejected (real `RestFn` supports it via additional lower-arity
  `invoke` overrides, a further slice not attempted here) rather than
  silently mishandled. Also added `count` (`RT.count`, boxed to `Integer`
  — confirmed via the same `javap -c` pass that this is `Integer` not
  `Long`, unlike every other numeric op here). Verified against real host
  `eval` before adding (3-arg and exactly-1-arg/empty-rest cases, arity
  mismatch throwing `ArityException` on both sides, mixed-arity rejection).
  DDC row: 47→50. `-M:conformance` 116/116 unaffected, `bin/clj-meta-gate`
  `metacircular gate: READY`, no regressions.

  **Third slice landed same day: `case`** (61→65), via macro expansion
  (`expand-case`) into a `let` + nested `if`/`=` chain — reuses existing
  machinery, no new ASM code. Caught a real gap before it became a
  fixture: a default-less `case` with no match should throw
  `IllegalArgumentException` on the real host (confirmed live), but this
  tiny language has no `throw` special form at all — rather than silently
  return `nil` (wrong), `expand-case` now requires a trailing default clause
  and rejects the default-less shape outright. DDC row: 50→52.
  `-M:conformance` 116/116 unaffected, `bin/clj-meta-gate` `metacircular
  gate: READY`, no regressions.

  **Fourth slice landed 2026-08-14: single-clause `try`/`catch`** (65→69).
  Real ASM exception-table emission (`visitTryCatchBlock` + labels), not a
  macro expansion. Scope: exactly one body expr + one `catch` clause, catch
  class from a small explicit allowlist (`ArithmeticException`/`Exception`/
  `RuntimeException`/`Throwable`) rather than general class resolution. This
  only *catches* exceptions arising from ops already supported here (mainly
  `quot`/`rem` divide-by-zero) — a user-facing `throw` special form is still
  open and is what would fully close `case`'s no-default gap noted above.
  Verified against real host `eval` (caught divide-by-zero, non-throwing
  passthrough, caught-exception-object identity via `(nil? e)`, composition
  with `let`) before adding fixtures. DDC row: 52→54. `-M:conformance`
  116/116 unaffected, `bin/clj-meta-gate` `metacircular gate: READY`, no
  regressions.

  **Fifth slice landed 2026-08-14: `throw` + allowlisted `ClassName.`
  exception construction** (69→74), closing the `case` no-default gap for
  real. Both reverse-engineered from real host bytecode (`javap -c` on
  AOT-compiled `throw`/constructor forms) before writing any ASM code:
  `throw` emits `CHECKCAST Throwable; ATHROW` (exact real shape, works on
  re-thrown caught-exception locals too); `ClassName.` constructor calls
  emit `NEW; DUP; [args]; INVOKESPECIAL <init>` (exact real shape), scoped
  to the same small exception-class allowlist `catch` already uses (now
  including `IllegalArgumentException`). `expand-case`'s no-default
  rejection is lifted: no-match now expands to `(throw
  (IllegalArgumentException. "No matching clause"))` — real host's message
  is dynamic (needs `str`, which this language doesn't have) so the message
  text is a fixed approximation, but the exception class and the fact that
  it throws at all match exactly. DDC row: 54→56. `-M:conformance` 116/116
  unaffected, `bin/clj-meta-gate` `metacircular gate: READY`, no
  regressions.

  **Sixth slice landed 2026-08-14: general `.methodName` instance interop**
  (74→79), via `clojure.lang.Reflector.invokeInstanceMethod` -- confirmed
  via `javap -c` that this is the EXACT mechanism real host `eval` already
  uses for every untyped-receiver interop call (type hints don't exist in
  this tiny language, so this isn't a narrower approximation, it's the same
  fallback path real Clojure takes here too). Reuses `emit-object-array`
  verbatim for the args array. Unlike the exception-class allowlist
  elsewhere in this file, this is genuinely general (any method name,
  receiver, arg count). Verified against real host `eval`: `.getMessage` on
  a caught exception (closing the earlier `(nil? e)` workaround),
  `.length`/`.toUpperCase` on strings (matches real conformance-corpus rows
  directly), `.equals` both branches. DDC row: 56→58. `-M:conformance`
  116/116 unaffected, `bin/clj-meta-gate` `metacircular gate: READY`, no
  regressions.

  **Seventh slice landed 2026-08-14: static interop `ClassName/methodName`**
  (79→83), via `Reflector.invokeStaticMethod(Class, String, Object[])` — the
  runtime-dispatch counterpart to what instance interop already used, scoped
  to a small class allowlist (`Math`/`Integer`/`Long`/`String`; real host
  resolves the class+method at compile time here since the class name is
  syntactically present, confirmed via `javap -c`, but matching that exact
  mechanism was out of scope — this backend uses Reflector's own runtime
  static-dispatch primitive instead, same behavior-not-bytecode-shape bar as
  `case`). Verified against real host, including a notable negative case:
  `(Math/max 1 2.0)` is rejected by real host (ambiguous int/double overload,
  `IllegalArgumentException: "No matching method max found taking 2 args"` —
  this is one of `conformance.clj`'s own negative-corpus rows) and this
  backend's Reflector-based call rejects it with the *exact same* exception
  class and message, confirming the substitution is faithful on the
  rejection path too, not just the happy path. DDC row: 58→60.
  `-M:conformance` 116/116 unaffected, `bin/clj-meta-gate` `metacircular
  gate: READY`, no regressions.

  **Eighth slice landed 2026-08-14: `try`/`finally`** (83→88), matching the
  real host's exact shape (finally block duplicated on both normal and
  exceptional exit, confirmed via `javap -c` before writing code). Scope:
  single body + single `finally`, not combined with `catch` on the same
  `try` (separate slice). **Found and fixed a real bug in the process, not
  just added a feature**: nesting `try/finally` inside an outer `try/catch`
  silently skipped the inner `finally` — root cause was `visitTryCatchBlock`
  being called *before* emitting the body in both `emit-try` and
  `emit-try-finally`, so the outer handler's exception-table entry got
  registered ahead of any nested try's own entry; the JVM uses the first
  matching entry in registration order, so the outer handler always won,
  bypassing the inner handler entirely for any overlapping, matching PC
  range. Caught by testing the nested shape against real host `eval` with a
  live mutable counter (host: ends at 1, this backend before the fix:
  stayed 0), confirmed via `javap -c -v` on the actual exception table. Fix:
  call `visitTryCatchBlock` last in both functions (valid ASM usage — Label
  positions are fixed by `mark`, independent of when the referencing call
  happens). Re-verified the full existing try/catch fixture set against
  real host after the fix; all still agree. DDC row: 60→62 (pure-value
  fixtures only — a mutable AtomicInteger arg shared across the row's three
  sequential legs would accumulate cross-leg mutation, so those stay
  U6-only). `-M:conformance` 116/116 unaffected, `bin/clj-meta-gate`
  `metacircular gate: READY`, no regressions.

  **Ninth slice landed 2026-08-14: combined `catch`+`finally` on one
  `try`** (88→94), closing the gap explicitly deferred above. Real host
  duplicates `finally` on THREE exit paths, not two (normal, caught, and
  any exception unhandled by either — including one from inside
  `catch-body` itself), confirmed via `javap -c -v` before writing code.
  Needs three exception-table entries: the specific `catch-class` and a
  catch-all "any" handler both covering the try-body's range (specific
  registered first — same ordering lesson the previous slice's real bug
  taught), plus a catch-all covering the catch-body's own range too. `try`'s
  arity check widened to 2-or-3 args (`catch` alone, `finally` alone, or
  `catch` then `finally`); the existing catch-only/finally-only paths are
  byte-for-byte unchanged (verified via matching digests before adding any
  new fixture). Verified against real host: normal/caught value+counter
  paths, and the earlier nesting-bug scenario reused as a regression check
  on this new combined path (an unmatched exception type still runs
  `finally` before reaching an outer `catch`). DDC row: 62→64.
  `-M:conformance` 116/116 unaffected, `bin/clj-meta-gate` `metacircular
  gate: READY`, no regressions.

  **10번째 슬라이스, 2026-08-14: `str`** (94→100). 실제 host에서 `str`은
  컴파일러 특수형이 전혀 아니다 — `javap -c`로 확인: `RT.var("clojure.core",
  "str")`로 Var를 찾고 `getRawRoot()`로 실제 함수 값을 읽어 `IFn`으로
  캐스트해서 `.invoke(...)` — 일반 사용자 정의 함수 호출과 똑같은
  메커니즘. 원리상 다른 `clojure.core` 함수로도 일반화되는 재사용 가능한
  메커니즘이지만, 이번 슬라이스는 `str`만 fixture로 검증해 연결. 실제 host
  `eval` 대비 검증(추가 전): 2-인자 연결, 숫자 인자, 1/0-인자, `nil` 인자가
  `"null"`이 아니라 빈 문자열이 되는 Clojure 고유 동작, 문자열 리터럴과
  섞어 쓰기. DDC 행: 64→66. `-M:conformance` 116/116 영향 없음,
  `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **11번째 슬라이스, 2026-08-14: field access(`.-fieldName`)와 `set!`**
  (100→103). host-AOT `(.-x p)`/`(set! (.-x p) v)`를 `javap -c`로 확인:
  field GET은 `Reflector.invokeNoArgInstanceMember(Object, String,
  boolean)`을 `true`로 호출(`.methodName` 무인자 호출이 쓰는 `false`와
  구분), `set!`은 `Reflector.setInstanceField(Object, String, Object)`로
  가는데 이 메서드 자체가 대입값을 반환해 Clojure `set!`의 "대입값으로
  평가됨" 의미와 정확히 일치. `.-x`가 naive한 "`.`으로 시작" 검사로는
  기존 `.methodName` 판정과 충돌하므로, cond 순서상
  `field-access-name`을 먼저 검사하고 `interop-method-name` 쪽에도
  `.-` 제외를 명시적으로 추가하는 이중 방어. `set!`은 `(set! (.-field
  expr) value)` 형태만 허용(dynamic var 등 다른 대입 대상은 범위 밖).
  실제 host `eval` 대비 검증(추가 전): `java.awt.Point` 필드 읽기, `set!`
  반환값, `set!`이 실제로 mutate하는지(대입 후 재조회) 전부 일치. DDC 행:
  66→67(읽기 전용만 — mutation 관찰은 공유 mutable arg 문제로 U6 전용).
  `-M:conformance` 116/116 영향 없음, `bin/clj-meta-gate` `metacircular
  gate: READY`, 회귀 없음.

  **12번째 슬라이스, 2026-08-14: `locking`** (103→105). host-AOT
  `(locking sb (.append sb "x"))`를 `javap -c -v`로 확인하니
  `emit-try-finally`와 구조적으로 완전히 동일 — "finally"에 해당하는
  게 항상 lock 객체에 대한 `MONITOREXIT` 하나뿐이고, 보호 구역 시작
  전에 `MONITORENTER`가 한 번 더 실행된다는 점만 다름. 실제 host가
  남기는 `MONITORENTER`/`MONITOREXIT` 뒤 `ACONST_NULL` push-then-pop은
  관찰 가능한 차이 없는 화장용 잔재라 재현 안 함. 범위: lock 표현식 +
  단일 body 표현식만. 실제 host `eval` 대비 검증(추가 전): 정상 경로
  `StringBuilder` append, `locking` body에서 던진 예외가 `try`/`catch`를
  정상 통과하는지(host·backend 둘 다 `:caught`, 예외 클래스·메시지
  일치). DDC 행: 67→68(예외 경로만 — 정상 경로는 `StringBuilder`가
  leg마다 누적 mutate되므로 U6 전용). `-M:conformance` 116/116 영향
  없음, `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **13번째 슬라이스, 2026-08-14: multi-catch** (105→111). host-AOT
  `(try (quot 10 x) (catch ArithmeticException e :divzero) (catch
  IllegalArgumentException e :bad-arg))`를 `javap -c -v`로 확인: `finally`의
  catch-all `nil`-type entry와 달리 각 `catch` 절이 자기만의 handler +
  exception-table entry를 가지며, 전부 같은 범위를 커버하지만 (handler,
  구체 클래스) 쌍만 다르고 소스 순서대로 등록됨(real host와 일치). 기존
  단일-catch/`finally`-only/`catch`+`finally` 경로는 전혀 안 건드리고
  `:try-multi-catch`라는 새 AST 노드+emitter를 별도 추가해 이미 검증된
  경로의 리스크를 최소화. 김에 `(try body)`(clause 없는 bare try)도
  지원(그냥 `body`와 동일). multi-catch+`finally` 결합은 범위 밖, 별도
  슬라이스. 실제 host `eval` 대비 검증(추가 전): 2-catch 중 첫 번째
  매치, 2-catch 중 두 번째 매치, 3-catch 중 세 번째 매치 — 전부 host와
  일치. DDC 행: 68→70. `-M:conformance` 116/116 영향 없음,
  `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **14번째 슬라이스, 2026-08-14: multi-catch + `finally` 결합**
  (111→117), 바로 앞에서 미룬 것을 닫음. host-AOT `(try (quot 10 x)
  (catch ArithmeticException e :divzero) (catch IllegalArgumentException e
  :bad-arg) (finally (.incrementAndGet a)))`를 `javap -c -v`로 확인 —
  `emit-try-catch-finally`의 N-catch 일반화. 각 catch는 여전히 자기만의
  구체 클래스 entry를 try-body 범위에 갖고, 거기에 try-body 범위 전체를
  덮는 공유 catch-all `finally` entry 하나, 그리고 **각 catch-body
  자신의 범위도** 독립적으로 같은 `finally` handler를 가리키는 catch-all
  entry를 가짐(catch-body 안에서 예외가 나도 `finally`가 돌게). 실제
  host `eval` 대비 검증(추가 전): 정상 경로 값+counter, 첫/두 번째
  catch 매치, 그리고 가장 까다로운 경우인 catch-body 안에서 예외가 다시
  던져져도 `finally`가 돌고 바깥 `try`/`catch`로 정상 전파되는지(host·
  backend 둘 다 `:outer-caught`, counter `1`) — 전부 일치. DDC 행:
  70→72(상수 `finally` fixture만). `-M:conformance` 116/116 영향 없음,
  `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **15번째 슬라이스, 2026-08-14: 일반 클래스 생성**(117→121, 작은 예외
  허용목록을 넘어서). `java.awt.Point.`/`java.util.ArrayList.`를
  `javap -c`로 확인: (클래스, arity)가 컴파일타임에 생성자를 유일하게
  특정하면 real host가 직접 `NEW`/`INVOKESPECIAL`을 내지만, 특정 안
  되면(예: `ArrayList(int)` vs `ArrayList(Collection)`, 둘 다 arity 1)
  `RT.classForName(String)` + `Reflector.invokeConstructor(Class,
  Object[])`로 런타임 반사 디스패치 폴백 — `.methodName`/
  `ClassName/methodName`이 이미 쓰던 것과 같은 메커니즘. 이 백엔드는
  애매한-arity 폴백 경로만 사용(유일-arity 컴파일타임 최적화는 안 함,
  기존 behavior-not-bytecode 기준과 동일). 기존 예외 허용목록 경로는
  안 건드리고 새 `general-constructor-class-name`을 폴백으로 추가.
  정직한 caveat: 존재하지 않는 클래스는 analyze 시점이 아니라 런타임에
  실패(잘못된 소스에 대해서만 나는 차이). 실제 host `eval` 대비
  검증(추가 전): 유일-arity, 애매한-arity(int/Collection 두 오버로드
  각각), 무인자 — 전부 host와 일치, field access·instance interop과의
  합성도 확인. DDC 행: 72→74. `-M:conformance` 116/116 영향 없음,
  `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **16번째 슬라이스, 2026-08-14: 일반 static interop**(121→124, 작은
  static-interop 허용목록을 넘어서 — 앞 일반 클래스 생성과 같은 원리).
  `(Character/isDigit c)`를 `javap -c`로 확인: real host는 짧은 클래스
  이름도 자기 default-import 표로 완전히 해석하고, `isDigit`의
  `(char)`/`(int)` 오버로드가 애매하니 앞의 일반 클래스 생성과 **같은**
  `RT.classForName` + `Reflector.invokeStaticMethod` 폴백을 씀. 이 tiny
  언어엔 import 표가 없어 real host와 달리 완전히 정규화된 클래스
  이름을 요구함(`java.lang.Character/isDigit`, 짧은 이름 안 됨) — 정직하게
  더 좁은 범위. 기존 작은 `known-static-classes` 허용목록은 안 건드리고
  새 `general-static-interop-target`을 폴백으로 추가. 실제 host `eval`
  대비 검증(추가 전): `isDigit` 참/거짓, 무인자 static 호출 — 전부 host와
  일치. DDC 행: 74→75. `-M:conformance` 116/116 영향 없음,
  `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **17번째 슬라이스, 2026-08-14: bignum 리터럴**(`N`/`M` 접미사,
  124→130) — **진짜 버그를 값만이 아니라 클래스까지 대조해서 잡음.**
  `5N`/`1.5M`을 `javap -c`로 확인하니 real host는 리터럴 소스 텍스트를
  문자열 상수로 저장했다가 `<clinit>`에서 `RT.readString`을 한 번
  호출해 static field에 캐싱 — 이 메커니즘은 일부러 재현 안 함(그대로
  하면 이 witness의 상수 생성이 real reader를 거쳐 독립 witness라는
  존재 이유가 무너짐). 대신 표준 라이브러리로 직접 값을 만듦(같은 값,
  다른 생성 경로).

  첫 시도에서 `N` 값을 `java.math.BigInteger`로 바로 만들었더니
  `(= tiny host)`는 `true`(Clojure `=`가 숫자 타입 교차 비교)였지만
  `(class tiny)` ≠ `(class host)` — `(class 5N)`은 실제로
  `clojure.lang.BigInt`이지 raw `BigInteger`가 아니었다. **값만 비교했으면
  놓쳤을 버그**를 클래스까지 명시 대조해서 잡고 `BigInt.fromBigInteger(new
  BigInteger(String))`로 고침. `M`은 처음부터 진짜 `BigDecimal`이라
  문제없었음(라이브 확인). 추가로 `analyze-expr`의 기존 `(integer? form)`
  분기가 `(long form)`으로 캐스팅하는데 `BigInt`도 `integer?`라 그 분기를
  먼저 타면 `Long/MAX_VALUE` 넘는 값이 조용히 잘렸을 것 — `emit-const`/
  `analyze-expr` 둘 다 BigInt/BigDecimal 분기를 generic `integer?`보다
  먼저 검사하도록 함.

  기존 `+`/`-`/`*`/`=`가 이미 모든 숫자 타입에 다형적이라 이 리터럴들과
  자연스럽게 합성 — 리터럴 하나 추가가 아니라 임의 정밀도 산술이라는 진짜
  새 능력이 열림. 실제 host `eval` 대비 검증(추가 전): 리터럴 값+클래스,
  `Long/MAX_VALUE` 넘는 `+`(오버플로 없이 정확), `BigDecimal` 곱셈,
  `BigInt`/`Long` `=` 비교 — 전부 host와 일치. DDC 행: 75→78.
  `-M:conformance` 116/116 영향 없음, `bin/clj-meta-gate` `metacircular
  gate: READY`, 회귀 없음.

  **18번째 슬라이스, 2026-08-14 (배치 진행): regex(`#"..."`)/ratio(`1/3`)
  리터럴** (130→134). regex는 real host도 `Pattern.compile(String)`
  직접 호출(리더 의존 없음, 그대로 재현). ratio는 bignum과 같은
  `RT.readString` 경로라 또 일부러 안 씀 — `Numbers/divide`를 parse
  시점에 호출(실제 reader와 동일 메커니즘, 라이브 확인: `4/2`→`Long`
  collapse)해서 이미 reduce된 numerator/denominator로 `new
  Ratio(BigInteger,BigInteger)` 생성. `Pattern`은 `.equals` 미오버라이드
  (`(= #"a+" #"a+")` → `false`, 라이브 확인)라 fixture는 `.pattern`
  문자열로 비교. DDC 행: 78→81. `-M:conformance` 116/116 영향 없음,
  `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **19번째 슬라이스, 2026-08-14 (배치 진행): `binding` + dynamic var
  deref** (134→137). `javap -c -v` 확인: real host `(binding [*x* 42]
  *x*)`는 `push-thread-bindings`/`pop-thread-bindings`(이미 `str`에 쓴
  `RT.var`+`IFn.invoke` 메커니즘)를 `emit-locking`과 구조적으로 동일한
  try/finally로 감싸는 shape — 그대로 재현(behavior뿐 아니라 bytecode
  shape까지 일치, 우회 불필요). 이 tiny 언어엔 `def`가 없어서 `binding`
  대상 dynamic var 하나를 `frontend_selfhost.clj` 자신에 미리 선언해두고
  allowlist로 연결. DDC 행 연결을 위해 bare/qualified 심볼 둘 다 같은
  항목에 매칭되도록 정규화(`dynamic-var-target`) — real host
  `eval`/`compiler.clj`가 다른 `*ns*`에서 심볼을 풀어야 해서 fixture
  source는 fully-qualified 심볼 사용. 정상/예외 종료 둘 다 root로
  복귀하는지 세 다리 전부 대조 검증. DDC 행: 81→84. `-M:conformance`
  116/116 영향 없음, `bin/clj-meta-gate` `metacircular gate: READY`,
  회귀 없음.

  `letfn`은 조사 후 보류: real host가 mutual recursion을 위해 binding당
  별도 클래스 + 상호 참조 필드를 만드는 걸 `javap -c`로 확인했는데, 지금
  U6는 클래스를 하나만 찍는 구조라 `deftype`/`reify`급 아키텍처 확장이
  필요함 — 그 둘과 묶어서 나중에.

  **20번째 슬라이스, 2026-08-14 (배치 진행): 고정 arity + variadic `&`
  ceiling 혼합** (137→141). `javap -c`로 `(fn ([a] a) ([a b] (+ a b))
  ([a b & r] ...))`를 확인하니 `RestFn` 상속 클래스 안에 고정 arity마다
  `invoke(N)` + variadic 절 하나만 `doInvoke`+`getRequiredArity`인 단순
  조합 — 별도 클래스 불필요, `letfn`/`deftype`급이 아니었다. 고정 절
  param 개수가 variadic과 같으면 그 arity는 고정 절의 `invoke(N)`
  오버라이드가 이긴다는 것도 라이브 확인(`(f 1 2)`가 `([a b] ...)` 선택).
  real host의 "고정 arity가 variadic보다 param 많으면 거부" 검증도
  재현. DDC 행: 84→86. `-M:conformance` 116/116 영향 없음,
  `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **21번째 슬라이스, 2026-08-14 (배치 진행): 임의 `clojure.core` 함수를
  일반 call head/값으로** (141→149). `str`이 애초에 compiler special이
  아니라 평범한 `RT.var`+`IFn` Var 룩업이었던 걸 이전 슬라이스에서 이미
  확인했는데, 이번엔 `(map inc coll)`을 `javap -c`로 확인해서 call
  head(`map`)와 값으로 전달되는 인자(`inc`, `.invoke` 없이 `getRawRoot()`
  만)가 똑같은 메커니즘임을 재확인 — `str` 하드코딩을 일반화해서
  `analyze-call`/`analyze-expr` 둘 다 `clojure.core`에 실제 존재하는
  아무 함수 이름이나 받아들이게 함. allowlist 아니라 `ns-resolve`로 진짜
  존재 검증(오타는 analyze 시점에 real host처럼 즉시 거부, `RT.var`가
  없는 이름도 그냥 인턴해버리는 함정을 막음). `map`/`filter`/`reduce`/
  `apply`/`conj`/`assoc`/`vec`/`into` 등 표준 라이브러리 표면 전체가
  바이트코드 추가 없이 열림. DDC 행: 86→91. `-M:conformance` 116/116
  영향 없음, `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **22번째 슬라이스, 2026-08-14 (배치 진행): local을 함수로 호출** (고차
  함수 파라미터, 149→152). `(fn [f x] (f x))`가 이전까지 전부
  "unsupported call"이었음 — `analyze-call`의 fallback이 local(env)은
  고려 안 하고 `clojure.core` Var 존재만 체크했기 때문. `javap -c`
  확인: real host는 Var 룩업 없이 local 값을 바로 `IFn`으로 `checkcast`
  후 `invokeinterface` — 기존 `emit-local` 재사용, 새 바이트코드 메커니즘
  불필요. env 매칭을 `core-var-exists?`보다 먼저 둬서 local이 동명의
  `clojure.core` 함수를 shadow하는 순서도 재현. 21번째 슬라이스의
  core-fn-value와 합쳐져 `(map f coll)`처럼 local 함수를 `map`에 넘기는
  것도 가능. DDC 행: 91→93. `-M:conformance` 116/116 영향 없음,
  `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **23번째 슬라이스, 2026-08-14 (배치 진행): `+`/`-`/`*` variadic화 +
  unary `-`** (152→161). `javap -c`로 `(+ a b c)`가
  `Numbers.add(Numbers.add(a,b),c)`처럼 왼쪽부터 fold되는 걸 확인 —
  3개 이상 인자는 analyze 시점에 기존 `:binary` 노드로 desugar, emitter
  변경 없음. `(+)`→0/`(*)`→1/`(+ a)`·`(* a)`→`a` 그대로도 반영. `(- a)`
  단항 음수는 `Numbers.minus(Object)` 1-인자 오버로드(2-인자와 다른
  메서드, `javap -c` 확인)라 `inc`/`dec`류 `:unary`에 추가. `(-)` 0-인자
  거부(real host도 `ArityException`)도 재현. DDC 행: 93→97.
  `-M:conformance` 116/116 영향 없음, `bin/clj-meta-gate`
  `metacircular gate: READY`, 회귀 없음.

  **24번째 슬라이스, 2026-08-14 (배치 진행): 연쇄 비교 + `get` 3-인자**
  (161→171). `+`와 달리 `<`는 3개 인자에서 fold 안 함 — `javap -c`
  확인: `(< a b c)`도 `(< a)`도 그냥 기존 `core-fn-call`과 똑같은
  Var-call(`RT.var`+`IFn.invoke`). 정확히 2-인자만 기존 `Numbers.lt`
  fast path, 나머지 arity는 `core-fn-call` 폴백 — behavior-equivalence가
  아니라 real host가 실제로 쓰는 바로 그 메커니즘. `get` 3-인자
  기본값(`(get m k d)`)은 별개 패턴 — `RT.get(Object,Object,Object)`
  직접 호출이라 새 `:get3` 노드 추가. DDC 행: 97→100. `-M:conformance`
  116/116 영향 없음, `bin/clj-meta-gate` `metacircular gate: READY`,
  회귀 없음.

  **25번째 슬라이스, 2026-08-14 (배치 진행): named 자기재귀 `fn`**
  (171→175). `(fn foo [n] ... (foo ...))`가 전부 "malformed fn
  clause"였음 — `analyze-fn`이 이름 있는 형태를 파싱 못 함. `javap -c`
  확인: real host는 self-reference를 그냥 `this` 로드+`IFn`
  checkcast+invoke로 컴파일 — 22번째 슬라이스의 `emit-local-fn-call`과
  완전히 같은 모양. `emit-local`에 `:self` kind(`this` 로드) 하나만
  추가, `analyze-fn`이 이름을 파싱해 각 arity 절 env에 미리 넣음.
  파라미터가 같은 이름이면 shadow하는 것도 real host와 동일(라이브
  확인). 고정+variadic 혼합 arity와 named self-recursion을 같이 쓰는
  경우도 검증. DDC 행: 100→102. `-M:conformance` 116/116 영향 없음,
  `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **26번째 슬라이스, 2026-08-14 (배치 진행): 계산된 call head +
  keyword-as-fn** (175→178). `((constantly x) 99)`처럼 head가 심볼이
  아니라 표현식인 경우가 전부 "unsupported call"이었음 — `javap -c`
  확인: real host는 head가 만든 값을 그냥 `IFn`으로 checkcast해서
  invoke, Var/local 룩업 전혀 없음 — `local-fn-call`/`core-fn-call`의
  "cast하고 invoke" 꼬리와 같은 모양이라 `(not (symbol? op))`를 잡는
  새 `:computed-fn-call` 노드 하나로 처리. 구현 후 발견한 덤: keyword가
  이미 `:const`로 analyze되고 `Keyword`가 `IFn`을 구현해서
  `(:a m)`(keyword-as-fn)도 새 코드 없이 통과 — 라이브 확인 후 fixture
  추가. DDC 행: 102→104. `-M:conformance` 116/116 영향 없음,
  `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **27번째 슬라이스, 2026-08-14 (배치 진행): `fn` body 자체 tail 위치의
  `recur`** (178→182). 지금까지 `loop` 없이 `fn` body 꼬리에서 바로
  `recur`하면 "recur outside loop"였음 — 25번째 슬라이스의 named
  self-recursion(진짜 `IFn.invoke` 재호출, 스택 쌓임)과는 의미가 다른
  기능. `javap -c` 확인: real host는 인자 슬롯에 `astore`하고 메서드
  맨 위로 `goto`, 스택 안 쌓이는 진짜 루프. 기존 `loop`/`recur`의
  GOTO 메커니즘을 일반화(`recur-target-key`의 slot을 `{:kind
  :local}`/`{:kind :arg}`로 태깅)해서 `analyze-fn-clause`가 각 arity
  절 env에 자기 param을 recur target으로 미리 깔고, `emit-class`가
  각 메서드 맨 앞에 label을 찍음. 중첩 `loop`가 같은 env key로 자연
  shadow하는 것도 확인. `(f 100000)`으로 스택 안 쌓이는 것 직접
  확인(named self-recursion이었으면 StackOverflow 위험 구간), variadic
  arity도 동작 확인. DDC 행: 104→106. `-M:conformance` 116/116 영향
  없음, `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **28번째 슬라이스, 2026-08-14 (배치 진행): 임의 fully-qualified 예외
  타입 `catch`** (182→185). 5개짜리 작은 allowlist만 가능했음 —
  `clojure.lang.ExceptionInfo`(`ex-info`가 실제로 던지는 타입!)조차
  못 잡았다. `:catch-class`를 쓰는 곳은 전부 `Type/getInternalName`만
  호출 — JVM 예외 테이블은 컴파일타임 문자열 상수만 필요, 런타임
  `RT.classForName` 바이트코드 호출 불필요. host 쪽 analyze 시점에
  `Class/forName`으로 풀면 끝, 새 바이트코드 메커니즘 없음.
  fully-qualified 이름만 허용(import 표 없어서 짧은 이름 불가 —
  general-static-interop과 같은 범위 제한). `ExceptionInfo`
  `ex-data`/`.getMessage`, `NullPointerException` 잡기 전부 실제 host
  대비 검증. DDC 행: 106→108. `-M:conformance` 116/116 영향 없음,
  `bin/clj-meta-gate` `metacircular gate: READY`, 회귀 없음.

  **29번째 슬라이스, 2026-08-15: 진짜 중첩 closure + boolean identity
  버그 수정** (185→194). 사용자가 "왜 진짜 컴파일러 코드를 재사용
  안 하냐"고 물어서 Trusting-Trust/DDC 독립성 원칙 설명 — 공유 코드로는
  백도어/버그를 못 걸러낸다는 게 요지, 사용자 동의로 U6 독립 확장
  방향 재확인. 남은 4개 큰 항목 중 사용자가 "중첩 fn 리터럴"을 직접
  선택.

  `javap -c`로 `(fn [x] (fn [y] (+ x y)))` 확인: real host는 내부 fn마다
  별도 클래스 + 캡처 변수를 인스턴스 필드로 + 생성자가 그 값들을 받아
  저장. 자유변수는 **analysis 이후** AST를 재귀적으로 훑어
  `:local`/`:local-fn-call` 참조를 모으고 이 closure 자신의 파라미터/
  self-name을 빼서 계산(analyze 시점 env엔 `:kind` 구분이 없어서 그
  자리에서 "내 것/바깥 것" 구분 불가). `emit-class`를 일반화해서
  `:captures`가 있으면 필드+다중인자 생성자, `emit-closure`가 정의
  지점에서 재귀적으로 `emit-class` 호출 후 `NEW; DUP; <캡처값>;
  INVOKESPECIAL` 방출. 일부러 좁힌 범위: 중첩 1단계까지만, closure는
  단일 arity 절만 — 둘 다 명확한 에러로 거부되는 것 확인.

  **구현 후 `filter`로 검증하다가 진짜 버그 발견**: `(filter (fn [x]
  (> x threshold)) coll)`이 아무것도 안 걸러냄 — `map`/직접호출/
  `apply`는 정확했는데 `filter`만 틀림. 원인: `GeneratorAdapter/box`가
  boolean에 대해 `new Boolean(z)`(deprecated 생성자)를 쓰지
  `Boolean.valueOf(z)`를 안 씀 — `<`/`>`/`=`/`zero?`/`pos?`/`neg?`/
  literal `true`/`false` 전부 매번 새 non-singleton Boolean 인스턴스를
  만들고 있었음. 이 witness 자신의 `if`는 `RT.booleanCast`(진짜
  변환)라 문제없이 삼켰지만, `javap -c`로 까본 real host의 `if`는
  `RT.booleanCast` 호출이 전혀 없고 `Boolean.FALSE`와의 순수 레퍼런스
  identity 비교(`if_acmpeq`) — non-singleton false는 real host
  입장에서 **항상 truthy**. `clojure.core/filter`의 실제 컴파일된
  `(if (pred f) ..)`가 정확히 이 경로. `map`은 술어 결과를 `if`로
  검사 안 해서 증상이 안 보였을 뿐. 라이브로 `(if (Boolean. false)
  :truthy :falsy)` → `:truthy` 재현·확인. 수정: `.box` 9곳 전부
  `GeneratorAdapter/valueOf`로 교체(raw ASM 프로브로
  `Boolean.valueOf` 방출 확인). **이 버그는 독립 witness가 real host와
  실제로 상호작용할 때만 드러났다** — U6를 독립적으로 키우는 이유
  자체를 증명하는 사례. compiler.clj(production backend)는 closure와
  boolean identity 둘 다 문제없음 확인 후 DDC 행에도 연결. DDC 행:
  108→112. `-M:conformance` 116/116 영향 없음, `bin/clj-meta-gate`
  `metacircular gate: READY`, 회귀 없음(기존 185개 fixture는 자기
  `if`가 애초에 관대해서 fix 전후 모두 통과).

  **30번째 슬라이스, 2026-08-15: `letfn`(비-상호재귀만)** (194→197).
  `javap -c`로 single-binding 비상호재귀 `letfn`이 real host에서
  `let`+named-`fn`+closure와 완전히 같은 바이트코드임을 확인 —
  `expand-letfn`으로 매크로 확장 레이어에서만 `(name [params] body)`를
  안쪽부터 `(let [name (fn name [params] body)] ...)`로 감싸는 nested
  `let`로 변환, 새 바이트코드 전혀 불필요. 진짜 상호재귀(`even?`/
  `odd?`)는 real host가 null-초기화 후 생성, 전부 만들고 나서
  `putfield`로 되짚어 채우는 2단계 메커니즘이 필요해 이번엔 시도 안
  함 — 매크로 확장 시점에 raw-form 심볼 스캔으로 형제 참조를 감지해
  명확한 에러로 거부(자기재귀는 정상 허용). 단일 바인딩/자기재귀/
  독립된 두 바인딩/상호재귀 거부 전부 실제 host 대비 검증.
  compiler.clj도 `letfn` 지원 확인 후 DDC 행 연결. DDC 행: 112→114.
  `-M:conformance` 116/116 영향 없음, `bin/clj-meta-gate`
  `metacircular gate: READY`, 회귀 없음.

  U6에 아직 전혀 없는 큰 표면: `deftype`/`defrecord`/`reify`, protocol.
  각각 자체 multi-fixture 슬라이스이며 다중 클래스 생성이 필요해
  진짜 크다. 남은 gap: 중첩 closure는 1단계/단일 arity로 범위 제한(더
  깊은 중첩이나 multi-arity 중첩 closure는 아직), `letfn`은 진짜 상호
  재귀(생성-후-backpatch)는 아직 없음.
- **Remaining, size large/open-ended (may be permanently held)**: bit-identical
  (not just behavior-identical) compiler-binary DDC needs a *fully
  independent* second compiler targeting the same bytecode format by
  genuine design, not coincidence. Explicitly held; see U10's CakeML/
  CompCert/Octagon research trail for why this is a different order of
  project, not a slice-able TODO.

### 2. Full-language-correctness / formal proof (R7f) — LARGE, likely permanently held
`full-language-correctness = false` per STATUS.md. Current evidence: 112/112
conformance + 20+ negative cases + 10,000-case fuzz corpus (0 divergences) +
translation-validation VCs for the checked-long lowering. This is strong
*evidence*, explicitly not a *proof*. Closing this for real means a
machine-checked semantic-preservation proof (Coq/ACL2/HOL, CompCert-style) —
a standalone research project, not a coding task. The doc's own framing
(§16.6, §17) treats this as intentionally, permanently held rather than a
todo-list item; sizing it as "large" undersells it — it's arguably out of
scope for incremental work at all.

### 3. Production frontend / full runtime self-host (R5f/R6f) — LARGE
- R5f: `frontend_selfhost.clj` covers 55 fixtures (fn/if/do/let/loop-recur/
  17 macros/destructuring/data literals/fixed multi-arity fn) but is still a
  subset; the
  production path (`compiler.clj`'s main line) still depends on
  `tools.analyzer.jvm` + host reader for anything outside that subset. Full
  closure = self-hosting Clojure's entire reader+macroexpander+analyzer,
  which is a large, multi-session undertaking (same order of magnitude as a
  small Clojure-in-Clojure frontend from scratch).
- R6f: `runtime_selfhost.clj` has 8 host-core-free leaf helpers
  (inc/second/assoc/get/reduce/conj/map/count fragments); full closure means
  reimplementing `clojure.core` + persistent data structures without host
  delegation — large, and arguably in tension with the "host JVM performance,
  0 feature loss" design goal stated in §0, so likely stays intentionally
  held rather than pursued to completion.

### 4. JVM-free self-hosting — NOT SIZED (permanent architectural boundary, not a gap)
`JVM-free self-hosting = false` is correctly false, but §7 "Boundary"
explicitly declares JVM/Java runtime/Maven/JDK as **permanent substrate** —
this was never intended to close, the same way hy-meta doesn't aim to drop
CPython. Recommend STATUS.md keep stating this as `false` for honesty, but
it should not appear on anyone's "remaining work" punch list as if it were
actionable; there is no planned path to it in this codebase's design.

### 5. Product-integration readiness (pnix-clj launcher, M7) — SMALL-MEDIUM, deferred by decision, not by blocker
M7 is explicitly `PARKED`: clj-meta's receipts are marked
`:not-consumed-by "pnix-clj launcher"` by policy, pending "a complete
meta-circular stage15/N Clojure compiler" per the doc's own (very high) bar —
but the practical peer-floor gate (`selfhost`, `stage7`, `primary`) has been
**READY/PASS** since at least 2026-08-07 per STATUS.md. This reads like a
deliberate non-technical gate (governance decision to keep clj-meta
pnix-agnostic, consumed via the documented API surface in
`pnix.clj-meta.compiler`/`form-proof`/`host-reflection`) rather than missing
implementation. If/when the decision is made to lift PARKED, the API surface
is already documented as stable (see the "pnix-clj interop boundary" note
near the top of this file) — likely a small integration task on the pnix-clj
side, not new work here.

### 6. Minor loose ends — SMALL
- Stage14 cross-host law closure is `OK` but with **missing external
  transcripts** (hy-meta/pnix-hy/pnix-clj fixture files) held as optional
  evidence, not blocking — STATUS.md lists this explicitly. Small to close if
  another host's transcript becomes available; not actionable from clj-meta
  alone.
- No `Stage16`/"beyond stageN" concept exists as a distinct axis: `StageN` in
  §10a is already a generic *recursive* closure mechanism ("repeat the same
  closure law whenever a new host/runtime/proof surface appears"), and it's
  marked `[x]` done as a mechanism. There is currently no new surface pending
  that would trigger a "stage16" — this is not a gap, just a framework that
  activates on demand.

### Bottom line
Nothing found that contradicts the "a lot is already implemented" framing.
The "wire U6's remaining fixtures into the DDC row" sub-step is now closed
(2026-08-12, re-verified — nothing left at the current 46-fixture scope, see
item 1 above). The only concrete, scoped-but-large remaining slice is
growing U6 itself toward the 112-case corpus (item 1's other sub-step).
Everything else in items 2-4 is correctly framed in the existing docs as
intentionally-held research frontier or permanent architectural boundary,
not backlog; item 5 is a governance decision, not missing code.

---

## try in expression position — VerifyError fix (2026-07-03)

`compile-form` 이 표현 위치의 `try`(호출 인자 등 피연산자 스택 위에 값이 있는
자리)를 그대로 emit 해 `VerifyError: Operand stack underflow` 를 냈다 — JVM
예외 핸들러 진입은 스택을 비우기 때문. 실전 재현은 pnix-clj lowering 의
`(force-slot (get (try …) "success"))` (tryEval 결과 직접 select).

수정: `hoist-expression-tries` 프리패스를 `analyze-for-compile-form` 에 삽입
(:root/wrapper 두 경로 공통). 모든 `try` 를 zero-arg `fn*` 호출로 재작성 —
fn 본문은 RETURN 컨텍스트라 스택이 항상 빈다. Clojure 본가 Compiler.java
TryParser 의 FNONCE 래핑과 같은 전략. 위치 추적 없는 over-approximation
(tail try 도 감싸며 의미 동일 — recur 는 try 를 못 건넌다); quote 내부 불변,
메타데이터 보존. 검증: pnix-clj 풀 게이트(127 tests/2890 assn)가 패치된
compile-form 경로 위에서 그린; tryEval 직접 select 가 전 레인 collapse.

---

## pnix-clj interop boundary — host proof API surface (2026-07-01)

`pnix-clj`(= `../pnix-clj`, the pnix runtime) treats `clj-meta` as the
Clojure/JVM **host meta-circular compiler/evaluator proof lane** and consumes it
through an explicit interop boundary. Full plan:
`../pnix-clj/clj-meta-separation.md`. Principle: pnix-clj must **consume clj-meta's
existing API, not reinvent host machinery**; clj-meta is already mature so most of
this is "confirm/expose", not "build".

Layering: **clj-meta is pnix-agnostic** — it completes Clojure(JVM)
meta-circularity on its own (self-host ladder, kernel, import hook, artifact,
introspection = this todo's R/stage work) and knows nothing about pnix. pnix-clj
is purely the pnix layer on top, and its host IS clj-meta. Note: the
"host Clojure `eval` ≡ clj-meta compile" agreement that pnix-clj currently checks
in its `clojure_form` lane is a **Clojure <-> Clojure self-host proof** and so is
conceptually clj-meta's domain — a candidate to host here (host-eval oracle vs
compile) rather than in the pnix runtime, if/when consolidating host proofs.

What pnix-clj already calls (keep stable; these are the contract):
- `pnix.clj-meta.compiler/compile-form*`, `/compile-form`, `/eval-form`,
  `/compile-form-strict`, `/compile-to-dir`, `/compile-ns`, `/load-compiled-ns`,
  `/compile-classes`.
- `pnix.clj-meta.verified-compile/compile-classes-verified`.

To-do for clj-meta (so pnix-clj can delegate instead of re-deriving):
- [x] Expose a stable per-form **compile proof** API for pnix-clj:
      `pnix.clj-meta.form-proof/compile-receipt`. It owns the determinism,
      strict, bytecode-artifact, and verified-compile rows that pnix-clj used to
      assemble in `pnix-clj.clj-meta`; pnix-clj now delegates and records
      `:proof-owner` in the receipt. Global `determinism_policy` /
      `bytecode_witness` / `verified_compile` APIs remain the broader gate
      receipts and are recorded in `:related-global-proof-apis`.
- [x] Keep `compile-form-strict`'s no-fallback contract (already tracked here as
      신규-V) — pnix-clj's strict row depends on it. Contract is fixed by the
      UUID literal unsupported-op smoke: strict throws with no global fallback
      diagnostics, while `compile-form*` records host-fallback evidence.
- [x] (optional, future) host a generic **host-reflection snapshot** API
      (Var / Namespace / metadata / Class / classloader / macroexpand snapshots)
      **DONE 2026-07-02**:
      `pnix.clj-meta.host-reflection` now owns `snapshot` + per-kind snapshots
      (`:var`, `:namespace`, `:metadata`, `:class`, `:classloader`,
      `:java-object`, `:throwable`, `:macroexpand`), plus `pnix-clj` now consumes
      snapshots for host projection mapping where practical.
      that pnix-clj's interop layer can call, so the host introspection now in
      `pnix-clj/src/pnix_clj/clojure_projection.clj` becomes a clj-meta-provided
      service rather than pnix-clj-owned. Until then pnix-clj isolates it behind
      its own `pnix-clj.interop` namespace (host-side adapter), which is the
      lower-coupling first step.

No clj-meta source change is required for pnix-clj's near-term interop phases
(A–E in `../pnix-clj/clj-meta-separation.md`); they delegate to / wrap the
existing API above. Revisit the optional items only if pnix-clj's interop layer
needs them.

### Frontier-LIFT: recursive-binding lowering for pnix `let`/`rec` (DONE 2026-07-01)

Context: pnix-clj R1 lifted the former forward-reference frontier. clj-meta now
exposes `pnix.clj-meta.compiler/lazy-letrec`, a pnix-agnostic recursive value-slot
primitive. pnix-clj lowers pnix `let` and recursive attrsets to that form; clj-meta
does not learn pnix attrset semantics.

- [x] Confirmed Clojure `letfn` alone is not enough for lazy *value* recursion;
      added `lazy-letrec` with memoized recursive cells and bounded cycle
      detection (`:recursive-binding-cycle`) instead of host StackOverflow.
- [x] Kept pnix-agnostic: the primitive compiles ordinary Clojure binding forms
      with lazy recursive value slots. pnix-specific mapping lives in
      `../pnix-clj/src/pnix_clj/lowering.clj`.
- [x] Determinism/witness contract preserved. pnix-clj `eval-lowered` receipts
      remain deterministic; validation: pnix-clj `clojure -M:test` 71/1349,
      clj-meta `clojure -M:compiler-smoke` 159/159, clj-meta
      `clojure -M:conformance` 116/116 + negatives 22/22.

Host-layer principles for that boundary (2026-07-01 /deep-research, verified;
GraalVM Truffle / object-capability / effect systems):
- **Deny-by-default**: clj-meta (host floor) owns the allowlist of what is
  exported to pnix. Nothing reachable until explicitly exported; add one
  capability at a time. The unrestricted/reflection-all policy is for trusted
  guests only.
- **Host reflection/introspection stays here** (namespace/Var/metadata/classpath/
  class-artifact, dynamic require/resolve) — it is denied to the guest by default
  and is a host-proof concern, not pnix runtime core.
- **Effect-class gating**: expose host capabilities as orthogonal, individually
  gated switches (host-access / reflection / native / IO). Caveat: coarse grants
  (class loading, native, IO) "effectively grant all access" — keep them fine.
- **Content-addressed host version id**: bind the host floor to guest evidence by
  a content-addressed version id so pnix can record which host-floor version
  produced a term (cross-layer provenance).

---

## ★★ codex 구현 인계 — 프로덕션 하드닝 (claude 조사 결과, 우선순위순, 2026-06-29)

**배경**: 사용자 지시로 clj-meta 컴파일러를 ../pnix-clj 가 붙일 *프로덕션 제품*으로. claude 가
3-에이전트 감사(프로덕션-API ✅ / 성능 ✅ / 버그 ✅, 전부 완료) + 런타임 벤치로 **조사만** 했고,
**구현은 codex/다른 구현자가** 한다. claude 는 각 수정 후 gate/conformance/bench + 코드 grep 으로 검증.
규칙: 헌법(RAW-FREE/no-auto-promotion/정직 held) 준수, 각 수정 전후 gate READY·conformance 무회귀.

**★ 1차 라운드 완료 (claude 실측 검증 ✅, 2026-06-29)**: codex 가 **C-BUG1·C-BUG2·A-B1·A-B2·A-B3·
A-I1·A-I2·A-I4·B-F1·B-F2·B-F3·B-F4 (12항목)** 구현 → claude 검증 결과 **문제 없음**(요약엔 일부 누락됐으나
12항목 모두 정확): **gate READY ✅, conformance 116/116 + negative 22/22, compiler-smoke 151/151**,
m6aj digest `6eb83dc…` **불변**(raw 승격 무회귀), C-BUG1 Boolean **canonical**(`identical? Boolean/FALSE`
=true) + inner pred **direct-emit**, C-BUG2 `(= 1 1.0)`=false direct-emit, A-B2 gen 이름 source-hash
**결정적**(같은 source→같은 이름 → self-host 고정점 유지), C-BUG1/C-BUG2 **회귀 픽스처 고정**
(conformance:195-205).

**★ 2차 라운드 완료 (codex 실측 검증 ✅, 2026-06-29)**: **A-I3·A-I5·B-F5·B-F2b·A-D2·A-S1·
B-F6~F8 (7항목)** 구현. 최종 **gate READY ✅, conformance 116/116 + negative 22/22,
compiler-smoke 158/158**, M6aj digest `6eb83dc…` **불변**. B-F6~F8 은 line-number 중복 label skip,
primitive field direct emit, `free-locals` 직접 `sort` 로 완료.

**★ 신규 검증 항목 완료 (codex 실측 검증 ✅, 2026-06-29)**: **신규-V·신규-W** 완료.
`compile-form-strict` no-fallback 계약은 genuine unsupported UUID literal 로 strict throw + global fallback
diagnostics 0 + `compile-form*` host-fallback 차이를 smoke 에 고정. 동시성 스트레스는 기존 gate 연결에 더해
단독 `:concurrency-smoke` alias 추가. 최종 **compiler-smoke 159/159, concurrency-smoke 1/1,
conformance 116/116 + negative 22/22, gate READY ✅**. **현재 todo 큐 남은 구현 항목 없음.**

**★★ claude 독립 재검증 (2026-06-29, 2·신규 라운드)**: codex 자체보고를 신뢰하지 않고 실측 →
**문제 없음**: gate **READY**(재실행), **M6aj digest `6eb83dc…` 불변**(B-F1~F8/F2b 전부 raw 승격
0 회귀), 새 공개 API 전부 존재(compile-form*/compile-form-strict/eval-string/compile-and-load-ns/
compile-to-dir/load-from-dir/warm-up!/unload-ns!/clear-kept-classes!), **A-D2 roundtrip** 독립 OK(42),
**A-I5 강스트레스 24스레드×30=720 동시 compile**(ns 격리+fallback 섞기) 전부 host≡compiler·deadlock
0, `:concurrency-smoke` 1/1. ⟹ **프로덕션 하드닝(§18/★★) 전 항목 닫힘 + 검증 완료.** 남은 것은
오직 research frontier(R4f~R7f: full Wheeler DDC·full frontend/runtime self-host·형식증명) = 프로덕션
blocker 아님, 정직하게 held. ../pnix-clj 연결은 다른 세션 담당.

### A. 프로덕션 API/동시성/수명주기 (감사 ✅완료 — 즉시 착수 가능)
파일: `src/pnix/clj_meta/compiler.clj`. 공개 API 10개(compile-form 3229·eval-form 3253·
run-ns-form-strict 3283·compile-ns 3311·load-compiled-ns 3355·compile-classes 3379 등).
**Blockers(서버/멀티스레드 소비 전 필수):**
- [x] **A-B1** 전역 진단 atom `compile-form-fallback-diagnostics`(:133-136, write :173-176) =
  동시성 인터리빙 + **무한 증가(메모리 누수)** + racy `(count)` index. → per-call `binding`-scoped
  atom 으로, 진단을 *결과에 포함*(`compile-form*` → `{:fn :mode :diagnostics}`); 전역은 ring-buffer
  + monotonic `swap!` index.
  - 완료(2026-06-29 KST, codex): `*fallback-diagnostics-cap*`, 호출별 `compile-form*`, bounded/global
    diagnostics index를 추가. `compiler-smoke` 147/147, `conformance` 116/116, 멀티스레드 per-call
    diagnostics 격리 smoke, `audit-self-source`, `gate(READY)` 통과.
- [x] **A-B3** raw 예외 누수: analyze catch 가 `Exception`만(Error/VerifyError/NoClassDefFound 누수),
  emit catch 가 `:unsupported-op` ExceptionInfo 만(:3183-3219). 소비자가 맥락 없는 throwable 받음.
  → compile-form 전체를 `catch Throwable` 로 감싸 `{:phase :analyze|:emit|:instantiate :form :cause}`
  통일 ex-info, instantiate(.newInstance :3204)도 Throwable catch.
  - 완료(2026-06-29 KST, codex): `compile-form failed` envelope(`:type :compile-error`, `:phase`,
    `:form`, `:cause`)를 analyze/emit/instantiate 경계에 연결. `compiler-smoke` 148/148,
    `conformance` 116/116 + negative 22/22, `audit-self-source` 통과.
- [x] **A-B2(+D1)** gen-class 이름 cross-thread 충돌: `Fn__<n>` 이 매 compile-form 마다
  `*gen-counter*`=(atom -1) 재바인딩 → 항상 Fn__0. host DynamicClassLoader 정적 classCache(이름 키)
  에 서로 덮어씀(:2527 cname, :2624 defineClass). → 전역 `AtomicLong` + per-unit UUID/**content-hash**
  접두사로 전역 유일. (캐시 키도 source content-hash 로.)
  - 완료(2026-06-29 KST, codex): `Fn/Reify__<source-sha12>__<n>` 결정적 이름으로 전환하고
    `compile-form`/strict/classes 경계에 source-hash unit id를 바인딩. `jarproof` stage compare는 generated
    class hash만 normalized 비교해 clean-process replay 동등성을 유지. `compiler-smoke` 149/149,
    `conformance` 116/116 + negative 22/22, `determinism-policy`, `audit-self-source`, `gate(READY)` 통과.

**Important(견고한 라이브러리):**
- [x] **A-I1** 반환값에 direct-emit vs host-fallback 신호 없음(bare IFn) → map 반환 변형 +
  **공개 `compile-form-strict`**(현재 `compile-fn-strict` private :3259, no-fallback 보장).
  - 완료(2026-06-29 KST, codex): 기존 `compile-form*` mode/diagnostics 스키마를 docstring으로 고정하고
    공개 `compile-form-strict` 추가. strict unsupported form throw smoke 추가.
- [x] **A-I2** one-shot 진입점 없음 → `eval-string`(read-all→do→eval-form), `compile-and-load-ns`
  (compile-ns+load 합성) + compile-form/eval-form 반환 스키마 문서화(compile-ns 는 이미 :schema).
  - 완료(2026-06-29 KST, codex): `eval-string`, `compile-and-load-ns` 공개 진입점 및 smoke 추가.
    `compiler-smoke` 150/150, `conformance` 116/116 + negative 22/22, `gate(READY)` 통과.
- [x] **A-I3** 네임스페이스 격리 없음 — compile-ns/load-compiled-ns 가 global ns 레지스트리 오염
  (in-ns/intern/refer, cleanup 없음 :3304/3330/3355) → `{:isolate true}`(gensym ns + remove-ns) +
  `unload-ns!` + "global 변형" 문서.
  - 완료(2026-06-29 KST, codex): `compile-ns`/`load-compiled-ns` artifact 에 `:isolate`, `:source-namespace`,
    rewritten `:ns-form` 을 고정. 기본 경로는 host load 처럼 global ns 를 남기고, `{:isolate true}` 는 임시
    ns 에 준비·로드 후 `unload-ns!` 로 제거. 공개 `unload-ns!` 추가. `compiler-smoke` 152/152,
    `conformance` 116/116 + negative 22/22, `gate(READY)` 통과.
- [x] **A-I4** classloader churn + 숨은 128-unit 강참조 캐시(`kept-class-units` :140-161) release API
  없음 → `clear-kept-classes!` + cap 설정(dynamic var) + **session DCL**(파라미터화 `*dcl*` 로 한
  loader 재사용 → cross-form 해소 + 단일 해제 scope).
  - 완료(2026-06-29 KST, codex): `with-compiler-session`, `clear-kept-classes!`,
    `*kept-class-units-cap*` 추가. 기존 compile entrypoint 는 outer session DCL 을 재사용하고,
    동일 session 동일 source 재컴파일은 기존 class 로드로 처리. unit hash 는 namespace+source 기준.
    `compiler-smoke` 151/151, `conformance` 116/116 + negative 22/22, `determinism-policy`, `gate(READY)` 통과.
- [x] **A-I5** tools.analyzer.jvm 공유 global env 동시성: 동시 compile 가 공유 env/`*ns*` 에 간섭
  → compile lock(직렬화) or thread-safety 검증 + compile-form 이 deterministic `*ns*` 바인딩(`:ns` 옵션).
  - 완료(2026-06-29 KST, codex): compile entrypoint 분석/emit/fallback 구간을 `compile-lock` 으로
    직렬화하고 `:ns` opts 를 `compile-form*`/`compile-form`/`eval-form`/`eval-string`/
    `compile-form-strict`/`compile-classes` 에 추가. `eval-form`/`eval-string` 은 실행도 opts ns 에서 수행.
    8스레드 동시 compile/eval/fallback smoke 로 host≡compiler·per-call diagnostics 격리·크래시 0 확인.
    `compiler-smoke` 154/154, `conformance` 116/116 + negative 22/22, `gate(READY)` 통과.

**Determinism/Caching/Startup(nice-to-have):**
- [x] **A-D2** AOT-to-disk 없음 → `compile-to-dir`/`load-from-dir`(compile-classes 바이트 → .class 파일).
  - 완료(2026-06-29 KST, codex): `compile-classes` 산출물 `{classname byte[]}`를
    `out-dir/<internal>.class` 로 쓰는 `compile-to-dir`와, artifact/dir+main-class를 fresh/session
    `DynamicClassLoader`에 로드해 `:fn`을 반환하는 `load-from-dir` 추가. 디스크 round-trip smoke 추가.
    `compiler-smoke` 155/155, `conformance` 116/116 + negative 22/22, `gate(READY)` 통과.
- [x] **A-S1** warm-up 훅 없음(첫 compile 지연) → `warm-up!`; `-main`/`run-smoke` 는 smoke 전용(문서화).
  - 완료(2026-06-29 KST, codex): public `warm-up!` 추가. trivial direct compile을 한 번 실행하고
    `{:schema ... :value 42 :warmed true}` receipt 반환. `warm-up! compiles trivial direct form` smoke 추가.
    `compiler-smoke` 156/156, `conformance` 116/116 + negative 22/22, `gate(READY)` 통과.
- 우선순위: **B-F6~F8.**
- 한 줄 평(감사): direct-emit 코어와 compile-ns/load 모델은 견고하나, *공개 표면이 얇고 관측이
  전역 mutable 상태*라 A-B1~B3 전엔 동시·장수 호스트 임베드 불가.

### B. 성능/속도 (감사 ✅완료) — compile-time hotspot(F1~F8) + 런타임 타깃
**Compile-time (감사 발견, 우선순위순 — 전부 pure-fn 이라 semantic 무변경·gate 안전):**
- [x] **B-F1** ★최대 — `loop-unroll-ranges`(≤256 step unroll)가 *qualifying loop 마다 2번* 실행.
  `emit-loop`(:1978 `loop-ai-ranges` + :1979 `loop-independent-unroll-ranges`)이 둘 다 호출하고
  각자 내부에서 `loop-unroll-ranges`(:1947, :1968)를 동일 인자로 또 부름. → unroll 1회만 계산해
  공유(ai-ranges 입력 + independent 로 재사용). 추가 무료 early-out: :1979 를
  `(when-not *disable-independent-range-admission* …)` 로 감쌈.
  - 완료(2026-06-29 KST, codex): `emit-loop`에서 branch recur context와 bounded unroll 결과를 1회
    계산해 `loop-ai-ranges`/independent admission 이 공유. `rg` 기준 `loop-unroll-ranges` 실행 호출은
    `emit-loop` 1곳. `compiler-smoke` 149/149, `conformance` 116/116 + negative 22/22, `gate(READY)` 통과.
- [x] **B-F2** range 기계 중복 재계산 — `current-loop-recur-exprs`(AST walk + atom)가 loop 당 2+2k회,
  `loop-positive-index-iteration-count`·`long-range-of(:init)` 도 binding 당 5~6회 재계산
  (:1906-1907/1961-1962/1078-1079/1192-1193/1083). → `emit-loop` 에서 1회 계산해 recognizer 들에 전달.
  - 완료(2026-06-29 KST, codex): `emit-loop` branch recur AST walk, base-env positive-index counter,
    AI/unroll init range, per-binding init/independent range를 공유. loop init 은 순차 바인딩이라
    accumulator의 extended-env counter는 binding별 1회 유지. `compiler-smoke` 149/149,
    `conformance` 116/116 + negative 22/22, `gate(READY)` 통과.
- [x] **B-F3** `emit-node`(:2402) 디스패치가 ~30-branch `condp =`(노드당 평균 ~15 keyword `=`). 컴파일
  최핫 경로. → `(case (:op node) …)` 로 교체(상수 keyword → tableswitch). trivial·broad.
  - 완료(2026-06-29 KST, codex): `emit-node` dispatch 를 동일 op 목록의 `case`로 교체.
    `compiler-smoke` 149/149, `conformance` 116/116 + negative 22/22, `gate(READY)` 통과.
- [x] **B-F4** `DynamicClassLoader` form 마다 생성 — `load-compiled-ns` 가 body form N개당 N loader
  (:3235/3265/3383, :3368). → ns load 전체에 `*dcl*` 1개 공유(이미 bound 면 재사용) → N→1 + cross-form
  직접 해소. (A-I4 session DCL 과 합칠 것.)
  - 완료(2026-06-29 KST, codex): `load-compiled-ns` body form 전체를 session DCL 로 묶고, public
    `with-compiler-session`으로 외부 batch compile 도 loader 재사용 가능하게 함. A-I4 검증과 함께 gate 통과.
- [x] **B-F5** `resolve-method`/`resolve-ctor` 가 interop emit 마다 `.getMethods`(:1282/1303, 호출
  1322/1477/1492) 무캐시 + `emit-node-as` static-call 이중 reflection(:1452,1399). → `[cls mname
  arg-classes]` 키 memoize + m 1회 resolve 후 전달. (Numbers 는 이미 targeted라 영향 적음.)
  - 완료(2026-06-29 KST, codex): successful method/ctor resolution 을 `[cls mname arg-classes]` /
    `[cls arg-classes]` 키로 memoize. `emit-node-as` static-call primitive coercion 경로는 이미 구한
    Method 를 `emit-static-call-raw` 로 전달해 이중 reflection 제거. `compiler-smoke` 154/154,
    `conformance` 116/116 + negative 22/22, `gate(READY)` 통과.
- [x] **B-F2b** (큰 payoff·옵션) body 에 checked-long `Numbers` add/minus/multiply 후보가 *없으면*
  range 기계 전체(ai-ranges/independent/unroll/per-binding)를 short-circuit. range 는 오직
  `checked-long-static-no-overflow?`(:1360-1382) 의 LADD/LMUL/LSUB 승격에만 영향 → 없으면 무관.
  semantic 안전(승격만 포기, Numbers fallback).
  - 완료(2026-06-29 KST, codex): `has-checked-long-candidate?`로 현재 loop body 의 checked
    `Numbers/add|minus|multiply` 후보를 1-pass 탐지. 후보 없는 loop 는 recur/range/AI/unroll/per-binding
    range 도출을 skip, 후보 있는 loop 는 기존 경로 유지. `compiler-smoke` 154/154,
    `conformance` 116/116 + negative 22/22, `gate(READY)`, M6aj digest `6eb83dc…` 불변.
- [x] **B-F6~F8**(low) line-number Label 노드마다 생성(같은 line 중복) / primitive field box-unbox /
  `free-locals` `pr-str` 정렬(→ 직접 `sort`).
  - 완료(2026-06-29 KST, codex): method 단위 `*last-emitted-line*` 으로 연속 같은 source line
    `visitLineNumber` 중복 emit 을 skip. `emit-node-as` 에 primitive `:static-field`/`:instance-field`/
    closure `:field` direct path 추가. `free-locals` 는 symbol 직접 `sort` 로 변경. primitive field typed-let
    smoke 2개 추가. `compiler-smoke` 158/158, `conformance` 116/116 + negative 22/22, `gate(READY)` 통과.
- *NOT issue(감사 확인)*: const/var dedup 은 O(1)(hashed), tv VC 는 cheap.
- *구조적(큰 feature, cheap 아님)*: fn 경계가 전부 Object(invoke(Object…)) → numeric fn 호출마다
  box/unbox. 진짜 런타임 win 은 Clojure primitive IFn$LL/$DD/$LO… invoke 구현 필요(대형).

**런타임 bench 타깃 (ratio=ours/host, >1=느림 — 위 fix 의 관측 지표):**
- typed-long let **1.27**(구조적 boxing) · multi-arity **1.28**(arity dispatch) · variadic **1.20**
  (RestFn) · map/closure **1.11**. (loop ^long 0.44 등은 이미 우리가 빠름.)
- 우선순위: **F6~F8.** (F1/F2 가 compile-time, B-bench 의
  multi-arity/variadic 은 dispatch 구조 — 별개 트랙.)

### ▣ 구현·검증 상세 (codex 용 — 항목별 how-to + 체크방법)
공통 검증(매 항목 후): `clojure -M:gate`(READY), `clojure -M:conformance`(112/112 + negative),
`clojure -M:compiler-smoke`(전부 OK). **self-host 고정점은 결정적이어야 하므로 class 이름/순서
비결정 도입 금지**(아래 A-B2 주의). 성능 항목은 `clojure -M:bench` 런타임 + 큰 루프/ns 컴파일
체감 시간으로 측정.

#### A-B1 — 진단 atom: per-call 결과화 + bound + racy index 제거
- 구현: (1) `record-compile-form-fallback!`(:173) 의 index 를 `swap!` 내부에서 부여 —
  `(swap! a (fn [v] (conj v (assoc row :index (count v)))))` (count 를 swap 안에서 읽어 atomic).
  (2) 무한증가 방지: `*fallback-diagnostics-cap*`(기본 ~1024) dynamic var, cap 초과 시 oldest drop
  (ring). (3) 공개 `compile-form*` 추가: per-call `binding` 으로 *지역* 진단 atom 을 묶어 수집,
  `{:fn <ifn> :mode (:direct | :host-fallback) :diagnostics [...]}` 반환. 기존 `compile-form` 은
  `(:fn (compile-form* …))` thin wrapper. 전역 atom 은 opt-in 집계로만 유지.
- 검증: (a) `grep ':fallback-diagnostics-cap\|compile-form\*' compiler.clj` 로 실연결. (b) 멀티스레드:
  N 스레드가 동시에 fallback form(예 `(gen-class)`) compile-form* → 각 결과 :diagnostics 가 *자기
  호출 것만* + 전역 atom count ≤ cap. (c) gate/conformance 무회귀.

#### A-B3 — 예외 envelope 통일 (opaque throwable 차단)
- 구현: `compile-form` 본문을 최종 `(catch Throwable t …)` 로 감싸 단계 표시 ex-info 재throw:
  `(throw (ex-info "compile-form failed" {:type :compile-error :phase <:analyze|:emit|:instantiate>
  :form (pr-str form) :cause t} t))`. analyze catch 를 `Exception`→`Throwable`(:3187), instantiate
  `.newInstance`(:3204)도 Throwable catch. `:unsupported-op` fallback 경로는 그대로(정상 위임).
- 검증: (a) 비-Throwable catch / 알 수 없는 var / 강제 VerifyError form → `(:phase (ex-data e))`,
  `(:form …)`, `(:cause …)` 채워진 ex-info 받는지(REPL). (b) 정상 form 무영향. gate 무회귀.

#### A-B2(+D1) — gen-class 이름 전역 유일 (단, **결정성 유지 필수**)
- 구현: cname 을 전역 유일하게. **주의: 전역 mutable AtomicLong 접두사는 self-host stage1→7
  byte-identical 고정점을 깨뜨린다**(이름이 실행마다 달라짐 → 임베드된 클래스명 bytecode 변동).
  → **source content-hash 기반 결정적 접두사** 사용: `(str "Fn__" (subs (sha256 unit-source) 0 12)
  "__" n)` 또는 unit 별 결정적 id. self-host 고정점 lane 은 같은 소스 → 같은 hash → 같은 이름(결정적).
  서로 다른 compile 는 다른 hash → 충돌 없음. cname 생성부 :2527, defineClass :2624/2858/3094.
- 검증: (a) 서로 다른 두 source 의 gen 이름이 다름(`.getName`). (b) **핵심: `clojure -M:gate` 의
  full-source stage1→7 고정점·m6aj·bytecode-witness digest 가 여전히 통과**(결정성 안 깨짐).
  (c) 같은 source 두 번 compile → 같은 이름(결정적).

#### A-I1 — 공개 strict + :mode 신호
- 구현: `compile-fn-strict`(:3259, private) → 공개 별칭 `compile-form-strict`(no host fallback,
  미지원 op 면 throw). `compile-form*`(A-B1)이 `:mode :direct|:host-fallback` 반환.
- 검증: `(compile-form-strict '(fn [] (gen-class)))` 류가 throw(fallback 안 함); 정상 form → :mode :direct.

#### A-I2 — one-shot 진입점
- 구현: `eval-string`(src) = read-all → `(eval-form (cons 'do forms))`; `compile-and-load-ns`(src opts)
  = `(load-compiled-ns (compile-ns src opts))`. compile-form/eval-form 반환 스키마 docstring 명시.
- 검증: compiler-smoke 픽스처 추가(`(eval-string "(+ 1 41)")`=42, compile-and-load-ns 결과). 무회귀.

#### A-I3 — ns 격리 + 정리
- 구현: `compile-ns`/`compile-and-load-ns` opts `:isolate true` → gensym ns 에 컴파일·load 후
  `remove-ns`. 공개 `unload-ns!`(ns-sym). "기본은 global ns 변형(host load 의미)" docstring.
- 검증: `(compile-ns src {:isolate true})` 후 `(find-ns <gensym>)`=nil; `unload-ns!` 후 var 사라짐.
- 완료(2026-06-29 KST, codex): 구현 완료. `compile-ns isolate and unload-ns!` smoke 추가
  (원본 ns 미생성, 임시 ns compile/load 후 제거, 기본 ns 는 `unload-ns!` 로 제거).

#### A-I4(+B-F4) — session DCL + 해제 API
- 구현: `compile-form`/`compile-classes` 가 `*dcl*` *이미 bound 면 재사용*(없을 때만 새 DCL).
  `load-compiled-ns` 가 ns 전체에 1 DCL `binding`(N form → 1 loader, cross-form 직접 해소).
  공개 `clear-kept-classes!` + `*kept-class-units-cap*` dynamic var(기본 128). (:140-161, :3235/3265/3383)
- 검증: load-compiled-ns(N-form) → 생성 DCL 1개(계측/로그); `clear-kept-classes!` 후 kept atom 빔;
  cross-form 참조 form 동작(smoke). gate 무회귀.

#### A-I5 — analyzer 동시성
- 구현: `compile-form` 에 `:ns` 옵션(없으면 deterministic ns 바인딩 — 현재는 caller `*ns*` 의존).
  tools.analyzer.jvm 공유 global env 동시성 검증; 불안하면 analysis 직렬화(`locking` compile lock).
- 검증: 멀티스레드(서로 다른 ns) 동시 compile 스트레스 → host≡compiler 전부, 크래시/오염 0.
- 완료(2026-06-29 KST, codex): 구현 완료. `compile entrypoints :ns opts` smoke +
  `compile-form/eval-string concurrent ns isolation` smoke 추가.

#### A-D2 / A-S1 (옵션)
- A-D2: `compile-to-dir`(out-dir)·`load-from-dir` — compile-classes 바이트를 `out/<internal>.class`
  로 쓰고 URLClassLoader/DCL 로 로드. 검증: dir 라운드트립 후 invoke 결과 동일.
- 완료(2026-06-29 KST, codex): 구현 완료. `compile-to-dir/load-from-dir roundtrip` smoke 추가,
  smoke/conformance/gate 무회귀.
- A-S1: `warm-up!`(trivial form 1회 compile 로 analyzer/JIT 예열). `-main`/`run-smoke` 는 smoke 전용 docstring.
- 완료(2026-06-29 KST, codex): 구현 완료. `warm-up! compiles trivial direct form` smoke 추가,
  smoke/conformance/gate 무회귀.

#### B-F1 — loop unroll 1회로 (루프당 2회→1회)
- 구현: `emit-loop`(:1971) 에서 `then-recurs`/`else-recurs`/`guard-spec` + `(loop-unroll-ranges …)`
  를 **1회** 계산. `loop-ai-ranges`(:1894)는 내부 unroll 재계산(:1947) 대신 그 raw map 을 *인자로*
  받게 시그니처 변경(widen 못 채운 name 만 unroll 로 보충). `loop-independent-unroll-ranges`(:1951)
  제거하고 같은 raw unroll map 을 independent 로 재사용. 추가 early-out: independent 계산을
  `(when-not *disable-independent-range-admission* …)` 로 감쌈.
- 검증: **gate/m6aj(`6eb83dc…`)·bytecode-witness(`df0837…`) digest 불변**(승격 동일 = 0 회귀) — 이게
  핵심(F1 은 pure-fn 중복 제거라 결과 동일해야). conformance 무회귀. 큰 루프 다수 코드 compile 시간 체감 단축.

#### B-F2 — range 기계 중복 재계산 제거
- 구현: `emit-loop` 에서 `current-loop-recur-exprs`(then/else)·`loop-positive-index-iteration-count`·
  `long-range-of(:init b)` 를 1회 계산해 `loop-binding-bounded-step-range`/`loop-binding-accumulator-range`/
  `loop-unroll-ranges` 에 인자로 전달(각 함수가 재계산하던 것 제거). (:1076-1226, :1906-1907, :1961-1962)
- 검증: digest 불변(B-F1 과 동일 기준) + conformance 무회귀. compile 시간 단축.

#### B-F3 — emit-node `condp =` → `case`
- 구현: `emit-node`(:2402)의 `(condp = (:op node) :const … :if … )` 를 `(case (:op node) :const … :if …
  (throw …))` 로 교체(분기값이 전부 컴파일타임 keyword 상수 → tableswitch).
- 검증: compiler-smoke/conformance/gate 무회귀(의미 동일). 큰 코드 compile 시간 체감.

#### B-F5 — resolve-method/ctor memoize + 이중 reflection 제거
- 구현: `resolve-method`(:1282)/`resolve-ctor`(:1303) 결과를 `[cls mname (vec arg-classes)]` 키
  atom-map(또는 `memoize`)으로 캐시(JVM 세션 내 pure). `emit-node-as` static-call 이 `static-call-method`
  를 2번(:1452, :1399) 부르던 것 → 1회 resolve 후 전달.
- 검증: interop 무거운 form compile 시간 단축; conformance 무회귀(해소 결과 동일).
- 완료(2026-06-29 KST, codex): 구현 완료. smoke/conformance/gate 무회귀.

#### B-F2b — range 기계 short-circuit (옵션, 큰 payoff)
- 구현: fn/loop body 에 checked-long `Numbers` add/minus/multiply 후보가 있는지 1-pass 술어
  `has-checked-long-candidate?`. 없으면 `loop-ai-ranges`/`loop-independent-unroll-ranges`/per-binding
  range 도출을 nil 로 skip(range 는 `checked-long-static-no-overflow?` :1360-1382 의 raw 승격에만 영향).
- 검증: gate/m6aj digest 불변(승격 케이스 영향 0 — arith 있는 코드는 그대로); 비-arith 코드 compile 단축.
- 완료(2026-06-29 KST, codex): 구현 완료. smoke/conformance/gate 무회귀, M6aj digest 불변.

#### B-F6~F8 (low)
- F6 line-number: 같은 source line 연속 노드면 `visitLineNumber` skip(직전 line 추적). F7 primitive
  field box/unbox: `emit-node-as` 에 `:static-field`/`:instance-field` primitive 분기. F8 `free-locals`
  (:2504) `sort-by pr-str` → 직접 `sort`(symbol 은 Comparable). 검증: 무회귀 + 미세 단축.
- 완료(2026-06-29 KST, codex): 구현 완료. primitive field smoke 추가, smoke/conformance/gate 무회귀,
  M6aj digest 불변.

#### 신규-V — `compile-form-strict` no-fallback 계약 회귀 고정
- 구현: 별도 코드 수정 없을 수 있음(이미 동작 가능) — *검증/픽스처*가 핵심. compile-form-strict 가
  host eval fallback 을 *절대* 안 한다는 계약을: (a) 진짜 미지원-op/analyze-fail form 에 대해 throw
  (`:type :compile-error` 또는 `:unsupported-op`), (b) compile-form 은 같은 form 에 host eval fallback
  (또는 A-B3 envelope) — 둘 차이를 smoke 픽스처로 못박는다. 진짜 미지원 케이스 예: 함수 *내부*
  `(defprotocol …)`+`deftype` 구현(analyzer NPE) 또는 backend emit 테이블에 없는 op.
- 검증: `(compile-form-strict <fallback-form>)` → throw; `(compile-form <fallback-form>)` → 동작(또는
  envelope). gen-class no-op 처럼 *실제로 emit 가능한* form 은 strict 도 throw 안 함(정상). smoke 추가.
- 완료(2026-06-29 KST, codex): UUID literal unsupported const form 으로 strict 는 `:unsupported-op` throw,
  global fallback diagnostics 0, 같은 form 의 `compile-form*` 는 `:host-fallback` + `{:phase :emit
  :reason :unsupported-op}` 를 남기는 smoke 를 추가. `compiler-smoke`/`gate` 에 고정.

#### 신규-W — 프로덕션 동시성 스트레스 lane (A-I5 검증 겸 상시 게이트)
- 구현: 새 lane `concurrency-smoke`(deps alias) — N 스레드(예 8)가 동시에 (i) 서로 다른 ns 의
  `compile-form*`/`eval-string`/`compile-and-load-ns`, (ii) fallback 유발 form 섞어 M회 컴파일·실행.
  각 결과를 host `eval` 와 대조. gen 이름 충돌/진단 누수/크래시 감지.
- 검증(invariant): (a) 모든 결과 host≡compiler, (b) 각 호출 `compile-form*` 의 `:diagnostics` 가
  *자기 것만*(per-call 격리, A-B1), (c) 생성 gen 클래스 이름 충돌 0(A-B2), (d) ThreadDeath/VerifyError/
  ConcurrentModification 등 0. → A-I5(analyzer 동시성) 해결 여부의 *실측 게이트*. gate 에 연결(옵션).
- 완료(2026-06-29 KST, codex): 기존 8스레드 `compile-concurrency-smoke` 를 단독 실행 가능한
  `:concurrency-smoke` alias 로 노출하고, 전체 `compiler-smoke`/`gate` 연결 유지. 결과 host≡compiler,
  per-call diagnostics 격리, fallback 동시 실행, 크래시 0 확인.

### C. 버그 (감사 ✅완료 — 신규 confirmed silent miscompile 2개)
둘 다 host `(eval form)` 대조로 확인된 *silent miscompile*(에러 없이 틀린 값). conformance 112/112
이 놓친 이유 = 이 패턴(우리-컴파일 fn 을 host 고차함수에 넘김 / `(= int float)`)을 픽스처가 안 침.
**C-BUG1 은 전체에서 최우선**(기본 HOF 사용이 깨짐, 프로덕션 차단급) — A/B 보다 먼저.

#### [x] C-BUG1 ★CRITICAL — 비표준 Boolean boxing → host 가 우리 `false` 를 truthy 로 봄
- **repro(host vs compiler)**:
  ```
  (filterv (fn [x] (> x 5)) [1 2 3])   host=[]            compiler=[1 2 3]
  (remove  (fn [x] (> x 1)) [1 2 3])   host=(1)           compiler=()
  (every?  (fn [x] (> x 10)) [1 2 3])  host=false         compiler=true
  (take-while (fn [x] (< x 0)) [1 2 3])host=()            compiler=(1 2 3)
  (do (def f (< 5 1)) (if f :T :F))    host=:F            compiler=:T
  ```
- **근본원인**: 모든 primitive→Object boxing 이 `GeneratorAdapter.box` 사용 → `new Boolean(z)`
  (비표준 인스턴스, `Boolean/FALSE` 아님). host-컴파일 코드의 truthiness 는 *identity* 기반
  (`x != null && x != Boolean.FALSE`, `.booleanValue()` 아님) → 우리 `false` 가 truthy 로 취급됨.
  우리 자신 `emit-if`(:1583)는 `RT/booleanCast` 라 *우리 분기*는 정상 → 버그는 **clj-meta↔host 경계**
  (host filter/remove/every?/some/take-while/drop-while/split-with + def 된 boolean 분기)에서만 발현
  = conformance 계약 핵심.
- **callsites(전부 `.box`)**: `result-of`(:1233, 주 boolean 경로 — Numbers.gt/lt/Util.equiv 반환 박싱),
  boolean const(:265), `emit-instance?`(:1542), `emit-local` field/arg(:359/363/369), 필드(:1508/1519).
- **구현**: `.box` → `clojure.asm.commons.GeneratorAdapter.valueOf(Type)` (→ `Boolean.valueOf(Z)`
  =canonical) 로 교체. *최소* Boolean/TYPE, *이상적으로* 전 wrapper(Long/Char interning 도 복원 →
  `identical?` 충실도↑). `valueOf` 는 같은 클래스에 이미 존재. (헬퍼 `(box-value ga t)` 하나 만들어
  모든 `.box` 호출을 그쪽으로 돌리면 일괄.)
- **검증**: (a) 위 repro 5개 전부 host≡compiler. (b) `(identical? (<우리컴파일> '(fn [] (< 5 1)))
  false)` … 실은 `(identical? ((compile-form '(fn [] (< 1 5)))) true)` 류로 canonical 확인. (c)
  **conformance 에 회귀 픽스처 추가**: host HOF 에 우리-컴파일 boolean-pred 넘기는 케이스(filterv/
  every?/remove) + def 된 boolean 분기 → host≡compiler. (d) gate/smoke 무회귀.
  - 완료(2026-06-29 KST, codex): `box-value` 헬퍼로 모든 `.box` 경로를 `GeneratorAdapter.valueOf`
    로 교체. `compiler-smoke` 145/145, `conformance` 115/115 통과.

#### [x] C-BUG2 HIGH — overload resolution 과도 widening → `(= <int> <float>)` 오답
- **repro(host vs compiler)**:
  ```
  (= 1 1.0)              host=false  compiler=true
  (= 1.0 1)              host=false  compiler=true
  (if (= 5 5.0) :eq :neq)host=:neq   compiler=:eq
  (Math/max 1 2.0)       host=IllegalArgumentException  compiler=2.0
  ```
- **근본원인**: `(= 1 1.0)` 이 `=` :inline 으로 `(clojure.lang.Util/equiv 1 1.0)` :static-call 이 됨.
  우리 overload resolution `param-score`(:550)가 `long→double` 을 싼 primitive widening 으로 점수화
  (`primitive-widening` 표 :496-502 에 `Long/TYPE [Float Double]` 등 *lossy* 포함) → `equiv(double,double)`
  이 `equiv(Object,double)`/`equiv(long,Object)` 보다 이김 → `1`→`1.0` 강제 → 수치 `1.0==1.0`=true.
  host `Compiler.paramArgTypeMatch` 는 더 엄격(long/int arg 가 double/float param 에 *절대* 안 맞음)
  → category-aware `equiv(Object,…)` → false. host `(Util/equiv 1 1.0)`=false 확인됨.
- **callsites**: `primitive-widening`(:496-502, lossy int/long→float/double 포함), `param-score`(:550),
  `widening-distance`(:515).
- **구현**: primitive arg→param 매칭을 host `paramArgTypeMatch` 규칙으로 제한 — `double`←{Double,Float,
  float}, `float`←{Float}, `long`←{Long,int,Integer,short,byte}, `int`←{Integer,long,Long,short,byte}.
  즉 **arg-matching 경로에서 int/long→float/double 항목 제거**(실제 산술 강제 변환 emit 경로와는 구분).
- **검증**: (a) `(= 1 1.0)`/`(= 1.0 1)`/`(if (= 5 5.0) …)` host≡compiler(=false/:neq). (b) `(Math/max
  1 2.0)` 가 host 처럼 *거부*(IllegalArgumentException) — host 와 같은 실패. (c) 기존 interop overload
  smoke(typed long/double 정상 매칭) 무회귀 — conformance/smoke/gate.
- **주의**: 산술 `(+ 1 2.0)` 같은 *진짜 mixed 승격*은 analyzer/Numbers 가 처리(이건 overload 매칭과
  별개) → 그쪽 깨지지 않게 arg-matching 만 제한.
  - 완료(2026-06-29 KST, codex): overload 매칭 전용 `primitive-param-widening`/`primitive-param-distance`
    를 추가해 int/long→float/double 후보화를 제거. `compiler-smoke` 145/145, `conformance` 116/116 +
    negative 22/22 통과.

#### 감사로 SOUND 확인(수정 불필요): R1 independent-range/unroll(overflow checked, min-stride 상한),
recur/loop edge, try/finally, 숫자타워 컬렉션(`[1N 2N]`/`{:k 1/2}`/mixed), variadic/apply/multi-arity,
deftype/defrecord/protocol, set!/volatile, do statement-pop — 전부 host 일치.

> **검증 루프(claude)**: codex 가 한 항목 닫으면 claude 가 `gate`(READY)·`conformance`·`bench`(런타임
> 회귀)·코드 grep(전역상태 제거/이름유일성/예외envelope 실연결)으로 "존재가 아니라 정확성" 재확인.
> 동시성 항목은 멀티스레드 compile 스트레스로 검증.

---

## 🟢 현황 — fixable 전부 완료, frontier 는 슬라이스로 전진(full 은 held) (2026-06-29 3.5회차)

**★ 3.5회차 (claude 직접 frontier 슬라이스 전진, 2026-06-29)**: 사용자 지시("네가 전부 해결")로
claude 가 직접 frontier 슬라이스를 더 멀리 밀었다. **정직 원칙 그대로**: full(R4f~R7f)은 닫을 수
없으므로 held 유지, *자작 표면만* 확대.
- **R5+/R4+**: 자작 frontend_selfhost(tools.analyzer.jvm/host-macroexpand 0) 표면 확대 —
  매크로 +13(`not`/`nil?`/`->>`/`cond`/`when-not`/`if-not`/`if-let`/`when-let`/`as->`/`cond->`/
  `cond->>`/`some->`/`some->>`, 전부 순수 rewrite → let/if/->/= 로 환원, host macroexpand 0),
  코어 연산자 +13(`> >= <= quot rem inc dec zero? pos? neg? first next get`, Numbers/Util/RT 직접·
  clojure.core Var 0). 자작 매크로 총 17종. **벡터 구조분해도 자작**(host clojure.core/destructure
  없이 `(let [[a b] v] …)`→`first`/`next` 환원, 중첩/out-of-bounds→nil 포함). fixture 21→51 전부
  accepted. 새 매크로/연산자/구조분해는 DDC mini-backend 에도 추가해 **host≡compiler.clj≡독립
  mini 3-way(14 fixtures)** 로 *host 대조* 검증(teaching-to-test 아님 — host destructure 와 결과 일치).
- **R6+**: host-core-free runtime fragment 1→3(unchecked add/mul/sub, ASM 스캔으로 clojure.core/Var
  deref 0 + raw ladd/lmul/lsub 확정).
- **검증**: gate READY ✅, frontend-selfhost OK(38 accepted), diverse-double-compile OK(mini-backend
  11 fixtures), runtime-selfhost OK(fragment 3). compiler.clj **여전히 미변경** → core 회귀 0.
- **여전히 held(정직)**: R4f full Wheeler DDC(독립 *전체* 컴파일러)·R5f production reader/
  macroexpander/analyzer(여전히 subset, tools.analyzer.jvm 의존)·R6f full clojure.core/clojure.lang·
  R7f 형식증명. 슬라이스는 *증거 폭만* 넓혔고 full 은 한 발도 닫지 않았다(닫을 수 없음).

---

**claude 실측 검증 결론(3회차)**: R4~R7 첫 라운드도 codex 가 **정직한 frontier slice 로 처리(과장 없음)**.
재검증:
- **R4 ✅**(독립 DDC slice): `frontend_selfhost` 가 `pnix.clj-meta.compiler` 의존 **0**(ns grep 확인) →
  진짜 독립 미니 컴파일러. 4-fixture subset 에서 host≡compiler.clj≡mini-backend, full-wheeler-ddc 는 not-claimed.
- **R5 ✅**(macroexpander slice): host `macroexpand` 호출 **0**(hand-written rewrite: when/and/or/->),
  production macroexpander held.
- **R6 ✅**(runtime fragment slice): `(fn [^long x] (unchecked-add x 1))` 바이트코드에 clojure.core/Var
  참조 **0**(claude 직접 ASM 스캔 확인) → host-core-free 진짜. full runtime held.
- **R7 ✅**(증거 강화): fuzz 에 sub-overflow/raw-lsub sentinel + `:evidence-not-proof-claim` invariant.
  형식증명 아님 유지.
- compiler.clj **미변경**(evidence lane 만 추가) → core 회귀 0. **gate READY ✅, smoke 145/145,
  conformance 112/112, m6aj/bytecode-witness digest 불변.**

### ✅ 닫힌 것 (전부 fixable/closeable)
B1~B8 · G1 · T1~T7 · Finding A/B/2 · R1~R3 · **R4~R7 frontier slice**. = §17 deep-research 의 모든
*고칠 수 있는* 항목 + frontier 의 *점진 슬라이스*까지 닫고 claude 가 실측 검증.

### ⛰ 남은 것 = research frontier "full" (R4f~R7f) — **슬라이스 루프로 닫을 수 없음**
이건 버그가 아니라 *대형 독립 연구 프로젝트*다. 점진 슬라이스(위 R4~R7)는 **증거 폭만** 넓히고
"full" 은 정직하게 held 로 남는다(지금까지 매 라운드 그렇게 유지됨, 옳음).
```
R4f  full Wheeler DDC = 언어 *전체*를 덮는 독립 2nd 컴파일러로 같은 소스 두 번 컴파일 bit/behavior
     동일. 현재: mini-backend 4-fixture subset + cross-host(emit-determinism) + kernel(value-model).
R5f  production frontend self-host = 전체 reader + clojure.core 매크로 전개기 + 전체 analyzer.
     현재: 자작 reader/4-macro/작은 special-form subset(tools.analyzer.jvm 의존 잔존).
R6f  full runtime self-host = clojure.core fn + persistent 자료구조 재구현. 현재: leaf fragment 1개
     (host-core-free) + helper body 8개(런타임은 host 위임).
R7f  완전 언어정확성 형식증명 = theorem prover(Coq/ACL2/HOL) machine-checked. 현재: conformance+fuzz
     = 증거(증명 아님). gate proof-claim 이 not-claimed 로 정직 명시.
```

### 🔀 결정 필요 (사용자)
- **(A) 완료 선언**: 지금이 원칙적 정지점 — gate READY, 모든 fixable 닫힘, frontier 4종은 receipt/
  gate 에 *왜 held 인지* 정직 명시. 더 이상 "고칠 버그"는 없음.
- **(B) frontier 증거 슬라이스 계속**: diminishing returns. 아래 후보(각 honesty guard: receipt 에
  subset/partial/fragment + full=held, self-test-only/teaching-to-test/증거를-증명이라 함정 회피):
  - R5+ : 매크로 더(`cond`/`if-let`/`when-let`/`->>`) 자체 전개(host macroexpand 0 유지)
  - R4+ : mini-backend fixture/op 확대(여전히 subset, 독립성 grep 으로 재확인)
  - R6+ : host-core-free fragment 더(순수 산술/비교; persistent 자료구조는 frontier)
  - R7+ : fuzz 형태/invariant 확대(증명 아님 유지)
  ※ (B)의 어떤 것도 R4f~R7f 를 *닫지* 못함 — 증거만 넓힘. claude 는 매번 grep(실연결/독립성)+
  무력화/스캔으로 정직성 재검증.

---

## ☑ R1~R3 닫힘(claude 검증) + R4~R7 frontier slice 완료 (2026-06-29)

**★ 최신 검증(claude 실측, 2026-06-29 2회차)**: 이전 라운드의 과장 V-U2/U5/U8 을 codex 가
R1/R2/R3 로 **진짜로 닫았고 claude 가 재검증함(이번엔 과장 없음)**:
- **R1 ✅**(V-U2 해소): 독립 bounded-unroll range 가 *실 컴파일러* admission(compiler.clj
  `checked-long-static-no-overflow?`)에 `:independent` 로 연결됨(grep 확인). smoke
  "checked-long admission uses independent bounded-unroll range" → homogeneous-stride gate 를
  꺼도 Finding B 가 admission 에서 reject→checked overflow throw. **무회귀 증거: m6aj digest
  `6eb83dc…`·bytecode-witness digest `df0837…` 둘 다 불변** → 정당한 raw 승격 0 회귀. join(⊔)
  이라 sound-by-construction(새 miscompile 불가).
- **R2 ✅**(V-U8 해소): fuzz 가 `^long`/long-init·Long/MAX 경계값으로 overflow 도달 **2,893건**
  (host≡compiler 둘 다 throw)+raw opcode `[:ladd :lmul]` 도달. **mutant 검출력: 두 가드(엔진 fix+
  독립 가드) 다 끄면 Finding B 가 host=throw/compiler=wrap 으로 검출(`would-fail? true`)** → 진짜
  이빨 있음.
- **R3 ✅**(V-U5 해소, 경미): DDC kernel claim 을 `partial-independent-value-semantics-model`
  (host clojure.core 공유, 별도 bytecode 컴파일러 아님, full Wheeler DDC 아님)로 정직 격하.
- 전체: **gate READY ✅, conformance 112/112, smoke 145/145, m6aj/bytecode-witness 불변.**

아래 V/R1/R2/R3 상세는 *기록(done)*. **Tier C 의 R4~R7 은 full claim 이 아니라
actionable frontier slice 를 닫고 full research frontier 는 held/not-claimed 로 유지**한다.
이전 검증(1회차): gate READY, conformance 112/112, negative 21/21, kernel 3-way 112/112,
smoke 144/144 — 기능 회귀 없었으나 V-U2/U5/U8 과장이 있었고, 위처럼 해소됨.

2026-06-29 codex pass: R4~R7 점진 슬라이스 처리 완료. 검증:
`clojure -M:frontend-selfhost` OK (21 accepted + production held),
`clojure -M:diverse-double-compile` OK (`:independent-mini-backend-subset` accepted),
`clojure -M:runtime-selfhost` OK (`:rt-unchecked-inc-long-leaf` host-core Var deref 0 + raw `:ladd`),
`clojure -M:fuzz-conformance` OK (257 programs, 10,007 comparisons, failures=0, raw `:lsub`
sentinel 포함). Full Wheeler DDC / production frontend / full runtime / formal proof 는 계속 held.

### ⚠ 검증에서 드러난 과장 (V) — "틀린 것"
```
V-U2  octagon "defense-in-depth"가 실 컴파일러에 연결 안 됨 (가장 중요)
      tv/lowering-sound? 는 :effective range(=공급 range ⊔ 독립 octagon range)로 검사하도록
      바뀌었으나, *실제 컴파일러* admission(compiler.clj:786-792 checked-long-static-no-
      overflow?)은 make-vc 에 :independent 를 **안 넘긴다**(grep :independent in compiler.clj
      = 0). → 실 컴파일러에선 independent-lhs/rhs=nil → effective=공급 그대로 → 방어 무효.
      octagon 방어는 translation_validation 의 *self-test 후보*(손수 만든 :independent)에서만
      작동. 즉 Finding B 형태의 너무-좁은 engine range 는 *여전히* 실 컴파일러 validator 를
      통과한다(실제 보호는 내가 한 engine-레벨 homogeneous-index-stride? gate 뿐). U2 수용기준
      ("Finding B 가 엔진 수정 없이도 validator 에서 걸린다")은 **미충족**.
V-U5  kernel "독립 2nd backend"의 독립성 과장
      kernel 이 deftype/defrecord 를 **맵으로 모델링**(::instance/::record/::type, fake-class/
      fake-field)하고 .getModifiers/.getDeclaredField/.getLookupThunk 를 가로채 *기대답*을
      돌려준다(특히 volatile-field reflection 행은 conformance 가 검사하는 값을 그대로 모델링).
      → "host≡compiler≡kernel 112/112 독립 crosscheck"는 value-semantics 모델일 뿐, 독립 JVM
      type-generation/reflection 이 아니다(teaching-to-test 성격). full typegen 은 held 라벨돼
      있어 *부분 정직*하나, "독립" 단어가 과하다. (전역 class/record?/dissoc 오버라이드는
      fall-through 라 무해 — 버그는 아님.)
V-U8  fuzz 가 soundness-critical 경로를 못 친다
      fuzz_conformance 생성기 값이 bound 0~8·factor[-2..2]·init/arg[-10..10] → acc 최대 ~±2560.
      Long/MAX 근처 overflow/raw-opcode prover 에 *절대* 안 닿는다. backend 는 실제 사용(host≡
      compiler 진짜 비교)이라 회귀안전망으론 유효하나, "language-correctness evidence"치곤
      저-파워 — Finding B 류 overflow miscompile 은 이 fuzz 로 못 잡는다.
```

2026-06-29 codex pass: R1/R2/R3 처리 완료. 검증:
`clojure -M:compiler-smoke` ALL OK (145/145), `clojure -M:translation-validation` OK,
`clojure -M:lowering-admission` OK, `clojure -M:diverse-double-compile` OK,
`clojure -M:fuzz-conformance` OK (255 programs, 10,005 comparisons, failures=0,
overflow comparisons=2,893). `grep -n ':independent' clj-meta/src/pnix/clj_meta/compiler.clj`
에서 실제 admission 경로 연결 확인.

### Tier A — 과장을 실제로 닫기 (P1, 닫기가능) — 각 항목 "어떻게 닫나" 코드 분석 포함

#### [x] R1 (V-U2 닫기) — independent 2차 도메인을 *실 컴파일러* admission 에 연결
- **안 닫힌 것**: validator(tv/lowering-sound?)는 `:effective = 공급 ⊔ 독립` range 로 검사하게
  바뀌었지만, 실 컴파일러는 `:independent` 를 안 넘겨서(아래) 방어가 self-test 에서만 작동.
- **현재 코드**: `checked-long-static-no-overflow?` (compiler.clj:781-792)
  ```
  ar (long-range-of env (first args)) ; 피연산자 range (interval 엔진 — fast recognizer 포함)
  br (long-range-of env (second args))
  (tv/lowering-sound? (tv/make-vc {:op .. :opcode .. :lhs ar :rhs br}))  ; :independent 없음
  ```
  `long-range-of` 는 loop 지역에 대해 *fast 닫힌식 recognizer*(loop-binding-accumulator-range
  등)를 우선 쓴다 — Finding B 의 버그가 바로 이 fast recognizer 였다. validator 가 같은 fast
  range 만 받으면 절대 못 잡는다.
- **핵심 통찰(함정 회피)**: "독립"이려면 *공급 range 와 구조적으로 다른 경로*여야 한다. 같은
  interval 엔진/같은 fast recognizer 를 다시 호출해 :independent 로 넘기면 join 이 항등 → 무의미
  (현재 self-test 처럼 손수 만든 octagon 만 의미 있었던 이유). 올바른 *독립 오라클* = 항상-건전한
  **bounded-unroll 엔진**(loop-unroll-ranges / loop-ai-ranges, compiler.clj:1724~). fast
  recognizer 는 속도용 최적화이고 unroll 은 건전한 기준선이다 — 둘이 불일치하면 fast 가 너무 좁은
  것이다.
- **어떻게 닫나(2단계)**:
  1. loop-derived 지역의 range 를 admission 직전에 *두 경로*로 구한다: (i) 현행 `long-range-of`
     (fast recognizer 포함), (ii) bounded-unroll 전용 도출(`loop-unroll-ranges` 만, fast 우회).
     unroll 결과를 `:independent` 로 `make-vc` 에 넘긴다. validator 의 `effective = (i)⊔(ii)` 가
     너무 좁은 (i) 를 자동으로 넓혀 overflow → reject.
  2. unroll 이 적용 불가(미지 iteration 등)면 `:independent` 생략 — 그 경우는 fast recognizer 도
     range 를 못 줘서 어차피 checked fallback(R 무관). octagon(abstract_octagon)을 2차 도메인으로
     추가로 쓰고 싶으면 같은 자리에 끼워도 됨(단 *독립 도출*이어야 — 공급 range 재사용 금지).
  - 더 단순·강한 대안: `long-range-of` 가 bounded loop 지역에 대해 *항상* `fast ⊔ unroll` 을
    반환하게 해 fast 단독 신뢰를 폐기(validator 거치기 전에 이미 건전). U2 의 "validator 가 잡는다"
    프레이밍을 원하면 1안, 최대 건전성을 원하면 이 대안.
- **수용기준(claude 검증)**: (a) `grep -n ':independent' compiler.clj` 에서 admission 경로가
  실제로 넘김. (b) homogeneous-index-stride? gate 를 테스트에서 *일시 무력화*(또는 fast recognizer
  를 강제 too-tight)해도 Finding B 입력이 **admission(lowering-sound?) 단계에서 false → checked
  fallback → host≡compiler=throw**. (c) gate/m6aj/range-migration 무회귀(정당한 raw 승격은 유지).
- **함정**: 독립 경로가 fast 와 *같은* 코드면 검증에서 즉시 들통(grep + 무력화 테스트). 반드시
  algorithmically 다른 경로(unroll)로.
- **결과(2026-06-29)**: loop env entry 에 bounded-unroll range 를 fast range 와 별도 proof 슬롯으로
  저장하고, `checked-long-static-no-overflow?` 가 `tv/make-vc`에 `:independent` interval 로 넘긴다.
  `translation_validation`은 octagon self-test 와 compiler bounded-unroll interval 모두를 independent
  operand derivation 으로 받아 effective range 를 검사한다. `loop-unroll-ranges`는 다중 then-recur 를
  join 하므로 Finding B 이질 stride도 독립 range 를 얻는다. smoke 에 homogeneous stride gate 를
  비활성화한 Finding B 회귀를 추가했고, independent admission 때문에 raw 승격이 거부되어 checked
  overflow 로 fail-closed 됨을 고정했다.

#### [x] R2 (V-U8 닫기) — fuzz 를 soundness-critical(overflow/raw-opcode) 범위로 확장
- **안 닫힌 것**: fuzz_conformance 생성기 값이 ~±2560 라 raw-long prover/overflow 경로에 안 닿음
  → Finding B 류 miscompile 검출력 0.
- **현재 코드**: fuzz_conformance.clj `gen-loop`(:54) bound[0,8]·step[1,3]·init[-10,10]·
  factor[-2,2]; `gen-args`(:106) [-10,10]; 파라미터에 `^long` 태그 없음.
- **핵심 통찰(함정 회피)**: (1) `^long` 태그가 없으면 전부 Object boxing → 항상 checked
  Numbers 경로 → raw opcode 자체가 안 나옴 → prover 를 안 친다(증명력 0). 반드시 `^long` 파라미터/
  loop 지역. (2) 값이 작으면 overflow 안 남 → checked/unchecked 차이가 안 드러남.
- **어떻게 닫나**:
  1. 생성기에 *큰 값* 분포 추가: init/arg 를 {0,1,-1, Long/MAX_VALUE, Long/MIN_VALUE,
     Long/MAX_VALUE-k, 2^40 류}에서도 뽑게.
  2. *Finding B 형태* 생성: 두 then-recur 가 index 를 서로 다른 stride(+3/+2)로 올리며 `(+ acc i)`
     누적 + 큰 K 더하기. *param-bound nonlinear* `(* acc acc)` 도.
  3. 파라미터/loop 지역에 `^long` 부여(raw 경로 강제). equivalence 체크는 이미 error-class 매칭
     지원(둘 다 ArithmeticException "long overflow" 면 OK).
- **수용기준**: (a) fuzz corpus 에 host=compiler 둘 다 `long overflow` throw 케이스 ≥1(=overflow
  도달 증명). (b) **회귀 검출력 테스트**: homogeneous-stride fix(또는 임의 prover)를 일시 무력화한
  빌드에서 *이 fuzz 가 실패*(host≠compiler 검출). 자동화하려면 `*disable-...*` 플래그/시드 제공.
  (c) 기본 fuzz 는 그대로 host≡compiler(failures=0).
- **함정**: 큰 값만 넣고 `^long` 안 붙이면 raw 경로 안 타서 "통과"만 늘 뿐 증명력 0 — 검증 때
  생성 corpus 에 `^long`+raw 케이스 있는지 grep/샘플 확인.
- **결과(2026-06-29)**: `fuzz_conformance` 생성기에 Long 경계값/2^40 계열/3037000500 등 큰 값 분포와
  long-typed params 를 추가하고, loop 는 host-compatible source 로 두되 long init 기반 primitive slot
  경로를 타게 했다. Finding B 이질 stride, overflow arg add/mul, raw nonlinear-unroll sentinel 을
  고정 corpus 에 포함했다. receipt 는 overflow comparisons 2,893개, raw opcode sentinel
  `[:ladd :lmul]`, homogeneous gate+independent guard 를 같이 끈 mutant 에서 host=throw/compiler=value
  불일치 검출을 기록한다.

### Tier B — 독립성 주장 정직화 (P2, 대부분 문서/프레이밍)
#### [x] R3 (V-U5 정직화) — kernel "독립 2nd backend" 주장을 실제 범위에 맞춤
- **안 닫힌 것**(경미): kernel 은 tree-walker + 맵-모델(host clojure.core 공유)이라, "독립 2nd
  *backend*"(=별도 bytecode 컴파일러)가 아니다. 단 value-semantics(=/hash/assoc/dissoc 강등 등)는
  맵-모델로 *진짜 독립 계산*이고, B6(volatile) 같은 컴파일러 버그도 kernel≠compiler 로 잡아내므로
  cross-check 가치는 실재한다 — 이 항목이 세 V 중 가장 경미.
- **어떻게 닫나**: DDC receipt(`independent-kernel-evaluator-supported-corpus`)의 문구를 정직화 —
  "tree-walking **value-semantics 모델**(host clojure.core 공유; 별도 bytecode 컴파일러 아님,
  partial 독립). reflection 행(getModifiers/isVolatile/getLookupThunk)은 JVM 규칙을 *모델링*한
  것"으로. 원하면 corpus 를 "value-semantics 독립" vs "reflection 모델" 두 그룹으로 나눠 독립 주장
  범위를 명시. full bytecode 2nd compiler = R4 로 분리.
- **수용기준**: receipt 독립성 문구가 "tree-walker/맵-모델/partial 독립"을 명시(과장 단어 "독립
  2nd backend" 단독 사용 제거). 동작 무변경, gate 무회귀.
- **결과(2026-06-29)**: DDC row `:independent-kernel-evaluator-supported-corpus`의 kind/claim/scope 를
  `partial-independent-value-semantics-model`로 정직화했다. host clojure.core 공유, deftype/defrecord/
  reflection 맵 모델, 별도 bytecode compiler 아님, full Wheeler DDC 아님을 receipt 에 명시했다.

### Tier C — 남은 것 = research frontier R4~R7 (held; 점진 진척은 가능, "full" 은 억지 금지)
**중요(codex 必)**: R4~R7 은 한 슬라이스로 "full" 을 닫는 게 *불가능*하다(독립 컴파일러/자작
analyzer/runtime 재구현/theorem prover 급 대형 작업). 그러니 목표는 **(a) 정직한 점진 진척을
한 조각씩 올리되 receipt 에 "subset/partial/fragment, full 은 held(왜)" 를 명시**하거나, **(b)
못 올리면 held 유지 + 이유 기록**. 아래 각 항목에 *현재 / full 이 요구 / 이번에 올릴 수 있는
진짜 한 조각 / 함정 / 수용기준* 을 적었다.

#### [x] R4 — full Wheeler DDC (독립 2nd 컴파일러)
- **현재**: cross-host(같은 backend, host 버전만 다름)=emit-determinism + kernel(tree-walker
  value-semantics 모델, R3)=partial. 둘 다 *독립 2nd 컴파일러*가 아님.
- **full 이 요구**: compiler.clj 와 *알고리즘적으로 독립*인 두 번째 bytecode 컴파일러(다른 emit
  전략, recognizer 공유 X)로 같은 소스를 컴파일해 behavior(또는 bit) 동일성 확인 → backdoor 가
  한쪽에만 있으면 불일치로 검출(진짜 Trusting-Trust 방어).
- **이번에 올릴 한 조각**: `frontend_selfhost.clj` 의 tiny emitter(이미 compiler.clj 와 별개 코드)를
  *독립 미니 backend* 로 키워, conformance subset 을 compiler.clj backend 와 **둘 다 컴파일→behavior
  동일** 비교하는 DDC row 추가(host≡backend1≡backend2). 이게 kernel(인터프리터)보다 강한 "독립
  *컴파일러*" 증거.
- **함정**: tiny backend 가 compiler.clj 의 recognizer/range 엔진/emit helper 를 재사용하면 독립
  아님 — 반드시 자체 emit. cross-host 를 "full DDC" 라 부르지 말 것(이미 not-claimed).
- **수용기준**: 독립 미니 backend 가 subset 에서 host≡compiler≡mini-backend; receipt 가 "independent
  *compiler* (subset), full Wheeler DDC=held" 로 명시.
- **결과(2026-06-29 codex)**: `diverse_double_compile` 에
  `:independent-mini-backend-subset` row 추가. `frontend_selfhost/compile-source` 의 자체
  reader+rewrite macroexpander+analyzer+direct ASM emitter 를 독립 미니 backend 로 사용해
  arithmetic/let/loop/thread-first macro subset 에서 host≡compiler.clj backend≡mini-backend 를 확인.
  receipt 는 `:full-wheeler-ddc`, `:compiler-binary-ddc`, production frontend/runtime replacement 를
  not-claimed 로 유지. 검증 `clojure -M:diverse-double-compile` OK.

#### [x] R5 — full frontend self-host (reader/macroexpander/analyzer)
- **현재**: tiny frontend 17-fixture subset(자작 reader+analyzer+emit, tools.analyzer.jvm 0회), 단
  *매크로 전개 없음*·특수형식 일부만.
- **full 이 요구**: production reader + **macroexpander(clojure.core 매크로 전개)** + 전체 analyzer.
  매크로 전개가 핵심 난관(when/and/or/-> 등이 fn/if/let 으로 풀려야).
- **이번에 올릴 한 조각**: tiny frontend 에 **최소 macroexpander** 추가 — `when`/`and`/`or`/`->`
  몇 개를 자체 전개 규칙으로 fn/if/let 으로 풀어 자작 analyzer 가 받게(예: `(when c a)`→`(if c a)`).
  "매크로 전개도 self-host(subset)" witness.
- **함정**: host `macroexpand` 를 호출하면 self-host 아님 — 자체 전개 규칙이어야. subset 을 "frontend
  self-host 완료"로 부르지 말 것(receipt 항상 "covered subset + production held").
- **수용기준**: 자작 macroexpander 가 매크로 fixture 를 host `macroexpand` 없이 전개→emit→host≡결과;
  receipt 에 전개 규칙 목록 + production held 명시.
- **결과(2026-06-29 codex)**: `frontend_selfhost` 에 host `macroexpand` 호출 없는 자체 rewrite
  macroexpander 추가(`when`, `and`, `or`, `->`). macro fixture 4개가 자작 reader→macroexpander→
  analyzer→ASM emitter 로 accepted. receipt 에 `:uses-host-macroexpand false`, rule 목록,
  production clojure.core macroexpander held 를 명시. 검증 `clojure -M:frontend-selfhost` OK.

#### [x] R6 — full runtime self-host (clojure.core/clojure.lang)
- **현재**: helper 8개의 *본문*은 우리 bytecode 지만 호출하는 fn(+,first,reduce)·자료구조는 host
  clojure.core/clojure.lang. = body self-host, runtime host.
- **full 이 요구**: clojure.core fn + persistent 자료구조 재구현(거대). 
- **이번에 올릴 한 조각**: host clojure.core 를 *전혀 호출하지 않는* **leaf fn** 을 우리 backend 로
  직접 emit — 예: `(fn [^long x] (inc x))` 를 host `inc` Var 호출 없이 raw `LADD 1` 로, 또는 작은
  순수 산술 fn. "runtime *fragment* self-host(host-core 호출 0)" witness(R6 의 진짜 독립 조각).
- **함정**: 본문이 host `reduce`/`first`/`+`(Var)를 부르면 runtime self-host 아님 — 검증 때 emit
  bytecode 에 `clojure.lang.Numbers`/`RT` 외 clojure.core Var deref 가 없는지 확인. helper "8개"를
  "runtime self-host" 라 과장 말 것(body 만 우리 것).
- **수용기준**: 새 fragment 가 clojure.core Var deref 0회(bytecode grep)로 host≡compiler; receipt 에
  "runtime fragment(host-core 호출 0), full runtime=held" 명시.
- **결과(2026-06-29 codex)**: `runtime_selfhost` 에 `:rt-unchecked-inc-long-leaf` 추가. generated
  class bytecode 를 ASM 으로 스캔해 `clojure.core`/`Var` deref 0회와 raw `:ladd` 존재를 invariant 로
  고정했다. 기존 helper 들은 host runtime boundary 로 유지하고 full clojure.core/clojure.lang runtime
  은 held. 검증 `clojure -M:runtime-selfhost` OK.

#### [x] R7 — 완전 언어정확성 형식증명 (held, 증거 강화만 가능)
- **현재**: conformance(112) + fuzz(10k, overflow 도달) = *증거*. 형식증명 아님.
- **full 이 요구**: theorem prover(Coq/ACL2/HOL) 안에서 source↔bytecode semantic-preservation
  machine-checked 증명(CompCert/CakeML 류) — 별도 대형 연구.
- **이번에 올릴 한 조각**: 증거 강화만(fuzz 프로그램 형태/입력/invariant 확대, property 추가).
  *절대 "proof" 라 라벨 금지* — gate proof-claim 의 not-claimed 유지.
- **함정**: conformance/fuzz 를 "correctness proof" 로 부르는 순간 과장. gate 문구 그대로.
- **수용기준**: 증거 lane 확대 시 receipt 가 "evidence-strengthening, not proof" 유지.
- **결과(2026-06-29 codex)**: `fuzz_conformance` 에 subtraction overflow sentinel
  `(- Long/MIN_VALUE 1)`, raw `:lsub` sentinel, `:evidence-not-proof-claim` invariant 를 추가.
  receipt policy 는 `"evidence-strengthening only"` 와 `:not-claimed :full-language-correctness-proof`
  를 canonical receipt 에 포함한다. 검증 `clojure -M:fuzz-conformance` OK (257 programs,
  10,007 comparisons, failures=0).

> **정직성 원칙(codex 必, R1~R3 에서 검증됨)**: frontier 를 억지로 accepted 로 칠하지 말 것 —
> subset/partial/fragment 면 receipt 에 그렇게 + "full=held(왜)". 3대 함정: **self-test-only**(실
> 코드 미연결)·**teaching-to-test**(모델이 정답을 알게)·**저-파워/증거를-증명이라**(raw·overflow
> 경로 안 침, evidence 를 proof 라 함).
> **검증 루프(claude)**: codex 가 한 라운드 닫으면 `gate`(READY)·해당 lane + **코드 grep(실 연결/
> host-core 호출 여부)** + **무력화/회귀 검출력 테스트**로 "존재가 아니라 정확성"을 재확인(R1 을
> grep+무회귀 digest 로, R2 를 mutant 검출력으로 검증했듯이).

---

## ☑ codex 다음 라운드 — U1~U10 fixable slice 완료 + held frontier 명시 (2026-06-29)

B1~B8/G1 + T1~T7 + Finding A/B/2 는 모두 닫힘(아래 §17). 아래 U 항목도 fixable slice 는 닫고,
완전판이 필요한 영역은 receipt/gate 에 held/not-claimed 경계로 명시했다.
각 항목: 무엇 / 해결방향(코드 수준) / **수용기준(=내가 검증할 것)** / 난이도.
규칙: diagnose-first(픽스처 먼저), gate 무회귀 유지, 슬라이스별 커밋/푸시(한글, Co-Authored-By),
RAW-FREE/no-auto-promotion 헌법 준수. **닫을 수 없는 frontier 는 "왜 held 인지" 코드/receipt 에
정직히 명시**(억지 emit 금지). codex 가 한 라운드 돌면 내가 gate+conformance+코드로 검증한다.

2026-06-29 codex pass: U1~U10 처리 완료. 검증: `clojure -M:compiler-smoke` ALL OK
(144/144), `clojure -M:translation-validation` OK, `clojure -M:bytecode-witness` OK,
`clojure -M:conformance` ALL OK (112/112, negative 21/21), `clojure -M:lowering-admission` OK,
`clojure -M:language-surface` OK, `clojure -M:diverse-double-compile` OK,
`clojure -M:frontend-selfhost` OK, `clojure -M:runtime-selfhost` OK,
`clojure -M:fuzz-conformance` OK (10,000 host≡compiler comparisons, failures=0),
`clojure -M:gate` READY, `git diff --check` clean. kernel 3-way 는 conformance 전체
112/112 일치, unsupported 0.
U5 는 kernel 독립 evaluator 가 conformance 전체를 accepted 로 덮고, U6/U7 은 닫을 수 있는
최소 증거를 accepted 로 올렸다. 완전판은 held/not-claimed 로 고정했다.
추가 pass: U6 tiny frontend 는 자체 reader/analyzer/direct ASM emitter 표면을 `do`/sequential
`let`/tail `loop`+`recur`/boolean+nil/string/keyword literal/`=`/vector/map/set literal/
explicit `quote`(symbol/list/nested data) 까지 확장해 17개 fixture
accepted, tools.analyzer/Clojure reader 0회.
추가 pass: U7 runtime helper witness 는 `get`/`reduce`/`conj`/`vec`/higher-order `map` 표면을
더해 8개 helper accepted, fallback diagnostics 0회. 완전 runtime 은 계속 held/not-claimed.
U10 deep-research 결과 새 즉시-fixable 구현 항목은 없음; full Wheeler DDC/in-logic bootstrap/
완전 runtime 은 연구 frontier 로 남는다.

### Tier A — 지금 닫을 수 있음 (fixable-now, 우선)
- [x] **U1** compile-form 비-fn top-level 가 host eval → backend-wrap-emit
  · 무엇: `compile-form` 의 마지막 분기(루트 op ≠ `:fn`)가 host `eval` 로 떨어짐. `eval-form`
    은 이미 `((compile-form (list 'fn [] form)))` 로 backend wrap-emit 함 → 같은 경로로 통일.
  · 해결: 비-fn top-level 을 `(fn [] form)` 로 감싸 backend 컴파일·invoke. self-host 파이프라인엔
    영향 없고(이미 wrap 됨) 일반 API 의 host-Compiler 터치 1곳 제거.
  · 수용기준: `(compile-form '(+ 1 2))` 류가 host eval 0 회로 실행; smoke/conformance/gate 무회귀.
  · 난이도: 낮음(닫기가능).
  · 결과(2026-06-29): non-fn top-level 은 wrapper fn 으로 backend emit; fallback 진단 0, generated fn invoke 검증.
- [x] **U2** TV validator 독립 operand-range 재도출 (defense-in-depth)
  · 무엇: `tv/lowering-sound?` 는 *공급된* range 위 finiteness 만 본다(§17.8 Finding 2). Finding B
    처럼 너무 좁은 range 는 validator 가 못 잡고 엔진만 책임진다.
  · 해결: validator 가 *구조적으로 다른* 2번째 도메인(octagon `abstract_octagon`)으로 operand range
    를 독립 재도출해, 두 도메인이 불일치하거나 더 넓은 쪽이 overflow 가능하면 reject. = 엔진 버그를
    validator 가 2차로 잡는 진짜 defense-in-depth.
  · 수용기준: 의도적으로 너무 좁은 range 를 주입한 적대 VC 가 reject; Finding B 형태가 (엔진 수정
    없이도) validator 단계에서 걸리는 회귀 추가; gate 무회귀.
  · 난이도: 중(닫기가능, 단 2nd 도메인 soundness 에 상대적 — 한계 명시).
  · 결과(2026-06-29): 독립 octagon-derived range 를 VC에 결합하고 더 넓은 effective range 로 검증.
    too-narrow adversarial VC 는 validator/lowering-admission 에서 held/reject.
- [x] **U3** compile-form fallback 하드닝 (저가치)
  · analyze-failure 의 blanket `(catch Throwable _)` 를 좁히고 무엇이 잡혔는지 기록; 비정상
    free-var 분기를 `eval` 대신 `throw`(정상 top-level fn 은 자유변수 0 이라 무영향).
  · 수용기준: 정상 케이스 무회귀 + 잡힌 원인이 진단에 남음.
  · 난이도: 낮음(닫기가능).
  · 결과(2026-06-29): analyzer catch 를 `Exception` 으로 좁히고 fallback diagnostics atom 추가.
    top-level free-var 는 host eval 대신 `:compile-form-free-vars` 예외로 실패.

### Tier B — 공략 가능(진짜 진척, 큼)
- [x] **U4** deftype/reify host-eval 잔여 직접 emit
  · 무엇: 사용자 `defprotocol` 구현 deftype 는 `tools.analyzer.jvm` 자체 NPE 로 form 전체 host eval
    fallback(§17.2 G2). "host Compiler 0회" 의 마지막 type-gen 구멍.
  · 해결: analyzer NPE 우회(deftype* 직접 분석 경로 or 최소 stub) 후 `emit-deftype-class` 로 직접 emit.
  · 수용기준: 사용자 defprotocol 구현 deftype 가 host eval 0 회로 backend emit, host≡compiler
    (conformance), gate 무회귀.
  · 난이도: 중~높음(공략가능; analyzer 우회가 관건).
  · 결과(2026-06-29): top-level `compile-ns` incremental 경로에서 사용자 `defprotocol` 구현
    `deftype` 가 fallback diagnostics 0으로 backend load 됨을 `language-surface` 전용 row
    `:defprotocol-deftype-direct` 로 고정. host(load-string)≡backend 결과 42, named class 직접 로드.
- [x] **U5** 독립 2nd 백엔드 = genuine (full Wheeler) DDC 근접
  · 무엇: 현 cross-host DDC 는 *같은* 백엔드를 두 host 버전에서 돌려 emit-determinism 만 증명
    (§17.3 H7). compiler.clj/asm 의 backdoor 는 양쪽 상쇄 → Trusting-Trust 방어 아님.
  · 해결: *독립 구현* 2번째 백엔드(느린 트리워킹 evaluator 또는 다른 emit 전략)를 만들어 같은 target
    의 *동작*(또는 bytecode 동등성)을 교차검증. kernel.clj(보조 인터프리터)가 출발점일 수 있음.
  · 수용기준: 독립 백엔드가 conformance corpus 에서 host≡backend1≡backend2; DDC receipt 가
    "emit-determinism" 에서 "독립 교차검증"으로 격상; 무회귀.
  · 난이도: 높음(공략가능, 큰 작업). 못 닫으면 정확히 어디까지 독립인지 명시.
  · 결과(2026-06-29): `diverse-double-compile` 에 `:independent-kernel-evaluator-supported-corpus`
    row 추가. `kernel.clj` tree-walking evaluator 가 conformance corpus 전체에서
    host≡compiler≡kernel 112/112, unsupported 0 으로 accepted. `deftype` volatile field
    reflection, `defrecord` map ctor/lookup/dissoc/equality/hash semantics 는 kernel 모델로
    직접 처리한다. 단, 일반 독립 `deftype`/`defrecord` runtime 과 full JVM type-generation
    second backend 은 `:independent-kernel-evaluator-typegen-gap` held/not-claimed 로 명시.
    full Wheeler compiler-binary DDC 는 계속 held(not-claimed).
- [x] **U6** 프론트엔드 self-host (analyzer/reader 외부 의존 축소)
  · 무엇: read/macroexpand/analyze 가 외부 `tools.analyzer.jvm`+host `clojure.core` 매크로 →
    "컴파일러가 자기를 컴파일"은 backend-compiles-backend(프론트엔드는 self-host 밖, §17.3 H3).
  · 해결: 최소 서브셋이라도 자작 analyzer/reader 로 컴파일러 일부 form 을 분석→emit, 외부 analyzer
    0회 경로를 1개라도 확보(점진적 self-host).
  · 수용기준: 자작 analyzer 로 분석·emit 한 form 이 host≡compiler; 외부 analyzer 미사용 witness.
  · 난이도: 높음(공략가능, 매우 큼). 부분만 닫고 나머지 held 명시 가능.
  · 결과(2026-06-29): `pnix.clj-meta.frontend-selfhost` 추가. 자체 tiny reader/parser +
    tiny analyzer + direct ASM emitter 가 `tools.analyzer.jvm`/Clojure reader 없이 17개 fixture 를
    직접 컴파일·실행. 지원 표면: `fn`, `if`, `do`, sequential `let`, tail `loop`+`recur`,
    `+`/`-`/`*`/`<`/`=`, locals/long/boolean/nil/string/keyword constants,
    vector/map/set literal, explicit `quote` symbol/list/nested data.
    production frontend replacement 는 held 로 고정.

### Tier C — deep frontier (진척 = 증거 강화; 닫거나 "왜 held" 명시)
- [x] **U7** 완전 runtime self-host 탐색
  · clojure.core/clojure.lang 위임(신뢰 host base, §17.3 H6) 중 직접 emit 가능한 핵심 fn 을 식별·자작.
    대부분 held 가 정직 — 닫을 수 있는 최소 부분만 닫고 경계 명시.
  · 수용기준: 새로 자작·직접 emit 한 runtime fn 이 host≡compiler; 나머지 held 이유 명시. 난이도: 높음.
  · 결과(2026-06-29): `pnix.clj-meta.runtime-selfhost` 추가. `rt-inc-long`, `rt-second-seq`,
    `rt-assoc-keyword-lookup`, `rt-get-after-assoc`, `rt-reduce-sum`, `rt-conj-vector`,
    `rt-map-inc-materialize`, `rt-count-loop` helper 는 clj-meta generated fn 으로 직접 emit되고
    fallback diagnostics 0. clojure.core/clojure.lang/JVM 전체 재구현은 held 로 고정.
- [x] **U8** 언어정확성 증거 강화 (형식증명은 held)
  · 완전 형식증명은 theorem prover(ACL2/Coq) 필요 → held. 대신 property-based/fuzz conformance 를
    대폭 확대(랜덤 프로그램 host≡compiler differential)해 miscompile 표면을 더 훑는다(Finding B 류).
  · 수용기준: fuzz/PBT 하니스가 N(예 ≥10k) 랜덤 케이스에서 host≡compiler, 발견 miscompile 0 또는 수정.
    난이도: 중(증거강화 닫기가능; 형식증명 자체는 held).
  · 결과(2026-06-29): `pnix.clj-meta.fuzz-conformance` lane + `:fuzz-conformance` alias 추가.
    기본 seed `1729176329`, 250개 generated program × 40 inputs = 10,000 comparisons,
    failures=0. receipt: `clj-meta/proof/fuzz-conformance.receipt.edn`.
- [x] **U9** statically-unknown-iteration nonlinear recurrence
  · 반복 횟수가 정적 미지(param bound)인 nonlinear(예 param `(* acc acc)`)은 어떤 도메인으로도 long
    안에서 bound 불가 → **checked fallback 유지가 정답**(이미 그러함). 닫는 게 아니라 *정확히 held*
    임을 보장: 이런 후보가 raw 로 새지 않는지(항상 checked) 회귀로 고정.
  · 수용기준: param-bound nonlinear 누적이 항상 checked(overflow 시 throw, host≡compiler) 회귀 추가.
    난이도: 낮음(held 고정).
  · 결과(2026-06-29): compiler-smoke/conformance negative/bytecode-witness 에 param-bound nonlinear
    회귀 추가. `LMUL` forbidden, `clojure.lang.Numbers.multiply` fallback evidence 고정.

### Tier D — 별도 리서치
- [x] **U10** (3)(4)(5) 정식 deep-research 패스
  · T7 은 코드감사+not-claimed 경계로 대체했고 §17.8 은 코드감사. 웹/논문 기반 *정식* deep-research
    (meta-circular 진짜성·DDC·AI-soundness frontier 의 최신 기법: 독립 컴파일러 구축법, in-logic
    bootstrap, octagon/polyhedra 신뢰경계)는 미실행. 새 발견은 위 U 항목에 환류.
  · 수용기준: 리서치 리포트 + 발견된 fixable 항목이 U 리스트에 추가됨. 난이도: 중(리서치).
  · 결과(2026-06-29): primary/공식 자료 확인.
    - Wheeler DDC: full Trusting-Trust 방어는 trusted/diverse compiler 로 compiler source 를 두 번
      컴파일하고 bit-for-bit 동일성을 확인해야 함. 현재 clj-meta 의 cross-host/backend/kernel
      evidence 는 partial 이며 full compiler-binary DDC 는 held 가 정직.
    - CakeML: compiler backend 와 bootstrap 을 HOL 내부(in-logic)에서 수행해 machine-code compiler
      correctness theorem 까지 얻는 방향이 frontier. clj-meta 의 U6 tiny frontend witness 는 그쪽으로
      가는 작은 증거일 뿐 production frontend replacement 는 held.
    - CompCert: Coq 안에서 source/target semantic preservation 를 증명하는 verified compiler 모델.
      clj-meta gate 는 language-correctness theorem 이 아니므로 not-claimed 유지.
    - Octagon domain: `±x±y<=c` 관계, DBM 기반 O(n^2) memory/O(n^3) operators. nonlinear recurrence/
      full polyhedra/Presburger 증명은 별도 domain/prover 필요. U2/U9 held 경계와 일치.
    새 즉시-fixable 구현 항목은 없음. 발견된 작업은 모두 U5/U6/U7 의 accepted+held receipt 로 반영.

> 검증 루프: codex 가 위 항목을 한 라운드 닫으면, 내가 `clojure -M:gate`(READY)·`-M:conformance`·
> `-M:compiler-smoke`·해당 lane + 코드 grep 으로 **존재가 아니라 정확성**을 실측 검증한다(§17.8 처럼
> 과장/죽은코드/우회 여부 확인). 닫혔다 주장만 믿지 않는다.

---

## ★ codex 재시작 작업지시 — §17 미해결 항목 + 해결방법 (2026-06-29)

**배경**: §17 deep-research(자가 4-에이전트 코드 감사 + 웹 1차소스 교차검증)에서 나온
**확정 버그 B1~B8 + 리터럴 갭 G1 은 해결·검증 완료**(smoke 141/141, conformance 112/112
+ negative 19/19, gate READY ✅, self-host stage1→7 고정점 무회귀). 아래 T1~T7도 2026-06-29
패스에서 코드/fixture/receipt 정리 완료. 상세 근거: §17.5(V1~V3)·§17.3
(H1~H8)·§17.4(D1)·§17.6. 이후 새 작업은 아래 항목을 다시 여는 대신 새 TODO로 분리할 것.
이전 작업지시 원문은 재발 방지를 위해 보존한다.
<!--
아래는 **아직 안 된 것**
— 각 항목 *문제 → 왜 중요 → 해결방법(코드 수준) → 검증*. 상세 근거: §17.5(V1~V3)·§17.3
(H1~H8)·§17.4(D1)·§17.6. **착수 전 해당 fixture부터 추가(diagnose-first)**, gate 무회귀 유지,
슬라이스별 커밋/푸시(메시지 한글, Co-Authored-By).
-->

### T1 [완료 2026-06-29] [P1] V3 — 직접-emit defrecord 의 IKeywordLookup/getLookupThunk + dissoc 강등 + hasheq
- **문제**: `grep IKeywordLookup|getLookupThunk compiler.clj` = **0**. record 는 `ILookup.valAt`
  으로 `(:k rec)` 동작(conformance 통과)하나, (a) keyword 조회 인라인캐시 fast-path 부재 +
  host record 인터페이스 목록에 IKeywordLookup 이 있으면 getLookupThunk 미emit → AbstractMethodError
  위험, (b) **dissoc 로 선언필드 제거 시 plain map 강등**(record? → false) 미검증, (c) hasheq/
  hashCode/equals ≡ equiv 일관성(= 외) 미검증.
- **왜**: 진짜 record 의미 + hash 컬렉션(set/map 키) 정확성(일관성 깨지면 silent lookup 실패).
- **해결방법**:
  1. *진단 먼저*: defrecord 하나 emit 후 `(.getInterfaces cls)` 에 IKeywordLookup 있는지,
     `(.getMethod cls "getLookupThunk" (into-array [Class Object]))` 존재하는지 확인. 인터페이스
     有 + 메서드 無면 즉시 버그(verify/AbstractMethodError).
  2. getLookupThunk(Class gclass, Object k) → ILookupThunk emit: 선언 keyword 필드별로,
     target 의 class==gclass 면 그 필드를 반환, 아니면 thunk 자신(miss sentinel)을 반환하는
     ILookupThunk 를 돌려줌. reify 기계(emit-reify-class) 재사용 또는 per-field 소형 thunk 클래스.
     참조: clojure.lang.Compiler NewInstanceExpr.emitGetLookupThunk, core_deftype emit-defrecord.
  3. dissoc 강등/hasheq 는 host 매크로 메서드(`without`/`hasheq`)를 generic emit 이 내는지 확인
     — 안 나면 보강.
- **검증**(conformance 픽스처 추가): `(record? (dissoc (->R 1 2) :a))`=false; `(:a (->R 1 2))` via
  set 멤버십 `(contains? #{(->R 1 2)} (->R 1 2))`=true; `(= (hash (->R 1 2)) (hash (->R 1 2)))`=true;
  host≡compiler.

### T2 [완료 2026-06-29] [P1] V2 — catch 타입 Throwable-subclass 검증 (CLJ-2345)
- **문제**: `(try 1 (catch Object o 2))` 같은 비-Throwable catch. host 는 parse-time 에 안 걸러
  VerifyError 발생. 우리 emit-try(:1864 예외를 throwable-type checkCast)가 catch_type 에 사용자
  클래스 internal name 을 그대로 쓰면 동일 VerifyError 상속(또는 다른 동작 → host 불일치).
- **왜**: host parity(conformance) + 견고성(컴파일러가 VerifyError 클래스를 내면 안 됨).
- **해결방법**:
  1. *진단*: `(try 1 (catch Object o 2))` 백엔드 컴파일 → 결과(VerifyError? 동작?) host(VerifyError)와 대조.
  2. 권장 = **fix(soundness)**: emit-try 에서 각 catch 등록 전 `(.isAssignableFrom Throwable
     catch-class)` 체크, 아니면 `:unsupported-op` throw(→ host fallback, host 도 에러 → parity).
     대안 = host 그대로 매칭(VerifyError 상속)이나 컴파일러가 잘못된 bytecode 내는 건 피하는 게 정직.
- **검증**: negative conformance `(try 1 (catch Object o 2))` host≡compiler 둘 다 에러.

### T3 [완료 2026-06-29] [P1] V1 — ASM #317786 후방점프 widening VerifyError (진단 우선)
- **문제**: 조건부 *후방* 점프 offset 이 -32768 미만으로 넓어지면 ASM MethodWriter 가
  (역조건+GOTO_W) 재작성하며 fall-through 타깃에 stackmap frame 누락 → verifier 거부. 우리는
  COMPUTE_FRAMES 라 ASM 이 프레임 계산하나, vendored clojure.asm revision 이 이 버그 fork 면
  대형 메서드(긴 loop/case 테이블)에서 재현 가능. [src25] OW2 ASM #317786.
- **왜**: 대형 self-source/사용자 코드가 silent 하게 VerifyError 날 수 있음(드물지만 치명적).
- **해결방법**:
  1. *진단*: vendored `clojure.asm` revision 확인(clojure jar 내 clojure/asm), #317786 fix 포함 여부.
  2. *스트레스 픽스처*: 조건부 후방점프 offset 을 -32768 아래로 밀어내는 대형 메서드(거대 body
     loop, 또는 수백 분기 case) 생성 → 백엔드 emit → bytecode_verifier full verify. VerifyError 면 재현.
  3. 재현 시: (a) vendored asm 패치/업그레이드, (b) 메서드 분할로 offset 회피, (c) 삽입 fall-through
     에 frame 보장. → bytecode-verifier/bytecode-witness lane 에 회귀로 고정.
- **검증**: 스트레스 픽스처가 bytecode-verifier 에서 PASS.

### T4 [완료 2026-06-29] [P1/P2] H4 — locals-clearing (head-holding/누수 divergence)
- **문제**: `grep locals?.?clear compiler.clj` = **0**. emit-let(:1377±)/emit-loop 이 슬롯을 last-use
  후 null 처리 안 함 → lazy-seq head 가 메서드 프레임 내내 reachable(host 는 clear). 의미 divergence
  + 공간누수.
- **해결방법**:
  1. analyzer 의 clearing 정보(`:to-clear`/clear-locals 패스 결과)가 노드에 있으면 그 지점에서
     `ACONST_NULL; storeLocal`. 없으면 보수적으로 let/loop body 의 *Object* 슬롯 last-read 후 null
     (primitive/this/arg 제외, 분기 뒤 재read 있는 슬롯 제외 = verifier 무파손).
  2. `*disable-locals-clearing*` 플래그(기본 clear ON = host 의미)로 게이트.
- **검증**: 큰 lazy-seq head 를 local 바인딩 후 소비 → heap 안정(정성) + bytecode-verifier PASS 유지.

### T5 [완료 2026-06-29] [P2] M22 / D1 — raw-long prover 적대적 테스트 (soundness 신뢰경계)
- raw LADD/LMUL/LSUB 승격(checked-long-static-no-overflow? :662-687 + AI 엔진)이 unsound 일 수
  있는 입력을 적대적으로 생성 → soundness sentinel(:3088±)이 실제로 잡는지/raw 가 안전한지 패스.
  §16.1/16.2 translation-validation 신뢰경계 강화. (별도 검증 lane.)

### T6 [완료 2026-06-29] [P1-docs] H1~H8 — 정직성 어휘 한정구 (코드 아닌 문서)
- gate/receipt *요약 문자열*에 한정구 명시(본문 §7/§13 엔 이미 있음): "stage1→7 = backend
  self-emit **결정성+자기일관성** 고정점(정확성 아님), **frames=ASM(COMPUTE_FRAMES)**, frontend
  (analyzer/reader)+runtime(clojure.core)=**신뢰 host base**, cross-host DDC=emit 결정성
  (Trusting-Trust 아님)". gate.clj 요약 + 관련 receipt 문구 수정. "byte-identical/meta-circular/
  직접 emit" 단독 표현이 과대독해되지 않게.

### T7 [완료 2026-06-29] [research] (3)(4)(5) — 웹이 못 덮은 영역 별도 targeted deep-research
- (3) meta-circular 진짜성: stage1→7 고정점이 증명하는 것(자기일관성 vs 정확성), host clojure.core
  위임이 주장과 충돌하는 지점 → selfhost.clj/stagen.clj/full_source_stage1.clj 대상.
- (4) DDC: cross-host bit-identical 한계 + full Wheeler DDC(독립 2nd 컴파일러) 요건 → cross_host_ddc/
  diverse_double_compile 대상.
- (5) range-proof/AI soundness: interval/octagon/geometric/conserved/unrolling unsound 케이스 +
  validator 신뢰경계 → abstract_interval/octagon/translation_validation 대상. [src29] Alive2.

**우선순위 요약**: T1(V3 record)·T2(catch)·T3(ASM widening 진단)·T6(정직성 docs) → T4(locals-
clearing)·T5(prover) → T7(research). 각 T 는 §17.5/17.6 의 해당 항목과 1:1.

### T1~T7 적용 결과 (2026-06-29)
- T1(V3 defrecord): 진단 결과 실제 `IKeywordLookup.getLookupThunk(Keyword)` 는 host defrecord
  매크로가 만든 메서드를 generic method emit 이 이미 내고 있었음. TODO의 `(Class,Object)`
  시그니처는 현 런타임과 달랐다. `dissoc` 선언필드 제거 plain-map 강등, keyword thunk 조회,
  set membership, hash/hasheq 일관성을 compiler smoke + host≡compiler conformance 로 고정.
- T2(V2 catch): `emit-try` 가 catch table 등록 전 `Throwable` 하위 타입을 검증한다. 비-Throwable
  catch 는 `:unsupported-op` 으로 host fallback 하며, negative conformance 에서 host/compiler 둘 다
  VerifyError 경로로 실패함을 고정.
- T3(V1 ASM #317786): `bytecode-verifier` 에 vendored `clojure.asm` 조건부 후방점프 widening
  스트레스 클래스(40k NOP 뒤 `IFGT` loop) 추가. ClassReader/fresh loader/CheckClassAdapter/invoke PASS.
- T4(H4 locals-clearing): tools.analyzer.jvm AST 에 clear metadata 가 없음을 확인했고, `let`/`loop`
  scope exit 에서 body 결과를 보존한 뒤 Object local 슬롯만 `ACONST_NULL; ASTORE` 로 clear. primitive
  local/arg/this 는 건드리지 않는다. bytecode witness 에 null-store fixture 추가.
- T5(M22/D1 raw-long prover): translation-validation 에 boundary accepted + overflow subtract/multiply/
  wide-interval rejected adversarial VC 추가. compiler smoke/negative conformance 에 subtract/multiply
  overflow sentinel 추가.
- T6(H1~H8 wording): gate receipt/CLI 에 `:proof-claim` 추가. stage1→7 은 backend 결정성+자기일관성
  고정점이지 언어 정확성 증명이 아니며, frames=ASM COMPUTE_FRAMES, analyzer/reader/runtime=trusted
  host base, cross-host DDC=emit 결정성 evidence(Trusting-Trust 방어 아님)로 명시.
- T7(research): 별도 구현 버그가 아니라 claim-boundary 연구 항목으로 처리. full Wheeler 독립
  compiler-binary DDC, 독립 reader/runtime, 완전한 언어 정확성은 gate `:proof-claim :not-claimed` 로
  명시하고, 현재 게이트가 과대 주장하지 않도록 닫음.

---

## 0. 정직한 현황 (Honest Status) — 가장 먼저 읽을 것

**`stage7-gate.sh` 재현빌드 레인은 meta-circular compiler/interpreter가 아니다.**
별도 meta 레인(`compiler.clj` + `selfhost.clj`)은 Clojure로 쓴 analyzer/ASM
bytecode compiler와 self-host 고정점 게이트까지 올라왔지만, 아직 전체
`compiler.clj`가 자기 자신 전체를 컴파일하는 단계는 아니다.

지금까지 만들어진 `stage7-gate.sh` + `jarproof.clj`가 한 일은 상용 업스트림
`clojure-clojure-1.12.5`를 Maven으로 7번 재빌드해 jar 고정점을 확인한 것이다.
이것은 **"stock Clojure의 재현빌드(reproducible-build) 게이트"**이며 빌드
결정성은 증명됐지만, **"Clojure 언어로 직접 쓴 컴파일러" 검증은 아니다.** 그래서
이 레인은 meta-circular가 아니다(검증된 것은 "Java로 된 host Compiler가 결정적").

### 확정된 진짜 경로 (사용자 결정, 2026-06-28)

> **"Clojure로 쓴 컴파일러(매크로확장 → 분석 → JVM bytecode emit)"를 만들어,
> 그 컴파일러가 자기 자신을 컴파일하는 self-host 체인(stage1→7)에 태운다.
> 실행은 host JVM이 하므로 성능·기능 손실이 없다. 고정점은 생성된 bytecode.**

핵심 통찰: meta-circular의 본질은 "인터프리터"가 아니다. hy-meta의 `kernel.hy`도
Hy를 인터프리트하지 않고 **Python AST로 컴파일**하며 실행은 CPython(host)이 한다.
즉 "언어로 쓴 컴파일러 + host 실행"이 정통 형태다. 트리워킹 인터프리터는 느려서
성능을 잃지만, 컴파일러는 진짜 bytecode를 내므로 **host급 성능 + Clojure 전체
기능**을 유지한다.

부품(바닥부터 안 만들어도 됨 — 이미 *Clojure로 작성된* 검증된 부품):
- `org.clojure/tools.analyzer.jvm` — Clojure로 쓴 Clojure-on-JVM 분석기.
- `org.clojure/tools.emitter.jvm` — 분석 결과를 JVM bytecode로 emit(host
  `clojure.lang.Compiler` 대체).
- 선례: ClojureScript 컴파일러 자체가 Clojure로 작성됨.

### 두 emit 백엔드 (단계적: B로 시작 → A로 강화)
- **(A) 순수 최대**: `tools.emitter.jvm`로 bytecode 직접 방출. host `Compiler`
  0회. 진짜 독립 컴파일러. 단점: emitter 성숙도 실험적, 성능은 host에 *근접*.
- **(B) 성능 최대**: 매크로확장+분석은 Clojure로 쓴 코드가 소유, **최종 emit만
  host `Compiler` 위임**. 성능 100% 보장, 작업량 적음. 단점: 백엔드는 host.
- 둘은 **같은 프론트엔드**를 공유한다. self-host의 핵심(언어 처리를 Clojure
  코드가 소유)은 A/B 공통이다.

### 재현빌드 게이트 검증 결과 (디스크 증거, 2026-06-28)
`clj-meta/proof/stage*-jars.sha256` stable digest:
```text
main    stage1 = stage2 = stage3 = stage4 = stage5 = stage6 = stage7  82fef134…
slim    stage1 = stage2 = stage3 = stage4 = stage5 = stage6 = stage7  7a269959…
sources stage1 = stage2 = stage3 = stage4 = stage5 = stage6 = stage7  3489bc73…
```
판정: 재현빌드 레인으로서는 일관 완료. stage snapshot 에 locals-clearing 비활성화와
closed-over local 안정 정렬 패치를 적용해 JVM 프로세스별 class bytecode 흔들림을 제거했다.
단 stock Clojure 재빌드라 meta-circular 아님. 이 인프라(jarproof/stage7-gate)는
**진짜 컴파일러 self-host의 bytecode 고정점 비교에 재활용**한다.

---

## 1. Resume Here (재개 지점)
```sh
cd ~/pnix-clj
git status -sb

# 재현빌드 레인(stock Clojure)
clj-meta/stage7-gate.sh status

# 인터프리터 커널(의미론 명세/거울) smoke — 동작 확인됨(24/24)
clojure -M:kernel-smoke

# Clojure로 쓴 컴파일러 / self-host / mirror
clojure -M:compiler-smoke
clojure -M:conformance
clojure -M:selfhost-check
clojure -M:mirror-smoke
clojure -M:audit-self-source
clojure -M:gate
```
진행 cadence: fast loop → 이 todo.md 갱신 → 커밋 → 필요 시 백그라운드 게이트.

---

## 2. Directory Roles (디렉토리 역할)
```text
clojure-clojure-1.12.5/        업스트림 Clojure 1.12.5 소스 (입력, 불변)
clj-meta/
  src/pnix/clj_meta/
    compiler.clj    [축 B-2 · 주 경로] Clojure로 쓴 컴파일러                  ← 핵심/신규
                    (매크로확장→분석→JVM bytecode; 백엔드 A/B)
    selfhost.clj    [축 A · 주 경로]  컴파일러가 자기 자신을 컴파일하는 체인   ← 핵심/신규
                    (stage1→7, 고정점 = 생성 bytecode, jarproof 재활용)
    kernel.clj      [축 B-1 · 보조]   메타순환 인터프리터 = 의미론 명세/거울    ← 동작중(1a/1b 일부)
                    (성능 무관; 투명성·교차검증·mirror 용도)
    conformance.clj                  host ≡ compiler ≡ kernel 등가 검증        ← 신규
    mirror.clj                       analyzer AST → canonical mirror IR        ← 신규
    selfaudit.clj                    compiler.clj 자기소스 op census/ledger   ← 신규
    jarproof.clj                     bytecode/jar 고정점 비교 (기존, 재활용)
    core.clj                         receipt 리포트 (기존)
    stm.clj                          ref 데모 (example 강등 예정)
  stage7-gate.sh                     [재현빌드 레인] stock Clojure 재빌드 (기존)
  examples/*.clj                     컴파일러/커널이 돌릴 예제 (신규)
  work/ proof/ logs/                 생성물 (git ignore)
```

용어:
- **재현빌드 레인** = stock Clojure 재빌드. meta-circular 아님(유지: 빌드 결정성).
- **meta 레인** = `compiler.clj`(주) + `selfhost.clj`(주) + `kernel.clj`(보조).
  진짜 meta-circular. 이 todo의 목표.

---

## 3. 설계 개요 (Design Overview)

### 축 B-2 (주 경로): Clojure로 쓴 컴파일러 `compiler.clj`
폼 → **매크로확장 → 분석(analyze) → JVM bytecode emit** → host JVM 실행.
host `clojure.lang.Compiler`(Java)를 대체/우회하되 실행은 JVM이 하므로
**성능 host급 + Clojure 전체 기능**.
- 프론트엔드(매크로확장+분석): `tools.analyzer.jvm` 활용 또는 자체.
- 백엔드 emit: (A) `tools.emitter.jvm` 직접 / (B) host `Compiler` 위임.
- 산출물: JVM `.class` / bytecode. 이것이 self-host 고정점 비교 대상.

### 축 A (주 경로): self-host 체인 `selfhost.clj`
```text
stage1 = host Clojure 가 compiler.clj(우리 컴파일러) 로드
stage2 = 우리 컴파일러로 compiler.clj 소스를 컴파일 → 2세대 컴파일러(진짜 bytecode)
stage3..7 = 직전 세대 컴파일러로 같은 소스 반복 컴파일
고정점 = 각 stage 가 생성한 bytecode/.class 가 stage 간 byte-동일 (jarproof 재활용)
```
이것이 "Clojure가 Clojure를 컴파일하는" 진짜 meta-circular tower. 현재 재현빌드
인프라를 그대로 격상시켜 쓴다.

### 축 B-1 (보조): 메타순환 인터프리터 `kernel.clj`
성능 경로가 아니다. 가치는 **투명성·검사가능성·변형가능성**: 평가 과정이 데이터로
노출돼 자기 분석/변형(mirror, 한글-codec)이 가능. 용도:
- 컴파일러가 구현할 **언어 의미론의 실행 가능한 명세**(reference semantics).
- conformance에서 **host ≡ compiler ≡ kernel** 3-way 교차검증의 한 축.
- mirror IR의 거울. (속도와 무관하므로 "성능 손실"과 별개)

---

## 4. Active Goals — 단계별 로드맵

### Phase 0 — 정직화 / 레인 분리 (P0)
- [x] `deps.edn` 추가(`{:paths ["src"]}` + alias).
- [x] README.md/todo.md에서 stock 재빌드를 meta-circular로 오인시키는 표현 제거.
- [x] `stage7-gate.sh` receipt를 `:kind "reproducible-build"`로 정직 라벨.

### Phase 1 — 인터프리터 커널(보조: 의미론 명세/거울) (P1)
- [x] **1a. 최소 평가기 골격** — smoke 7/7(`clojure -M:kernel-smoke`): let/fn/
      if/def/재귀 factorial(120), 커널fn⨉host map 혼용. 재귀 본문이 host Compiler
      없이 k-eval 로 해석됨 확인.
- [x] 1b. `loop`/`recur`(비스택 증가), 다중 arity, **named fn self-ref** 지원 완료.
- [x] 1c. 핵심 core 매크로(when/cond/and/or/->/->>) + try/catch/throw +
      기본 host interop(ctor/static/instance call, field read), `set!`, `binding`,
      `locking`, `var`, `case`, `letfn`, custom `defmacro` 완료. kernel smoke **24/24**.
- [x] 1d. 커널을 conformance의 reference semantics 축으로 고정(성능 목표 아님).
      현재 kernel 3-way 참고 **100/100** 일치, unsupported 0.
      ※ 컴파일러 주 경로가 앞서면 커널은 컴파일러 의미론에 맞춰 따라간다.

### Phase 2 — Clojure로 쓴 컴파일러 `compiler.clj` (주 경로) (P0)
- [x] **2a. 부품 실측(PoC)** — 2026-06-28:
  - 프론트엔드 `tools.analyzer.jvm 1.2.3`: Clojure 1.12에서 **살아있음**.
    `(fn [n] (* n n))` → `:fn` AST(:methods 보유), `(when true (inc 41))` →
    매크로확장 `(if true (do (inc 41)))`. 매크로확장+분석을 *Clojure로 쓴
    부품*이 정상 수행 → 프론트엔드 self-own 가능.
  - 백엔드 `tools.emitter.jvm 0.1.0-beta5`: **방치 부품, 부분 동작**.
    `(e/eval '(+ 1 2))` → 3 (host Compiler 없이 bytecode 성공) 이지만
    `(fn …)` 에서 NPE(closed_overs null). Clojure 1.12 fn 클로저 미지원.
  - **확정 아키텍처**: 프론트엔드 = analyzer 1.2.3 / 백엔드 = 우리가 채움.
- [x] **2b. 백엔드 결정 = 자작 ASM (A 확정)** — 2026-06-28:
  - `clojure.asm`/`clojure.asm.commons.GeneratorAdapter`/`DynamicClassLoader`/
    `AFn` 모두 번들로 가용(외부 ASM 의존성 0).
  - `compiler.clj` 첫 조각 동작 — **smoke 5/5** (`clojure -M:compiler-smoke`):
    상수 thunk, 항등(파라미터 통과), 상수, 2-arity 첫/둘째 인자.
    파이프라인: form → analyzer AST → **우리 clojure.asm emit** → AFn 서브클래스
    → DynamicClassLoader → host JVM 실행. **host Compiler 0회로 target fn bytecode 생성.**
  - emit 가능 op: :fn(단일 arity) / :fn-method / :do / :const(int) / :local.
  - (emitter beta5 수선은 폐기 — 구버전 analyzer 0.5.2 에 묶여 신버전 1.2.3 과
    AST 스키마 불일치. 자작이 정도.)
- [x] **2c-1. invoke / var / static-call** (smoke 9/9): `(* n n)`→81,
      `(+ a b)`→42, 중첩 `(* (+ n 1) 2)`→42. `*`/`+` 의 inline → `:static-call`
      (clojure.lang.Numbers) → 정적 arg 타입 기반 overload 를 리플렉션으로 잡아
      invokeStatic. **host 급 최적화 경로 + host Compiler 0회.**
- [x] **2c-2. :if / :let / :const + 재귀** (smoke 14/14): :if(booleanCast 분기),
      :let(newLocal 슬롯, 순차 바인딩), :const(nil/bool/string), fn self-ref(this).
      **재귀 factorial `(fn fact [n] (if (< n 2) 1 (* n (fact (- n 1)))))` → 120 을
      우리 bytecode 로 컴파일.** (keyword/symbol/컬렉션 const 는 미구현)
- [~] **2c-3. self-host 부분집합 확장(op 묶음)** — analyzer 가 매크로를 primitive 로
      확장해주므로(defn/when/cond/->/doseq… → if/let/fn/do/loop) **primitive op 만
      늘리면** 그 위 매크로가 자동 컴파일된다. 직접 emit subset 은 stage15/N gate 에
      들어갔고, `:reify` 중 simple Object/interface/capture method 는 직접 emit 으로 승격했다.
      남은 `:deftype`/general `:reify`/namespace side-effect 는 held boundary 로 추적.
      묶음별:
  - [x] (i) **컬렉션/상수**: `:vector` `:map` `:set` + const(keyword/symbol/컬렉션)
        + `:quote`. RT.vector/map/set(Object[]) + Keyword/Symbol.intern + 재귀 const.
        compiler smoke 21/21, conformance 20/20(kernel 3-way 19/19).
  - [~] (ii) **제어흐름**: `:loop`/`:recur`(JVM goto, 비스택) ✓, `:throw`(athrow) ✓,
        `:try`/`:catch`/`:finally`(정상/handled/uncaught 경로) ✓,
        `:case` direct emit(compact hash `tableswitch`/`lookupswitch` + `Util.equiv`
        guard) ✓, `:letfn` mutual recursion direct emit ✓.
  - [x] (iii) **interop**: `:new`/`:instance-call`/`:static-call`(일반화)/
        `:static-field`/`:instance-field`/`:host-interop`/`:the-var` + 인자 타입 coercion
        (unbox/checkCast) + analyzer 정적 타입 기반 overload 점수화 + double const.
        `:protocol-invoke`/`:instance?` 직접 emit. 현재 전체 gate 는 compiler 106/106,
        conformance 100/100.
  - [~] (iv) **fn 고급**: 클로저 캡처 ✓ + 중첩 fn ✓ + 다중 arity ✓ +
        **variadic(& rest → RestFn: getRequiredArity + doInvoke)** ✓ +
        **fixed+variadic 혼합** ✓, `:letfn` mutual recursion ✓.
  - [~] (v) **top-level**: `:def`(Var bindRoot) ✓ + `eval-form`(임의 폼을 0-arity
        fn 으로 감싸 실행, `(do …)` 로 여러 폼) ✓ + var deref 정확화 ✓.
        compiler 40/40, conf 37/37, kernel 3-way 25/25. 남음: `:ns`/`:import` 부수효과.
  - [~] (vi) **기타**: `:set!`(Var thread binding + public field) ✓,
        `:with-meta` ✓, dynamic var binding(`binding` macro expansion + try/finally) ✓,
        `:monitor-enter/exit` ✓, `:keyword-invoke` ✓.
        남음: primitive 타입 힌트/coercion(성능).
- [x] **2c-conf. conformance 게이트** (`clojure -M:conformance`): host≡compiler
      **100/100** 자동 등가 + negative **15/15**. kernel 3-way 참고 **100/100** 일치,
      unsupported 0. op 추가마다 회귀 자동 감지 안전망 확보.
- [x] 2d. 컴파일 파이프라인 API: `(compile-form form)→IFn`, `(compile-ns src)→
      in-memory loadable artifact`, `load-compiled-ns` 로 순차 로드. smoke 포함.
- [x] 2e. 벤치: 우리 컴파일러 산출물 vs host 직접 컴파일 성능 측정.
      `clojure -M:bench` 기준 loop 0.81, 다중arity 0.83, mixed variadic 0.98,
      factorial 1.03, map+캡처 1.04, `*` 1.11 등 host급 범위 확인.

### Phase 3 — self-host 체인 `selfhost.clj` (주 경로) (P0)
- [x] **3a (M5a). deterministic self-compile 고정점** (`clojure -M:selfhost-check`):
      작은 self-host 타겟(분기/재귀/loop/클로저/컬렉션 산술)을 우리 컴파일러로
      stage1..7 반복 컴파일 → 7 classes bytecode **byte-동일**. program 실행
      `[42 81 55 15 (11 12 13)]` 정확. 결정적 클래스명(*gen-counter* 단위 리셋) +
      gensym 없는 emit 으로 결정성 확보.
      ※ 정직: 아직 '컴파일러 결정성 고정점'이지 '컴파일러가 자기를 컴파일'은 아님.
- [x] **3b (M5b). 진짜 self-host — 미니 메타순환 평가기** (`clojure -M:selfhost-check`):
  - [x] 3b-1. `mini-eval`: 우리 부분집합으로 쓴 트리워킹 평가기
        (quote/if/do/let/벡터/함수적용 + env 맵, cond→if, 재귀 me, 클로저).
  - [x] 3b-2. 우리 컴파일러로 mini-eval 소스 컴파일 → `C(mini-eval)` (우리 bytecode).
  - [x] 3b-3. conformance: `C(mini-eval) ≡ host mini-eval` — **8/8 프로그램 동일**.
  - [x] 3b-4. bytecode 고정점: mini-eval 소스 stage1..7 반복 컴파일 → byte-동일(4 classes).
  - [x] 3b-5. **완전 메타순환 tower** (`full-eval`, named fn+variadic+fn 해석):
        - if/fn/named-fn/let self-interp 통과 ✅ (host≡compiled≡tower).
        - **부산물: 진짜 컴파일러 버그 발견·수정** — COMPUTE_FRAMES 의
          getCommonSuperClass 가 gen 클래스를 Class.forName 으로 못 찾아 깊은 중첩에서
          ClassNotFound → gen 클래스 쌍은 Object 로 override. + `:prim-invoke` 지원.
        - full-eval 의 다중-binding `let` self-interp hygiene 를 `bindl` 로 수정.
- [x] 3c. receipt: `clj-meta/proof/selfhost-chain.receipt.edn`.
- [~] 3d. (격상) 재현빌드 레인 stage 정의를 self-host 로 점진 대체.
      `full_source_stage1.clj` receipt 로 compiler.clj self-source 의 direct subset,
      host side-effect boundary, fallback-free genuine stage1 held 상태를 gate 에 고정.
      남음: `:import`/namespace side-effect wrapper 제거 또는 직접 emit/admission.

### Phase 4 — conformance / 등가성 (P0)
- [x] 4a. 코퍼스: `examples/*.clj` + 인라인. factorial/클로저/재귀/자료구조/try/set.
- [x] 4b. 3-way 등가(conformance.clj): host≡compiler **100/100 동작**. kernel 은
      현재 conformance corpus 전체에서 참고 비교(**100/100**, unsupported 0).
- [x] 4c. 음성 테스트 + 결정성. negative **15/15**, selfhost bytecode 고정점 통과.
      `determinism_policy.clj` receipt 로 `*gen-counter*` per-compilation reset(-1),
      `pnix.clj_meta.gen.Fn__N` contiguous naming, 반복 compile class-name/class-digest
      안정성, `case` 직접 emit 및 `letfn` mutual-recursion cyclic capture class 의
      반복 digest 안정성, gensym/random/time 금지 정책을 gate 에 고정.
      determinism policy digest:
      `e3819be2667a21b96c22e97c79494a67eaa78ef837ab4a2edabc24127588b58b`.

### Phase 5 — receipt / 게이트 / mirror (P1/P2)
- [x] 5a. 통합 게이트(`gate.clj`, `clojure -M:gate`): **meta 레인**(compiler 106/106 +
      conformance 100/100 + negative 15/15 + mirror 2/2 + self-source audit +
      full-source accounting + full-source stage1 boundary + kernel smoke 24/24 +
      generated-name determinism + M9/M10/M11/M12/M13/stageN receipts +
      generated compiler stage-driver chain + full namespace artifact/load smoke +
      M5a/M5b 고정점 + full-eval tower OK)을 단일 receipt 로 = **READY ✅**.
      재현빌드 레인(stage7-gate.sh)은 DDC evidence-only 로 통합했고, pnix-clj 런처 소비는
      완전한 meta-circular stage15/N Clojure compiler 전까지 계속 금지.
- [x] 5b. 컴팩트 receipt 생성: `clj-meta/proof/metacircular.receipt.edn`
      (host Compiler 0회 경계 명시. 응용/런처 연결용 아님).
- [x] 5c. mirror IR: analyzer AST 를 canonical mirror form으로 노출.
      smoke **2/2**(`fn`, macroexpanded `when`). northstar 확장 여지는 남음.

---

## 5. What Is Done (완료)
- [x] 재현빌드 레인 stage1~7 (stock Clojure 빌드 결정성).
- [x] jar 고정점 비교를 Clojure(`jarproof.clj`)로 수행.
- [x] hy-meta 두 축 정밀 분석 + Clojure 포팅 청사진.
- [x] 인터프리터 커널 1a + 1b + 1c + 1d 동작(smoke 24/24) — loop/recur, 다중 arity,
      named fn self-ref, core macro(when/cond/and/or/->/->>), try/catch/throw,
      기본 host interop, set!/binding/locking/var, case/letfn reference 포함.
- [x] **진짜 경로 확정**: 트리워킹 인터프리터가 아니라 "Clojure로 쓴 컴파일러
      self-host"로 성능·기능 손실 없이 stage7.
- [x] 부품 실측: 프론트엔드 analyzer 1.2.3 살아있음 / 기성 emitter 죽음 →
      백엔드 자작 ASM 확정.
- [x] `compiler.clj` 동작(smoke 14/14): analyzer AST → 자작 clojure.asm emit →
      AFn 서브클래스 → host JVM 실행. fn/const/local/invoke/var/static-call/
      if/let + fn self-ref. **재귀 factorial → 120 을 우리 bytecode 로 컴파일,
      host Compiler 0회.**
- [x] conformance 게이트(conformance.clj): host≡compiler **100/100** 자동 등가 +
      negative **15/15**. 앞으로 op 확장의 회귀 안전망.
- [x] (i) 컬렉션/상수 emit: `:vector`/`:map`/`:set` + keyword/symbol/컬렉션 const
      + `:quote`. compiler 21/21, conformance 20/20(kernel 3-way 19/19).
- [x] (ii) 제어흐름 일부: `:loop`/`:recur`(JVM goto)/`:throw`/`:try`-`:catch`.
      compiler 27/27, conformance 25/25.
- [x] (iii) interop: `:new`/`:instance-call`/`:static-field`/`:instance-field`/
      `:the-var` + 인자 타입 coercion + analyzer 정적 타입 기반 overload 점수화.
      `:host-interop`(동적 field/0-arg member) + `:protocol-invoke` + `:instance?` 직접 emit.
      현재 전체 gate 는 compiler 106/106, conformance 100/100.
- [x] (iv) 클로저 캡처 + 중첩 fn: 자유변수 분석 → 필드+생성자 주입, 공유 *dcl*.
      compiler 38/38, conformance 35/35, kernel 3-way 23/23.
- [x] (v) top-level `:def` + `eval-form` + var deref 정확화.
      compiler 40/40, conformance 37/37, kernel 3-way 25/25.
- [x] **M5a self-host 고정점**(selfhost.clj): 작은 프로그램을 stage1..7 반복 컴파일 →
      7 classes bytecode **byte-동일**. 결정적 클래스명 + gensym-free emit.
- [x] **M5b compiler-compiles-evaluator**: 우리 컴파일러가 미니 메타순환 평가기를
      컴파일 → host≡compiled 8/8, 소스 bytecode 고정점. 메타순환 타워의 실제 한 층.
- [x] variadic(& rest) + fixed/variadic 혼합 — RestFn 서브클래스 + fixed invoke override.
      compiler **80/80**, conformance **80/80**. 일반 fn/apply + 완전 tower(3b-5)
      선행조건 충족.
- [x] **M6 통합 게이트**(gate.clj, `clojure -M:gate`): compiler 106/106 + conformance
      100/100 + negative 15/15 + mirror 2/2 + self-source audit + full-source accounting +
      compiled proxy-super smoke + compiled compiler API artifact smoke +
      compiled self-source compiler impl var smoke + generated compiler stage-driver chain +
      full namespace artifact/load smoke + M5a/M5b 고정점 + full-eval tower OK →
      단일 receipt **READY ✅**. host Compiler 0회 경계 명시.
- [x] **compiler.clj self-source audit 1차** — `clojure -M:audit-self-source`:
      `compiler.clj` 192개 top-level form 분석, analyzer `:op` 32종 census,
      unknown op 0. host-maintained present 는 `:import`만 ledger 에 고정.
      direct-subset 후보 top-level **191/192**.
      receipt: `clj-meta/proof/self-source-audit.receipt.edn`.
- [x] **compiler.clj 자기소스 host-maintained `case` 제거 + target `:case` 직접 emit** —
      emit-local/recur/set!/emit-node/free-locals* 내부 dispatch 는 direct-subset 형태로 유지하고,
      target 프로그램의 `:case` 는 analyzer `case*` compact key 를 따라
      `tableswitch`/`lookupswitch` 로 dispatch 하고 `Util/equiv` guard 로 확정한다.
      `:case`/`:case-test`/`:case-then` 은 selfaudit direct/direct-helper ledger 로 이동.
- [x] **compiler.clj bytecode-safe stage target 1차** — self-source audit 가
      host-maintained `ns`/`:import` 를 제외한 direct-subset **191/191** top-level form 을
      `(fn [] (do ...))` target 으로 묶고, stage1→7 반복 컴파일 결과 **199 classes
      bytecode fixed-point OK**. receipt:
      `clj-meta/proof/self-source-stage-target.receipt.edn`.
- [x] **상수/interop emit 확대** — `Class` 리터럴, `char`, quoted list 상수,
      analyzer `:tag` 우선 타입 추론, 비모호 reference overload, 동적 instance-call
      fallback 을 직접 emit. compiler smoke 76/76 유지, self-source bytecode-safe
      form 76→121 증가.
- [x] **full-source accounting + 실행 smoke** — `compiler.clj` 전체 192개 top-level form 을
      host side-effect `ns`/`:import` 1개 + bytecode stage target 191개로 회계 처리.
      stage1→7 fixed-point 199 classes OK. compiled `proxy-super` dynamic instance-call
      smoke 도 `java/lang/Object` 결과로 통과.
- [x] **compiled compiler API artifact smoke** — `compile-ns` 로 격리 namespace artifact 를
      로드하고 그 안의 compiled wrapper 함수가 `compile-form`/`compile-classes` 를 다시
      호출한다. host-loaded wrapper 와 실행 결과 42 및 생성 class bytecode fingerprint 가
      일치. wrapper/thunk 함수 class 도 `pnix.clj_meta.gen.*` 로 확인.
- [x] **compiled self-source compiler impl var smoke** — 원본 `ns` form 의 이름만 격리
      namespace 로 바꾸고, self-source direct-subset 161개 top-level form 을 compiled
      wrapper 로 실행해 `compile-form`/`compile-classes` var root 자체를 생성한다.
      그 생성된 compiler 구현 var 로 작은 프로그램을 다시 컴파일하고 host-loaded compiler 와
      실행 결과 42 및 class bytecode fingerprint 일치 확인. 이 과정에서 발견된
      self-compiled `resolve-method` 후보 필터 오컴파일을 `loop/recur` 후보 수집 +
      reflection Method local type hint 로 수정.
- [x] **generated compiler stage-driver chain** — 생성된 `compile-classes` var 자체를
      다음 stage driver 로 사용한다. 같은 격리 namespace 안에서 self-source stage target 을
      stage1→7 반복 컴파일→로드하고, 매 stage bytecode fixed-point + host-driven
      stage target receipt bytecode digest 일치를 확인. 이 과정에서 `GeneratorAdapter`
      constructor overload 재분석에 필요한 `Method` local/type hint 를 보강.
- [x] 깊은 중첩 컴파일 견고화: COMPUTE_FRAMES getCommonSuperClass 버그픽스 +
      `:prim-invoke`. 3b-5 tower 시도의 부산물(진짜 컴파일러 버그). 회귀 없음.
- [x] **host fallback**(compile-form, §13): 우리 emit 못 하는 op(ns/
      deftype/general reify/…)는 form 전체를 host eval 위임 → **Clojure 전체 기능 손실 0**.
      목표 stage7(고정점은 stage 수 무관, 결정적).
- [x] **다중arity 직접 emit + `:case` 직접 emit 1차** + 성능 벤치 인프라(benchmark.clj).
      다중arity 0.69(우리>host) → 직접 채택, `:case` 는 compact hash
      `tableswitch`/`lookupswitch` 직접 emit 으로 승격. 사용자 성능 정책(측정→빠른 것) 적용.
- [x] **try/finally + set! + binding + monitor 직접 emit** — 2026-06-28:
      `:try` finally 는 정상/handled/uncaught 모든 경로에서 실행. 중첩 try handler
      순서(JVM exception table order)와 컬렉션 원소 stackmap 문제 수정. `:set!` 은
      Var thread binding 및 public instance/static field 지원, `locking` 의
      `:monitor-enter/exit`, `:letfn` mutual recursion, simple `reify Object` method 지원.
      현재 전체 gate 는 compiler **106/106**,
      conformance **100/100**.
- [x] **examples corpus + negative conformance** — `examples/conformance_cases.clj` 로
      외부 코퍼스 로드, 실패 동작도 host≡compiler 비교(throw/overflow/type error/
      wrong arity 4/4).
- [x] **mirror IR 1차** — analyzer AST 를 stable map/vector 데이터로 낮춤.
      `clojure -M:mirror-smoke` **2/2**.
- [x] **재현빌드 안정화** — stage snapshot 에 locals-clearing 비활성화와
      closed-over local 안정 정렬 패치를 적용해 stage1..7 main/slim/sources exact.
- [x] **Numbers 정적 primitive overload 정밀화** — `add`/`multiply`/`minus`/`lt`
      에서 analyzer 가 long/double 로 아는 인자를 `(Object,long)`/`(long,long)` 등
      checked primitive overload 로 선택. conformance 80/80, negative 4/4.
- [x] **일반 host interop overload 정밀화** — static/instance/constructor 후보를
      analyzer 정적 타입으로 점수화. `Object` 우선 오선택으로 char-array overload 가
      깨지던 `String/valueOf` / `StringBuilder.append` 케이스 수정.
      compiler/conformance 80/80.
- [x] **`:keyword-invoke` / `:instance?` 직접 emit** — `(:k m)`은 `Keyword`의
      `IFn.invoke(Object)` 경로로, `(instance? C x)`는 JVM `INSTANCEOF`+Boolean box 로 emit.
- [x] **`:host-interop` 직접 emit** — 정적 타입 미해소 `(.m x)`/`(.-f x)`를
      `Reflector.invokeNoArgInstanceMember`로, 동적 field `set!`을
      `Reflector.setInstanceField`로 emit. field `set!` 평가 순서도 host와 일치.
- [x] **`:protocol-invoke` 직접 emit** — analyzer 가 분리한 protocol fn Var와
      target/args 를 `IFn.invoke` 호출로 직접 emit. `coll-reduce` 2/3 arity로 회귀 고정.
- [x] **source 위치/line 메타 보존** — AST `:env/:line` 을 JVM LineNumberTable 로
      기록하고 SourceFile attribute 설정. stacktrace smoke `line_smoke.clj:3`.

## 6. What Is Not Done (미완료, 우선순위)
- **meta-circular stage15~N 핵심 = 종합 완성(2026-06-29)**: gate READY + stage8~15/N closure +
  self-host fallback-free(M12, host clojure.lang.Compiler 0회) + M6aj 흡수(M9b 4케이스) +
  **전체 언어 표면 직접 emit**(reify/deftype/defrecord/defmulti/defprotocol/protocol-invoke/ns,
  `language_surface` held 0) + self-host stage1→7 fixed-point(257 classes) + full-eval tower
  (host≡compiled≡tower). smoke 127/127, conformance 100/100.
- pnix-clj 런처 연동(M7): 여전히 PARKED(제품 연결은 별도 결정).
- 남은 frontier(연구/독립-toolchain/형식증명 경계 — backend 로 더 닫을 것 아님):
      (1) statically-unknown iteration count nonlinear recurrence(런타임 param 의존, held 정직),
      (2) full Wheeler compiler-binary DDC(독립 Clojure 컴파일러 필요; cross-host 2-버전
      bit-identical partial 증거 완료), (3) CompCert-급 validator 기계검증 soundness.
- P1: `compiler.clj` 전체 self-source stage7/N: full-source accounting 192/192,
      direct-subset 191/191 bytecode stage target, compiled API artifact smoke,
      compiled self-source compiler impl var smoke, generated compiler stage-driver
      chain, generated stage-chain 별도 receipt, full namespace artifact/load smoke,
      generated stage-chain on-disk jar bundle + jarproof-compatible digest,
      disk-reloaded generated compiler entry payload/self-source compile smoke,
      fresh disk stage8 exact digest 고정점, 별도 compare/receipt,
      stage9 clean process compiler runtime replay, stage10 isolated matrix 는 완료.
      stage11 multi-surface compiler adapter closure 는 gate receipt 로 완료.
      stage12 self-improvement quarantine closure 는 gate receipt 로 완료.
      stage13 long-horizon compiler organism closure 는 gate receipt 로 완료.
      stage14 cross-host/cross-implementation law closure 는 gate receipt 로 완료.
      stage15 open-world evidence federation closure 는 gate receipt 로 완료.
      stageN recursive closure ladder 는 gate receipt 1차 완료.
      stage10 cwd hardening 은 sandbox cwd variant receipt 로 1차 완료.
      typed locals primitive 와 typed loop/recur primitive 는 artifact 고정점까지 완료.
      primitive bytecode witness/disasm receipt, safe double raw arithmetic opcode lowering,
      explicit unchecked long raw arithmetic opcode lowering, literal checked-long static
      no-overflow proof, let-local checked-long range proof, invariant-loop checked-long
      range proof, bounded-step changing-loop range proof, positive non-unit loop stride
      range proof, decreasing loop stride range proof, constant accumulator recurrence
      range proof, branch-local fn argument guard range proof, bounded index-accumulator
      sum range proof, multiplicative accumulator range proof 는 완료.
      **한 패턴씩(one-pattern-at-a-time) 라인은 M6ai 에서 종료.** 남음은 deep-research(§16)
      로드맵 — M9 abstract-interpretation 엔진(interval+octagon+widening)으로 ad-hoc 인식기
      대체, M10 translation-validation 게이트, M12 fallback-free genuine stage1,
      M11 DDC 신뢰, M13 verifier witness 로 확장하는 일.
- P1: 인터프리터 커널 custom macro/`defmacro` reference 축 1차 완료. 커널은 계속 보조축.
- P2: mirror IR 확장. pnix-clj 응용/런처 연동은 M7 금지 조건 해제 전까지 제외.

---

## 7. Boundary (경계 — 무엇을 주장하지 않는가)
- [x] JVM, Java 런타임, Maven, 로컬 JDK는 영구 substrate.
- [x] read는 host reader 위임이 기본(hy-meta 동일 경계).
- [x] 백엔드 B에서는 최종 emit이 host `Compiler`다 — 이 경우 "host Compiler를
      안 거친다"고 주장하지 않는다. 프론트엔드(매크로확장+분석)만 Clojure 소유.
      백엔드 A에서만 host `Compiler` 0회를 주장한다.
- [x] 런타임 라이브러리(clojure.core)는 host 위임. 커널/컴파일러는 언어 처리 소유.
- [x] `clj-meta`는 pnix 런타임 의미론/brain codec/redb ingest/자동 promotion을
      소유하지 않는다.
- [x] stage 불일치는 hard failure.

---

## 8. Validation Commands
```sh
clj-meta/stage7-gate.sh stage7-check   # 재현빌드 레인
clojure -M:kernel-smoke                # 인터프리터 커널(명세/거울) — 24/24
clojure -M:compiler-smoke              # 자작 bytecode 컴파일러 — 106/106
clojure -M:conformance                 # host≡compiler 100/100 + negative 15/15
clojure -M:selfhost-check              # self-host 고정점(M5a) + 미니평가기(M5b)
clojure -M:mirror-smoke                # mirror IR — 2/2
clojure -M:audit-self-source           # compiler.clj 자기소스 op census/ledger + stage target
clojure -M:bytecode-witness            # primitive + raw arithmetic bytecode witness
clojure -M:abstract-interval           # M9 interval lattice/transfer/fixpoint witness
clojure -M:range-migration             # M9 M6z~M6ai range ledger migration witness
clojure -M:translation-validation      # M10 overflow VC + validator + emit-or-refuse witness
clojure -M:bytecode-verifier           # M13 ClassReader + fresh loader verifier/loadability witness
clojure -M:verified-compile            # M13 compiler artifact API verifier hard-fail witness
clojure -M:diverse-double-compile      # M11 DDC behavior-equivalence + TCB witness
clojure -M:reproducible-ddc            # M11e stock Clojure 재현빌드 레인 DDC evidence
clojure -M:cross-host-ddc              # M11 cross-host DDC: 2개 Clojure 버전 bit-identical target emit
clojure -M:gate                        # 통합 receipt(meta 레인) — READY ✅
```

---

## 9. clj-meta vs hy-meta 관계 (전략)
hy-meta/todo.md Judgment와 일치:
```text
clj-meta = canonical mirror / 기호 코어 / 한글-codec 추론 엔진   ← 메인
hy-meta  = Python 생태계 브리지 / 생성코드 백엔드
pnix     = substrate / 헌법 / proof 게이트 / 언어-중립 계약
```

---

## 10. Current State (이번 패스)
- 방향 확정: **인터프리터(보조) + Clojure로 쓴 컴파일러 self-host(주)** 로
  성능·기능 손실 없이 stage15 까지 간다.
- 동작: kernel 24/24, compiler 106/106, conformance host≡compiler 100/100 +
  negative 15/15, kernel 100/100(unsupported 0), mirror 2/2,
  self-source audit 192 forms / 32 ops / unknown 0 / host-maintained `:import` only,
  compiler.clj bytecode-safe stage target 191/191 direct-subset forms / 199 classes /
  fixed-point OK, full-source accounted 192/192, compiled proxy-super smoke OK,
  compiled compiler API artifact smoke OK, compiled self-source compiler impl var smoke OK,
  generated compiler stage-driver chain OK, full namespace artifact/load smoke OK,
  generated stage-chain receipt 생성, stage1→7 on-disk jar bundle digest OK,
  disk-reloaded stage7 compiler entry payload/self-source compile OK,
  fresh disk stage8 exact digest OK, disk proof receipt/compare report OK,
  stage9 clean-process compiler-runtime replay OK,
  stage10 isolated matrix compiler closure OK,
  stage11 multi-surface compiler adapter closure OK,
  stage12 self-improvement quarantine closure OK,
  stage13 long-horizon compiler organism closure OK,
  stage14 cross-host/cross-implementation law closure OK,
  stage15 open-world evidence federation closure OK,
  stageN recursive closure ladder OK, typed locals + typed loop/recur primitive artifact closure OK,
  primitive bytecode witness/disasm receipt OK, safe double raw arithmetic opcode lowering OK,
  simple `reify Object/toString` + `Callable/call` + captured `Callable` direct emit +
  verifier/verified compile artifact OK,
  letfn mutual recursion direct emit + cyclic capture field patching + deterministic class digest OK,
  M9 abstract interval lattice/transfer/fixpoint witness OK,
  M9 M6z~M6ai range ledger migration witness OK,
  M10 translation-validation overflow VC/validator/compiler-admission witness OK,
  M11 DDC behavior-equivalence/TCB/reproducible-lane witness OK,
  M13 bytecode verifier/loadability + verified compile artifact hard-fail witness OK,
  explicit unchecked long raw arithmetic opcode lowering OK, literal checked-long
  static no-overflow proof OK, let-local checked-long range proof OK,
  invariant-loop checked-long range proof OK, bounded-step changing-loop checked-long
  range proof OK, positive non-unit loop stride checked-long range proof OK,
  decreasing loop stride checked-long range proof OK,
  constant accumulator recurrence checked-long range proof OK,
  branch-local fn argument guard checked-long range proof OK,
  bounded index-accumulator sum recurrence checked-long range proof OK,
  multiplicative accumulator recurrence checked-long range proof OK,
  branch-dependent/mixed-sign recurrence checked long raw opcode 는 proof 전까지 held/fallback,
  gate READY.
- 커밋: `c39e0c82`(착수), 그 다음(if/let/재귀+conformance) push 완료
  (브랜치 `feat/clj-meta-metacircular`).
- **M12 fallback-free genuine stage1 달성(2026-06-29)**: `:import` op 직접 emit
  (RT/CURRENT_NS deref→Namespace.importClass∘classForNameNonLoading; host ImportExpr 동일
  bytecode) + `compile-ns :direct-compiled`(compile-fn-strict, no eval fallback) →
  compiler.clj 의 단일 `ns` form 까지 host clojure.lang.Compiler 0회로 backend 컴파일.
  audit witness: `host-compiler-fallback-forms=0`, `ns-side-effect-backend-compiled?=true`,
  `fallback-free-genuine-stage1=accepted`. 4 receipt(audit/full_source_stage1/bytecode_witness/
  language_surface/diverse_double_compile) 일관 반영, gate READY.
  honest 구분: host *Compiler* fallback(=0 달성) ≠ host *runtime-lib* side-effect
  (require/import/in-ns = §13 영구 경계, fallback 아님).
- **M9b 완료(2026-06-29)**: abstract-interp 엔진을 compiler raw-opcode admission 에 sound
  연결해 M6aj **네 케이스 모두** 흡수했다 — branch-dependent stride(interval engine fixpoint) +
  mixed-sign sum(보존 선형량 acc+i=0) + negative-factor(bounded geometric acceleration) +
  non-constant-factor nonlinear acc*acc(bounded interval unrolling, known N → acc∈[2,65536]).
  iteration count 가 정적으로 미지(param bound)인 nonlinear 만 held(어떤 도메인으로도 long
  안에서 bound 불가). 별도 ad-hoc 패턴 없음, 모든 승격은 tv validator 통과 + overflow soundness
  sentinel + bytecode witness. smoke 115/115, conformance 100/100, gate READY.
- **언어 표면 완성(2026-06-29)**: reify(simple+IObj)/deftype(immutable+mutable)/defrecord/
  defmulti/defprotocol(+reify-구현)/protocol-invoke/ns(simple+require/import) **전부 직접 emit**,
  `language_surface` held 0. self-host fallback-free(M12) + M6aj 흡수(M9b) + 전체 stage8~15/N
  closure + self-host stage1→7 fixed-point(257 classes) + full-eval tower(host≡compiled≡tower).
  smoke 127/127, conformance 100/100, gate READY.
- 남은 frontier(§16, 연구/독립-toolchain 경계 — 본질적으로 backend 밖):
  (1) statically-unknown iteration count 의 nonlinear recurrence(런타임 param 의존 → long 안에서
  bound 불가, held 가 정직), (2) full Wheeler compiler-binary DDC(독립 Clojure 컴파일러 필요;
  cross-host 2-버전 bit-identical partial 증거는 완료), (3) CompCert-급 validator 기계검증
  soundness(formal proof). 모두 supervised/독립-구현/형식증명 필요.
  ※ 상세 로드맵·이론·출처는 **§16** 참조.

---

## 10a. Stage8~Stage15 / StageN Ladder (clj-meta compiler 기준)

`clj-meta`의 stage ladder 는 pnix-clj 제품 런처 소비가 아니라, Clojure로 쓴
compiler/runtime artifact 의 폐쇄성을 단계적으로 올리는 내부 proof 레인이다.

```text
stage7  = semantic/self-host closure
stage8  = bytecode/class artifact reproducibility
stage9  = clean process compiler-runtime replay closure
stage10 = isolated classpath/session/sandbox compiler closure
stage11 = multi-surface compiler adapter closure
stage12 = self-improvement quarantine closure
stage13 = long-horizon compiler organism closure
stage14 = cross-host/cross-implementation law closure
stage15 = open-world evidence federation closure
stageN  = 위 closure 를 새 host/runtime/proof surface 로 반복 확장
```

- [x] **Stage7 — semantic/self-host closure**
      self-source stage target 이 generated compiler driver 로 stage1→7 반복 컴파일되고,
      host reference / receipt digest / load smoke 가 수렴한다.
- [x] **Stage8 — artifact reproducibility closure**
      stage1→7 class bundle 을 on-disk jar 로 쓰고, disk-reloaded stage7 compiler 가 만든
      fresh stage8 bundle 까지 byte-identical 로 수렴한다.
- [x] **Stage8 proof split**
      artifact proof 를 `self-source-generated-stage-chain-disk.receipt.edn` 과
      `work/self-source-generated-stage-chain/compare.txt` 로 분리한다.
- [x] **Stage9 — clean process compiler-runtime replay closure**
      같은 stage8 artifact 를 새 JVM/fresh process 에서 읽어 compiler entrypoint 를 실행하고,
      canonical compiler receipt / payload result / artifact digest 를 재현한다.
- [x] **Stage10 — isolated classpath/session/sandbox compiler closure**
      project-root-bound cwd 에서 locale/timezone, 임시 work dir, clean namespace/session 변화를
      matrix 로 돌려도 compiler verdict 가 drift 하지 않는지 검증한다. cwd 자체 이동은
      source path resolver 일반화 후 다음 hardening 으로 확장한다.
- [x] **Stage11 — multi-surface compiler adapter closure**
      source form, mirror IR, self-source target, conformance corpus, kernel/evaluator target 이
      모두 같은 accepted/held/fallback 경계와 receipt schema 를 지키는지 검증한다.
      receipt: `clj-meta/proof/stage11-multisurface.receipt.edn`.
- [x] **Stage12 — self-improvement quarantine closure**
      compiler rule/emit/fallback 개선 후보가 live truth 를 직접 바꾸지 않고 quarantine →
      replay → gate → admission 경계를 지키는지 검증한다.
      receipt: `clj-meta/proof/stage12-quarantine.receipt.edn`.
- [x] **Stage13 — long-horizon compiler organism closure**
      여러 날/여러 snapshot/여러 source update 를 replay 해 stale artifact, cache, namespace,
      explanation drift 가 fail-closed 되는지 검증한다.
      receipt: `clj-meta/proof/stage13-long-horizon.receipt.edn`.
- [x] **Stage14 — cross-host/cross-implementation law closure**
      clj-meta, hy-meta, pnix-hy/pnix-clj 의 canonical fixture verdict/answer-plan hash 를
      비교하고 cross-host drift 를 held/fail-closed 로 남긴다.
      receipt: `clj-meta/proof/stage14-crosshost.receipt.edn`.
- [x] **Stage15 — open-world evidence federation closure**
      Lean/Z3/CAS/GitHub/LLM/document/sandbox 결과를 evidence-only 로 받아 canonicalization,
      provenance, replay, gate/admission 전에는 accepted 로 승격하지 않음을 검증한다.
      receipt: `clj-meta/proof/stage15-openworld.receipt.edn`.
- [x] **StageN — recursive closure ladder**
      stage15 이후 새 host/runtime/proof surface 가 추가될 때마다 artifact, runtime, adapter,
      quarantine, federation closure 를 같은 receipt law 로 반복 확장한다.
      receipt: `clj-meta/proof/stageN-recursive.receipt.edn`.

## 11. Self-host 완성 — emit op 전수 체크리스트 (싹 다)

`compiler.clj` 가 자기 자신을 컴파일하려면(= stage1 self-host) 아래 analyzer
`:op` 들을 우리 emit 이 전부 처리해야 한다. 상태: [x]=구현, [~]=부분, [ ]=미구현.

### A. 직접 emit (host Compiler 0회) — 완료 [x]
식(expression):
- [x] `:const` — primitive(long/double/bool/string/nil)=ldc/box 직접; 복합(keyword/
      symbol/vector/map/set)= **static field 캐싱**(§14)
- [x] `:local` — env [:arg i]=loadArg / [:local slot]=loadLocal / [:this]=loadThis /
      [:field name]=getfield(클로저 캡처)
- [x] `:var` — **static field 캐싱**(`<clinit>` RT.var 1회) + Var.deref
- [x] `:the-var` — static field 캐싱된 Var (deref 안 함)
- [x] `:invoke` / `:prim-invoke` — IFn.invoke(Object…)
- [x] `:protocol-invoke` — protocol fn Var + target + args 를 IFn.invoke 로 직접 호출
- [x] `:static-call` — analyzer 정적 타입 기반 overload 점수화; 인자 coerce; 반환 box
- [x] `:if` — RT.booleanCast + ifZCmp 분기
- [x] `:do` — statements(pop) + ret
- [x] `:let` — newLocal 슬롯, 순차 바인딩
- [x] `:loop` / `:recur` — loop 슬롯/fn 인자 + JVM goto(비스택 반복)
- [x] `:vector` / `:map` / `:set` — RT.vector/map/set(Object[]) (동적); const 는 캐싱
- [x] `:quote` — :expr(const) emit
- [x] `:throw` — checkCast Throwable + athrow
- [x] `:try` / `:catch` / `:finally` — visitTryCatchBlock + handler, 정상/handled/uncaught
      모든 경로 finally 실행. 중첩 try 는 JVM exception table 순서 보존.
- [x] `:new` — NEW+dup+인자 coerce+invokespecial <init>
- [x] `:instance-call` — invokevirtual/invokeinterface + 인자 coerce + 반환 box
- [x] `:instance-field` / `:static-field` — getfield/getstatic (+ primitive box)
- [x] `:host-interop` — Reflector field/0-arg member + 동적 field `set!`
- [x] `:keyword-invoke` — `Keyword`를 IFn 으로 직접 호출
- [x] `:instance?` — JVM `INSTANCEOF` + Boolean box
- [x] `:fn` — AFn(다중 fixed arity: 각 invoke) / RestFn(variadic 및 fixed+variadic
      혼합: getRequiredArity + doInvoke + fixed invoke override) + **클로저 캡처**
      (free-locals→필드+생성자); 중첩 fn=NEW+캡처 주입
- [x] `:letfn` — 상호재귀 closure 를 generated fn class 로 만들고, 생성 후
      mutable capture field(`putfield`) 를 채워 cyclic reference 를 닫는다.
- [x] `:with-meta` — 내부 expr 위임
- [x] `:set!` — Var.set(thread binding) + public instance/static/dynamic field set
- [x] `:monitor-enter` / `:monitor-exit` — `locking` macro expansion 지원
- [x] dynamic var binding(`binding` 매크로) — analyzer macro expansion
      (`push-thread-bindings` + `try/finally`) 경로 직접 emit
top-level:
- [x] `:def` — push-var + Var.bindRoot (Var 반환); 값 위치 :var 는 deref
- [x] 여러 top-level form — `eval-form`(0-arity 래핑) + `(do …)`

### B. host 위임(의도적 — 성능 정책상 host 가 최적, §13/§14) [x]
- [x] 현재 `:letfn` 과 simple `:reify` Object/interface/capture method 는 직접 emit 으로 승격.
      남은 host 위임은 namespace/general type-generation side-effect 계열로만 제한한다.

### C. host-maintained boundary (직접 emit 안 함; Clojure 기능 손실 0) [~]
- [x] `:ns` / `:import` / `:require` — `language_surface.clj` 와
      `full_source_stage1.clj` receipt 로 namespace/import side-effect boundary 고정.
- [~] `:deftype` / general `:reify` (+`:method`) — 새 JVM 타입 / 익명 구현 클래스.
      simple `reify Object/toString` 은 직접 emit accepted. `deftype` 과 IObj/meta
      semantics 같은 general reify 는 host-maintained boundary 로 유지.

### D. 분석·정확성
- [x] var/const **static field 캐싱** — 매 호출 생성 제거, 전반 host급(§14)
- [x] COMPUTE_FRAMES getCommonSuperClass override — 깊은 중첩 ClassNotFound 해결
- [x] **결정적 클래스명**(*gen-counter* 단위 리셋) + gensym-free → self-host 고정점
- [x] SoftReference GC 회피 강참조(kept-class-units bounded cache)
- [x] primitive 타입 추론/coercion — Numbers long/double 정적 인자 overload,
      함수 인자 coercion, let/loop/recur typed primitive local slots, bytecode witness,
      lowering admission receipt 완료. AFn public invoke 경계의 Object box/unbox 는 JVM/IFn
      ABI boundary 로 유지.
- [x] 메서드/생성자 오버로드 정밀 선택 — Numbers 정적 인자 + 일반 host interop
      static/instance/new 후보 점수화 완료.
- [x] source 위치/line 메타 보존(stacktrace 정확도)

> 인터프리터 커널(`kernel.clj`)의 self-host 경로는 별도다(Phase 1): 커널이
> 커널 부분집합만으로 작성됐는지 감사 → k-eval 로 kernel.clj 해석 → 고정점.
> 컴파일러 self-host(아래 §12)가 주 경로이고 커널은 reference semantics.

---

## 12. Milestones (마일스톤 순서 — self-host stage7 까지)

```text
M1  [완료] 인터프리터 커널 동작(명세/거울)                     kernel 24/24
M2  [완료] 자작 bytecode 컴파일러 — 산술/분기/재귀(factorial)   compiler 14/14
M3  [완료] conformance 게이트 host≡compiler                     13/13
M4  [완료] §11 op (i)~(vi 일부) → 작은 프로그램(분기/재귀/loop/클로저/컬렉션/interop/
            def/try-finally/set!/binding/locking/fixed+variadic 혼합) 컴파일 가능.
            compiler 80/80, conf 80/80, negative 4/4.
            (잔여 op: ns/deftype/general reify 등은
            fallback 또는 host 유지)
M5a [완료] deterministic self-compile 고정점: stage1..7 bytecode byte-동일(7 classes)
M5b [완료] compiler-compiles-evaluator: 우리 컴파일러가 미니 메타순환 평가기를 컴파일,
            host≡compiled 8/8 + 소스 bytecode 고정점 + full-eval tower OK.
M6  [완료] 통합 게이트(gate.clj): meta 레인 단일 receipt = READY
            (full-eval tower 포함, metacircular.receipt.edn)
M6a [완료] compiler.clj self-source audit: analyzer op census/ledger,
            unknown op 0, direct-subset 후보 191/192, gate 포함.
M6b [완료1차] compiler.clj bytecode-safe stage target: direct-subset 191/191
            top-level form 으로 stage1→7 bytecode 고정점(199 classes) receipt 생성,
            gate 포함.
M6c [완료1차] full-source accounting: `ns`/`:import` 1개는 host side-effect wrapper 로,
            나머지 191개는 bytecode stage target 으로 분리해 192/192 회계 완료.
            compiled proxy-super dynamic instance-call 실행 smoke gate 포함.
M6d [완료1차] compiled compiler API artifact smoke: `compile-ns` 로 격리 namespace 를
            compiled artifact 로 로드하고, 그 안의 compiled wrapper 가
            `compile-form`/`compile-classes` 를 호출해 작은 프로그램을 재컴파일.
            host-loaded wrapper 와 결과/bytecode fingerprint 일치.
M6e [완료1차] compiled self-source compiler impl var smoke: self-source direct-subset
            163개 form 을 격리 namespace 에 compiled wrapper 로 실행해 생성된
            `compile-form`/`compile-classes` var 자체로 작은 프로그램을 재컴파일.
            host-loaded compiler 와 결과/bytecode fingerprint 일치.
M6f [완료1차] generated compiler var 자체를 stage driver 로 사용해 self-source target 을
            stage1→7 반복 컴파일→로드. 매 stage fixed-point + host-driven stage target
            receipt bytecode digest 일치.
M6g [완료1차] generated stage-driver chain 을 별도 receipt 로 분리하고 gate 에 compact 요약
            추가. full namespace artifact/load(`ns`/`:import` side-effect wrapper 포함)
            검증으로 manual retargeted stage target 의 남은 경계를 줄임.
            receipt: `clj-meta/proof/self-source-generated-stage-chain.receipt.edn`.
M6h [완료1차] generated stage-chain 의 in-memory class map 을 on-disk jar bundle 로 쓰고,
            jarproof 재사용 가능한 stable jar digest 로 stage1→7 exact 비교.
            bundles: `work/self-source-generated-stage-chain/*/classes.jar`.
M6i [완료1차] generated stage-chain 의 on-disk jar bundle 을 다시 로드해 generated compiler
            entry(`compile-form`/`compile-classes`)가 디스크 산출물만으로 재실행되는지 검증.
            stage7 jar reload → payload bytecode host 일치 + self-source stage target 재컴파일 가능.
            fresh reload 의 stage8 exact digest 일치는 별도 M6j 로 분리.
M6j [완료1차] fresh disk stage8 exact digest 고정점: disk-reloaded stage7 compiler 로
            self-source target 을 재컴파일한 class digest 까지 stage7 jar 와 byte-identical 로 맞춘다.
            원인 수정: generated class static var/const field emission 을 field name 순서로 결정화.
M6k [완료1차] generated stage-chain disk reload proof 를 별도 receipt/compare report 로 분리해
            stage-chain receipt 를 더 작고 재사용 가능하게 정리.
            receipt: `clj-meta/proof/self-source-generated-stage-chain-disk.receipt.edn`.
            compare: `work/self-source-generated-stage-chain/compare.txt`.
M6l [완료1차] stage9 clean process compiler-runtime replay: 새 JVM 에서 stage8 artifact 를
            로드해 canonical compiler receipt/payload/artifact digest 를 재현.
            receipt: `clj-meta/proof/self-source-stage9-clean-process.receipt.edn`.
            child receipt: `work/self-source-stage9-clean-process/child.receipt.edn`.
M6m [완료1차] stage10 isolated classpath/session/sandbox compiler closure:
            project-root-bound cwd + locale/timezone/namespace/work-dir matrix 에서
            같은 stage9 canonical digest 를 재현.
            receipt: `clj-meta/proof/self-source-stage10-isolated-matrix.receipt.edn`.
            compare: `work/self-source-stage10-isolated-matrix/compare.txt`.
M6n [완료1차] stage11 multi-surface compiler adapter closure: source form / mirror IR /
            self-source target / conformance corpus / kernel-evaluator target 의
            accepted/held/fallback 경계와 receipt schema 를 한 matrix 로 비교.
            receipt: `clj-meta/proof/stage11-multisurface.receipt.edn`.
M6o [완료1차] stage12 self-improvement quarantine closure: compiler rule/emit/fallback 개선
            후보가 live truth 를 직접 바꾸지 않고 quarantine → replay → gate →
            admission 경계를 지키는지 proof schema 와 fixture 로 검증.
            receipt: `clj-meta/proof/stage12-quarantine.receipt.edn`.
M6p [완료1차] stage13 long-horizon compiler organism closure: 여러 snapshot/day/session label 로
            같은 compiler artifact/verdict 를 replay 하고, artifact/source/stage11-12 digest
            drift 는 stale/held 로 강등하는 audit receipt 를 만든다.
            receipt: `clj-meta/proof/stage13-long-horizon.receipt.edn`.
M6q [완료1차] stage14 cross-host/cross-implementation law closure: clj-meta 단독 canonical
            host transcript 를 먼저 정의하고, hy-meta/pnix-hy/pnix-clj transcript 가 아직
            없으면 held 로 남긴다. cross-host drift 는 accepted 로 승격하지 않는다.
            receipt: `clj-meta/proof/stage14-crosshost.receipt.edn`.
M6r [완료1차] stage15 open-world evidence federation closure: Lean/Z3/CAS/GitHub/LLM/document/
            sandbox 결과를 evidence-only 로 받아 provenance/canonicalization/replay/gate
            전에는 accepted 로 승격하지 않는 receipt 를 만든다.
            receipt: `clj-meta/proof/stage15-openworld.receipt.edn`.
M6s [완료1차] stageN recursive closure ladder: 새 host/runtime/proof surface 추가 시
            stage8~15 receipt law 를 반복 적용하는 registry/schema 를 만들고, 미지원
            surface 는 held 로 남긴다.
            receipt: `clj-meta/proof/stageN-recursive.receipt.edn`.
M6t [완료1차] stage10 cwd hardening: cwd 자체를 sandbox/work dir 로 옮겨도 source path resolver 와
            artifact path 가 같은 canonical digest 를 재현한다.
            receipt: `clj-meta/proof/self-source-stage10-isolated-matrix.receipt.edn`,
            cwd-policy: `:root-bound-cwd-independent`, variant: `sandbox-cwd`.
M6u [완료1차] typed locals primitive 확대: primitive invoke 결과뿐 아니라 함수 인자와
            let 지역 슬롯의 long/double/boolean primitive 경로를 env 정책으로 고정한다.
            `compile-form` 과 `compile-classes` artifact 경로 모두 typed locals 를 켜고,
            line-number metadata 만 artifact 에서 정규화해 stage1→7, fresh stage8,
            stage9 clean process, stage10 isolated matrix digest 를 통과시킨다.
            compiler/conformance 80/80, self-source 132/132, classes 117.
M6v [완료1차] typed loop/recur primitive slots: loop 바인딩 init 타입으로
            long/double/boolean primitive local slot 을 잡고, recur expr 를 target 타입으로
            먼저 평가한 뒤 역순 store 한다. typed long loop 와 mixed long/double loop 를
            smoke/conformance/bench 에 추가했고, stage1→7/fresh stage8/stage9/stage10
            self-source artifact 고정점을 통과했다.
            compiler/conformance 80/80, self-source 132/132, classes 117.
M6w [완료1차] primitive bytecode witness/disasm receipt: typed let/loop/recur 산출물에서
            `lstore`/`dstore`/`lload`/`dload` 와 `Numbers.*` primitive descriptor call 이
            실제로 남는지 ASM normalized witness 로 별도 receipt 에 고정했다.
            raw `ladd`/`dadd` 직접 lowering 은 overflow 의미 보존 검토 후 M6x 로 분리.
            receipt: `clj-meta/proof/primitive-bytecode-witness.receipt.edn`.
M6x [완료1차] safe raw primitive arithmetic opcode lowering: overflow/ratio 승격 의미를
            깨지 않는 typed double arithmetic 은 `dadd`/`dmul`/`dsub` 직접 opcode 로
            내리고, typed long arithmetic 은 Clojure `Numbers.*` overflow 의미 보존 proof
            전까지 `Numbers.* (JJ)` primitive descriptor call 로 held/fallback 한다.
            bytecode witness 에 `typed-let-double-direct` 를 추가했고 mixed loop 에서 `dadd`
            가 실제로 남는지 고정했다. long overflow sentinel 은 기존 negative conformance 로
            유지한다.
            compiler/conformance 80/80, self-source 132/132, classes 117, gate READY.
M6y [완료1차] explicit unchecked long raw arithmetic lowering: `unchecked-add`,
            `unchecked-subtract`, `unchecked-multiply` 는 사용자가 overflow wraparound
            의미를 명시한 경계이므로 typed long 에서 `ladd`/`lsub`/`lmul` 로 직접
            내린다. checked `+`/`-`/`*` long 은 Clojure overflow 의미 보존 proof 전까지
            계속 `Numbers.* (JJ)` 로 남긴다. bytecode witness 에 `unchecked-long-direct`
            를 추가했고 long overflow negative sentinel 은 계속 고정한다.
            compiler/conformance 83/83, self-source 132/132, classes 117.
M6z [완료1차] checked-long static no-overflow proof: 두 인자가 literal long 이고
            `Math/addExact`/`subtractExact`/`multiplyExact` 로 overflow 없음이 컴파일 시점에
            증명되는 checked `+`/`-`/`*` 만 `ladd`/`lsub`/`lmul` 로 내린다.
            proof 가 없거나 range 가 열려 있으면 계속 `Numbers.* (JJ)` fallback.
            `(+ 9223372036854775807 1)` 과 `(* 9223372036854775807 2)` 는 negative
            sentinel 로 계속 overflow 실패를 보존한다. bytecode witness digest:
            `8473d9f937403ba3077d6a5362d421840a3e2bf55d38d05782bd4588c964fe89`.
            compiler/conformance 86/86, negative 6/6, self-source 134/134, classes 119.
M6aa [완료1차] checked-long let-local range proof: `let` 지역 env tuple 에 optional
            `{:min ... :max ...}` range ledger 를 붙이고, literal/local/static-call result
            range 가 `Math/*Exact` endpoint 검사를 통과할 때만 checked `+`/`-`/`*`
            long 을 `ladd`/`lsub`/`lmul` 로 내린다. `let [a Long/MAX_VALUE] (+ a 1)`
            은 fallback `Numbers.* (JJ)` 로 남아 overflow 실패를 보존한다.
            bytecode witness digest:
            `58e7d4e65b73648c813d601492d1ac0a2ec8d6c372be99b50c09abd195a44202`.
            compiler/conformance 89/89, negative 7/7, self-source 142/142, classes 126.
M6ab [완료1차] checked-long invariant-loop range proof: loop 바인딩 init 의
            literal/local/static-call range 를 env ledger 에 보존하되, 현재 loop 의 모든
            `recur` expr 가 해당 바인딩에 같은 local 을 되돌려주는 경우에만
            invariant 로 인정한다. 이 닫힌 범위에서 `Math/*Exact` endpoint 검사를 통과한
            checked `+`/`-`/`*` long 만 `ladd`/`lsub`/`lmul` 로 내린다.
            changing loop recurrence 와 fn argument range 는 아직 precondition ledger 가
            없으므로 계속 `Numbers.* (JJ)` fallback/held.
            bytecode witness digest:
            `01810d941c5a74df9533ba67bac97f020da19c7bea672bbd41129ec5d8a67c26`.
            compiler/conformance 92/92, negative 8/8, self-source 146/146, classes 131.
M6ac [완료1차] checked-long bounded-step changing-loop range proof: `loop [i init]`
            본문이 top-level `if (< i bound)` 이고, then 쪽 `recur` 가 모두 해당
            바인딩을 `(+ i 1)` 로만 갱신하며 else 쪽에는 current-loop recur 가 없을 때만
            `i` range 를 `[init,bound]` 로 인정한다. 그 range 에서 `Math/*Exact`
            endpoint 검사를 통과한 checked long `+`/`-`/`*` 만 `ladd`/`lsub`/`lmul`
            로 내린다. overflow sentinel 은 bound 끝에서 계속 fallback 되어 host 와 같은
            long overflow 를 보존한다.
            bytecode witness digest:
            `07f6acdc799a8e53abe4067b6181e10290c2c537d80d6325aee391c9c08fe501`.
            compiler/conformance 93/93, negative 9/9, self-source 150/150, classes 136.
M6ad [완료1차] checked-long positive non-unit loop stride range proof: `loop [i init]`
            본문이 top-level `if (< i bound)` 이고, then 쪽 current-loop `recur` 가 모두
            해당 바인딩을 `(+ i k)` 양수 literal stride 로만 갱신하며 else 쪽에는
            current-loop recur 가 없을 때만 `i` range 를 `[init,bound+k-1]` 로 인정한다.
            `bound+k-1` 자체도 `Math/*Exact` 범위 계산을 통과해야 하며, overflow 하거나
            stride 가 0/음수/비선형이면 계속 `Numbers.* (JJ)` fallback 으로 남긴다.
            bytecode witness digest:
            `e3a7334c2f268f08cfe3e12a2e9491a869e092336f08f133e5bed4d9dae8e2e0`.
            compiler/conformance 94/94, negative 10/10, self-source 150/150, classes 136.
M6ae [완료1차] checked-long decreasing loop stride range proof: `loop [i init]`
            본문이 top-level `if (> i bound)` 이고, then 쪽 current-loop `recur` 가 모두
            해당 바인딩을 `(- i k)` 양수 literal stride 로만 갱신하며 else 쪽에는
            current-loop recur 가 없을 때만 `i` range 를 `[bound-k+1,init]` 로 인정한다.
            `bound-k+1` 자체도 `Math/*Exact` 범위 계산을 통과해야 하며, underflow 하거나
            stride 가 0/음수/비선형이면 계속 `Numbers.* (JJ)` fallback 으로 남긴다.
            bytecode witness digest:
            `c175bf4abfd17db24c1b3aa400c5ea5fda081895fa919541b6b4460f6e328a1b`.
            compiler/conformance 95/95, negative 11/11, self-source 152/152, classes 139.
M6af [완료1차] checked-long constant accumulator recurrence range proof: top-level
            `if (< i bound)` + then-only current-loop `recur` + positive literal index
            stride 로 반복 횟수를 exact ceil-div 로 증명하고, 다른 loop binding 이
            `(+ acc k)` 또는 `(- acc k)` 양수 literal stride 로만 갱신될 때 total delta 를
            `Math/*Exact` 로 계산해 accumulator range 를 보존한다. overflow/widening/unknown
            이면 계속 `Numbers.* (JJ)` fallback.
            bytecode witness digest:
            `3a5155a991afd4ed4140d924e018cc47642fbbe995335f12c13bb51b519d9ed6`.
            compiler/conformance 96/96, negative 12/12, self-source 155/155, classes 146.
M6ag [완료1차] checked-long branch-local fn argument guard range proof: `if (< x bound)` 와
            `if (> x bound)` 의 then/else 분기에서 local long range 를 좁히고, 중첩 guard 는
            range intersection 으로 닫힌 fn argument precondition 을 만든다. 이 range 가
            `Math/*Exact` endpoint 검사를 통과할 때만 checked `+`/`-`/`*` long 을
            `ladd`/`lsub`/`lmul` 로 내린다. guard 가 부족해 underflow/overflow 가능성이
            남으면 계속 `Numbers.* (JJ)` fallback.
            bytecode witness digest:
            `77038e3bfa83c39e7a0963f86a71fe0e674bdbec0da1f8a8af78cec27717c7d4`.
            compiler/conformance 97/97, negative 13/13, self-source 159/159, classes 150.
M6ah [완료1차] checked-long bounded index-accumulator sum recurrence range proof:
            positive bounded index loop 의 iteration count 와 index arithmetic-series sum 을
            exact 계산하고, accumulator 가 `(+ acc i)` 또는 `(- acc i)` 로만 갱신될 때
            total delta range 를 보존한다. index init 이 음수이거나 sum/add/subtract 가
            overflow 가능하면 계속 `Numbers.* (JJ)` fallback.
            bytecode witness digest:
            `1f50c05c96f12bd037f135b5fd93b4c5996ba4587cb16860b2202bb3de9941ea`.
            compiler/conformance 98/98, negative 14/14, self-source 162/162, classes 157.
M6ai [완료1차] checked-long multiplicative accumulator recurrence range proof:
            positive bounded index loop 에서 accumulator 가 `(* acc k)` 양수 literal factor 로만
            갱신되고 init 이 non-negative singleton 일 때, `k^iterations` 를 exact 계산해
            multiplicative range 를 보존한다. factor power 또는 init*scale 이 overflow 하면
            계속 `Numbers.* (JJ)` fallback.
            bytecode witness digest:
            `5cc53d819697151690ab3feb42aacadadc77a2d274345dbc0c1cf294b044f380`.
            compiler/conformance 99/99, negative 15/15, self-source 164/164, classes 160.
M6aj [흡수예정→M9b] checked-long branch-dependent/mixed-sign recurrence range proof:
            branch-dependent stride, mixed-sign index sum, negative factor recurrence 처럼 일반
            induction/ranking proof 가 필요한 recurrence 는 별도 admission 전까지 직접 opcode 로
            승격하지 않는다. widening/unknown 이면 계속 `Numbers.* (JJ)` fallback.
            ※ deep-research(§16) 결론: 이걸 **또 하나의 ad-hoc 패턴으로 추가하지 말고**
            §16.1 abstract-interpretation 엔진(interval+octagon+BMS ranking) 의 첫 소비자로
            구현한다(M9b). 한 패턴씩 늘리는 라인은 M6ai 에서 종료.
M7  [PARKED] 재현빌드/meta receipt 의 pnix-clj 런처 소비는 완전한 meta-circular
            stage15 Clojure compiler 전까지 금지. 응용 연결보다 compiler 완성이 우선.
M8  [완료1차] mirror IR: analyze 결과를 canonical mirror form 으로(smoke 2/2)
```

핵심 게이트(M6): 각 stage 가 생성한 `.class`/bytecode 가 stage 간 byte-동일이면
"Clojure 로 쓴 컴파일러가 자기 자신을 안정적으로 재생산"한다는 진짜 meta-circular
stage 증명. 이때 §11 의 '결정적 클래스명/gensym' 이 필수.

---

## 13. 전략 — host 위임 우선 + stage7 목표 (사용자 지시 2026-06-28)

> **"Clojure 기능을 손실 없이 그대로 이용하며 빠른 걸 우선 택하고, 안 되면 직접
> 구현해서 clojure meta-circular stage7 compiler 를 완성한다. hy-meta 가 CPython
> 에 위임하듯 host 위임 경계를 todo 에 명시적으로 적는다."**

### 목표
- 목표 = meta-circular **stage7** compiler. (stage 숫자는 self-host 반복 횟수일 뿐
  난이도가 아니다 — 우리 고정점은 결정적이라 stage1→7 byte-동일. hy-meta 도 stage7.)
- 우선순위 = **host 위임으로 빠르게**. host clojure.lang.Compiler / clojure.core /
  reader 를 손실 없이 활용. host 로 안 되는 것만 우리가 직접 emit.
- hy-meta 모델과 동일: hy-meta 도 lower 의 상당부분을 `hy.compiler`/CPython 에 위임
  하고, `compiler.hy`(프로토콜, 206줄)만 Hy 로 작성해 self-host. 우리도 같다.

### host 위임 경계 (hy-meta CPython 위임에 대응 — 명시적)
```text
read        → host reader (clojure reader). 리더 재구현 안 함.
runtime lib → clojure.core 전부 host 위임 (+, map, first, assoc, …).
macroexpand → host (analyzer 가 host 매크로를 확장; defn/when/cond/-> 등 자동).
compile fallback → 우리 emit 못 하는 form(잔여 op: ns /
              deftype / general reify / …)은 **form 전체를
              host clojure.lang.Compiler(eval)에 위임**. → Clojure 전체 기능 손실 0.
JVM/Java/Maven/JDK → 영구 substrate.
```

### 우리가 직접 emit (host Compiler 0회) — "안 되면 직접 구현"의 실증
- `compiler.clj`: 식 op 전반 + 클로저/캡처 + interop + def + variadic/mixed +
  try/finally + set! + monitor + line metadata + compile-ns artifact API +
  overload 점수화 + keyword-invoke + instance? + host-interop + typed let/loop/recur locals +
  letfn mutual recursion + simple reify Object/interface method.
  compiler 106/106, conformance host≡compiler 100/100.
- 즉 우리는 host 위임 가능한 빠른 경로를 우선하되, 핵심 의미론은 직접 emit 으로
  소유하고 있음을 이미 증명했다. fallback 은 '아직 직접 emit 안 한 op' 의 빠른 보완.

### 잔여 op 직접 emit + 성능 정책 (사용자 지시 2026-06-28)
- **잔여 op 를 우선순위대로 직접 emit 으로 끌어옴**(host fallback → 우리 bytecode):
  `case` → 다중arity → `letfn` → `try-finally` → `set!` → `ns` 순서 중
  다중arity/fixed+variadic 혼합/try-finally/set! 은 완료. clojure 원기능(host
  위임)으로 stage7 까지 가되, 직접 emit 이 필요/유리하면 구현. host 위임은 임시 보완.
- **성능: 한 동작을 여러 emit 방향으로 구현 → 벤치마크 → 빠른 것만 남긴다.**
  (host 위임이 빠르면 host 유지, 우리 직접 emit 이 빠르면 직접 채택. 측정으로 결정.)

**진척(측정 기반, `clojure -M:bench`):**
- 다중arity/fixed+variadic 혼합: 직접 emit ✅ (현재 전체 gate compiler 106/106).
  mixed variadic 벤치
  ratio **0.98** → 직접 채택.
- case: **직접 emit 채택** — analyzer `case*` compact key 를 따라
  JVM `tableswitch`/`lookupswitch` 로 dispatch 하고 `Util/equiv` guard 로 확정.
- **[개선] Numbers 정적 primitive overload 선택**: analyzer 가 이미 long/double 로 아는
  상수/static-field 인자는 `(Object,long)`/`(long,long)` 등으로 호출. 런타임 guard 없이
  Clojure checked overflow 의미 보존.
- 벤치 인프라(benchmark.clj): 대부분 host급 이상(loop 0.81 / 다중arity 0.83 /
  mixed variadic 0.98 / factorial 1.03 / map+캡처 1.04 / `*` 1.11).
- **[개선] var static field 캐싱**: 참조 Var 를 클래스 static field 로 두고 `<clinit>`
  에서 RT.var 1회 초기화, 호출 시 getstatic(host 와 동일). map 2.98→1.53 등.
- **[개선] const 컬렉션 static field 캐싱**: 복합 const(벡터/맵/keyword/symbol)를
  static field + `<clinit>` 1회 생성(host 와 동일). 원인=const 매번 RT.vector 생성
  (const vector 단독 2.96). 결과: **map 1.53→1.21, 전반 벤치 host급 이상(0.90~1.21).**
- **letfn: 직접 emit 채택** — 상호재귀 순환 캡처(`:letfn`)는 generated fn class 를
  먼저 만들고, local 슬롯 저장 후 peer capture field 를 `putfield` 로 채워 닫는다.
  language-surface/bytecode-witness/verifier/verified-compile receipt 에서 accepted.

### stage7 self-host
- 우리가 100% 직접 emit 하는 부분집합 프로그램을 stage1→7 반복 컴파일 →
  bytecode 고정점. fallback form 은 host(비결정적)라 고정점 타겟엔 쓰지 않는다
  (고정점은 우리 emit 결정성으로만).
- 즉 "stage7 완성" = (a) host 위임으로 임의 Clojure 컴파일·실행 가능(기능 손실 0)
  + (b) 우리 직접 emit 부분집합의 stage1→7 self-compile 고정점.

---

## 14. 현재 구현 방식 상세 (세세히 — 코드 기준)

### 14.1 컴파일 파이프라인
```text
form
 → tools.analyzer.jvm/analyze   (매크로확장 + 분석; host 매크로/특수형식 정규화)
 → AST (analyzer :op 트리)
 → 우리 emit (clojure.asm)       (compiler.clj — host clojure.lang.Compiler 0회)
 → AFn / RestFn 서브클래스 bytecode
 → DynamicClassLoader.defineClass → newInstance
 → JVM 실행 (진짜 bytecode = host급 성능)
```
진입점:
- `compile-form`  — 단일 `(fn …)` → IFn. 우리 emit, 미구현 op 면 `(eval form)` fallback.
- `eval-form`     — 임의 top-level form 을 `(fn [] form)` 으로 감싸 컴파일·실행(def 부수효과 포함).
- `compile-ns`    — source 문자열 → ns 준비 + top-level thunks 를 가진 in-memory loadable artifact.
- `load-compiled-ns` — `compile-ns` 산출물을 현재 JVM 에 순차 로드.
- `compile-classes` — 실행 없이 `{classname byte[]}` 반환(self-host 고정점 비교용).

### 14.2 fn 클래스 구조
- 고정 arity: `clojure.lang.AFn` 상속. arity 마다 `invoke(Object*n)` override.
- variadic: `clojure.lang.RestFn` 상속. `getRequiredArity()=fixed`,
  `doInvoke(Object*(fixed+1))`(마지막 인자=rest seq). RestFn.invoke 가 초과 인자를 seq 로 묶음.
  fixed+variadic 혼합은 RestFn 에 fixed `invoke` 메서드도 함께 override.
- 클래스명: `pnix.clj_meta.gen.Fn__N` (N=*gen-counter*, compile 단위마다 -1 리셋 → 결정적).
- 클로저 캡처: `free-locals`(자유 지역변수 분석) → `ACC_FINAL` Object 필드 `__cap0..` +
  생성자 `<init>(Object*nfv)` 가 필드 저장. 본문은 getfield(this). 중첩 fn 은 `emit-fn`
  이 NEW+dup+캡처값(둘러싼 env 에서 평가)+invokespecial `<init>`.
- self-ref(named fn): env 에 `[:this]` → loadThis.
- recur: env `:recur-target {:label :targets[[:arg i]|[:local slot ?primitive-class]]}`.
  recur 는 모든 expr 을 target 타입으로 평가(옛 값 기준)→역순 store→goTo(비스택 반복).

### 14.3 emit 컨텍스트 env
```text
{심볼 → [:arg i ?primitive-class] | [:local slot ?primitive-class] | [:this] | [:field name],
 :self-ctype  <이 클래스 Type>,
 :var-fields  <atom {Var → "VAR_n"}>,
 :const-fields <atom {복합const값 → "CONST_n"}>,
 :recur-target {:label :targets}}
```

### 14.4 값/호출 규약
- 일반 값은 Object 참조(박싱)로 반환한다. 단, typed locals 가 켜진 직접 emit 경로에서는
  함수 인자 coercion, let 지역 슬롯, loop/recur target 이 long/double/boolean primitive
  slot 을 쓸 수 있고, Object 위치로 나올 때만 box 한다.
- static-call/instance-call/new: 인자를 Object 로 올린 뒤 파라미터 타입으로 coerce
  (primitive→`unbox`, ref→`checkCast`). 반환 primitive→`box`, void→nil(ACONST_NULL).
- 메서드/생성자 디스크립터: analyzer 정적 타입(`:tag`/`:o-tag`)으로 후보를 점수화하고
  리플렉션으로 정확히 확보. 정적으로 안전하지 않은 모호 호출은 host fallback.

### 14.5 캐싱 (성능 — host 와 동일 방식)
- **var**: static field `VAR_n`, `<clinit>` 에서 `RT.var(ns,name)` 1회, 호출 시 getstatic+deref.
- **const(복합)**: static field `CONST_n`, `<clinit>` 에서 1회 생성(`emit-const-value`),
  사용 시 getstatic. 값 기준 dedup. primitive const 는 캐싱 안 함(ldc 충분).
- 효과: map 2.98→1.21, 전반 벤치 host급 이상(§14.8).

### 14.6 bytecode 생성 세부
- `ClassWriter(COMPUTE_FRAMES|COMPUTE_MAXS)`. **getCommonSuperClass override**: gen 클래스
  쌍은 `Object` 반환 — COMPUTE_FRAMES 가 분기 merge 시 `Class.forName` 으로 우리 클래스를
  찾으려다 실패(`*dcl*` 에만 존재)하는 깊은 중첩 ClassNotFound 회피.
- `GeneratorAdapter` 로 메서드 emit.
- `defineClass` 한 클래스는 컴파일 단위별 `kept-class-units` bounded cache 로
  강참조해 SoftReference GC 를 회피하면서 전역 무한 누적은 막는다.

### 14.7 host fallback (§13)
- `emit-node` 가 미구현 op → `ex-info {:type :unsupported-op}` throw.
- `compile-form` 이 catch → `(eval form)` 으로 form 전체 host 위임 → Clojure 기능 손실 0.
- fallback 트리거(코드 위치): emit-node default / `:ns` require/import side-effect /
  `:deftype`·general `:reify` 등 직접 emit 미지원 op / emit-const-value 미지원값.
  단순 `(ns foo)` compile-ns 는 `:direct-simple` 로 host eval 없이 namespace 를 준비하고,
  simple `reify Object/toString` 은 anonymous class bytecode 로 직접 emit 한다.

### 14.8 self-host (축 A) + 성능 정책 결과
- self-host: 결정적 클래스명 + gensym-free emit → 같은 소스 같은 bytecode.
  M5a: 작은 프로그램 stage1..7 반복 `compile-classes` → byte-동일(고정점).
  M5b: mini-eval(우리 부분집합 평가기)을 우리 컴파일러로 컴파일 → host≡compiled.
- 성능(`clojure -M:bench`, ratio=ours/host, <1=우리가 빠름):
  `*` 0.96 / factorial 0.69 / untyped loop 0.73 / typed loop 1.14 /
  mixed variadic 0.98 / map+캡처 1.08. → 전반 host급, typed loop 는
  verifier-level witness 와 더 좁은 primitive path 를 다음 슬라이스에서 본다.

---

## 15. stage7 완성까지 — 미구현 전수 + 우선순위 (싹 다)

### A. 직접 emit 으로 끌어올 후보 (성능 정책: 구현 후 측정, 빠르면 채택)
- [x] **`:try` 의 `:finally`** — finally region(정상/catch/uncaught 모든 경로 실행).
- [x] **`:set!`** — `Var.set` / 인스턴스·static field set(putfield/putstatic).
- [x] **dynamic var binding(`binding`)** — macro expansion + try/finally 경로.
- [x] **variadic + 다중 fixed arity 혼합 fn** — RestFn + 추가 fixed invoke 메서드.
- [x] **`:letfn` mutual recursion** — generated fn class + mutable capture field patching.
- [x] **simple `:reify` Object/interface/capture method** — anonymous class +
      ctor/capture fields + reflected/analyzer method descriptor 직접 emit.
      general reify/IObj/meta semantics 는 held.
- [x] **primitive 경로(long/double/boolean)** — Numbers 정적 arg overload,
      함수 인자 coercion, let 지역 슬롯, loop/recur target primitive slot 1차 완료.
      남음: verifier-level instruction witness 와 더 넓은 primitive kind 확대.

### B. host 유지 결정 (성능/복잡도상 host 가 옳음 — 직접 emit 안 함, fallback 으로 동작)
- `:deftype`/general `:reify`(JVM 타입 생성은 host 가 안전·최적) ·
  `:import`/`:ns`(부수효과). → Clojure 기능 손실 0.
  `:case`, `:letfn`, simple `reify Object` method 는 직접 emit 완료.

### C. self-host / mirror (주 경로)
- [x] **3b-5 완전 메타순환 tower** — `full-eval-src` 다중-binding let hygiene 수정.
      if/fn/named-fn/let self-interp 모두 host≡compiled≡tower.
- [x] **M7 (PARKED)** — 재현빌드/meta receipt 의 pnix-clj 런처 소비는 완전한
      meta-circular stage15/N Clojure compiler 전까지 금지. 현재 clj-meta receipt 들은
      `:not-consumed-by "pnix-clj launcher"`/evidence-only 정책으로만 남기고 제품 연결 없음.
- [x] **M8 1차** — mirror IR (analyze 결과를 canonical mirror form 으로; smoke 2/2).

### D. 정확성/견고성/성능 잔여
- [x] 메서드 오버로드 정밀 선택 — Numbers long/double 정적 인자 + 일반 host interop
      static/instance/new 후보 점수화 완료.
- [x] source 위치/line 메타 보존 (stacktrace 정확도).
- [x] map 남은 1.21 — lazy-seq / 안쪽 클로저 미세 오버헤드 (한계효용).
      2026-06-28 `clojure -M:bench` 재측정: map+inner closure ratio 1.08(ours 50ms,
      host 47ms)로 host급 범위. 별도 특수 lowering 은 보류/종료.
- [x] kept-classes 무한 누적 제거 — 컴파일 단위별 bounded cache(`kept-class-units`)로 교체.

### 인터프리터 커널(`kernel.clj`) — 보조 축(별도)
- reference semantics / 3-way 교차검증 / mirror 거울. self-host 주 경로는 위 컴파일러.
- [x] named fn self-ref / loop·recur / 다중 arity / core macro / try·throw /
      기본 interop / `set!` / binding / locking / var / case / letfn 완료.
- [x] kernel custom macro/`defmacro` reference — 커널 env 에 macro table 을 추가하고,
      `defmacro` expansion 을 raw args → expanded form → `k-eval` 경로로 해석.
      `unless` smoke 로 gate 의 kernel smoke 24/24 에 포함.

### compiler.clj self-source stage7 감사 체크리스트(추가)
- [x] `compiler.clj` 전체 top-level source form 을 읽어 analyzer `:op` census 를 만들고,
      직접 emit / host-maintained / fallback 이유를 표로 고정한다.
      현재 192 forms / 32 ops / unknown 0, receipt `self-source-audit.receipt.edn`.
- [x] `clojure -M:audit-self-source` 와 `clojure -M:gate` 에 self-source audit 를
      연결해 accidental fallback op 가 생기면 실패하게 한다.
- [x] compiler 자기소스 내부의 dispatch용 `case` 를 제거해 self-source audit 에서
      host-maintained `:case`/`:case-test`/`:case-then` 을 없앴고, target `:case` 직접 emit
      1차를 추가했다.
- [x] `compile-classes` 타겟을 작은 self-host 예제에서 `compiler.clj` 부분집합 source 로
      확장하고, stage1→7 class bytecode 고정점 비교를 별도 receipt 로 남긴다.
      현재 bytecode-safe direct-subset 191/191 forms, 199 classes fixed-point OK.
- [x] host-maintained op(`ns`, `import`, `deftype`, general `reify`)는
      accidental fallback 과 구분해 ledger 에 명시한다. 현재 compiler.clj 에 실제
      등장하는 host-maintained op 는 `:import` 뿐이다.
- [x] 고정점 타겟에서는 audit 가 표시한 결정적 direct-subset top-level
      191/192만 먼저 사용하고, 남은 1개 host-maintained form(`ns`/`:import`)의
      처리 방침을 별도 stage target ledger 로 고정한다.
- [x] direct-subset 중 bytecode-safe 에서 탈락한 40개 form 을 축소한다.
      현재 direct-subset 탈락 0. 처리한 축: `Class`/`char`/quoted list 상수,
      analyzer `:tag` 우선 타입 추론, 비모호 reference overload, source type hint,
      동적 instance-call fallback.
- [x] full compiler source 를 stage target 으로 올릴 때 `ns`/`:import` host side-effect
      wrapper 를 분리하고, 생성 클래스명/라인메타/const-cache 순서가 stage 간
      byte-identical 인지 검증한다.
- [x] 컴파일된 `compile-fn-class` 실행 경로에서 동적 instance-call fallback 이
      `proxy-super` 의미를 충분히 보존하는지 별도 실행 smoke 를 만든다.
- [x] 컴파일된 compiler API(`compile-form`/`compile-classes`)를 격리된 namespace/artifact 로
      로드해 작은 프로그램을 다시 컴파일하고, host-loaded compiler 와 결과/bytecode 를 비교한다.
- [x] wrapper 함수가 host-loaded compiler API 를 호출하는 smoke 를 넘어, self-source
      stage target 이 생성한 compiler 구현 var(`compile-form`/`compile-classes`) 자체를
      격리 namespace 에 로드하고 그 var 로 작은 프로그램을 다시 컴파일한다.
- [x] 생성된 compiler 구현 var 자체를 다음 stage driver 로 사용해 self-source stage target 을
      stage1→7 반복 컴파일하고, host-driven stage target receipt 와 bytecode 를 비교한다.
- [x] generated stage-driver chain 을 별도 receipt 로 분리하고, gate compact receipt 에
      stages/classes/fixed-point/host-receipt-match 요약을 추가한다.
- [x] `ns`/`:import` host side-effect wrapper 와 161개 bytecode target 을 하나의
      full namespace artifact/load 검증으로 묶어 manual retargeted stage target 경계를 줄인다.
- [x] generated stage-chain 의 각 stage class map 을 `work/` 아래 on-disk jar bundle 로
      쓰고, jarproof-compatible stable digest 로 stage1→7 exact 비교한다.
- [x] generated stage-chain 의 on-disk jar bundle 을 다시 로드해 generated compiler entry 가
      디스크 산출물만으로 재실행되는지 검증한다.
- [x] fresh disk stage8 exact digest 고정점: disk-reloaded stage7 compiler 의 self-source
      recompile class digest 까지 stage7 jar 와 byte-identical 로 맞춘다.
- [x] generated stage-chain disk reload proof 를 별도 receipt/compare report 로 분리한다.
- [x] stage9 clean process compiler-runtime replay: 새 JVM 에서 stage8 artifact 를 로드해
      canonical compiler receipt/payload/artifact digest 를 재현한다.
- [x] stage10 isolated classpath/session/sandbox compiler closure: root-bound cwd-independent +
      locale/timezone/namespace/work-dir/sandbox-cwd matrix 에서 같은 stage9 canonical digest 를 재현한다.
- [x] stage10 hardening: cwd 자체를 sandbox/work dir 로 옮겨도 source path resolver 와
      artifact path 가 같은 canonical digest 를 재현한다.
- [x] stage11 multi-surface compiler adapter closure: source form / mirror IR /
      self-source target / conformance corpus / kernel-evaluator target 의 경계와 receipt schema 를 비교한다.
- [x] stage12 self-improvement quarantine closure: compiler rule/emit/fallback 개선 후보를
      quarantine evidence 로만 남기고, replay/gate/admission 전에는 live truth 로 승격하지 않는다.
- [x] stage13 long-horizon compiler organism closure: 여러 snapshot/day/session label 에서
      stage artifact, stage11/12 digest, compiler verdict 를 재생하고 stale/drift 는 held 로 남긴다.
- [x] stage14 cross-host/cross-implementation law closure: clj-meta canonical transcript 를
      만들고 hy-meta/pnix-hy/pnix-clj transcript 미제공 상태를 held 로 분류한다.
- [x] stage15 open-world evidence federation closure: 외부 solver/proof/repo/document/LLM
      결과를 evidence-only 로 수집하고 gate/admission 전에는 accepted 로 승격하지 않는다.
- [x] stageN recursive closure ladder: 새 host/runtime/proof surface 를 registry 로 추가하고
      stage8~15 closure law 를 반복 적용한다. 미지원 surface 는 accepted 가 아니라 held 다.
- [x] stage10 cwd hardening: cwd 자체를 sandbox/work dir 로 옮겨도 source path resolver 와
      artifact path 가 같은 canonical digest 를 재현한다.
- [x] typed locals primitive 1차: 함수 인자/let 지역 슬롯까지 primitive 로 내리는 경로를
      `compile-form`/`compile-classes` 양쪽에 적용하고 stage7/N artifact receipt 로 고정한다.
- [x] typed loop/recur primitive slots: loop 바인딩과 recur target primitive frame 을
      추가하고 frame merge/bytecode verifier/stage8 raw artifact 재현성을 receipt 로 고정한다.
- [x] primitive bytecode witness/disasm receipt: typed let/loop/recur 산출물의 primitive
      instruction shape 를 normalized witness 로 고정한다.
- [x] raw primitive arithmetic opcode lowering: Clojure `Numbers.*` overflow 의미와 다른
      case 를 분류한 뒤 안전한 typed double 경로만 직접 `dadd`/`dmul`/`dsub` 계열로 내린다.
- [x] explicit unchecked long raw arithmetic lowering: 사용자가 wraparound 의미를 명시한
      `unchecked-add`/`unchecked-subtract`/`unchecked-multiply` typed long 경로만
      `ladd`/`lsub`/`lmul` 로 직접 내린다.
- [x] checked-long static no-overflow proof: 두 인자가 literal long 이고 overflow 없음이
      `Math/*Exact` 로 증명될 때만 checked `+`/`-`/`*` long 에 `ladd`/`lsub`/`lmul` 를 허용한다.
- [x] checked-long let-local range proof: `let` 지역 literal/local/static-call result range 가
      `Math/*Exact` endpoint 검사를 통과할 때만 checked long 산술을 직접 opcode 로 승격한다.
- [x] checked-long invariant-loop range proof: loop 바인딩이 모든 `recur` 에서 같은 local 로
      유지될 때만 init range 를 보존해 checked long 산술을 직접 opcode 로 승격한다.
- [x] checked-long bounded-step changing-loop range proof: top-level `<` guard + then-only
      `(+ i 1)` recur 일 때만 loop index range 를 보존해 checked long 산술을 직접 opcode 로 승격한다.
- [x] checked-long positive non-unit loop stride range proof: top-level `<` guard + then-only
      `(+ i k)` positive literal recur 일 때만 loop index range 를 `[init,bound+k-1]` 로
      보존해 checked long 산술을 직접 opcode 로 승격한다.
- [x] checked-long decreasing loop stride range proof: top-level `>` guard + then-only
      `(- i k)` positive literal recur 일 때만 loop index range 를 `[bound-k+1,init]` 로
      보존해 checked long 산술을 직접 opcode 로 승격한다.
- [x] checked-long constant accumulator recurrence range proof: bounded positive index loop 의
      반복 횟수와 `(+ acc k)`/`(- acc k)` total delta 가 `Math/*Exact` 를 통과할 때만
      accumulator range 를 보존해 checked long 산술을 직접 opcode 로 승격한다.
- [x] checked-long branch-local fn argument guard range proof: `<`/`>` guard 의 then/else
      분기에서 fn argument range 를 좁히고 중첩 guard intersection 이 닫힌 range 를 만들 때만
      checked long 산술을 직접 opcode 로 승격한다.
- [x] checked-long bounded index-accumulator sum recurrence range proof: positive bounded
      index loop 에서 `(+ acc i)`/`(- acc i)` total delta 를 arithmetic-series sum 으로
      exact 계산할 수 있을 때만 checked long 산술을 직접 opcode 로 승격한다.
- [x] checked-long multiplicative accumulator recurrence range proof: positive bounded
      index loop 에서 `(* acc k)` 양수 literal factor 와 non-negative singleton init 의
      `k^iterations` 를 exact 계산할 수 있을 때만 checked long 산술을 직접 opcode 로 승격한다.
- [~] checked-long branch-dependent/mixed-sign recurrence range proof:
      `abstract_octagon.clj`, `linear_ranking.clj`, `m6aj_framework.clj` 로 branch-dependent
      stride/mixed-sign index sum 은 proof input 으로 분류했고 negative factor recurrence 는
      nonlinear held + checked fallback 으로 고정. compiler raw opcode admission 은 아직 없음.
- [x] pnix-clj 런처/receipt 소비는 위 self-source stage15 고정점이 준비될 때까지 계속 PARKED.

---

## 16. Deep-Research 로드맵 — meta-circular stage15/N 완성 (2026-06-28)

`/deep-research` 5각도(abstract interpretation / verified compilation / bootstrapping
trust / Clojure surface emit / JVM bytecode correctness) fan-out → 23 소스 / 111 claim →
25 검증(24 확정, 1 기각) 결과를 로드맵으로 고정한다. 영역 1·2·3 은 강하게 검증됐고,
영역 4·5 는 verified claim 0(1차 소스만 확보) → **별도 후속 research 패스 필요**(§16.6).

### 16.0 한 줄 전략 피벗 (research 결론)
> 지금까지의 **"한 패턴씩(one-pattern-at-a-time)" checked-long range proof** (M6z~M6ai,
> invariant→bounded→stride→accumulator→multiplicative …)는 구조적 청사진일 뿐 *전이된
> 증명*이 아니다. 다음 단계는 (A) ad-hoc 패턴 인식기를 **원리적 abstract-interpretation
> 엔진**(interval+octagon reduced product + widening/narrowing)으로 대체하고, (B) forward
> "lower 결정" 로직을 **a-posteriori verified validator**(translation-validation, CompCert
> Theorem 1 형)로 재구성하며, (C) byte-identical jar 고정점 위에 **Diverse Double-Compiling**
> 신뢰 게이트를 얹는 것이다. M6aj(branch-dependent/mixed-sign)는 **마지막 ad-hoc 패턴**으로
> 두고, 그 다음부터 프레임워크로 전환한다.

근거(핵심 검증 claim):
- AbsIntIO(AsiaCCS'23): overflow 부재 증명은 **sound one-sided "not-may" 분석**(false
  negative rate 0; superset over-approximation). → "증명되면 raw opcode, 아니면 checked 유지"
  의 정식 형태. [src1]
- Astrée(DASIA-2009/ESOP'05): interval+octagon **reduced product** + 도메인 추가로 filters/
  integrators/recurrence idiom 을 패턴 인식 없이 흡수, Airbus 코드 false alarm 0. [src2,src3]
- Bradley-Manna-Sipma(CONCUR'05): **linear ranking function + supporting linear invariant**
  합성이 integer linear loop 클래스에 완전, **Presburger 로 환원(결정가능)**. invariant/
  bounded/stride/accumulator 케이스를 한 프레임워크로 포섭. [src4]
- CompCert(Leroy'09): 'verified' = **semantic preservation 기계검증**. heuristic 은
  **translation validation**(untrusted pass → Coq-proved validator 통과 시만 채택, 아니면
  abort/fallback)으로 게이트. **Theorem 1**: verified-validator+unverified-pass = fully
  verified compiler 와 동급 보증. [src5,src6,src7]
- CakeML(CPP'17): (a) 정확성을 **실제 emit 바이트**까지 내림(AST-level 모델로는 못 잡는
  silent encoding error 배제) (b) target-specific 의무를 **하나의 named property**
  (`asm_to_target_correct`) 뒤로 factor (c) **emit-or-refuse**(증명 가능한 코드만 내고
  아니면 명시적 CompileError, 절대 unproven 코드 silent emit 금지). [src8]
- Wheeler DDC + Thompson "Trusting Trust": byte-identical 고정점은 **reproducibility/
  determinism** 증명일 뿐 self-perpetuating backdoor 방어 아님. **Diverse Double-Compiling**
  (다른/신뢰 컴파일러로 재컴파일 후 bit-for-bit 비교)이 Trusting Trust gap 을 닫는다(기계검증
  Prover9/ACL2 증명 존재). [src9,src10]

---

### 16.1 영역① checked-long range proof → 원리적 abstract-interpretation 엔진 (P0 핵심)

**이론**: lattice 위 monotone transfer function + Galois connection; over-approximation 이라
proof on abstract superset 가 모든 concrete 실행으로 전이(Cousot-Cousot 1977). domain:
non-relational **interval** ⊑ relational **octagon**(±x±y≤c) ⊑ **polyhedra**(일반 선형) +
**congruence**(a·ℤ+b). 정밀/비용 tradeoff: octagon O(n²)·O(n³) vs polyhedra 지수. recurrence
는 **widening ▽**(상승열 종료) → **narrowing △**(정밀화 회복).

- [~] **16.1a — interval lattice + transfer function 1차**: `abstract-interval.clj` 로
      `bottom`/`top`, `join`/`meet`, `+`/`-`/`*` endpoint transfer, `Var→Interval`
      environment, overflow→`top` conservative fallback, gate receipt 를 구현. 남음:
      actual compiler raw opcode admission 을 이 transfer function/validator 경로로 이식.
- [~] **16.1b — fixpoint + widening/narrowing**: `abstract-interval.clj` 에 loop-head
      widening + guard narrowing smoke 를 추가해 finite counter range 를 회복하는 receipt 를
      만들었다. `range-migration.clj` 로 M6z~M6ai checked-long range ledger 를
      `abstract-interval` API 위에서 재현하는 receipt 를 gate 에 추가했다. 남음:
      loop body CFG 에 연결하고 M6aj 관계 케이스를 M9b 에서 흡수.
- [~] **16.1c — octagon domain(관계 도메인)**: `abstract_octagon.clj` 로
      JVM-native 최소 octagon(`±x±y≤c`) witness 를 gate 에 연결. interval-only 로는
      잃는 `acc+i=0` mixed-sign relation 과 branch-dependent `1≤next-i≤2`
      stride relation 을 회복하고, `x-y≤a ∧ y-z≤b ⇒ x-z≤a+b` transitive
      closure witness 를 추가했다. 비선형 negative-factor recurrence 는 held 로 남김.
      남음: compiler CFG/VC 쪽으로 연결해 M6aj 흡수.
- [~] **16.1d — linear ranking function 합성(BMS)**: `linear_ranking.clj` 로
      bounded affine template(`c + Σ ai*xi`) synthesis witness 를 gate 에 연결.
      increasing/decreasing counter 와 branch-dependent stride(`i'=i+1|i+2`)에 대해
      ranking 후보를 합성하고, non-decreasing sentinel 은 held 로 남김. 남음:
      Presburger/SMT 급 unbounded checker 와 accumulator total-delta exact bound 연결.
- [x] **16.1e — M6aj 를 프레임워크로 흡수 (compiler admission 연결 완료, 2026-06-29)**:
      §16.1a~d 엔진을 *별도 ad-hoc 패턴 없이* compiler raw-opcode admission 에 실제 연결했다.
      `compiler.clj` `loop-ai-ranges`(emit-loop 마지막 fallback)가:
      (1) **branch-dependent stride** → abstract_interval 엔진(interval lattice +
      Cousot-Cousot widening/narrowing fixpoint, guard narrowing)으로 sound finite range
      (i∈[0,11]) 도출 → validator 승인 → raw `ladd/lsub/lmul`.
      (2) **mixed-sign index sum** → 정수 선형형(linform) 위의 보존 선형량 acc+i=0
      (관계 도메인 최소 구현) 추론으로 acc∈[-10,0] 도출 → validator 승인 → raw `lmul`.
      (3) **negative-factor recurrence** → bounded geometric acceleration 으로
      |init|·|factor|^N magnitude 를 exact 계산해 symmetric range acc∈[-32,32] (부호 교번
      sound) 도출 → validator 승인 → raw `ladd/lmul`.
      (4) **non-constant-factor nonlinear(acc*acc 이중지수)** → bounded interval unrolling 으로
      known iteration count N(≤256)만큼 interval transfer 를 펼쳐 각 step join → sound finite
      range(acc∈[2,65536]) 도출 → validator 승인 → raw `lmul/ladd`. 선형/기하/nonlinear 을
      통일적으로 덮는 가장 일반적 기법.
      recognizer 가 우선이라 회귀 0, 모든 승격은 tv/lowering-sound? validator 통과 필수,
      각 케이스에 overflow soundness sentinel(잘못 승격 시 wraparound 로 깨짐) + bytecode
      witness. `m6aj_framework`: 네 케이스 모두 accepted(bytecode 검증), 단 iteration count 가
      정적으로 미지(param bound)인 nonlinear 만 `general-nonlinear-recurrence` 로 정직하게 held
      (어떤 도메인으로도 long 안에서 bound 불가).
      `lowering_admission` 일관. compiler smoke 113/113, conformance 100/100, gate READY.
- [x] **16.1f — bytecode witness 회귀**: M9 `range-migration` receipt 로 M6z~M6ai
      range decision replay 를 고정했고, `lowering_admission.clj` 로 M10 VC digest 와
      primitive bytecode witness opcode/evidence 를 한 receipt 로 교차검증. accepted VC 는
      matching bytecode evidence 가 있어야 하고, rejected VC 는 fallback/forbidden opcode 로만
      남는다. M6aj relation/ranking proof-input 은 bytecode witness 가 없으므로 held 로 고정.

### 16.2 영역② proven vs heuristic 게이트 = translation validation (P0)

**핵심 caveat(research)**: 현재 range proof 는 "lower 할지 forward 로 결정"하는 인식기지
**a-posteriori verified validator 가 아니다** → CompCert Theorem 1 의 보증이 *아직 전이되지
않음*. 이걸 validator 구조로 바꿔야 "증명을 믿는다"가 성립.

- [x] **16.2a — overflow-freedom VC(verification condition) 분리**: `translation_validation.clj`
      로 lowering 후보마다 명시적 overflow-freedom VC 를 만든다. VC 는 input interval,
      raw opcode, claimed result, digest 를 포함한다. `compiler.clj` 의 checked-long raw emit
      후보도 같은 VC 를 생성해 admission 에 사용한다.
- [x] **16.2b — a-posteriori validator**: untrusted candidate → independent checker
      (`validate-vc`) → finite interval + opcode/result 일치일 때만 accepted. overflow/opcode
      mismatch sentinel 은 rejected/fallback. M6aj octagon/ranking proof-input 은
      `lowering_admission.clj` 에서 held-admission 으로 소비하되 accepted 로 승격하지 않는다.
- [x] **16.2c — emit-or-refuse 불변식(CakeML lesson c)**: translation-validation receipt 에서
      rejected row 는 모두 fallback 으로만 남는 invariant 를 gate 에 연결했고, compiler
      checked-long overflow sentinel 은 `LADD` 금지 + `Numbers.add (JJ)J` fallback 으로
      bytecode witness 에 고정.
- [x] **16.2d — named correctness property(CakeML lesson b)**: `lowering-sound?` 단일 술어를
      추가해 opcode/result/finite proof 를 한 곳에 모았다. `compiler.clj` 의 checked-long
      `add/minus/multiply` raw opcode admission 은 이 술어를 통과해야만 `LADD/LSUB/LMUL`
      을 선택한다.
- [x] **16.2e — translation-validation receipt**: validator 통과/거부 카운트 + VC digest 를
      `proof/translation-validation.receipt.edn` 으로 생성하고 gate 에 연결.

### 16.3 영역③ self-host 신뢰 — Diverse Double-Compiling (P1, 개념상 필수)

**핵심 caveat(research)**: stage1→N **byte-identical jar digest 고정점은 determinism/
reproducibility 증명일 뿐 Trusting Trust(self-perpetuating backdoor) 방어가 아니다.**

- [x] **16.3a — diverse 2차 컴파일러 + cross-host independent-toolchain 증거(2026-06-29)**:
      `diverse_double_compile.clj` 로 host `eval`/Compiler 와 clj-meta backend 의 behavior
      replay + full-source self-compiler transcript + compile-ns transcript 를 accepted
      evidence 로 편입. **추가: `cross_host_ddc.clj`(`:cross-host-ddc`) — 우리 backend 를
      서로 다른 독립 clojure.lang.Compiler 버전(1.11.1, 1.12.0)이 호스팅했을 때 고정 target
      bytecode 가 BIT-IDENTICAL(`8741167b…`)** 임을 subprocess 로 검증. emit 결정성으로
      host 버전과 무관 → 한 버전 Compiler 에만 Trusting-Trust 백도어가 있으면 출력이 달라지므로
      cross-host emit determinism = 부분 Trusting-Trust 증거. DDC receipt 에 `cross-host-emit-ddc`
      accepted row + trust-gap ledger(:partial) 로 편입. subprocess/Maven 의존이라 evidence-only
      레인(main gate 안 막음). 남음(정직 held): full Wheeler compiler-binary DDC(host Compiler 와
      bit-identical 한 compiler artifact 를 내는 *완전 독립* compiler) → `bit-identical-artifact-ddc`
      held 유지(`:required-before-closed [fully-independent-compiler-binary-transcript …]`).
- [x] **16.3b — DDC 비교 게이트**: behavior-equivalence + clj-meta backend artifact digest 를
      receipt 로 고정했다. M12 `:case` table-switch/lookup-switch fixtures 와 `:letfn`
      mutual-recursion fixture, simple `reify Object` method fixture, `compile-ns`
      direct-simple transcript 는 backend artifact 를 가진 accepted 로 승격.
      bit-identical artifact DDC 는 두 backend target 이 달라 held 로 명시.
- [x] **16.3c — TCB 명시**: JVM, DynamicClassLoader, tools.analyzer.jvm, host reader/core,
      host Compiler reference, clj-meta backend 를 trust-base ledger 로 receipt 에 고정.
- [x] **16.3d — full-source 진짜 stage1 (달성, self-source 기준)**: `full_source_stage1.clj`
      가 이제 "host clojure.lang.Compiler fallback" 과 "host runtime-lib namespace side-effect
      (§13)" 를 분리해 회계한다. `:import` op 직접 emit + `compile-ns :direct-compiled` 로
      compiler.clj 의 단일 `ns` form 도 backend 가 3 classes 로 컴파일됨을 audit 가 witness
      한다(`ns-side-effect-backend-compiled?=true`). 결과: `host-compiler-fallback-forms=0`,
      `fallback-free-genuine-stage1=accepted`. require/import/in-ns 의 runtime side-effect 는
      host runtime-lib 위임(§13, clojure.core 위임과 동급)으로 fallback 이 아니다.
      남음: deftype/general reify 직접 emit 은 §16.4c 에 host-maintained 로 유지(compiler.clj
      에는 등장하지 않으므로 self-source genuine stage1 에는 영향 없음).
- [x] **16.3e — reproducible-build 레인과 통합 재검토**: `reproducible_ddc.clj` 로
      stock Clojure 재빌드 레인(`stage7-gate.sh`)의 `stage-chain.receipt.edn`/fixed-point
      proof/smoke 산출물을 읽어 DDC 의 독립 toolchain evidence 로 재해석했다.
      trust/status 는 evidence-only, pnix-clj 런처 소비/bit-identical DDC admission 은 금지.
      digest: `c109342608b4214b0ba75069382df8dcc103d29220dad264f147f408beabe5cd`.

### 16.4 영역④ 언어 표면 완성 — host-fallback op 직접 emit (P1)

**research gap**: 이 영역 verified claim 0. **1차 소스 = `tools.emitter.jvm/emit.clj`**
(우리와 *동일한* tools.analyzer.jvm AST 를 소비하는 Clojure-written emitter) + `clojure.lang.
Compiler.java` + `core_deftype.clj`. → 직접 emit 의 reference 구현으로 정독·차용. [src14,src15,src16]

- [x] **16.4a — `:case`(case*) 직접 emit**: analyzer 가 만든 compact `case*` key 를
      사용해 dense case 는 `visitTableSwitchInsn`, sparse case 는
      `visitLookupSwitchInsn` 으로 직접 emit 한다. `:int` 는 non-number/char 입력을
      default 로 보낸 뒤 `RT.intCast` key 를 쓰고, `:hash-identity`/`:hash-equiv` 는
      `Util.hash` + analyzer shift/mask 를 쓴다. 모든 bucket 은 `Util.equiv(test,literal)`
      guard 를 통과해야 accepted. compiler smoke 106/106, conformance 100/100,
      bytecode witness digest:
      `3afececc716f137b172b43557cd603a822c599dd2120c9ac54c6b52f9027bf97`.
- [x] **16.4b — `:letfn` 직접 emit**: mutual recursion `:letfn` 을 generated fn
      class + mutable capture field 2-phase 로 직접 emit 한다. 각 fn instance 를 먼저
      local slot 에 저장하고, 모든 instance 생성 후 peer capture 를 `putfield` 로 채워
      cyclic reference 를 닫는다. `language_surface.clj` 에서 accepted, bytecode witness 는
      `getfield`/`putfield`/`invokeinterface` 를 증거로 고정. compiler smoke 106/106.
- [x] **16.4c — `:reify` + `:deftype` 직접 emit 완성; `:defrecord`/protocol 정의는 held**:
      compiler backend 가 reify/deftype 를 named/anonymous class bytecode 로 직접 emit(host
      Compiler 0회). 
      **reify**: simple `Object/Callable`/capture + general reify = host 처럼 IObj 자동 구현
      (`__meta` 필드 + `IMeta.meta()` + `IObj.withMeta(m)` 복사본 생성자 auto-emit). 리뷰 발견:
      host reify 는 사용자 meta/withMeta 거부 → 진짜 general reify = auto-IObj. 
      **deftype(2026-06-29, 이전 held 평가 뒤집음)**: `emit-deftype-class` 가 deftype* 의 named
      클래스를 emit(선언 필드 + 생성자 + 사용자 메서드; 필드는 `:local :field`→[:field name] env
      로 reify 기계 재사용). 핵심 통찰: analyzer 가 분석-시점에 만든 stub 은 sibling loader 에
      있고, 우리 full 클래스를 `*dcl*` 에 정의하면 findLoadedClass 가 우리 것을 먼저 반환 →
      factory(`new Name`)/`instance?` 가 우리 클래스로 resolve(dual-class 안 깨짐, 이전 우려 해소).
      single-unit + cross-form(compile-ns) 모두 host≡backend. 버그픽스: :class-name=Class 라
      `.getName`, primitive 반환 심볼(int/long)→Class, compile-form 이 analyzer 실패 시 host
      eval fallback. `language_surface` accepted row `deftype-direct`(named class bytecode 확인),
      smoke 126/126, conformance 100/100, verifier OK.
      **defrecord(2026-06-29, 추가로 뒤집음)**: 막던 `set! :local`(mutable __hash 캐시)을 지원하고
      defrecord 클래스 구조를 emit 한다 — typed 필드(:tag; Object/int) + mutable(non-final) +
      full 생성자 typed params + **record multi-constructor 규약**((nfld-2)-arg `__hash/__hasheq=0`,
      (nfld-4)-arg `__meta/__extmap=null` 생성자 추가해 factory(->R 2-arg)/withMeta(4-arg) resolve).
      `emit-set! :local`→putfield, `emit-local :field`→primitive getfield+box. host≡backend
      (field/등가/assoc/keys/into-map/hash/count; raw record cross-compilation 클래스 정체성은
      값 반환 src 로 회피). `language_surface` `defrecord-direct` accepted.
      **held(정직): `defprotocol`/`defmulti` 정의만** (Var/method-table side-effect). 기능 손실 0.
      즉 reify(simple+IObj)/deftype(immutable+mutable)/defrecord 모두 직접 emit.
      require/import `ns` `:direct-compiled` accepted row 도 포함.
- [x] **16.4d — `:protocol-invoke` + `defmulti` 직접 emit; `defprotocol` 부분(2026-06-29)** —
      `:protocol-invoke`(coll-reduce 등) accepted. **`defmulti`/`defmethod` 완전 직접 emit**:
      Var 상수(`#'global-hierarchy` → RT.var) 지원 + fn base 를 AFunction(__methodImplCache)으로
      바꿔 protocol/multimethod fn 호환 + compile-ns incremental(전방참조). host(load-string)≡
      backend [:C :D], compile-classes fallback 없음. `language_surface` `defmulti-direct` accepted.
      **`defprotocol` 완전 직접 emit(2026-06-29)**: 정의 + extend-protocol + **reify-구현** +
      dispatch 전부 host≡backend("yo"/5, compile-classes fallback 없음) → `language_surface`
      `defprotocol-direct` accepted. 마지막 갭(:on parity)을 const quote-strip 으로 해소:
      tools.analyzer.jvm 가 code-position 컬렉션 `{:k 'sym}` 을 const-fold 할 때 값에 quote 를
      한 겹 더 남기는 quirk(`{:on 'foo}`→`{:on (quote foo)}`, host `{:on foo}`)를, emit-const 가
      `:form` 으로 code-literal/quoted-data 를 구별해 code-literal 이면 quote 한 겹을 구조적
      strip(eval 없이, ambiguity 없음). {:k 'sym} 일반 정확성도 수정.
      **→ language surface held 0**: reify(simple+IObj)/deftype/defrecord/defmulti/defprotocol/
      protocol-invoke/ns 전부 직접 emit.
- [x] **16.4e — `:ns`/`:import`/`:require` side-effect (직접 emit 완료)**: `compile-ns` 는
      단순 `(ns foo)` 를 `:direct-simple` 로, require/import clause 가 있는 ns form 을
      `:direct-compiled` 로 처리한다. 후자도 host clojure.lang.Compiler 0회로 backend 가
      컴파일한다: `:import` op 을 `RT/CURRENT_NS` deref → `Namespace.importClass`∘
      `RT.classForNameNonLoading` (host `ImportExpr` 동일 bytecode)로 직접 emit 하고,
      in-ns/refer/require 는 analyzer 가 `:invoke`/`:do`/`:if` 로 확장해 직접 emit 한다.
      runtime side-effect(클래스 등록/네임스페이스 로드)만 host runtime-lib 위임(§13).
      `compile-fn-strict` 가 eval fallback 없이 컴파일해 0 host Compiler 를 보장한다.
- [~] **16.4f — self-source op census 갱신**: `:case`/`:letfn` 직접 emit 후
      `clojure -M:audit-self-source` 기준 source 192/192, stage target 191/191,
      full-source accounted 192/192, classes 199, host-maintained ledger 는 `:import`
      만 남음 확인. `:case`/`:letfn`/simple Object/interface `:reify` 는 direct emit,
      `:case-test`/`:case-then` 은 direct-helper 로 이동. 남음: `:deftype`/general
      `:reify` 직접 emit 후 같은 census 재회귀.

### 16.5 영역⑤ JVM bytecode 정확성 — verifier/stackmap witness (P1)

**research gap**: verified claim 0. **1차 소스 = ASM developer guide / ClassWriter javadoc /
Java7 stackmap 설계 글**. 우리는 이미 COMPUTE_FRAMES + getCommonSuperClass override 로 깊은
중첩 ClassNotFound 를 우회 중 — 이를 정식 근거로 고정. [src17,src18,src19]

- [x] **16.5a — stackmap frame 정합성 witness**: `bytecode_verifier.clj` 로 generated class
      bytes 를 `ClassReader` 로 parse 하고 fresh `DynamicClassLoader` 에 define 한 뒤,
      ASM util `CheckClassAdapter.verify(ClassReader, ClassLoader, ...)` 를 같은 loader 로
      실행한다. literal / primitive long loop / try-catch-finally / closure capture fixtures
      및 simple reify Object/interface method fixtures 모두 full verifier +
      instantiate/invoke OK 로 gate 에 연결.
- [x] **16.5b — COMPUTE_FRAMES vs 수동 frame 결정 문서화**: receipt 에
      `COMPUTE_FRAMES|COMPUTE_MAXS`, fresh loader define, `CheckClassAdapter` full verify
      경계를 명시. `getCommonSuperClass` override 근거는 §14.6/§16.5 policy 로 고정.
- [x] **16.5c — emit-or-refuse 와 결합**: `bytecode_verifier.clj` 의
      `verify-classes` API 와 `verified_compile.clj` receipt 를 gate 에 추가했다.
      `compile-classes-verified` 는 ClassReader/fresh loader/CheckClassAdapter 를 모두
      통과한 class bundle 만 accepted 로 반환하고, invalid class bundle sentinel 은
      `:verifier-rejected` hard fail/held 로 고정한다. 기본 compiler emit 은 그대로 두되,
      stage15 artifact publish API 는 verifier reject 산출물을 정상 산출물로 내보내지 않는다.
- [x] **16.5d — primitive descriptor witness 확대**: typed primitive loop, try/catch/finally,
      closure capture, simple reify Object/interface fixtures 가 ClassReader +
      CheckClassAdapter + fresh loader invoke 를 통과함을 고정했다. 남음 artifact-law 결합은
      16.5c/compiler admission wiring 에 포함.

---

### 16.6 후속 research / 미해결 (research 가 남긴 open question)
- [x] **영역④·⑤ 전용 deep-research 후속 패스**: 1차 소스 재확인 완료.
      `tools.emitter.jvm/emit.clj` 는 analyzer AST 를 `defmulti -emit`으로 처리하며
      `:letfn` 전용 emit 경로(`emit-binds`/closed-over 필드 주입)를 갖는다. Clojure
      `Compiler.java` parser table 은 `CASE`/`LETFN`/`IMPORT`/`DEFTYPE`/`REIFY` 를
      별도 parser 로 둔다. `core_deftype.clj` 의 interface/type 계열은 gen-interface/import
      류 namespace side-effect 와 얽힌다. ASM `ClassWriter`/`CheckClassAdapter.verify`
      문서는 현재 `COMPUTE_FRAMES` + custom loader verify 경로와 일치. 결론:
      `:letfn` 은 emitter.jvm 구조 차용 후보, simple `:reify` Object/interface/capture method 는
      최소 anonymous class generator 로 직접 emit 가능하다고 확인했다. `:deftype` 과
      general `:reify` 는 새 type generator라 host-maintained boundary 유지가 정직하다.
- [~] **abstract domain 구현 선택**: 1차는 `abstract_octagon.clj` 의 JVM-native 최소
      octagon 으로 선택. Apron/polyhedra 는 비용과 native dependency 때문에 보류하고,
      §16.1c 의 두 변수 관계 케이스만 gate receipt 로 실측. 남음: BMS/ranking 및
      M6aj compiler admission 연결 전 정밀도/비용 재평가.
- [~] **CompCert-style validator 실현성**: interval validator(`translation_validation.clj`)
      + compiler admission wiring + `lowering_admission.clj` bytecode cross-witness 로
      checked-long→raw lowering 의 local Theorem-1형 구조를 1차 구현. 남음: validator 자체의
      기계검증 soundness(CoQ/HOL급) 또는 독립 Presburger/SMT checker 검토.
- [x] **현실적 DDC 정의**: `diverse_double_compile.clj` receipt 에 trust-gap ledger 를
      추가해 두 backend(우리 emit vs host Compiler)의 bit-identical 불가를 held 로 고정.
      behavior-equivalence fixture 와 backend artifact digest 가 닫는 범위는 partial 로,
      Trusting Trust/source-executable correspondence gap 은 independent transcript 전까지
      닫히지 않는다고 명시.

### 16.7 새 마일스톤 (M9~M14) — §16 로드맵 대응
```text
M9   [완료1차] abstract-interpretation 엔진 1차: interval lattice + transfer +
             widening/narrowing receipt 를 gate 에 연결. 다음은 compiler range ledger 를
             이 엔진으로 이식해 M6ab~M6ai 결정을 재현.
             abstract interval digest:
             `8e55c384de91086edb0f40dda8dc52255c1855f7e02744a471c9505093d41e57`.
M9c  [완료1차] range ledger migration witness: M6z~M6ai checked-long direct-lowering
             range ledger 를 `abstract-interval` API 로 재현하고, overflow sentinel 은
             `top`/fallback 으로 유지. compiler admission 은 M10 에서 validator 경유로 연결.
             range migration digest:
             `c750edb4f86d097bcb0ab3b0e50e4499ffc84f99e17189dc0ca2c825b7a0e644`.
M9a  [완료1차] octagon 관계 도메인: mixed-sign sum, branch-dependent stride,
             transitive constraint closure, nonlinear held boundary 를 gate 에 연결.
             abstract octagon digest:
             `2a6b8ec7dabaee5f70bfd7f812c836d696384aa5c4d97269d9f4c13988a8fd5a`.
M9b  [완료]   M6aj 흡수 = compiler raw-opcode admission 에 엔진 실제 연결(2026-06-29):
             네 케이스 모두 sound 흡수 — branch-dependent stride(interval 엔진
             widening/narrowing fixpoint), mixed-sign sum(보존 선형량 linform 관계 도메인),
             negative-factor(bounded geometric acceleration |init|·|factor|^N), non-constant-factor
             nonlinear acc*acc(bounded interval unrolling, known N) → validator 승인 시 raw
             `ladd/lsub/lmul`. iteration count 미지(param bound)의 nonlinear 만 held + Numbers
             fallback. 별도 ad-hoc 패턴 추가 없음, 각 케이스 soundness sentinel + bytecode
             witness 로 검증. m6aj-framework digest:
             `6eb83dc3796523aa9bfede26dab79f36fbc475c67f96eed19dba8f757e897e33`,
             lowering-admission digest:
             `465c2307b0006fcfec56de51017b0113cd04ae007d5ac65485e8db5f7fdbb572`.
M10  [완료]   translation-validation 게이트: overflow-freedom VC + a-posteriori validator +
             emit-or-refuse 불변식 + named correctness property + compiler checked-long
             raw emit admission wiring 을 연결. overflow sentinel 은 `Numbers.add (JJ)J`
             fallback + `LADD` 금지로 bytecode witness 에 고정.
             translation-validation digest:
             `aba6a4915ed00c275f1af9ed6009bc886b000bac8f3c60e4d7cb76895ca13a59`.
             primitive bytecode witness digest(+import-direct case 추가):
             `efc64f58623470875d3802fbe6566ceb69a5d469366b7ca174978752b6d57631`.
             (import* → RT.classForNameNonLoading invokestatic + Namespace.importClass
             invokevirtual opcode 증거 = host ImportExpr 동일 경로, 0 host Compiler.)
             lowering admission digest:
             `b17aaf2c824f64e9733e55273ff51b0e4a0bbc777cd5f4487673fdc43de0c55c`.
M11  [완료1차] Diverse Double-Compiling 신뢰 게이트: host Compiler/eval reference 와
             clj-meta backend behavior-equivalence, backend artifact digest, compiler.clj
             full-source self-compiler transcript evidence, TCB ledger, stock Clojure
             reproducible-build lane evidence, drift sentinel held, bit-identical DDC
             held 를 gate 에 연결. compile-ns direct-simple + require/import(:direct-compiled)
             namespace artifact 의 host(load-string)≡backend behavior-equivalence transcript 포함.
             diverse double compile digest:
             `69be1ea9e60dc33ed7832b5889c2adc86b48940b6039de3dbc841082226e6307`.
             reproducible DDC lane digest:
             `c109342608b4214b0ba75069382df8dcc103d29220dad264f147f408beabe5cd`.
M12  [완료1차] 언어 표면 직접 emit + **compiler.clj self-source fallback-free genuine stage1**:
             `:case` compact hash tableswitch/lookupswitch direct emit + bytecode witness,
             `:letfn` mutual recursion direct emit + cyclic capture field patching,
             simple `reify Object/toString`/`Callable/call`/captured `Callable` anonymous
             class direct emit, 단순 `(ns foo)` direct-simple namespace preparation 완료.
             **`:import` op 직접 emit**(RT/CURRENT_NS deref→Namespace.importClass∘
             classForNameNonLoading; host ImportExpr 와 동일 bytecode)으로 require/import
             clause 가 있는 `ns` form 도 host clojure.lang.Compiler 0회로 backend 컴파일
             (`compile-ns` :direct-compiled, `compile-fn-strict` = no eval fallback).
             self-source audit witness: 단일 `ns` form(idx 0)이 backend 로 3 classes 컴파일
             → `host-compiler-fallback-forms=0`, `ns-side-effect-backend-compiled?=true`,
             `fallback-free-genuine-stage1=accepted`. require/import/in-ns 의 **runtime**
             side-effect 만 host runtime-lib 위임(§13 영구 경계 = clojure.core 위임 동급,
             compiler fallback 아님).
             남음(별도, compiler.clj 에는 미등장): 임의 프로그램의 `deftype`/general reify
             (+protocol/multimethod 정의 side-effect) 직접 emit 은 host-maintained boundary 로
             유지(§16.4c). M12 는 self-source 기준 genuine fallback-free 달성.
M13  [완료]   bytecode verifier/stackmap witness: ClassReader parse + fresh
             DynamicClassLoader define + ASM util CheckClassAdapter full verify +
             instantiate/invoke receipt, verified compiler artifact hard-fail receipt 를
             gate 에 연결.
             bytecode verifier digest:
             `4d2897f9332a6bd1306ec9849e277d54eea466845fa007ef346b97ac36d18670`.
             verified compile digest:
             `ac3eb215c9249cc5dc1420ff4b1cca7759fcc0ce5bedcbd42f754e57b779a965`.
M14  [후속]  영역④·⑤ deep-research 후속 패스 + abstract-domain 구현 선택 (§16.6).
```
우선순위: M9(엔진)·M10(validator)·M12(fallback-free stage1)·M11(DDC)·M13(verifier)·
**M9b(M6aj 흡수 = 엔진을 compiler admission 에 연결)** 모두 완료. 남은 frontier 는
negative-factor nonlinear 도메인, deftype/general reify 직접 emit, independent-toolchain
DDC 로 supervised 작업 필요. pnix-clj 런처 소비는 §M7 그대로 PARKED.

### 16.8 출처 (sources — verified 우선)
```text
[src1]  AbsIntIO, AsiaCCS'23 — not-may overflow 분석(FN rate 0)   dl.acm.org/doi/fullHtml/10.1145/3579856.3582814
[src2]  Astrée, DASIA-2009 — interval+octagon reduced product     astree.ens.fr/papers/DASIA-2009.pdf
[src3]  Cousot et al, ESOP'05 — Astrée 도메인 설계                di.ens.fr/~cousot/publications.www/CousotEtAl-ESOP05.pdf
[src4]  Bradley-Manna-Sipma, CONCUR'05 — integer linear loop      theory.stanford.edu/~arbrad/papers/z.pdf
        ranking function + Presburger 환원
[src5]  Leroy, CompCert backend, arXiv 0902.2137 — semantic       arxiv.org/pdf/0902.2137
        preservation + a-posteriori validator(8.2/10.2) + Thm 1
[src6]  Leroy, CACM 2009 — formal verification of a realistic     cacm.acm.org/research/formal-verification-of-a-realistic-compiler/
        compiler
[src7]  Necula, PLDI'00 — translation validation(sound, ~10% FP)  people.eecs.berkeley.edu/~necula/Papers/tv_pldi00.pdf
[src8]  CakeML, CPP'17(Fox/Tan/Myreen/…) — byte-level 정확성 +    acjf3.github.io/papers/cpp17.pdf
        asm_to_target_correct + emit-or-refuse
[src9]  Wheeler, Diverse Double-Compiling — Trusting Trust 방어    dwheeler.com/trusting-trust/
[src10] Wheeler dissertation(DDC 형식증명, Prover9/ACL2)           dwheeler.com/trusting-trust/dissertation/html/wheeler-trusting-trust-ddc.html
[src11] LLVM Scalar Evolution(SCEV) — nikic                       npopov.com/2023/10/03/LLVM-Scalar-evolution.html
[src12] SCEV & integer overflow(nsw/nuw)                          playingwithpointers.com/blog/scev-integer-overflow.html
[src13] reproducible-builds.org / Guix full-source bootstrap      reproducible-builds.org · guix.gnu.org/en/blog/2023/the-full-source-bootstrap-…
[src14] tools.emitter.jvm/emit.clj — 동일 AST 소비 Clojure emitter github.com/clojure/tools.emitter.jvm/…/jvm/emit.clj   ★직접 emit reference
[src15] clojure.lang.Compiler.java — host emit 내부               github.com/clojure/clojure/blob/master/src/jvm/clojure/lang/Compiler.java
[src16] core_deftype.clj — deftype/reify 매크로 확장              github.com/clojure/clojure/blob/master/src/clj/clojure/core_deftype.clj
[src17] ASM Developer Guide — COMPUTE_FRAMES/getCommonSuperClass  asm.ow2.io/developer-guide.html
[src18] ASM ClassWriter javadoc                                   asm.ow2.io/javadoc/org/objectweb/asm/ClassWriter.html
[src19] Java7 stackmap 설계(verifier 후퇴)                        chrononsystems.com/blog/java-7-design-flaw-…
```
※ ClojureScript self-host(clojurescript.org/guides/self-hosting): "Clojure 로 쓴 컴파일러가
자기 자신을 컴파일" 선례. analyzer/emitter 분리 + bootstrap 참조점.
※ 기각된 claim(투명성): CakeML bootstrap 의 "정확히 3개 HOL4 theorem 합성" 표현은 1-2 로
기각 — in-logic bootstrap *메커니즘*은 유효하나 정확한 theorem 조합 문구는 의존 금지.

## 17. Deep-Research 2회차 — 자가 코드 감사 (2026-06-29)

사용자 지시: "meta-circular stage15~N compiler/evaluator **미구현/잘못/빠진** 것 /deep-research
로 찾아 todo 에 적어라." 방법 = (A) 웹 리서치 워크플로우(deep-research, primary source) +
(B) **4-에이전트 자가 코드 감사**(compiler.clj 를 실제로 돌려 host clojure.lang.Compiler 와
대조 — 모든 항목 경험적 검증, 추측 아님). 아래는 (B) 결과(우리 코드 한정, file:line).
웹 리서치(일반론·1차 소스)는 §17.5 에서 교차검증.

### 17.1 확인된 버그 (miscompile / crash — "잘못된" 항목, P0)
실제 컴파일·실행으로 재현 확인. host 와 결과가 다르거나 죽는다.

```
B1  [P0] BigInt 리터럴 → Long 으로 잘못 emit (타입 오염, silent)
        `(fn [] 5N)` ⇒ java.lang.Long. host ⇒ clojure.lang.BigInt.
        원인: emit-const-value 의 `(integer? v)` 분기가 BigInt/BigInteger 에도 참 →
        `(.push ga (long v))` + box-to-Long. `=` 는 가리지만 class/type/instance?/
        auto-promotion 의미 깨짐. 동일 오분류: arg-class(interop overload 타이핑),
        emit-primitive-const.  위치: compiler.clj:197, :368, :1057.
B2  [P0] 범위초과 BigInt 리터럴 → 컴파일러 HARD CRASH (fallback 도 없음)
        `10000000000000000000N` ⇒ line 197 `(long v)`=RT.longCast 가
        IllegalArgumentException throw. 이건 `:unsupported-op` ExceptionInfo 가 아니라
        compile-form 게이트(2845-2848)가 못 잡음 → eval fallback 도 못 타고 전체 폭발.
        합법 리터럴인데 컴파일러가 죽는다.  위치: compiler.clj:197.
B3  [P0] `^:dynamic` def 가 backend 에서 setDynamic 안 함 → binding 깨짐
        `(def ^:dynamic *x* 1)` 후 `(.isDynamic #'*x*)`=false (host=true),
        `(binding [*x* 2] *x*)` ⇒ IllegalStateException "Can't dynamically bind
        non-dynamic var". 원인: emit-def 가 Var.setDynamic() 호출 안 함(파일 전체에
        setDynamic 0회). analyzer 가 :dynamic 을 var *meta* 에만 넣고, 실제 dynamic
        boolean 플래그는 안 세움. 기존 smoke 는 *print-length*(core 가 이미 dynamic)만
        써서 미검출.  위치: compiler.clj:326-333 (누락).
B4  [P0] case `:skip-check?` 무시 → hash 충돌 시 silent 오답 (soundness)
        `(case "Aa" "Aa" 1 "BB" 2 :other)` ⇒ :other (host=1). "Aa"/"BB" 둘 다
        Util/hash=2112 충돌 → core/case 의 merge-hash-collisions 가 한 switch 엔트리로
        합치고 test 상수를 hash 정수 2112 로 바꾸고 `:skip-check?` 에 넣어 사후 equiv
        체크를 *생략하라* 지시. 우리 emit-case 는 :skip-check? 를 파일 어디서도 참조 안
        함 → bucket 이 value 를 정수 2112 와 Util.equiv 비교(항상 false) → goto default,
        진짜 비교(condp)는 죽은 코드. emit-case 가 throw 안 하니 fallback 도 안 탐 →
        런타임 silent 오답. 문자열/충돌가능 keyword·symbol case 전부 영향.
        위치: emit-case 2090-2107, emit-case-bucket 2077-2088 (:skip-check? 처리 추가).
B5  [P0] defrecord static `create(IPersistentMap)` 누락 → map->Record 깨짐
        `(map->RB {:a 5 :b 6})` ⇒ NoSuchMethodError. emit-deftype-class 가 instance
        메서드만 emit(항상 ACC_PUBLIC, static 메서드 emit 경로 없음). map->R/Record/create
        는 record 의 정식 2대 생성자 중 하나. 위치 생성자 ->RB 는 동작(ctor)이라 기존
        test(라인 3154)가 ->R 만 써서 미검출.  위치: emit-deftype-class 2610-2738.
B6  [P1] `^:volatile-mutable` 필드가 ACC_VOLATILE 잃음 (JMM 가시성 silent 손실)
        field-mutable 을 boolean 으로 뭉갬(2623) → 필드 플래그는 PUBLIC|FINAL vs PUBLIC
        둘 중 하나(2667-2672), :volatile-mutable/:unsynchronized-mutable 구분 안 봄,
        파일 전체 ACC_VOLATILE 0회. volatile-mutable 의 happens-before 보장이 평범한
        non-final 로 깎임 = 동시성 정확성 버그(단일스레드 test 로 미검출).
        위치: compiler.clj:2623, 2667-2672.
B7  [P2] reify 가 IObj/IMeta 를 명시 구현하면 ClassFormatError (중복 meta, crash)
        `(reify clojure.lang.IObj (meta [_] {...}) (withMeta [this _] this))` ⇒
        ClassFormatError "Duplicate method name meta". 자동 IObj(meta/withMeta auto-emit)
        와 사용자 메서드 노드가 둘 다 emit 됨(인터페이스 목록에선 IObj/IMeta 빼지만 메서드
        노드는 남음). emit-reify 에 try/catch 없어 Error 가 :unsupported-op 게이트 통과 →
        fallback 없는 hard crash. 재확인 결과 current host Clojure 도 명시 IObj/IMeta reify 를
        거부하므로, 정확한 목표는 "성공 emit" 이 아니라 host 와 같은 실패 의미로 fail-close.
        위치: 2403, 2518-2537, 2568-2584.
```

### 17.2 직접-emit 갭 (eval fallback — 기능상 정확, "host Compiler 0회" 주장엔 미달, P1)
틀린 건 아니나 우리 backend 가 emit 못 해 form 전체가 host eval 로 떨어진다.

```
G1  BigDecimal(1.5M) / Ratio(1/3) / regex(#"...") / Float 리터럴 → 직접 emit 안 됨
        emit-const-value :else(220-221) → :unsupported-op → 그 리터럴 하나가 enclosing
        top-level form 전체를 eval 로 끌어내림(예: `(fn [^long n] [(+ n 1) (re-find ...)])`
        의 ^long primitive math 까지 통째로 host eval 로 강등). host 는 셋 다 진짜 interned
        상수로 emit(특히 Pattern 은 constant-pool 에 클래스-init 캐시). cacheable-const?
        (223-226)도 이들 제외.  위치: compiler.clj:220-221, 223-226.
G2  protocol(사용자 defprotocol) 구현 deftype → 직접 emit 안 됨(analyzer NPE→host eval)
        Java/clojure.lang 인터페이스 deftype 는 진짜 인터페이스 메서드로 직접 emit 되나,
        사용자 defprotocol 구현 deftype 는 tools.analyzer.jvm 자체가 NPE → form 전체 host
        eval fallback(메서드는 결국 host 가 emit). 위치: compiler.clj:2827 주석, 2830-2832.
G3  gen-class AOT 클래스 생성 미지원(niche) — standalone (gen-class) 은 host 도 no-op(nil),
        의미있는 AOT 는 *compile-files* 필요한데 인메모리 backend 가 안 세움. proxy/
        proxy-super/definterface 는 런타임 위임으로 동작(직접 emit 아님).
```

### 17.3 정직성/주장 갭 (overclaim 위험 — 코드는 정직, 요약 어휘가 과장 P1)
receipt/docstring 은 디테일 수준에서 정직. 위험은 "meta-circular / byte-identical / 직접
emit" 같은 *요약 어휘*가 실제보다 강하게 읽히는 것.

```
H1  StackMapTable 은 ASM(clojure.asm)이 계산, 우리 compiler 아님 (COMPUTE_FRAMES)
        세 클래스 다 COMPUTE_FRAMES|COMPUTE_MAXS(2256/2439/2626) + getCommonSuperClass
        override. opcode 선택은 우리지만, V1_8 검증 핵심인 stackmap 프레임은 ASM 이 합성.
        → "프로그램 bytecode 는 우리가 쓴다"(11-12)는 opcode 엔 참, *프레임엔 거짓*.
        self-host 고정점이 우리 프레임 로직을 한 번도 안 거침(로직이 없음). + host
        clojure.lang.Compiler 와 byte-identical 불가의 한 원인.
H2  고정점 = 결정성 + 자기일관성, *정확성 아님*
        stage1→7 byte-동일은 "결정적 컴파일러의 고정점"일 뿐(틀린-but-결정적 컴파일러도
        고정점 가짐). selfaudit 의 :matches-host-reference? 의 "host-reference"는 host
        clojure.lang.Compiler 가 아니라 *우리 backend 를 소스-로드한 복사본*(selfaudit
        :1252-1253, :1275) → 증명하는 건 self-compiled≡source-loaded(자기일관성)뿐. 유일한
        외부 정확성 오라클은 conformance.clj(host eval≡우리, 100/100)인데 *고정 코퍼스*지
        compiler.clj 자기 224 form 이 아님 → 자기소스는 런타임 의미 differential 검증 안 됨.
H3  프론트엔드(read/macroexpand/analyze) = 외부 tools.analyzer.jvm, self-host 안 됨
        "컴파일러가 자기를 컴파일"은 *host 프론트엔드를 써서 backend 가 backend 를 컴파일*.
        macroexpand 는 host clojure.core 매크로를 끌어옴. selfaudit 는 compiler.clj(backend)만
        감사, analyzer 는 고정점 밖.
H4  locals-clearing 없음 → head-holding/누수 위험 + host divergence
        emit-let(1377-1400)/emit-loop(1720-1760)이 슬롯을 last-use 후 null 처리 안 함.
        host Clojure 는 lazy-seq head 잡힘 방지로 locals 를 clear. 큰/lazy seq head 를 잡은
        local 이 메서드 프레임 내내 reachable → 공간누수 + 의미 divergence.
H5  일반 compile-form/load-compiled-ns API 는 fallback-free 아님
        호스트 eval fallback 4곳(analyze 실패 2832, top-level free var 2838, unsupported-op
        2847, 비-fn top-level 2849). 고정점 receipt 의 "0 host-compiler-fallback"은 curated
        compile-classes subset 경로에서만 참. 부트스트랩의 ns form(idx0)도 실제론 host
        eval(prepare-compiled-impl-ns! → eval, selfaudit:689) — witness 는 backend 가능을
        보였지만 돌아가는 부트스트랩은 host eval 사용.
H6  런타임 전체(clojure.core/clojure.lang) = 신뢰하는 외부 base
        RT.var/RT.vector/Var.deref/AFunction/RestFn/모든 함수호출이 런타임에 clojure.core
        Var 로 resolve. compile-time(opcode)만 자가, 런타임은 0% 자가 + compile-time 도
        ASM 프레임(H1)·host analyzer(H3) 의존. "meta-circular"는 "신뢰 host
        frontend+runtime+ASM 위에서의 backend self-emit 고정점"으로 한정해야 정직.
        (todo §13/§7 에 이미 명시됨 — 요약 어휘만 강화 주의.)
H7  cross-host DDC = host 버전 무관 emit 결정성, Trusting-Trust 아님 (이미 정직 disclaim)
        1.11.1/1.12.0 둘 다 *같은 backend 소스*를 실행 → compiler.clj/tools.analyzer.jvm/
        clojure.asm 의 backdoor 는 양쪽에 동일 존재해 상쇄. compromised-compiler 독립성 0.
        cross_host_ddc.clj:15-16/93-94 가 정확히 disclaim 함(과장 아님). full Wheeler DDC =
        *독립 구현된 두 번째 컴파일러* 필요(여전히 held).
H8  고정점 산출물은 line-stripped(compile-classes *emit-line-numbers* false :2975) ↔
        실행 산출물은 line-numbered(compile-form true :2826) → 고정점이 증명하는 artifact ≠
        실제 돌리는 artifact (둘 다 우리 backend 지만 line table 차이).
```

### 17.4 divergence-by-design / 사소 (감시 대상, P2)
```
D1  raw-long 산술 치환은 range prover 가 sound 일 때만 sound
        host clojure.lang.Compiler 는 +/*/- 를 *항상* checked Numbers.add(long,long) emit.
        우리는 정적 prover(checked-long-static-no-overflow? 662-687, AI 엔진 1454-1718)가
        non-overflow 증명 시 raw LADD/LMUL/LSUB(1149-1168). prover 가 한 번이라도 너무
        좁으면 throw 해야 할 op 가 silent wraparound = miscompile. soundness sentinel
        (3088-3128)이 2차 방어(틀린 승격 시 wraparound 로 깨지게)하나, 적대적 테스트 패스
        필요(범위 도메인 단독으로는 미증명). → §16.1/§16.2 validator 신뢰 경계와 직결.
D2  ^:const 컴파일타임 inlining 없음(emit-def 326-333, emit-var 315-319): deref 로 옳은 값
        반환하나 host 의 const-snapshot 의미와 divergence(이후 root rebind 가 여기선 보임).
D3  protocol-invoke 에 MethodImplCache inline fast-path 없음(344-352): 정확성 OK, perf 만.
D4  emit-finally-body/:do statement 의 1-slot .pop 가정(1796, 2167): Object statement 엔
        OK, primitive 2-slot statement 면 오pop — 전역 가정이라 try 한정 버그 아님.
```

### 17.4b 검증으로 정확 확인된 것 (positive — 회귀 감시 기준선)
try/finally/catch exception-table(finally 가 모든 출구·예외경로서 실행, 1798-1857) /
monitor(locking 매크로→emit-try 로 누수 없음, 1970-1974) / recur 병렬 재바인딩 + across-try
는 frontend 가 거부(1765-1785) / binding push·pop try/finally(매크로전개→emit-try) /
keyword·symbol·var·collection 상수 clinit 1회 intern 캐시(223-266, 2322-2343) / primitive
type-hint coercion(355-404, 1187-1227) / defrecord map-surface 29 메서드(매크로+generic
emit: =/hashCode/hasheq/assoc/count/seq/IRecord/java.util.Map 전부, **create 만 누락=B5**) /
multimethod hierarchy·prefer-method·custom :hierarchy + extend-via-metadata/extend-type/
extend-protocol(런타임 MultiFn/extend 위임) / definterface·proxy·proxy-super(런타임 위임) /
deftype IType(analyzer passthrough). → 이들은 건드리지 말 것.

### 17.5 웹 리서치 교차검증 + 1차 소스 (deep-research 워크플로우 — 완료 2026-06-29)
워크플로우: 6각 fan-out → 27소스 → 131 claim → 25 검증 → **23 confirmed(대부분 3-0 만장일치)
+ 2 refuted**. 전 claim 1차 소스(clojure.org reference, clojure/clojure master, JVMS, OW2 ASM
tracker) 근거. **중요 범위 한계**: 23 confirmed 가 *전부* (1)언어/특수형식 emit·(2)bytecode
정확성에만 해당. (3)meta-circular 진짜성·(4)DDC/Trusting-Trust·(5)abstract-interp soundness
는 **외부 검증 claim 0** — 웹으로 검증 불가(우리 코드/연구 frontier 특수). 즉 §17.3(H1~H8)·
§17.4(D1)·H7 의 (3)(4)(5) 발견은 *내부 감사 근거이며 외부 교차검증 없음* → 별도 targeted
research 필요(open Q below).

#### B8 [P1] — `case` int-switch 가 checked `RT.intCast` 사용 (웹 finding 정밀화 후 확정)
- **웹 claim(3-0)**: host clojure.lang.Compiler 는 int coercion 을 *unchecked-math* 플래그로
  `intCast`/`uncheckedIntCast` 중 컴파일타임 선택(HostExpr.emitUnboxArg, RT.UNCHECKED_MATH.deref()).
  hardcode 하면 overflow 의미 틀림. src grep: unchecked-math/uncheckedIntCast 참조 **0개**.
- **우리 코드 재확인(정밀화)**: 우리 backend 의 유일한 `*Cast` emit 은 `intCast`(:81 정의, **:2129
  `emit-case-int-switch-key` 단 한 곳**) — 일반 primitive-coercion 경로가 아니라 **case :int
  switch 의 dispatch 값→int 변환**. 산술의 *unchecked-math* 는 analyzer inlining 이 이미 처리
  (`unchecked_add/multiply/minus` 노드 → raw opcode, :122/:1203-1239) → 그쪽은 무관. 따라서 웹의
  *광의* 주장(일반 coercion)은 우리에 직접 적용 안 됨(우리는 long/double 유지 + Object boxing,
  일반 int narrowing emit 없음). **웹 claim 을 그대로 베끼지 않고 코드로 검증해 좁힌 사례.**
- **그러나 실제 버그는 따로**: case int-switch 가 `(.instanceOf Number)`/`(.instanceOf Character)`
  통과 후 **checked** `RT.intCast`(=Math.toIntExact) 호출 → dispatch 값이 **int 범위 밖 Number**면
  ArithmeticException **throw**. host case 는 비매칭 값을 **:default 로** 보냄(throw 안 함). 즉
  `(case (long 9999999999) 1 :a :default)` ⇒ 우리=예외, host=:default. **case 는 어떤 dispatch
  값에도 throw 하면 안 됨**(soundness).  위치: compiler.clj:2118-2129. (코드 reading 확정;
  픽스처로 재현 권장.)
- **해결방법(M24)**: case int-switch-key 의 intCast 를 *unchecked* 의미로 — int 범위 밖이면
  매칭 실패(→default)가 되도록 `RT.uncheckedIntCast(Object)` 사용. 절단된 int 가 엉뚱한 bucket
  으로 가도 `emit-case-bucket`(:2077-2088)의 equiv 재확인이 실제 값을 비교하므로 안전(절단
  충돌은 equiv 에서 걸러져 default). 새 상수 `rt-unchecked-intcast-object-method
  (reflect-asm-method RT "uncheckedIntCast" [Object])`.
- **테스트**: `(case (long 0x1FFFFFFFF) 1 :a 2 :b :default)`=:default(throw X); 정상 int 케이스 무회귀.

#### 검증 대기 항목 (웹 open question — 우리 코드 grep 으로 1차 확인)
```
V1 [verify] ASM #317786 backward-jump-widening VerifyError
   조건부 *후방* 점프 offset 이 -32768 미만으로 넓어지면 ASM MethodWriter 가 (역조건+GOTO_W)로
   재작성하며 fall-through 타깃에 stackmap frame 없어 verifier 거부("Expecting a stackmap frame
   at branch target"). 우리는 COMPUTE_FRAMES 라 ASM 이 프레임 계산하지만, vendored clojure.asm
   revision 이 이 버그 fork 면 대형 메서드(긴 loop/case)에서 재현 가능. → 우리 clojure.asm 버전
   확인 + bytecode_verifier 가 긴-후방점프 메서드도 full verify 하는 픽스처 추가. [src25].
V2 [verify] CLJ-2345 catch 타입 Throwable-subclass parse-time 미검증
   stock Clojure 는 `(try … (catch Object o …))` 를 parse-time 에 안 걸러 VerifyError 발생.
   우리 emit-try(:1864 예외를 throwable-type checkCast)가 catch_type 에 사용자 클래스 internal
   name 을 그대로 쓰면 동일 VerifyError 상속. → 같은지 확인/parse-time Throwable 체크 추가 결정.
   [src26].
V3 [verify] 직접-emit defrecord 의 IKeywordLookup/getLookupThunk + dissoc 강등 + IHashEq
   src grep: IKeywordLookup/getLookupThunk 참조 **0개**. defrecord map-surface 는 host 매크로
   전개 메서드를 generic emit 으로 내지만: (a) getLookupThunk(키워드 조회 인라인캐시; 본문이
   ILookupThunk anon 클래스 생성)을 generic emit-method 가 실제로 내는지, (b) **dissoc 로 선언
   필드 제거 시 plain map 강등**·assoc 새 키 record 유지(__extmap)·(c) hasheq/hashCode/equals ≡
   equiv 일관성 — 픽스처로 확인(감사는 assoc/extra-key/=만 확인, dissoc-강등 미확인). [src22].
```

#### 코드 감사(§17.1~17.4) ↔ 웹 1차 소스 교차검증 (상호 보강)
```
B3 ^:dynamic    ✓ 웹 확정: ^:dynamic = direct-linking 제외(+host DefExpr setDynamic). [src21]
B4 case 충돌    ✓ 웹: = 는 Util.equiv, hash 일관성 hard invariant. [src23]
B6 volatile     ✓ 웹 확정: ^:volatile-mutable=ACC_VOLATILE(JMM), ^:unsynchronized=plain. [src22]
B5 defrecord    ✓ 웹: defrecord=IObj/IPersistentMap/IRecord/IHashEq/ILookup/IKeywordLookup 전부
                  자동생성(create 포함); deftype=ctor 외 0. → B5(create 누락)+V3 보강. [src22]
try/finally     ✓ 웹: null catch-all + 모든 출구 finally inline = 정확(우리 emit-try 일치). 단
                  V2 catch-type 검증은 별개. §17.4b 재확인. [src20]
recur           ✓ 웹: TCO 없음, tail-position 검증, variadic-top rest 미수집, across-try 금지
                  (NO_RECUR). §17.4b 재확인. [src20]
multimethod     ✓ 웹: isa? 기반(=우선)+:default+prefer-method. §17.4b 재확인. [src24]
^:const         ✓ 웹: read-literal 만 inline, 의미관측가능. §17.4 D2 보강. [src21]
extend-via-meta ✓ 웹: 고정 우선순위 direct→metadata→external(재정렬 시 silent 오동작). 우리는
                  런타임 위임이라 OK지만 *직접 emit 시* 순서 보존 필수. [src16]
```

#### refuted (의존 금지 — 적대검증서 기각)
```
✗ "IN_CATCH_FINALLY 가 self-contained loop 의 catch/finally 내 recur 도 무조건 거부"(1-2):
   과대제약 — try BODY 의 recur 만 NO_RECUR 로 금지. 우리 frontend 위임이라 무관.
✗ "record hasheq=캐시된 per-type XOR ∘ APersistentMap/mapHasheq 파생"(0-3): 정확한 파생식 기각.
   *일관성 요구*(= → hash 동일)만 의존.
```

#### 새 1차 소스 (§16.8 보강)
```text
[src20] clojure.org/reference/special_forms — recur/try-finally/tail-position 규칙
[src21] clojure.org/reference/vars · /compilation — ^:dynamic direct-linking 제외, ^:const inline
[src22] clojure.org/reference/datatypes — deftype/defrecord 자동생성 표면, volatile-mutable
[src23] clojure.org/guides/equality — = 3-범주·== 교차범주·hash 일관성
[src24] clojure.org/reference/multimethods — isa? 디스패치·:default·prefer-method
[src25] OW2 ASM #317786 — backward-jump-widening VerifyError  gitlab.ow2.org/asm/asm/-/issues/317786
[src26] ask.clojure.org #3823 / CLJ-2345 — 비-Throwable catch VerifyError
[src27] JVMS se8 §4.7.3/4.10.1.6 — exception-table 순서·catch_type Throwable 요건
[src28] Amin & Rompf, POPL'18 — Collapsing Towers of Interpreters  cs.purdue.edu/homes/rompf/papers/amin-popl18.pdf
[src29] Alive2, PLDI'21 — bounded translation validation 신뢰경계  users.cs.utah.edu/~regehr/alive2-pldi21.pdf
```

#### open question (별도 targeted research 필요 — 웹이 못 덮은 (3)(4)(5))
- (3) meta-circular 진짜성: stage1→7 고정점이 증명하는 것(자기일관성 vs 정확성), host
  clojure.core 위임이 어디서 주장과 충돌? → selfhost.clj/stagen.clj/full_source_stage1.clj 대상.
- (4) DDC: cross-host bit-identical 한계 + full Wheeler DDC 요건(독립 2nd 컴파일러). §16.3/H7.
- (5) range-proof/AI soundness: interval/octagon/geometric/conserved/unrolling unsound 케이스 +
  validator 신뢰경계. §16.1/16.2/D1 + [src29] Alive2 로 후속.

#### M24 [P1] — B8 case int-switch unchecked cast (위 B8 해결방법 참조)
case `emit-case-int-switch-key`(:2129) 의 `RT.intCast` → `RT.uncheckedIntCast` 로 교체(throw→
default). 픽스처: 범위초과 long dispatch → :default.

### 17.6 수정 계획 — 문제 → 근본원인 → 해결방법(코드 수준) → 회귀 테스트
각 항목 "어디를 어떻게 고치는가"까지. 노드 키(:dynamic/:skip-check?/필드 :mutable 모양)는
"검증 후 사용" 표시한 곳에서 실제 analyzer 출력으로 1회 확인하고 적용.

#### M15 [P0] — B1·B2 BigInt 리터럴 정확 emit
- **문제**: `(fn [] 5N)` ⇒ `java.lang.Long`(host: `clojure.lang.BigInt`); `10000000000000000000N`
  ⇒ 컴파일러 IllegalArgumentException crash(fallback 도 못 탐).
- **근본원인**: `emit-const-value`(compiler.clj:197) 의 `(integer? v)` 분기가 BigInt/BigInteger
  까지 잡아 `(.push ga (long v))`(=RT.longCast, 범위초과 시 throw) + box-to-Long. 동일 오분류:
  `arg-class`(:368, BigInt→Long/TYPE), `emit-primitive-const`(:1057).
- **해결방법**:
  1. `:197` `(integer? v)` 분기를 *진짜 long 범위*로 좁힘:
     `(or (instance? Long v) (instance? Integer v) (instance? Short v) (instance? Byte v))`.
  2. 그 앞에 BigInt 전용 분기 추가:
     ```clojure
     (or (instance? clojure.lang.BigInt v) (instance? java.math.BigInteger v))
       (let [^java.math.BigInteger bi (biginteger v)]
         (.newInstance ga biginteger-type) (.dup ga)
         (.push ga (.toString bi))
         (.invokeConstructor ga biginteger-type
            (Method. "<init>" Type/VOID_TYPE (into-array Type [string-type])))
         ;; BigInt 면 BigInt.fromBigInteger 로 감싸고, BigInteger 리터럴이면 그대로 둠
         (when (instance? clojure.lang.BigInt v)
           (.invokeStatic ga bigint-type bigint-from-biginteger-method)))
     ```
     → BigInt 분기로 가므로 B2 의 longCast crash 도 자동 제거.
  3. 새 타입/메서드 상수(파일 상단 def 군에 추가):
     `biginteger-type (Type/getType java.math.BigInteger)`, `bigint-type (Type/getType
     clojure.lang.BigInt)`, `string-type (Type/getType String)`,
     `bigint-from-biginteger-method (Method/getMethod "clojure.lang.BigInt fromBigInteger(java.math.BigInteger)")`.
  4. `arg-class:368`/`emit-primitive-const:1057` 의 BigInt→Long 가드 제거(BigInt 는 boxed Object).
  5. `cacheable-const?`(:223-226) 에 BigInt/BigInteger 추가(clinit 1회).
- **host 참조**: clojure.lang.Compiler emitValue 의 BigInt 케이스([src15]).
- **테스트**: `(class (<backend> '(fn [] 5N)))`=clojure.lang.BigInt; `…N` round-trip `=`;
  `10000000000000000000N` 컴파일·동등; `(class 5N)` host 와 일치.

#### M16 [P0] — B4 case `:skip-check?` 처리 (hash 충돌 soundness)
- **문제**: `(case "Aa" "Aa" 1 "BB" 2 :other)` ⇒ `:other`(host=1). "Aa"/"BB" 둘 다 Util/hash=2112.
- **근본원인**: `emit-case-bucket`(:2077-2088)이 *항상* `Util.equiv` 사전체크 후 불일치면
  `goto default`. core/case 는 충돌 상수들을 한 switch 엔트리로 병합하고 test 를 hash 정수로
  바꾼 뒤 `:then` 을 `(condp = e k1 t1 k2 t2 default)` 로 만들고 그 hash 를 node `:skip-check?`
  (정수 집합)에 넣어 "사후 equiv 생략"을 지시 — 우리가 `:skip-check?` 를 파일 어디서도 안 봄.
- **해결방법**:
  1. `emit-case-bucket` 시그니처에 `skip?` 추가:
     ```clojure
     (defn- emit-case-bucket [ga env test-slot end-label default-label pairs skip?]
       (if skip?
         (do (emit-node ga env (:then (second (first pairs))))   ; 병합 then(condp) 직접
             (.goTo ga end-label))
         (doseq [[test-node then-node] pairs] …기존 equiv 체크…)))
     ```
     skip? 인 bucket 은 단일 pair 이고 그 :then 이 충돌 구분 condp 를 이미 담음.
  2. 호출부(:2101-2103): `(emit-case-bucket … pairs (contains? (set (:skip-check? node)) h))`.
  3. `emit-case-branch-chain`(비-switch 경로)은 무관(이미 equiv 직접 비교라 정확).
- **검증 후 사용**: node 의 `:skip-check?` 키 이름·값(정수 hash 집합) 실제 analyzer 출력 확인
  (감사에서 `skip-check?: #{2112}` 로 확인됨).
- **host 참조**: core.clj `merge-hash-collisions`/`case*` (:6724-6758)([src15]).
- **테스트**(회귀 픽스처): `(case "Aa" "Aa" 1 "BB" 2 :other)`=1, `"BB"`=2, `"zz"`=:other.

#### M17 [P0] — B3 `^:dynamic` def 가 setDynamic emit
- **문제**: `(def ^:dynamic *x* 1)` 후 `(.isDynamic #'*x*)`=false, `(binding [*x* 2] *x*)`
  ⇒ IllegalStateException.
- **근본원인**: `emit-def`(:326-333)가 Var.setDynamic() 안 함(파일 전체 0회). analyzer 는
  :dynamic 을 var *meta* 에만 넣고 실제 dynamic 플래그는 안 세움.
- **해결방법**: `emit-def` 에서 push-var 직후:
  ```clojure
  (push-var ga (:var node))                          ; Var
  (when (:dynamic (meta (:var node)))                ; 검증 후 사용: node 의 :dynamic? 키도 가능
    (.dup ga)
    (.invokeVirtual ga var-type var-set-dynamic-method)  ; setDynamic()→this
    (.pop ga))                                        ; Var (스택 균형)
  (when (:init node) … bindRoot …)
  ```
  새 상수 `var-set-dynamic-method (Method/getMethod "clojure.lang.Var setDynamic()")`.
  host 처럼 init(bindRoot) *전*에 setDynamic.
- **host 참조**: clojure.lang.Compiler$DefExpr.eval → `var.setDynamic()`([src15]).
- **테스트**: `(do (def ^:dynamic *t* 1) [(.isDynamic #'*t*) (binding [*t* 2] *t*)])`=[true 2].

#### M18 [P0] — B5 defrecord static `create(IPersistentMap)`
- **문제**: `(map->RB {:a 5 :b 6})` ⇒ NoSuchMethodError(positional `->RB` 는 동작).
- **근본원인**: `emit-deftype-class`(:2610-2738)가 instance 메서드만 emit; static
  `public static R create(IPersistentMap)` 없음(map->R 가 이걸 호출).
- **해결방법**: `record?` 이고 user 필드 식별 가능할 때(필드 = user… + __meta + __extmap) static
  create 추가:
  ```
  public static R create(IPersistentMap m):
    NEW R; DUP
    ;; user 필드 i: m.valAt(Keyword :fi)  (typed 필드면 결과 cast/unbox)
    for fi in user-fields: load m; getstatic CONST(Keyword fi); invokeinterface ILookup.valAt(Object)
    ACONST_NULL                              ;; __meta
    ;; __extmap = m 에서 user keyword 들을 .without 반복 제거한 나머지(빈 맵이면 그대로/ nil 무방)
    load m; (각 user kw 로 IPersistentMap.without) …
    invokespecial <init>(user…, IPersistentMap, IPersistentMap)
    ARETURN
  ```
  새 상수: `ilookup-type`, `ilookup-valat-method (Object valAt(Object))`, `ipm-without-method
  (IPersistentMap without(Object))`, `keyword-type`(이미 kw-intern 있음).
  필드 keyword 는 const-fields 캐시에 등록해 clinit getstatic 으로 로드(기존 메커니즘 재사용).
- **host 참조**: core_deftype.clj defrecord `create` 확장([src16]).
- **테스트**: `(= (map->RB {:a 5 :b 6}) (->RB 5 6))`; extra key `(:c (map->RB {:a 1 :b 2 :c 9}))`=9.

#### M19 [P1] — B6 `^:volatile-mutable` ACC_VOLATILE · B7 reify 명시 IObj
- **B6 문제**: volatile-mutable 필드가 ACC_VOLATILE 잃음(JMM 가시성 손실).
  - **근본원인**: `field-mutable (mapv #(boolean (:mutable %)) fields)`(:2623)로 종류 뭉갬;
    필드 플래그(:2667-2672)가 PUBLIC|FINAL vs PUBLIC 둘 중 하나(파일 전체 ACC_VOLATILE 0회).
  - **해결방법**: 필드별 mutability 종류 보존(검증 후 사용: analyzer field 노드의
    `:volatile-mutable`/`:unsynchronized-mutable`/필드 심볼 meta 확인) →
    ```clojure
    flags = (cond volatile?  (bit-or ACC_PUBLIC ACC_VOLATILE)
                  mutable?    ACC_PUBLIC
                  :else       (bit-or ACC_PUBLIC ACC_FINAL))
    ```
- **B7 문제**: `(reify clojure.lang.IObj (meta [_] {…}) (withMeta [t _] t))` ⇒
  ClassFormatError "Duplicate method name meta"(직접 emit crash, fallback 없음).
  - **근본원인**: `reify-user-interface-classes`(:2403)가 인터페이스 목록에선 IObj/IMeta 제거
    하나, `(:methods node)` doseq(:2536-2537)가 사용자 meta/withMeta 메서드를 그대로 emit →
    auto-IObj(meta:2518-2522, withMeta:2524-2535)와 중복.
  - **host 재확인**: current Clojure 도 명시 IObj/IMeta reify 를 거부한다. 따라서 정확한 해결은
    사용자 본문 성공 emit 이 아니라, 직접 emit crash 를 `:unsupported-op` 로 바꿔 host fallback 이
    host 와 같은 실패를 내게 하는 것.
  - **해결방법**:
    ```clojure
    (let [user-meta?    (some #(iobj-method? % "meta" 0) (:methods node))
          user-withmeta? (some #(iobj-method? % "withMeta" 1) (:methods node))]
      (when (or user-meta? user-withmeta?)
        (throw (ex-info "explicit IObj/IMeta reify matches host rejection"
                        {:type :unsupported-op :op :reify})))
      …정상 reify 에만 auto meta/withMeta emit…)
    ```
    `iobj-method?` 는 클래스 객체 set literal 을 쓰지 않고 명시 `or` 비교를 쓴다. self-source
    stage10 에서 `#{IObj IMeta}` 의 순서 비결정이 constant-pool drift 를 만들었기 때문이다.
- **테스트**: B6 `(.isVolatile (.getDeclaredField DtVol "v"))`=true; B7 은 negative conformance 에서
  host 와 compiler 가 모두 실패해야 함.

#### M20 [P1] — G1 BigDecimal/Ratio/regex/Float 직접 emit
- **문제**: `1.5M`/`1/3`/`#"a+"`/Float 리터럴이 `emit-const-value` :else(:220-221)→:unsupported-op
  →enclosing form 전체 host eval 강등(기능은 정확).
- **해결방법**: `emit-const-value` 분기 추가:
  - `(instance? java.math.BigDecimal v)`: `NEW BigDecimal; DUP; push (.toString v); <init>(String)`.
  - `(instance? clojure.lang.Ratio v)`: 두 BigInteger(num/den) 생성 후 `NEW Ratio; <init>(BigInteger,BigInteger)`.
  - `(instance? java.util.regex.Pattern v)`: `push (.pattern v); INVOKESTATIC Pattern.compile(String)`.
  - `(instance? Float v)`: `(.push ga (float v)) (.box ga Type/FLOAT_TYPE)` (double? 분기와 구분).
  - `cacheable-const?` 에 BigDecimal/Ratio/Pattern 추가(특히 Pattern 은 host 도 상수풀 캐시).
  새 상수: bigdec-type/ratio-type/pattern-type + `pattern-compile-method`, biginteger-type(M15 공유).
- **host 참조**: clojure.lang.Compiler emitValue([src15]); Pattern 캐싱.
- **테스트**: 각 리터럴 class·값 host 일치; 같은 regex 가 같은 Pattern 재사용.

#### M21 [P1] — H4 locals-clearing (head-holding/누수)
- **문제**: emit-let(:1377-1400)/emit-loop(:1720-1760)이 last-use 후 슬롯 null 안 함 → lazy-seq
  head 가 프레임 내내 reachable.
- **해결방법**: tools.analyzer.jvm 의 clear-locals 정보(노드의 `:to-clear`/clearing 패스 결과)가
  있으면 그 지점에서 `ACONST_NULL; storeLocal`. 없으면 보수적으로 let/loop body 끝(또는 recur
  직전)에 더 안 쓰는 *Object* 슬롯만 null(primitive/this/arg 제외, def-assignment 깨지지 않게).
  난이도 중 → 별도 supervised 슬라이스(verifier full 검증 필수).
- **host 참조**: clojure.lang.Compiler localsClearing / `*compiler-options* :disable-locals-clearing`.
- **테스트**: 큰 lazy-seq head 를 local 바인딩 후 소비 시 heap 안정(정성).

#### M22 [P2] — D1 raw-long prover 적대적 테스트
- §16.2 translation-validation 신뢰경계 강화. prover 가 너무 좁힐 수 있는 입력을 적대적으로
  생성해 soundness sentinel(:3088-3128)이 실제로 잡는지/raw 승격이 안전한지 패스. (별도 §16.1/2.)

#### M23 [doc] — H1·H2·H3·H6 정직성 어휘 한정구
- gate/receipt *요약*에 한정구 명시(본문 §7/§13 엔 이미 있음): "stage1→7 = backend self-emit
  **결정성+자기일관성** 고정점(정확성 아님), frames=ASM(COMPUTE_FRAMES), frontend(analyzer/
  reader)+runtime(clojure.core)=신뢰 host base". cross-host DDC=emit 결정성(Trusting-Trust 아님).
  → "byte-identical / meta-circular / 직접 emit" 단독 표현이 과대독해되지 않게.

※ **게이트 무회귀 원칙**: B1~B7 수정은 *테스트부터*(기존 smoke/conformance 가 7개 전부
미검출한 구멍). 새 회귀 픽스처 — BigInt class, 범위초과 BigInt, dynamic binding, case 충돌
"Aa"/"BB", map->R, volatile 필드 플래그, reify 명시 IObj — 추가 후 수정. 각 수정 전후
conformance(100/100)·smoke·self-host 고정점·gate READY 재확인, 슬라이스별 커밋/푸시.

### 17.7 적용 결과 메모 (2026-06-29)
- B1/B2: `BigInt`/`BigInteger` 상수 emit 을 long 경로에서 분리하고, out-of-long BigInt crash 제거.
- B3: `def ^:dynamic` 에서 `Var.setDynamic()` 을 `bindRoot` 전 호출.
- B4: analyzer `:skip-check?` hash bucket 은 사후 `Util.equiv` 체크를 생략하고 병합된 `:then` 을 직접 emit.
- B5: defrecord `public static create(IPersistentMap)` 추가, `map->R` 의 extra key 는 `__extmap` 으로 보존.
- B6: `:volatile-mutable` 필드에 `ACC_VOLATILE` 보존.
- B7: 명시 IObj/IMeta reify 는 host parity 에 맞춰 direct emit 을 거부하고 fallback 실패 의미로 정렬.
- B8: `case :int` switch key 는 checked `RT.intCast` 대신 `RT.uncheckedIntCast(Object)` 를 써서
  out-of-int dispatch 값을 예외가 아니라 default 로 보냄.
- G1: BigDecimal/Ratio/regex/Float 리터럴 direct const emit 추가.
- evaluator/kernel: `def` 를 값 맵만이 아니라 실제 `Var` 로 반영해 `var`/`binding`/`set!` 과
  `^:dynamic` 의미가 host/compiler 와 일치. conformance kernel 3-way 가 109/109 로 회복.
- T1/V3: defrecord `IKeywordLookup.getLookupThunk(Keyword)` 는 generic method emit 으로 이미
  존재함을 진단했고, dissoc plain-map 강등/hash/hasheq/set membership/keyword thunk fixture 추가.
- T2/V2: `emit-try` catch table 등록 전 catch class 가 `Throwable` 하위인지 검증. 비-Throwable
  catch 는 host fallback 실패 경로로 정렬하고 negative conformance 에 추가.
- T3/V1: `bytecode-verifier` 에 ASM #317786 조건부 후방점프 widening 스트레스 fixture 추가.
- T4/H4: `let`/`loop` Object local scope-exit clearing 추가(`ACONST_NULL; ASTORE`), primitive 슬롯 제외.
  bytecode witness 에 null-store 증거 추가.
- T5/M22/D1: translation-validation adversarial VC(boundary accepted, overflow sub/mul/wide interval rejected)
  추가 + compiler/conformance overflow sentinels 확대.
- T6/H1~H8/T7: gate receipt/CLI 에 `:proof-claim` 추가. stage fixed point/ASM frames/host trusted
  base/DDC 한계를 명시하고 full Wheeler DDC·독립 runtime/reader·완전 언어 정확성은 not-claimed 로 고정.

### 17.8 (3)(4)(5) 심층 코드 감사 + 수정 (2026-06-29) — 웹이 못 덮은 영역
T7 이 (3)(4)(5)를 "별도 research"가 아니라 not-claimed 경계로만 처리했으므로, 그 영역을
2-에이전트 적대적 코드 감사(selfhost/selfaudit + abstract-interval/translation-validation)로
직접 파서 **구체적 fixable 문제**를 찾고 수정했다. 결과 = 진짜 miscompile 1 + 정직성/일관성 2.

```
[Finding B / P0 soundness — 수정 def4ac0a] 이질적 index stride 누적합 miscompile
  loop-binding-accumulator-range 의 add-terms/sub-terms 분기가 Σi 를 exact-positive-
  index-sum(min-stride 닫힌식)으로 구하는데, then-recur 의 index stride 가 이질적(예
  +3/+2)이면 큰 초기 stride 가 이후 항을 들어올려 실제 Σi > 닫힌식 → 도출 acc range 가
  *너무 좁음*(unsound) → tv/lowering-sound? 가 잘못된 전제로 no-overflow 증명 → unsound
  raw LADD 승격 → host=long overflow throw, 우리=silent wraparound. **적대적으로 재현됨**
  (acc 실제 24, range [0,20] 도출). 수정: loop-positive-index-iteration-count 가
  :homogeneous-index-stride?(apply = strides)를 counter 에 기록, add/sub-terms 닫힌식은
  homogeneous 일 때만; 이질적이면 nil→sound unroll/interval 경로(모든 step join)로 fall
  through. 회귀: conformance negative 에 재현케이스(host=compiler=overflow throw). 무회귀:
  M6aj/range-migration digest 불변.

[Finding A / H5b — 수정 cd661dae] 부트스트랩 ns form host-eval ↔ receipt 불일치
  prepare-compiled-impl-ns!(selfaudit) 가 compiler.clj 의 ns form 만 host eval 하면서
  receipt 는 ns-form-backend-compiled=true 주장(별도 witness 만 backend, 실제 부트스트랩은
  host eval). 수정: 공개 헬퍼 comp/run-ns-form-strict(compile-fn-strict 로 host Compiler
  0회 ns 실행) 추가, 부트스트랩이 이 경로 사용 → *살아있는 self-host 경로*가 genuine 0
  host-Compiler. compile-ns :direct-compiled 도 DRY. 검증: gate READY, full-source stage1
  digest 갱신, M12 ACCEPTED(이제 부트스트랩 실측), 고정점 무회귀.

[Finding 2 / 정직성 — 수정 562523a1] TV validator "독립 2차 차단" 과장 라벨
  tv/lowering-sound? 는 *공급된* operand range 위 interval transfer finiteness 만 검사,
  operand range 자체를 독립 재도출 안 함 → 너무 좁은 range 못 잡음(Finding B 가 통과했음).
  result-ok? 항은 compiler 레인에서 항상 참(claimed-result=동일 transfer). 수정(동작
  무변경): docstring/주석/receipt 를 "공급된 range 위 finiteness gate, soundness 는 range
  제공자(AI 엔진)에 상대적" 로 교정 + receipt :trust-boundary 추가. canonical proof digest
  불변(주석/desc 만).

[감사로 SOUND 확인(수정 불필요)] widening/narrowing fixpoint(abstract_interval),
  interval transfer(corner+Math/*Exact, overflow→top), geometric-refine(부호교번 [-M,M]),
  conserved-refine(불변식 실제 보존 검증), loop-unroll-ranges(모든 step join, 이질 stride
  도 sound), 고정점 검사(generated? 강제, source-vs-source 약화 구조적 차단).

[research-frontier — held(정직 경계)] 완전 self-host 프론트엔드(tools.analyzer.jvm 외부),
  완전 runtime self-host(clojure.core 신뢰 base), full Wheeler DDC(독립 2nd 컴파일러),
  완전 언어정확성 형식증명(theorem prover) — gate proof-claim 이 not-claimed 로 명시.
  compile-form 의 host-eval fallback(analyze 실패=analyzer 외부 / deftype·reify 잔여=type-gen
  self-source) 도 같은 frontier.
```
**현 상태**: §17 의 미구현/잘못/빠진은 *fixable 한 것 전부 닫힘*(B1~B8/G1 + T1~T7 +
Finding A/B/2). 남은 것은 본질적 research-frontier 이며 gate 가 정직하게 not-claimed.
gate READY ✅, smoke 141/141, conformance 112/112 + negative 20/20, 고정점 무회귀.

## 18. 프로덕션 하드닝 — 성능·버그·API (2026-06-29, 진행중)

목표: clj-meta 컴파일러를 ../pnix-clj 가 붙여 쓸 **프로덕션 제품**으로. fallback/held 가
아니라 **실제 성능·속도 최적화 + 버그 수정**. (../pnix-clj 연결 자체는 다른 세션 담당; 여기선
clj-meta 본체 compiler 만.) 3-에이전트 감사(성능/버그/프로덕션-API) → triage → 슬라이스별
수정·검증(gate/conformance/smoke/bench)·커밋·체크.

규칙: 각 수정 전후 gate READY·conformance 무회귀, 성능은 `:bench`(런타임)·체감 compile-time 으로
측정. 헌법(RAW-FREE/no-auto-promotion/정직 held) 준수.

### 18.0 베이스라인 (수정 전)
- [x] 런타임 bench(`:bench`) 베이스라인 (ratio=ours/host, <1=우리가 빠름):
  ```
  (* n n)                         1.01   host급
  (let [a (+ n 1) b (* a 2)] …)   1.27 ★ 느림(typed-long let인데 27% 느림 — boxing 의심)
  fact(재귀)                       1.07
  loop(untyped)                   0.94   우리 빠름
  loop(^long)                     0.44   우리 훨씬 빠름(raw opcode)
  multi-arity ([][x][x y])        1.28 ★ 느림(arity dispatch)
  variadic [a & r]                1.20 ★ 느림(RestFn 경로)
  variadic multi-arity            0.75   우리 빠름
  map/closure                     1.11   약간 느림
  ```
  → 런타임 최적화 타깃: typed-long let(1.27)·multi-arity(1.28)·variadic(1.20)·map/closure(1.11).
- [x] compile-time: gate full ~4–5분(self-source stage chain). perf 감사로 hotspot 확인 중.

### 18.1 성능/속도 (perf 감사 결과 → 채움)
- [ ] (대기) perf 에이전트 발견 triage

### 18.2 버그 (bug 감사 결과 → 채움)
- [ ] (대기) bug 에이전트 발견 triage

### 18.3 프로덕션 API/동시성/수명주기 (production 감사 결과 → 채움)
- [ ] (대기) production-API 에이전트 발견 triage
