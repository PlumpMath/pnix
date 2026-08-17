# cljs-meta 상태 (peer host-meta floor)

최종 검증: 2026-08-07.

## Peer-floor 성명

**cljs-meta**는 PNIX의 ClojureScript 호스트 메커니즘이다. 실질 peer floor는
**fixed-point self-hosted 컴파일러**이다: 연속 self-recompile artifact가
바이트 동일해지고(stage2 == stage3), 최소 stage 수(≥15 generation)를 거친다.

| Peer | Peer floor | cljs-meta 대응 |
|---|---|---|
| hy-meta | bootstrap fixed point | stage2/stage3 artifact + source-closure 동일성 |
| rs-meta | stage3 B==C evaluator | fixed-point receipt + runtime evaluate/compile |
| clj-meta | bytecode selfhost | self-compiled analyzer/compiler/cljs.js payload |
| clr-meta | C0–C3 Stage1/2 | JVM stage0 → self-hosted stage1+ fixed point |

명시적 trust root는 그대로: Node.js, Google Closure runtime, `cljs.core`,
reader runtime, macro bootstrap kernel, stage harness, analysis cache.
Stage0 JVM 컴파일러는 fixed artifact에 **패키징되지 않는다**.

크로스 플랫폼: FIXED-POINT.md에서 `x86_64-darwin`만 닫힘으로 확인됨;
다른 플랫폼은 `platform-pending`.

## 닫힌 주장

이 세션에서 라이브 검증(2026-08-07):

```text
./bin/build-cljs stage0 artifacts         OK (cli/module/stage-runtime)
fixed-point builder                       OK  stage_count=15, fixed_point=true
  stage2_artifact_sha256 == stage3        true (1789364bda06a674…)
  source_closure_equal                    true
  stage0_compiler_embedded                false
  bootstrap_only_markers_absent           true
  compiler_payload_self_hosted            true
node cljs-meta/test/self_test.js          PASS
node cljs-meta/test/fixed_point_test.js   PASS
./cljs-meta/bin/cljs-meta-gate            PASS
```

## 열린 주장 (주장하지 말 것)

```text
multi_platform_byte_determinism = platform-pending (non-x86_64-darwin)
trusting-trust_defense = false
pnix_language_semantics_ownership = false
independent_of_Node_Closure_cljs.core = false
full_ClojureScript_product_replacement = false
```

## Trusting-Trust 방어 로드맵 (Diverse Double-Compiling)

Fixed-point 증명(stage2 == stage3, ≥15 self-recompile 후 바이트 동일)은
*재현성* 증거이지 Trusting-Trust 방어가 아니다 — stage0/stage1에 심긴
백도어는 영원히 동일하게 재현되고 그 검사도 통과한다. 독립적으로 작성된
제3의 ClojureScript 컴파일러도 없다: shadow-cljs 같은 대안도 여전히 fixed
point가 의존하는 동일 공식 `cljs.core`/analyzer 계보를 통해 컴파일하므로,
그 공유 계보의 결함을 잡지 못한다.

**이 세션에 추가된 독립 mini backend (2026-08-11):**
`independent_mini_backend.js`는 처음부터 새로 쓴 ClojureScript-subset-to-JS
emitter — 자체 tokenizer/reader + `new Function(...)` 경유 직접 JS 소스 텍스트
emission. `cljs.js`/`cljs.compiler`/`cljs.analyzer`와 코드 공유 제로.
`new Function`과 JS 엔진 자체는 신뢰 호스트 substrate로 남으며, clj-meta의
`frontend_selfhost.clj`에 대한 JVM classfile 형식, hy-meta의
`independent_mini_backend.py`에 대한 Python `ast`/`compile()`와 같은 정직한 역할이다.

34 fixtures 커버 (`let` 포함 재귀/중첩 벡터 구조분해, `if`, `do`, `when`,
`cond`, `->`, `+`/`-`/`*`, `<`/`>`/`<=`/`>=`/`=`, boolean, keyword 리터럴,
string 리터럴, vector/map/set 리터럴 반환값, seq ops
`get`/`nth`/`count`/`conj`/`nil?`, map 위 `assoc`/`update`, named `fn` 리터럴
포함 self-recursion — factorial 및 fibonacci). 실제 self-hosted 컴파일러
(`dist/cljs-meta-module.js`의 `evaluate()`, 실제 cljs.js 기반 production 경로)
와 교차 검증 — 34개 모두 일치. `test/independent_mini_backend_test.js`에
연결, `cljs-meta/bin/cljs-meta-gate`와 top-level `bin/pnix-cljs-gate` 양쪽에서
실행. 이 세션 라이브 검증(2026-08-11 widening, 당일 2026-08-12 2차
widening, 2026-08-13 3차 widening: nested destructuring/`assoc`/`update`/set
literals/`when`/`cond`/`->`):
`independent mini backend DDC: PASS (34 fixtures)`, full `pnix-cljs aggregate
gate: PASS`, self-test·runtime matrix·fixed-point runtime·identity gate
회귀 없음. Map 리터럴 키는 keyword/string 리터럴만, string 키 plain JS object로
emission — 실제 호스트에서 반환 map에 대한 `clj->js` 결과와 맞춰
`assert.deepEqual` 비교가 유지된다. Set 리터럴은 plain JS 배열로 표현
(라이브 확인: `clj->js`가 작은 cljs set을 안정 삽입 순서 JS 배열로 변환),
fixture 리터럴에 중복이 없으므로 emit 시 dedup 없음 — true set보다 좁은
표현이며 literal-return-value fixture 형태를 넘어 일반화한다고 주장하지 않는다.

