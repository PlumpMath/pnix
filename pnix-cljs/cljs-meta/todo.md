# cljs-meta TODO

이 파일은 cljs-meta의 meta-circular self-hosting 주장에 남은 작업을 추적한다.
현재 상태와 검증된 명령 출력은 `STATUS.md`(peer-floor 성명, 닫힌 주장, 기본
게이트)와 `FIXED-POINT.md`(stage 시퀀스, trust root, 크로스 플랫폼 체크리스트)
를 본다. 이 파일은 그 세부사항을 중복하지 않는다 — 남은 것을 축별로, 우선순위
있게 매핑한다.

## 현재 남은 작업 (검증 2026-08-11)

이 패스 검증: `./cljs-meta/bin/cljs-meta-gate` 라이브 PASS (self_test +
fixed_point_test + independent_mini_backend_test, 이 패스 시작 시
"independent mini backend DDC: PASS (8 fixtures)", 이후 14 fixtures로 확대 —
아래 §2 참고); `bin/build-cljs`, `bin/build-fixed-point.js`,
`bin/cljs-meta-gate`가 STATUS.md/FIXED-POINT.md 서술과 일치; top-level
`bin/pnix-cljs-gate`가 동일 세 cljs-meta 테스트 파일 실행;
`flake.nix`가 `x86_64-darwin` 외에 `aarch64-darwin`/`x86_64-linux`/`aarch64-linux`
를 나열하지만 나머지 셋은 한 번도 *실행*되지 않음(flake output으로 평가만),
FIXED-POINT.md의 명시적 "appears in flake.nix or evaluates successfully is not
supported" 주의와 일치. STATUS.md의 "Open claims (do not claim)" 블록은 이
패스 시점 정확 — 아래 항목 중 이미 닫혔는데 열린 것으로 잘못 표기된 것은
없었고, 이 세션 초반의 `independent_mini_backend.js` 작업은 이 패스에서 8→14
fixtures로 확대(`do`, strings, vectors-as-values, named-`fn` recursion),
양쪽 게이트에 연결, 실제 호스트 대비 검증, 회귀 없음.

다섯 열린 주장은 실제로 행동 가능한 축 둘과, 형제 호스트가 자체 trust root
또는 런타임 의존성을 "완료"가 아닌 영구 열림으로 유지하듯 **설계상 "false"로
읽는** 구조적 범위 경계 셋으로 나뉜다.

---

### 1. Multi-platform 바이트 결정성 (non-x86_64-darwin) — 행동 가능, 중간 규모

**상태:** `x86_64-darwin`만 닫힘으로 확인. `flake.nix`가 `aarch64-darwin`,
`x86_64-linux`, `aarch64-linux`를 빌드 타겟으로 선언하지만, 어느 것에 대한
receipt·hash·gate 실행도 없다.

**완료 모습:** FIXED-POINT.md "Cross-platform closure checklist"의 남은 세
플랫폼 각각 미체크 박스 전부 — 깨끗한 `./bin/build-cljs`, stage2==stage3
바이트 동일, source-closure 동일, stage-input-hash 체인, stage0 bootstrap-only
namespace 없음, `fixed_point_test.js` + `examples/fixed-point.js` +
`pnix-cljs-gate` 모두 green, `nix flake check` 네이티브 green, 각 플랫폼
artifact 해시 비교/설명(조용히 정규화하지 않음).

**규모:** 중간. 새 코드 불필요 — 빌드/게이트 기계는 이미 존재하고 플랫폼
일반적(Node.js + Clojure CLI + JDK). 차단 요인은 **실제 aarch64-darwin /
x86_64-linux / aarch64-linux 머신/CI runner 접근**, 이후 기존 수 분 cold
fixed-point 빌드를 세 번 실행하고 해시 조정. 노력 축은 infra·실행이지 설계가
아니다.

