# pnix-clj / clj-meta 분리 계획

갱신: 2026-07-01 KST

이 문서는 요청된 분리 계획("순수 clj-meta 컴파일러/evaluator 계층 + pnix-clj
런타임/interop 계층")을 **실제 현재 브랜치**와 맞춥니다. 모든 "stays / moves /
interop" 배정은 이 브랜치의 실제 파일을 인용합니다.

**브랜치 현실 (먼저 읽기).** 이 브랜치 — `feat/clj-meta-metacircular` — 는 깨끗한
병렬 재작성입니다. clj-meta 호스트 증명 하한 위의 Clojure/JVM 호스팅 pnix
런타임(parse -> evaluate -> lower -> clj-meta 레인 -> .px 런타임 레인 ->
mirror/receipt). merge-base `5a8db4d8`에서 `origin/main`과 갈라져
**0 behind / 407 ahead**. `origin/main`은 다른, 더 오래된 설계 라인
(content-addressed `cas.clj`, append-only `store.clj`, `stage.clj`, `purity.clj`,
`term.clj`/`stm.clj`/`resolve.clj`, gate-graph, 67-language emit, Korean
modules). 그 main 모듈은 **참조 자산 (MAIN-ONLY)** — 로드맵 항목이 부를 때
이식에 유용하지만, 이 브랜치에 "이미 있음"으로 판단해서도, "이 브랜치가 채워야
할 부재 기둥"으로 판단해서도 안 됩니다. 이 브랜치는 자체 파일만으로 분류:
BUILT / PARTIAL / TARGET / MOVED / HELD, 그리고 `origin/main`에만 있고 여기
없는 것은 MAIN-ONLY. 명시적 이식이 아니면 main 모듈을 끌어오지 마세요.

`todo.md`와 그 `## Completeness Roadmap`과 함께 읽으세요. 이것이 아키텍처 맵이고,
로드맵이 기능 백로그입니다.

---

## 0. 핵심 교정: 메타순환은 "mirror"만이 아님

이전 프레이밍은 메타순환 능력을 *mirror*가 있을 때만 존재하는 것처럼 다뤘습니다.
너무 좁습니다. Mirror는 **하나의 관찰 표면**입니다. 메타순환 능력은 전체 집합:

```
reader · parser · form-as-data · AST-as-data · canonical form · content hash ·
eval/apply · macroexpand · namespace/Var/metadata reflection · stage bootstrap ·
compiled-class-artifact proof · roundtrip · drift detection · witness/proof ·
gate/capability · interop · self-hosting ladder
```

Mirror의 현재 여러 조각 형태(별도 `mirror.clj`, `mirror_pair`, `mirror_error`,
`clojure_projection`, `clojure_form` 레인)는 설계 결과이지 요구사항이 아닙니다.
§6이 이를 트레이스 facet을 가진 singleton 런타임 mirror로 교정합니다.

세 계층, 하나의 브리지:

```
clj-meta   = Clojure/JVM 메타순환 컴파일러/evaluator PROOF 레인 (호스트 하한)
pnix-clj   = clj-meta 위 pnix 런타임 + pnix <-> Clojure/JVM interop
interop    = 경계의 명시적 양방향 브리지 (mirror 아님)
```

계층화 & 시퀀싱 (더 날카로운 모델): **clj-meta는 pnix-agnostic.** 일은 스스로
Clojure(JVM) 메타순환을 *완성*하는 것 — self-hosting ladder (stage1 -> 7 -> ...
-> N), kernel, import hook, artifact 재현, 호스트 introspection (clj-meta 자체
todo). pnix를 모름. **pnix-clj는 그 완성된 호스트 위 순수 pnix 계층이며, 그
"호스트" *가* clj-meta.** 따라서 pnix-clj는 호스트 증명을 다시 하면 안 됨:
"호스트 Clojure가 충실한가?" 작업은 모두 Clojure <-> Clojure이며 clj-meta 영역
(interop를 통해 도달), pnix 런타임 코어가 아님.

이 브랜치에 대한 귀결: `clojure_form.clj`(호스트 Clojure `eval` vs clj-meta
컴파일 동의)와 `clojure_projection.clj`의 호스트-reflection 반쪽
(Var/NS/Java/reflection 스냅샷)은 **Clojure-about-Clojure 호스트 도메인** 작업이지
pnix-런타임 코어가 아님. pnix-clj의 실제 코어는 `parser` / `evaluator` /
`lowering` / `px_runtime` / `mirror` / `receipt` (pnix 언어). 호스트 도메인
조각은 interop 뒤(Phases B/C)로 이동하며 개념적으로 clj-meta 레인에 속함.

---

## 1. 현재 현실 (검증됨 — 리팩터 전 읽기)

Git 루트는 `~/pnix-clj`. 형제:

```
clj-meta/                 성숙한 Clojure->JVM-바이트코드 메타순환 컴파일러
pnix-clj/                 이 프로젝트 (pnix 런타임)
clojure-clojure-1.12.5/   호스트 Clojure 소스 corpus
pnix-mirror-runtime/ pnixc-pnix/ stdlib/ corpus/ docs/ ingest/ scripts/
```

- `pnix-clj/deps.edn`: `pnix/clj-meta {:local/root "../clj-meta"}` — clj-meta가
  선언된 유일한 백엔드 의존성.
- **clj-meta가 이미 호스트 증명 레인을 소유.** 스텁이 아님. `src/pnix/clj_meta/`
  포함: `compiler.clj` (`compile-form`, `compile-form*`, `eval-form`,
  `compile-form-strict`, `compile-ns`, `load-compiled-ns`, `compile-classes`,
  `compile-to-dir`), `verified_compile.clj` (`compile-classes-verified`),
  `bytecode_verifier.clj`, `bytecode_witness.clj`, `determinism_policy.clj`,
  `translation_validation.clj`, `conformance.clj`, `fuzz_conformance.clj`,
  `kernel.clj`, `selfhost.clj`, `runtime_selfhost.clj`, `frontend_selfhost.clj`,
  `crosshost.clj`, `cross_host_ddc.clj`, `mirror.clj`, `stm.clj`, `gate.clj`,
  `language_surface.clj` 등.
  > 귀결: **"clj-meta로 이동"은 거의 항상 "clj-meta의 기존 API를 소비" 또는
  > "얇은 호스트-reflection 헬퍼를 clj-meta interop API 뒤로 재배치"를 뜻함 —
  > 새로 짓는 것이 아님.** clj-meta가 가진 것을 재발명하지 마세요.
- **pnix-clj의 호스트 기계는 정확히 세 파일에 국한** (`requiring-resolve` /
  호스트 `(eval ...)` / reflection을 `src/pnix_clj/*.clj` 전역 grep으로 검증):
  `clj_meta.clj`, `clojure_form.clj`, `clojure_projection.clj`. 나머지는 전부
  pnix-native.
- **이 브랜치에 없음** (MAIN-ONLY 참조 자산 — `origin/main`의 다른 설계 라인에
  살며, 여기 "이미 있음" 또는 "채울 부재 기둥"으로 다루지 말 것): `cas.clj`
  (content-addressing: `normalize-term`/`canonical-form`/`term-hash`/`term-key`),
  `store.clj` (append-only event log), `term.clj`, `stm.clj`, `resolve.clj`,
  `purity.clj`, `stage.clj`, `evidence.clj`, `verifier.clj`, `search.clj`, 및
  `README`. 이 브랜치는 parse -> evaluate -> lower -> (clj-meta 레인) ->
  (.px 런타임 레인) -> mirror/receipt 설계이며, cross-lane `receipt/verdict`가
  수락 게이트. 이후 로드맵이 content-addressed term이나 event store를 필요로
  하면 `origin/main`의 `cas.clj` / `store.clj`에서 PORT (명시적 브랜치 비교),
  처음부터 발명하지 말 것.

---

## 2. 파일별 소유권 맵 (핵심, 실제 파일 근거)

범례: **PNIX** = pnix-런타임, 유지; **HOST** = Clojure/JVM 호스트 기계,
clj-meta에 속함(이동 또는 소비); **INTEROP** = 경계 브리지; **CHECK** =
런타임 위 리포트/검증 harness, 별도 런타임 mirror 아님.

| file | bucket | notes |
|---|---|---|
| `parser.clj` | PNIX | pnix tokenizer/parser/AST; pnix 언어 표면. |
| `evaluator.clj` | PNIX | pnix 의미론, 값 모델, builtins, env (이제 lazy `let` + call-by-need args + `with`/`assert`/`a.b or d`/`@`-patterns). |
| `lowering.clj` | PNIX (출력이 INTEROP 교차) | pnix->Clojure lowering *정책*은 pnix-clj; 방출 form의 *compile/eval*은 clj-meta. 여기 유지; 방출 form을 경계 교차로 취급. |
| `clj_meta.clj` | **INTEROP** | 이미 clj-meta 컴파일러 seam. §5에서 clj-meta interop 클라이언트로 공식화. 현재 clj-meta가 이미 소유한 determinism/strict/bytecode/verified 증거를 *재유도* — 대신 위임 (§3.4). |
| `clojure_form.clj` | **HOST-DOMAIN (Clojure<->Clojure)** | NOT pnix: 호스트 Clojure `eval` vs clj-meta 컴파일 동의 검사 — 개념적으로 clj-meta에 속하는 Clojure self-host 증명. 호스트 eval은 이미 `interop` 경유; 동의는 호스트 CHECK, pnix "projection" 아님. |
| `clojure_projection.clj` | **HOST reflection (Clojure<->Clojure)** + PNIX term-mapping | 가장 큰 오배치: raw Clojure/JVM introspection (호스트 도메인). 분리: 호스트 스냅샷은 interop 뒤; pnix-term 매핑(`project-reader-value`) + `.px` 검증(`validate-term`)만 pnix-clj에 유지 (§3.1). |
| `px_runtime.clj` | PNIX | 내부 `.px` 런타임: pnix-in-pnix parse/eval, import graph/cache/cycle. |
| `mirror.clj` | PNIX | 런타임 mirror 행 + cross-mirror verdict; singleton으로 통합 (§6). |
| `receipt.clj` | PNIX | verdict / lane-summary / summarize (수락 게이트). |
| `core.clj` | PNIX | `run-source` / `report` 오케스트레이션; singleton `run-mirror`의 자연 거처. |
| `error.clj` | PNIX | pnix 구조화 오류 envelope (`:pnix-clj.error.v0`). |
| `version.clj` `math.clj` `json.clj` | PNIX | pnix builtin 헬퍼. |
| `oracle.clj` `rust_batch.clj` `stage7_core.clj` | PNIX | pnix corpus/fixtures. |
| `mirror_pair.clj` `mirror_error.clj` | CHECK | 런타임 위 리포트 harness; 별도 mirror가 아니라 check 범주로 재프레이밍. |
| `report_artifact.clj` `runtime_plan.clj` `smoke.clj` `benchmark.clj` | PNIX tooling | 유지. |
| `stage15.clj` `stage15_plan.clj` | **HOST stage control** | clj-meta 백엔드 게이트 계획, 현재 NOT-executed 제어 계획 (`:stage15-gates-not-executed`). 개념적으로 clj-meta의 gate/stage 레인; pnix-clj는 계획을 소유하지 말고 clj-meta가 제공한 stage 증명을 *소비*. |

---

## 3. clj-meta로 이동(또는 위임)할 것

오늘은 세 호스트-접촉 파일에 모두 산다.

### 3.1 `clojure_projection.clj`의 호스트 reflection/introspection (큰 것)

이 함수들은 순수 Clojure/JVM 호스트 introspection이며 호스트 증명/interop 레인에
속하고, pnix-clj에는 **interop API를 통한 호스트 스냅샷**으로 표면화
(clj-meta가 호스팅 가능; 최소 `pnix-clj.interop` 호스트 측 네임스페이스 뒤로
이동):

```
project-var-value · project-namespace-value · project-throwable-value ·
class-term · java-object-term · macroexpand-source-term ·
dynamic-binding-source-term · java-interop-source-term · reflection-source-term ·
classloader-source-term · namespace-resolution-source-term ·
host-object-construction-source-term · polymorphism-source-term ·
metadata-source-term · state-effect-source-term · lazy-evaluation-source-term ·
concurrency-source-term · coordination-source-term · control-flow-source-term
```

이 파일에서 pnix-clj에 **남는 것**: `project-reader-value` (호스트 값 -> pnix
투영 term 매핑)와 `validate-term` / `projection-runtime` (내부 `.px` 투영
아티팩트로 pnix term 검증). 그것이 브리지의 pnix 측.

여기서 **opaque-ref 규칙** 강제: `java-object-term`은 현재 `JavaObject` envelope를
직접 임베드. JVM/Clojure 객체는 그 자체로 pnix 정규 term에 들어가면 안 됨;
순수 pnix 값으로 명시 변환하지 않는 한 opaque ref로 교차 (§5).

### 3.2 `clojure_form.clj`의 호스트 `eval` / `macroexpand` / fresh-ns

새로 만든 네임스페이스의 `(eval form)`은 호스트-eval 기계. clj-meta 호스트-eval
interop API를 경유 (clj-meta `compiler/eval-form` 이미 존재; clj-meta와 구별되는
진짜 호스트-Clojure oracle이 필요하면 host-oracle eval 추가). **호스트-vs-clj-meta
동의**는 pnix-clj의 CHECK로 유지.

### 3.3 stage15 제어 계획

`stage15.clj` / `stage15_plan.clj`는 clj-meta 백엔드 게이트 명령을 서술하며
실행되지 않음. clj-meta gate/stage 관심사; pnix-clj는 계획을 들고 다니지 말고
*실행된* stage 증명을 소비. (로드맵 Axis-3 항목 "execute stage15 rather than
plan it" 참고.)

### 3.4 `clj_meta.clj`의 compile-proof 재유도

`clj_meta.clj`가 clj-meta의 `compile-form*` 주변에서 determinism / strict /
bytecode-artifact / verified-compile 증거를 재구축. clj-meta가 이미
`determinism_policy`, `verified_compile`, `bytecode_witness`, `bytecode_verifier`
를 소유. 재유도 대신 그것들에 위임해 호스트 증명에 단일 소유자를.

---

## 4. pnix-clj에 남는 것 (pnix-런타임 메타순환)

pnix-clj *가* pnix 런타임이므로 pnix-native 메타순환 표면을 소유:

- pnix tokenizer/parser/AST (`parser.clj`).
- pnix evaluator / apply / 값 모델 / builtins / env (`evaluator.clj`),
  laziness 작업(메모이즈-thunk `let`, call-by-need args)과 문법
  (`with`, `assert`, `a.b or d`, `@`-patterns) 포함.
- pnix lowering *정책* (`lowering.clj`) — pnix->Clojure 매핑; 출력이 interop
  경계를 건너 clj-meta로.
- 내부 `.px` 런타임 (`px_runtime.clj`) — pnix-in-pnix evaluator + import
  graph/cache/cycle. 자체로 pnix 측 메타순환 아티팩트.
- pnix 런타임 mirror (`mirror.clj`, singleton이 될 것), receipt/verdict
  (`receipt.clj`), 오케스트레이션 (`core.clj`).
- pnix 오류 모델 (`error.clj`), pnix 헬퍼 (`version`/`math`/`json`), pnix
  corpus/fixtures/reports.

TARGET (미래, 현재라고 주장하지 말 것): pnix CAS / canonical-term store /
event log / snapshot-resolve / stage tower. 채택되면 이것들은 pnix-런타임이며
pnix-clj에 남음 — 그러나 오늘은 존재하지 않음.

---

## 5. Interop 경계 (Clojure/JVM <-> pnix)

**Interop는 mirror가 아님.** Interop는 값/함수/모듈/effect를 변환하며 mirror가
꺼져 있어도 동작해야 함. Mirror는 interop를 *관찰*할 수 있지만 정의하지 않음.

- 호스트 측 (clj-meta): 객체 검사, IFn 호출, 네임스페이스 로드, Var 해석,
  macroexpand, 예외 캡처, JVM reflection, classpath/classloader 제어.
- pnix 측 (pnix-clj): pnix 값/함수/모듈/오류 표현, opaque 호스트 ref, interop
  호출 form, interop 증인.

공유 프로토콜 필드 (모든 교차에 부착):

```
interop/id · direction · source-language · target-language ·
input-kind · output-kind ·
loss-status      = lossless | lossy | opaque | effectful | unsupported | dangerous
effect-class     = pure | host-call | require | resolve-var | file-read |
                   file-write | thread/future | time | random | process |
                   network | global-mutation | namespace-mutation |
                   var-mutation | unknown
capability-required · host-object-policy · witness-id
```

값 매핑 (pnix <-> Clojure/JVM):

```
null<->nil  bool<->Boolean  int<->integer  float<->floating  string<->String
list/vector<->vector  attrset<->map  symbol<->symbol  keyword<->keyword
function<->IFn wrapper  module<->namespace/module wrapper  error<->ExceptionInfo
opaque JVM object<->pnix opaque ref
```

**규칙:** Clojure/JVM 객체는 pnix 정규 term에 직접 들어가면 안 됨 — 순수 pnix
값으로 명시 변환하지 않는 한 opaque ref로 래핑. (§3.1의 `java-object-term`
임베딩을 직접 수정.)

pnix-clj는 이미 그 씨앗을 가짐: `clj_meta.clj` (호스트 compile/eval seam)과
`error.clj` (구조화 envelope). 작업은 프로토콜을 명시적이고 양방향으로 만드는
것이며, 제로에서 발명하는 것이 아님.

---

## 6. Mirror 교정: 하나의 런타임 mirror, 많은 트레이스 facet

현재 (파편화): `mirror.clj` 행이 `core/run-source`에서 조립되고,
`mirror_pair`, `mirror_error`, `clojure_projection`, `clojure_form`은 별도
리포트 레인이며, clj-meta에 자체 `mirror.clj`가 있음. 단일 정본 런타임 mirror
경로, 결과 해시, 트레이스 id 없음.

목표:

```
pnix-clj.mirror/run-mirror(source, opts)
  parse -> (term) -> (resolve) -> eval -> record trace facets ->
  one result hash, one trace id, one witness
```

허용 트레이스 facet (별도 mirror 아님):

```
:host/parse :host/term :host/resolve :inner/eval-step :inner/value
:inner/effect :inner/error :interop/call :witness/event
```

clj-meta는 여러 호스트 **CHECK 범주** (compiler / macroexpand / namespace /
Var / class-artifact / host-eval 검사)를 유지 — 올바름; 경쟁 pnix 런타임
mirror가 아니라 범주로 조직된 호스트 증명 검사.

Singleton 이유: 하나의 정본 경로, 하나의 결과 해시, 하나의 트레이스 id, 하나의
수렴 목표; 중복 parse/eval 감소; 성능·분석·디버깅 개선; drift 감소.

---

## 7. 현재 vs 목표 (정직 장부)

| 개념 | 상태 |
|---|---|
| clj-meta as host proof lane | **present** (성숙) |
| pnix parser/evaluator/lowering/.px-runtime/mirror/receipt | **present** |
| host machinery isolated to 3 files | **present** (고칠 오배치) |
| explicit interop protocol (loss/effect/capability/witness) | **target** (`clj_meta.clj`/`error.clj`에 씨앗) |
| singleton `run-mirror` | **target** (오늘: `mirror.clj` + 리포트 레인) |
| opaque-ref discipline for JVM objects | **target** (오늘: 임베드 envelope) |
| CAS / event store / term store / snapshot-resolve | **MAIN-ONLY** (origin/main `cas.clj`/`store.clj`/`term.clj`/`resolve.clj`; 이 브랜치에 없음 — 로드맵 필요 시 port) |
| README | **MAIN-ONLY** (origin/main에 있음; 이 브랜치에 없음) |

현재 수락 규율은 cross-lane `receipt/verdict`
(evaluator / clj-meta / `.px` 런타임 / pnix-mirror) — N-version **휴리스틱**
차등 검사이며 형식 증명 아님 (로드맵의 framing invariant 참고).

---

## 8. 단계적 리팩터 (점진적, 단계마다 게이트 녹색)

- **Phase A — interop seam 공식화.** `clj_meta.clj`를 명시적 clj-meta interop
  클라이언트로; loss/effect/capability/witness를 운반하는 interop receipt 부착.
  낮은 위험 (이름 변경 + 메타데이터).
- **Phase B — `clojure_projection.clj`에서 호스트 reflection 추출.** 호스트
  스냅샷 함수(§3.1)를 호스트 측 interop API 뒤로; pnix-clj는
  `project-reader-value` + `validate-term` 소유. opaque-ref 규칙 강제.
- **Phase C — `clojure_form.clj`의 호스트 eval/macroexpand**를 clj-meta interop
  API 경유로; 동의는 CHECK로 유지.
- **Phase D — 런타임 mirror를** 트레이스 facet이 있는 singleton `run-mirror`로
  통합; `mirror_pair`/`mirror_error`/`clojure_projection`/`clojure_form`을
  하나의 mirror 위 CHECK 범주로 재프레이밍.
- **Phase E — compile proof 위임** (`clj_meta.clj`의 determinism/verified/
  bytecode)을 clj-meta의 `determinism_policy`/`verified_compile`/
  `bytecode_witness`에, 재유도하지 말고.
- **Phase F (로드맵 항목이 필요로 할 때만)** — `origin/main`에서
  content-addressed terms / event store / snapshot-resolve PORT (`cas.clj`/
  `store.clj`/`term.clj`/`resolve.clj`), 이 브랜치 값 모델에 맞게 명시적 브랜치
  비교. 상시 범위 아님; 구체 필요마다 끌어옴.

각 단계는 `bin/pnix-clj-gate`, `clojure -M:test`, 리포트 레인을 녹색으로 유지하고
자체 슬라이스로 commit/push.

---

## 9. 최종 아키텍처 (교정된 선언)

```
clj-meta = Clojure/JVM 메타순환 컴파일러/evaluator PROOF 레인
  owns: Clojure forms, macroexpand, eval/compile oracle, namespace/Var/metadata
  reflection, JVM/classpath/class artifacts, dynamic loading, host introspection,
  host-side interop, host witnesses and gates. (이미 성숙; 소비.)

pnix-clj = clj-meta 위 pnix 런타임
  owns: pnix tokenizer/parser/AST/eval/value/builtins/env, lowering 정책,
  내부 .px 런타임, pnix mirror, receipt/verdict, pnix 오류 모델,
  corpus/reports. (TARGET 추가: CAS/term-store/stage tower.)

interop = 명시적 양방향 브리지
  Clojure/JVM 호스트 객체와 pnix 값/함수/모듈은 loss-marked, effect-classified,
  capability-checked 어댑터를 통해서만 변환. JVM 객체는 순수 pnix 값으로
  변환되지 않는 한 opaque ref로 교차.

mirror = 메타순환의 원천이 아님
  pnix 런타임 측의 하나의 관찰/증명 진입점; 많은 트레이스 facet,
  하나의 정본 실행. clj-meta는 별도 호스트 CHECK 범주 유지.
```

핵심 원칙: **pnix-clj를 파편화된 mirror 더미로 만들지 말 것.** clj-meta를
Clojure/JVM 호스트 메타순환 증명 계층으로, pnix-clj를 pnix 런타임 계층으로,
interop를 명시적으로, pnix 런타임 mirror를 singleton으로, 모든 변환·effect·
재연·drift·stage 결과가 증인을 만들게.

---

## 10. 연구 근거 interop 경계 + capability 분배 (2026-07-01)

호스트 언어 위 guest 언어 호스팅에 대한 `/deep-research` 패스 (94 agents,
adversarially verified)가 계층 분리를 확인하고 interop 경계를 날카롭게 함.
인용 참조 시스템: **GraalVM Truffle** (deny-by-default `@HostAccess.Export`
allowlist; effect별 직교 스위치: host-access / reflection
(`allowHostClassLookup`) / native / IO), **object-capability 이론**
(designation+authority를 묶는 위조 불가 handle; "only connectivity begets
connectivity"; least authority), **정적/대수적 effect 시스템** (CallE
`restrict[ε]`; Wyvern이 단일 `system.FFI`를 도메인 effect로 승격),
**opaque-handle FFI** (Kernel-FFI가 호스트 객체를 UUID 아래 저장하고 참조만
전달), **content-addressed 코드** (Unison: hash-of-normalized-AST 정체성,
이름은 별도 메타데이터).

### Interop 경계 설계 원칙 (host <-> pnix)
1. **Deny-by-default.** 호스트 하한에서 명시 export되기 전 아무것도 pnix
   런타임에서 도달 불가. 호스트(clj-meta 측)가 allowlist 소유; pnix-clj는
   부여된 capability만 받고 호스트 기계에 ambient 도달하지 않음. 경계를
   allowlist로 짓고, capability를 하나씩 추가.
2. **모든 교차를 effect class로 분류** (pure / host-call / reflection /
   require / file / network / mutation / time / random / thread). Interop
   계층이 태깅·게이팅의 단일 장소; pnix 코어는 이미 분류·게이팅된 capability만
   봄.
3. **Opaque handle, 값 직렬화 금지.** 호스트 (Clojure/JVM) 객체는 opaque ref
   (designation + authority)로 교차, pnix 값으로 직렬화하지 않음, pnix
   정규 / content-addressed term에 들어가면 안 됨. (오늘의 `java-object-term`
   호스트 객체 envelope 임베딩을 직접 수정.)
4. **Object-capability 규율.** Authority는 handle 전달로만 이동; ambient/global
   이름이 부여하지 않음; 교차마다 least authority.
5. **Content-addressed cross-layer trust.** 호스트 하한을 content-addressed
   호스트 버전 id로 guest 증거에 바인딩; pnix term을 *정규화* AST 해시로
   식별, 사람 이름은 별도 메타데이터 (Unison 모델).

### 정직한 주의 (과대 주장 금지)
- Cross-layer 동의 (receipt/verdict N-version 검사)는 **휴리스틱, 형식 증명
  아님** — 호스트 "하한 증명"이 pnix에 의미론이나 건전성을 넘겨주지 않음.
  연구 확인: **eager Clojure 호스트 위 lazy Nix-like guest는 laziness를 명시적
  guest-계층 thunk로 구현해야 함** — 이미 착륙한 lazy `let` + call-by-need-args
  작업과 정확히 일치.
- Effect-system 건전성은 **정직한 foreign annotation**에 달림; 어떤 참조가
  교차하는지를 통제하는 것은 필요하나 충분하지 않음 — 참조 통제 + effect typing
  + membrane. (`HostAccess.SCOPED` 스타일 handle-escape 방지는 신뢰 불가로
  반박됨; 기대지 말 것.)
- 거친 호스트 grant (class loading, native, IO)는 "사실상 모든 접근 부여" —
  grant를 세밀하게 유지하지 않으면 경계 증명이 휴리스틱으로 붕괴.
- **Singleton-mirror** 선호는 OUR 설계 선택 (중복 감소, 하나의 수렴 목표),
  외부 증명된 법칙 아님 — 연구도 "트레이스 facet이 있는 하나의 정본 실행"
  패턴을 어느 쪽으로도 지지하지 않음. 근거로 유지, 증명으로 두지 말 것.

### Capability 분배 표 (host / guest / interop; feat-branch 상태)

| capability | layer | feat-branch status |
|---|---|---|
| Clojure form read/normalize/macroexpand/eval/compile oracle | **clj-meta (host)** | clj-meta 성숙; pnix-clj `clojure_form` 호스트-eval이 `interop` 경유 (Phase C 완료) |
| namespace/Var/metadata/classpath/class-artifact reflection; dynamic require/resolve | **clj-meta (host)** | clj-meta 도메인; pnix-clj `clojure_projection` 호스트 reflection = MOVE (Phase B) |
| host mutation/pollution detection; host introspection | **clj-meta (host)** | clj-meta 도메인 |
| deny-by-default allowlist + effect-class gating + capability grants | **interop** | TARGET (점진 구축) |
| value/function/module marshalling; opaque host refs | **interop** | seam 시작 (`pnix-clj.interop`, `clj_meta.clj`); opaque-ref 규칙 TARGET |
| pnix tokenizer/parser/AST | **pnix-clj (guest)** | BUILT |
| pnix evaluator/value/builtins/env; **laziness (thunks)** | **pnix-clj (guest)** | BUILT (lazy let + cbn args; lazy attrset/list TARGET) |
| pnix lowering policy (pnix -> Clojure) | **pnix-clj (guest)** | BUILT (출력이 interop 교차) |
| canonical term / CAS / content hash (names as separate metadata) | **pnix-clj (guest)** | TARGET — origin/main `cas.clj`에서 PORT |
| append-only event/evidence store; event hash/index; pointer-as-event | **pnix-clj (guest)** | TARGET — origin/main `store.clj`에서 PORT |
| stage tower (stage1..7); snapshot/resolve; purity/determinism | **pnix-clj (guest)** | TARGET — origin/main `stage.clj`/`stm.clj`/`resolve.clj`/`purity.clj`에서 PORT |
| singleton `run-mirror` + trace facets | **pnix-clj (guest)** | NOT YET (오늘: `mirror.clj` + 리포트 레인; Phase D) |
| witness / gate / loss schema | **pnix-clj (guest)** + interop fields | `error.clj` + `interop-meta`에 씨앗; TARGET |

### 분배 시퀀싱 (guest 측, feat-branch)
1. **Interop 강화**: 호스트 객체 opaque-ref 규칙 (`java-object-term` 수정),
   모든 교차에 effect-class, deny-by-default grant.
2. **분리 Phase B/C/E**: 호스트 reflection/eval을 interop 뒤로; compile proof를
   clj-meta에 위임.
3. **PORT CAS** (`cas.clj`) + **event store** (`store.clj`) from origin/main,
   이 브랜치 값 모델과 names-as-metadata 규칙에 맞게.
4. CAS/store 착륙 후 **Singleton `run-mirror`** (Phase D).
5. **Stage tower + snapshot/purity** (PORT `stage`/`stm`/`resolve`/`purity`),
   각각 명시적 증인이 있는 stage.

각 단계는 게이트 녹색을 유지하고 자체 슬라이스로 commit. 무거운 lexer / 큰
리팩터 / PORT 단계는 무인보다 감독하에 하는 것이 좋음.