범위 메모: `core/evaluate`는 `cljs.js`의 `eval-str`를 `:context :expr`로
실행하며 top-level 단일 식만 허용 — 다중 top-level 폼(예: `(defn ...) (foo)`)은
mini backend만이 아니라 실제 호스트에서도 실패한다. 그래서 `defn`/recursion은
DDC 비교 가능한 방식으로 표현: self-referencing named `fn` 리터럴을 자리에서
호출, 예: `((fn fact [n] (if (<= n 1) 1 (* n (fact (- n 1))))) 6)`. 테스트
파일 fixture 비교도 `assert.equal`/`strictEqual`에서 `assert.deepEqual`로
전환 — vector 반환 fixture가 두 독립 emitter 백엔드에서 구조 동일·참조 다른
JS 배열을 내기 때문이다.

**이것으로 닫히는 것과 아직 아닌 것:** 작은 fixture 세트에 대한 진짜 2-way
행동 비교(self-hosted cljs.js-backed 컴파일러 ≡ 독립 from-scratch mini
backend)이며, 계획만 문서화된 것이 아니다. (34→41 fixture, 2026-08-18 —
`loop`/`recur`와 클로저 추가, 아래 참조.) conformance 표면이 아니며 —
clj-meta·hy-meta가 이미 합의한 동일 정직한 기준 — behavior equivalence이지
bit-identical JS 텍스트가 아니다(동일 언어를 목표로 한 두 독립 작성 emitter가
동일 소스를 내리라 기대하지 않음).

**`loop`/`recur` + 진짜 클로저 추가, 2026-08-18:** clj-meta/clr-meta/rs-meta/
hy-meta가 이번 세션에 거친 "let → loop/recur → closure" 축의 대응물, 사용자가
"yes"로 cljs-meta 차례를 확인. 이 backend는 이미 `let`(중첩 벡터
구조분해까지), `do`, named `fn` 재귀를 갖추고 있어서 다른 host들처럼
blank slate가 아니었음 — 남은 진짜 격차는 `loop`/`recur`뿐이었고, 클로저는
**코드 변경 없이 이미 동작함을 먼저 실제로 테스트해서 확인**(가정 아님) —
JS 함수 표현식이 자기를 둘러싼 스코프를 자연스럽게 참조 캡처하고, 기존
`env.has(head.name)` 일반 호출 dispatch가 `let`으로 묶인 이름이든 top-level
바인딩이든 구분 안 해서 그냥 호출됨(hy-meta의 `ast.Lambda` 발견과 같은
성질).

**`loop`/`recur` 설계**: `cljs.js`의 `eval-str :context :expr` 제약(단일
top-level 식만) 안에서 표현되어야 하므로 `loop`는 IIFE `while(true){...}`로
컴파일. 새 `emitTailForm` 헬퍼가 loop 본문의 **tail 위치**만 특별
취급 — `recur`를 만나면 모든 새 값을 OLD 바인딩으로부터 임시 변수에 먼저
계산한 뒤(clr-meta/rs-meta에서 확립한 "동시 재바인딩, 순차 아님" 원칙과
동일) 그제서야 실제 loop 변수에 재대입하고 `continue`; `if`/`do`가 tail
위치에 나타나면 재귀적으로 그 안쪽까지 tail-aware하게 처리(각 분기가
독자적으로 `recur` 하거나 값을 `return`). **의도적으로 좁힌 경계**: 문서화된
tail-position 재귀는 `if`/`do` 안까지만(실제 Clojure는 `let`/`when`/`cond` 등
더 넓게 허용) — 필요한 fixture가 없어서 미확장. 또한 bare `fn`(명시적
`loop` 없이) 안의 `recur`도 미지원 — 이미 named-fn 자기재귀로 커버되는
경로라 별도 메커니즘 불필요 판단.

새 fixture 7개(34→41): 합산 loop, factorial loop, 동시 재바인딩을 검증하는
swap 케이스(`(loop [a 1 b 2 n 3] (if (= n 0) a (recur b a (- n 1))))`,
순차 구현이었다면 다른 값이 나왔을 것 — 실제로 2로 검증됨), 여러 번 호출되는
클로저, non-tail 호출, 2-파라미터, 클로저를 캡처하는 클로저. fixture 추가
전 mini backend 단독 실행 + 실제 `dist/cljs-meta-module.js`(self-hosted
컴파일러) 대비 둘 다 확인(가정 아님). 회귀 없음: `cljs-meta-gate` 전체
41/41 PASS(self-host matrix, fixed-point runtime 포함).

**다음 구체 단계:** `when-let`/`if-let`, `str`, 더 많은 seq ops 등 매크로
확대 계속, clj-meta의 ~50-fixture `frontend_selfhost.clj` 범위에 접근 —
작고 선택적·가산적 increment이며 하드 바가 아니다.

Node.js, Google Closure runtime, `cljs.core` 자체는 이 closed 이후에도 공유
trust-root substrate로 남는다 — 컴파일러 수준 DDC는 그 하위 층을 건드리지
않으며, 건드린다고 주장해서는 안 된다.

## 기본 게이트

```sh
# From pnix-cljs/
./cljs-meta/bin/cljs-meta-gate           # dist 있으면 사용; 없으면 빌드
./cljs-meta/bin/cljs-meta-gate --rebuild # ./bin/build-cljs 강제
```

필요: JDK + Clojure CLI + Node.js. Cold fixed-point rebuild는 수 분 소요.

## 마지막 실행 (이 머신, 2026-08-07)

| Gate | Result | Notes |
|---|---|---|
| stage0 JVM compile | **PASS** | local cljs snapshot clojurescript-r1.12.145 |
| fixed-point (≥15 gens) | **PASS** | receipt.fixed_point=true |
| `cljs-meta-gate` | **PASS** | self_test + fixed_point_test |
| env | Node v26.7.0, OpenJDK 21 | OK |