- [ ] `aarch64-darwin`: 전체 체크리스트 실행, receipt + 해시 비교 기록.
- [ ] `x86_64-linux`: 전체 체크리스트 실행, receipt + 해시 비교 기록.
- [ ] `aarch64-linux`: 전체 체크리스트 실행, receipt + 해시 비교 기록.
- [ ] 결정성 주장 전 플랫폼 receipt 간 path, tool-version, timestamp 차이
      정규화/설명.
- [ ] 세 플랫폼 모두 닫힌 뒤에만 STATUS.md의 `multi_platform_byte_determinism`
      flip (부분 완료는 `platform-pending` 유지).

---

### 2. Trusting-Trust / DDC 깊이 — 행동 가능, small-to-medium, 점진

**상태:** `independent_mini_backend.js`(2026-08-11 추가, 2026-08-11·08-12·08-13
확대)는 `cljs.js`/`cljs.compiler`/`cljs.analyzer`와 코드 공유 없는 진짜
from-scratch tokenizer/reader + 직접 JS-text emitter이며, 실제 self-hosted
컴파일러 `evaluate()`와 교차 검증. 34 fixtures: `let`(재귀/중첩 벡터
구조분해 포함), `if`, `do`, `when`, `cond`, `->`, `+`/`-`/`*`,
`<`/`>`/`<=`/`>=`/`=`, boolean, keyword 리터럴, string 리터럴,
vector/map/set 리터럴 반환값, seq ops `get`/`nth`/`count`/`conj`/`nil?`,
map `assoc`/`update`, named `fn` 리터럴 포함 self-recursion(factorial,
fibonacci). `test/independent_mini_backend_test.js`에 연결
(`assert.deepEqual`로 vector/map 반환 fixture 구조 비교), `cljs-meta-gate`와
`pnix-cljs-gate` 양쪽에서 실행. "DDC가 전혀 없다" 갭은 닫힘 — 누락으로 다시
표시하지 말 것.

**이 패스에서 발견·해결한 범위 메모:** `core/evaluate`는 `cljs.js`의
`eval-str`를 `:context :expr`로 실행해 *단일* top-level 식만 허용 —
`(defn ...) (foo)` 스타일 multi-form 소스는 mini backend만이 아니라 실제
호스트에서도 실패(라이브 확인). 따라서 `defn`은 이 evaluate 경로 하에서
도달 가능한 DDC fixture 형태가 아니다. Recursion은 두 백엔드가 합의할 수
있는 방식으로 표현: self-referencing named `fn` 리터럴을 자리에서 호출,
예: `((fn fact [n] (if (<= n 1) 1 (* n (fact (- n 1))))) 6)`. `defn`/multi-form
지원 누락으로 다시 표시하지 말 것 — 실제 호스트 설계상 이 DDC harness 밖이며
실수가 아니다.

**완료 모습(열망, 하드 바 아님):** clj-meta ~50-fixture `frontend_selfhost.clj`
범위에 접근하는 fixture 커버, 동일 정직한 behavior-equivalence 바
(bit-identical JS 텍스트 아님 — 독립 작성 두 emitter가 동일 소스를 내리라
기대하지 않음).

**규모:** 작은 가산 increment. 각 fixture 클래스는 자족 mini-backend 확장 +
소수의 교차 검증 fixture; 아키텍처 변경 불필요.

- [x] `do` (sequencing / 다중 body form).
- [x] String 리터럴 및 기본 string 처리(연결은 `str` 경유 가능, fixture로
      아직 미행사).
- [x] `fn` (named·anonymous 함수 정의 + 호출).
- [x] Recursion (named `fn` 리터럴 self-reference).
- [x] Map 리터럴 반환값 (keyword/string 키만).
- [x] `get`, `nth`, `count`, `conj`, `nil?` seq ops.
- [x] `let` 바인딩 벡터 구조분해 (범위 밖 위치는 이 백엔드 자체 `nil` 매핑에
      맞춰 `nil`/JS `null`로 바인드, `undefined` 아님 — 너무 짧은 소스 벡터에
      `(nil? c)` 대비 검증).
- [x] `let` 바인딩 중첩 구조분해 (`[[a b] c]`, `[a [b c] d]`) — `bindPattern`이
      이제 재귀; 실제 호스트 대비 검증. `let`/`fn` 파라미터 map 구조분해는
      아직 미지원 (`emitFn` params는 flat 심볼만 수용).
- [x] `assoc`(가변 key/value 쌍) 및 `update`(`fn`-literal updater; `inc` 같은
      bare-symbol updater는 아직 미지원 — `update` 세 번째 인자는 호출 가능
      식으로 emit되어야 하며 이 백엔드는 아직 builtin-symbol-as-value 테이블
      없음).
- [x] Set 리터럴 (`#{...}`) 반환값, plain JS 배열로 표현 — 실제 호스트에서
      `clj->js`가 작은 cljs set을 안정 삽입 순서 배열로 줌을 라이브 확인,
      `assert.deepEqual` 비교 성립; emit 시 dedup 없음(fixture에 중복 요소
      없음), 따라서 true set보다 좁은 모델.
- [x] `when`, `cond`, `->` 매크로 (thread-first는 단일 `emitExpr` 패스 전
      AST 수준 nested list form으로 rewrite, JS 수준 threading helper로
      emit하지 않음).
- [ ] non-branch 위치 keywords-as-values (현재 branch 결과/map 키/vector
      요소로만 등장 — 문서화된 갭은 아니고 fixture로 미행사).
- [ ] `let`/`fn` 파라미터 map 구조분해 (`{:keys [a b]}` 스타일) — 여전히
      열림, fixture·백엔드 지원 없음.
- [ ] Bare-symbol 호출 값 (예: `update` updater 또는 괄호 없는 `->` step으로
      쓰인 `inc`/`dec`) — 여전히 열림; call-head가 아닌 value 위치에 알려진
      builtin 심볼 이름을 인라인 JS arrow function으로 매핑하는 작은 테이블
      필요.
- [ ] `when-let`/`if-let`, `str`, 더 많은 seq ops (`map`/`filter`/`reduce`) —
      여전히 열림, 자연스러운 다음 widening 타겟.
- [ ] 각 widening 패스 후 STATUS.md "Trusting-Trust defense roadmap" 정직
      문구(fixture 수, 범위 주의) 재실행 — 문서가 실제 커버를 앞서지 않게.
- [x] `loop`/`recur` (2026-08-18) — clj-meta/clr-meta/rs-meta/hy-meta의
      let→loop/recur→closure 축 대응물. `cljs.js`의 `:context :expr` 제약
      때문에 `loop`는 IIFE `while(true){}`로 컴파일; 새 `emitTailForm`
      헬퍼가 loop 본문의 tail 위치(및 그 안의 `if`/`do`)에서만 `recur`를
      인식 — `recur`는 OLD 바인딩으로부터 모든 새 값을 임시 변수에 먼저
      계산한 뒤에야 재대입(동시 재바인딩, 순차 아님 — swap fixture로
      실증). bare `fn`(loop 없이) 안의 `recur`는 미지원(named-fn 자기재귀로
      이미 커버), tail 재귀는 `if`/`do`까지만(`let`/`when`/`cond` 안 tail
      위치는 미확장, fixture 불필요).
- [x] 진짜 클로저 (같은 날) — **코드 변경 없이 이미 동작함을 실제 테스트로
      확인**(가정 아님): JS 함수 표현식이 자연스럽게 참조 캡처하고, 기존
      `env.has(head.name)` 호출 dispatch가 이름 출신을 구분 안 해서
      `let`-바인딩된 `fn`이 여러 번/non-tail 위치에서도 그냥 호출됨.
      여러 번 호출, non-tail, 2-파라미터, 클로저-캡처-클로저까지 fixture로
      실증.
      fixture 7개 추가(34→41), mini backend 단독 + 실제
      `dist/cljs-meta-module.js` 대비 둘 다 확인. 회귀 없음:
      `cljs-meta-gate` 전체 41/41 PASS. 상세는 `STATUS.md` 참조.

---

### 3. `pnix_language_semantics_ownership = false` — 구조적, 갭 아님

**상태:** 설계상. README.md: "The evaluator is a host mechanism. It does not
own PNIX language semantics, service admission, or artifact approval."
`pnix-cljs/CLAUDE.md`도 동일 경계: cljs-meta는 pnix-agnostic,
`pnix-cljs`가 pnix parse/evaluate 소유, "`cljs-meta` proof or repeat
compilation may verify the implementation, but cannot gate ordinary
`pnix-cljs` evaluation."

**완료 모습:** 현재 아키텍처 하 N/A. 이 주장을 닫는 것은 cljs-meta가
pnix-cljs 런타임 책임을 흡수하는 것이며, 선언된 repo 경계를 완성하기보다
위반한다 — hy-meta의 명시적 "hard non-goal: never pursue independence from
the Python runtime" 항목과 같은 형태.

**규모:** 구현 작업 없음. 선택: STATUS.md "Open claims" 블록 이 줄을
hy-meta non-goal framing처럼 의도적 범위 경계로 표시해, 행동 가능 주장 둘
옆의 미닫힌 갭처럼 보이지 않게.

- [ ] (선택, docs-only) STATUS.md 이 주장 옆에 one-line non-goal 주석 추가해
      "pending"이 아니라 "by design"으로 읽히게.

---

### 4. `independent_of_Node_Closure_cljs.core = false` — 구조적, 추구 시 대규모

**상태:** FIXED-POINT.md 명시 영구 trust root: Node.js, Google Closure
runtime, `cljs.core` runtime + macro bootstrap kernel,
`cljs.reader`/`cljs.tools.reader`, fixed-point stage harness, embedded
`cljs.core` analysis cache — 모두 self-hosted artifact 밖 substrate로 명명.
clj-meta의 JVM classfile 형식, hy-meta의 CPython `ast`/`compile()`,
rs-meta의 `rustc`-as-toolchain과 같은 정직한 형태 — 형제 호스트마다
타협 불가 trust floor 하나.

**완료 모습:** 독립 구축 JS 실행 substrate + Node에 기대지 않는
from-scratch `cljs.core`/Closure-equivalent 의미 재구현 — 사실상 두 번째
ClojureScript 런타임.

**규모:** large-to-unbounded, 프로젝트 자체 trust-root 모델과도 상충
가능(호스트 언어는 항상 *어떤* 신뢰 substrate에 바닥을 둔다). 단기 작업으로
비권장; 형제 유사 trust root처럼 영구 열림으로 취급.

- [ ] 액션 아이템 없음. STATUS.md 정직 문구("Node.js, the Google Closure
      runtime, and `cljs.core` itself remain shared trust-root substrate")
      유지 — 닫을 수 있다는 암시 금지.

---

### 5. `full_ClojureScript_product_replacement = false` — 구조적, 현재 범위 밖

**상태:** 명시 non-goal. README.md는 "the first executable slice"라 부르며;
`pnix-cljs/CLAUDE.md`는 seed가 "does not claim full parity with the three
established hosts"라고 한다.

**완료 모습:** 전체 ClojureScript 언어/툴링 패리티 — bootstrap kernel 너머
완전 매크로 시스템, full core library 표면, source maps, REPL/툴링 생태계,
npm interop 등. 이 repo가 목표하는 self-hosting 증명과는 다른, 훨씬 큰
"ClojureScript를 제품으로 재구현" 프로젝트.

**규모:** 매우 큼; 우선순위 없음; 현재 추구 계획 없음.

- [ ] 프로젝트 범위가 명시적으로 재정의되지 않는 한 액션 아이템 없음.

---

## 우선순위

1. **DDC fixture widening (#2)** — 가장 저렴, 점진, 가장 새롭고 덜 성숙한
   닫힌 주장을 직접 강화.
2. **Multi-platform closure (#1)** — 잘 정의·기계적, 다만 코드가 아니라
   non-x86_64-darwin 머신/CI 접근에 차단.
3. **#3–#5**는 누락이 아니라 범위 경계. 권장 액션은 #3의 선택 STATUS.md
   문구 조정뿐 — "Open claims" 목록이 #1/#2와 같은 종류의 TODO 셋처럼
   읽히지 않게.

## 호스트 toolchain (dot-nix, 2026-08-13)

dot-nix는 `shadow-cljs`를 감싸 `PNIX_CLJS`를 주입하고, `pnix-cljs` /
`clojurescript`를 **런타임** 호스트로 설치한다. shadow를 **빌드 백엔드**로
완전 대체(shadow 없이 CLJS 프로젝트 컴파일)는 **주장하지 않는다**.

### 열림

1. 선택: non-Kimchi / 단순 모듈에 대해 shadow를 대체할 수 있는 pnix-native
   CLJS 빌드 파이프라인(원한다면).
2. eval-at-build-time에 `pnix-cljs` vs cljs-meta를 호출할 shadow hook 문서화.

## pnix 제품 라이브러리의 호스트 언어 import (사용자 의도, 2026-08-13)

**정식 doctrine:** [`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md)  
**HM mirror:** `~/dot-nix/dev/PNIX-HOSTS.md`

home-manager (`dot-nix`) 통합 맥락:

- `pnix-<host>-pnix` = 이 호스트에서 pnix 언어 표면 (`.px` REPL/eval).
- `pnix-<host>-<lang>` = 일상 호스트 개발용 호스트 언어 인터프리터/컴파일러.
- 이 호스트 **pnix 제품 반쪽**이 만든 라이브러리는 **호스트 언어 라이브러리**:
  *이* 호스트 언어에서 로드되어야 한다. 다른 호스트용 이식 가능 공통
  bytecode로 가정하지 않는다.
- 미래 **공통 이식 `.px` 라이브러리** 트랙(역사적 pnix-meta 스타일)은
  연기; 그 때문에 호스트-로컬 import 작업을 막지 말 것.

dot-nix는 PATH/env만 설정할 수 있다(classpath, PYTHONPATH, link paths,
NODE_PATH, DLL HintPath). 실제 packaging 형식이 필요한 것은 아래 제품 작업이다.


### cljs — 상태 (2026-08-14)

**착륙:**

1. Dual-axis 문서: `HOST_DEV_ENV.md`, 호스트 `CLAUDE.md` / `README.md`.
2. Flake 패키지가 `share/pnix-cljs` 선적 (Node/CommonJS 모듈 표면).
3. Host-main: bare `clojurescript` → `pnix-cljs`; HM `pnix-cljs-host`가
   `NODE_PATH` / `PNIX_CLJS_*`와 함께 `node` 래핑.
4. Pnix-main: `pnix-cljs-pnix`. 모듈 export에 `evalFile` / `evalSource` 포함.

**아직 열림:**

1. ~~Node require entry 문서화~~ → `../HOST_IMPORT.md` + flake install의
   scoped `lib/node_modules/@plumpmath/pnix-cljs` (2026-08-14).
2. 선택 npm 패키지 publish — 호스트 프로젝트가 nix store `NODE_PATH`에
   의존하지 않도록.
3. Shadow hook은 선택; 런타임 호스트는 **pnix-cljs**, 이식 multi-host `.px`
   bytecode 패키지가 아님.

## Post host-env 계획 (2026-08-14) — 계획만

이 호스트의 dual-axis + 라이브러리 import는 일상 사용 기준으로 **닫힘**.
선택 P2/P3 및 잔여 제품 작업: monorepo `HOST_ENV_P2_P3.md`.
env 계약이 깨지지 않는 한 host-env packaging을 기본 게이트로 다시 열지 말 것.
