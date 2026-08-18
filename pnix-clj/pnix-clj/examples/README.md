# examples — 그냥 실행하기 vs 검사하고 증거 남기기

> **Foundation entry point:** [FOUNDATION_PATH.md](FOUNDATION_PATH.md) and
> [`00-foundation`](00-foundation/README.md). Basic PNIX execution is not
> gated by proof receipts, mirrors, service admission, or owner verdicts.
> Meta-circular compiler/evaluator capability is basic; verification of that
> capability is an independent lane.

The older numbered examples below remain useful as a verification/research
catalog. References to `held`, receipts, gates, or deployment in those examples
must not be read as the semantics of `pnix-clj.core/run-source`: basic language
errors are structured failures, not proof-policy holds.

plain Clojure는 코드를 **바로 실행**하는 쪽에 가깝고, pnix-clj는 코드를 **검사하고, 멈출 이유를 설명하고, 증거를 남기며 실행**하는 쪽에 가깝습니다.

이 문서 안에는 `meta-circular`, `lane`, `receipt`, `witness` 같은 어려운 단어가 나옵니다. 처음에는 단어를 외우지 않아도 됩니다. 아래 입문 문서와 각 예제의 `초딩 설명`만 먼저 보면 됩니다.

처음이면 긴 표를 바로 읽지 말고 아래 문서부터 보세요.

- [START_HERE.md](START_HERE.md) — 10분 안에 전체 감 잡기
- [WORDS.md](WORDS.md) — 어려운 단어를 쉬운 말로 보기
- [BEGINNER_PATH.md](BEGINNER_PATH.md) — 어떤 순서로 볼지
- [REAL_WORLD_USE_CASES.md](REAL_WORLD_USE_CASES.md) — 실무 어디에 붙이는지
- [WHY_AI_DEVELOPMENT.md](WHY_AI_DEVELOPMENT.md) — AI 개발에서 왜 중요한지

이 폴더는 **사람이 직접 코드를 보고 "이 기능을 어디에 쓸지" 판단**하도록 만든 예제 모음입니다.
각 섹션은 pnix-clj의 meta-circular 능력 하나를 다루고, **두 파일을 나란히** 둡니다:

- `limit_clojure.clj` — **plain Clojure의 한계**: `eval`/`load-string`을 "그냥" 쓰면 왜 안 되는지 /
  무엇이 불가능하거나 위험한지.
- `pnix_clj_way.clj` — **pnix-clj로 같은 문제를 어떻게 해결**하는지 (Clojure/clj-meta ↔ pnix 방식).

모든 파일에 **한글 주석**이 있고, 각 섹션 `README.md`가 "무엇을 / 왜 / 어디에 쓰나 / 쉽게 말하면(비유)"을 정리합니다.

> pnix-hy가 Hy(Python)↔pnix라면, pnix-clj는 **Clojure(clj-meta)↔pnix**입니다. 같은 메타서큘러
> 기둥을 pnix-clj의 방식(4-lane 교차검증·게이트·receipt·JVM bytecode)으로 세웁니다.
> Python/Hy는 다루지 않습니다.

## 쉽게 말하면

```text
plain Clojure
= 실행은 되지만, 안전/증거/의미보존/재현성은 직접 다 챙겨야 함

pnix-clj
= 실행하면서 "왜 안전한가, 무엇을 했나, 같은 의미인가, 증거가 뭔가"를 같이 남김
```

초급자에게 `(eval (read-string "(+ 1 2)"))`는 결과 `3`만 준다. pnix-clj 방식은 결과에 더해:

```text
결과는 7이다 · 이 코드는 순수하다 · 파일/네트워크/환경변수를 안 썼다
잔여 프로그램은 이렇다(Futamura) · 4개 substrate에서 같은 값으로 수렴한다
내용주소 해시는 이렇다 · 공백만 다르면 캐시 히트 · JVM bytecode로도 같은 값이다
```
까지 같이 준다. → 이 폴더는 **"그냥 실행"과 "증거·안전·의미보존·재현성까지 포함해 다루기"의 차이**를
실행되는 예제로 보여준다.

## 초딩 버전으로 읽는 법

이전 설명: plain Clojure는 실행은 되지만 안전/증거/의미보존/재현성은 직접 챙겨야 하고, pnix-clj는 실행하면서 증거를 같이 남긴다.

초딩 설명: plain Clojure는 “일단 해 봐!”에 가깝다. pnix-clj는 “하기 전에 위험한지 확인하고, 끝나면 영수증을 붙여!”에 가깝다.

아주 쉽게 비유하면 이렇다.

```text
gate    = 문지기. 위험하면 못 지나가게 막는다.
held    = 잠깐 멈춤. 실패가 아니라 "이유가 있으니 사람이 봐라"라는 뜻이다.
reason  = 멈춘 이유 쪽지.
receipt = 영수증. 어떤 길로 실행했는지 적힌 종이다.
witness = 증인 도장. input/output/effect를 나중에 확인할 수 있게 찍은 도장이다.
hash    = 지문. 내용이 같으면 같은 지문이 나온다.
lane    = 같은 문제를 푸는 다른 길. 여러 길이 같은 답을 내는지 본다.
```

코드를 볼 때는 이렇게 읽으면 된다.

```clojure
;; limit_clojure.clj
;; 그냥 Clojure로 해 본다.
;; 답은 나올 수 있지만, 위험했는지/왜 멈췄는지/나중에 다시 확인 가능한지 잘 모른다.

;; pnix_clj_way.clj
;; pnix-clj에게 "먼저 검사하고, 결과표를 줘"라고 시킨다.
;; :ok    -> 초록불, 통과
;; :held  -> 잠깐 멈춤, 사람이 이유를 봐야 함
;; :reason -> 멈춘 이유
;; :value  -> 나온 답
;; assert  -> "정말 내가 기대한 답이 맞아?" 확인하는 줄
```

처음 읽을 때 추천 순서:

1. `83-ai-generated-config-gate`: AI가 만든 설정을 바로 믿지 않는 법
2. `86-service-option-contract`: 설정표에 빠진 칸/틀린 칸을 잡는 법
3. `84-ci-receipt-matrix`: CI에서 값만 보지 않고 영수증까지 남기는 법
4. `87-plugin-capability-boundary`: plugin이나 agent가 파일을 읽으려 할 때 막는 법
5. `88-refactor-cache-stability`: 공백만 바뀐 코드를 같은 코드로 알아보는 법

각 하위 README에는 `## 초딩 설명`이 따로 있다. 거기에는 `이전 설명:` 바로 밑에 `초딩 설명:`을 붙여서, 어려운 말을 쉬운 말로 다시 풀어 두었다.

## 먼저 용어를 풀면

이 README에서 쓰는 말이 낯설면 아래처럼 읽으면 됩니다.

- `gate`: 실행하거나 반영하기 전에 통과/거부를 판단하는 문. 예: 파일 읽기 요구를 실행 전 `:held`로 막기.
- `held`: 실패가 아니라 “증거 있는 보류”입니다. 왜 보류됐는지 `:reason`이 있습니다.
- `receipt`: 값만이 아니라 어떤 lane들이 어떤 결과를 냈는지 남긴 영수증입니다.
- `witness`: input/output/effect를 content hash로 묶은 증거 레코드입니다.
- `lane`: 같은 프로그램을 보는 다른 실행/검증 경로입니다. evaluator, clj-meta, px-runtime, machine 같은 경로가 있습니다.
- `frontier`: 아직 어떤 lane이 지원하지 않는 기능을 추측하지 않고 `:held`로 남기는 경계입니다.
- `content-address`: source 문자열이 아니라 정본 구조의 hash로 identity를 잡는 방식입니다.

## 현실 적용 지도

처음 볼 때는 번호 순서대로 다 읽지 않아도 됩니다. 필요한 문제에 맞춰 보면 됩니다.

| 현실 문제 | 먼저 볼 예제 | 어떻게 적용하나 |
|---|---|---|
| AI가 만든 config가 안전한지 모르겠다 | `83`, `85`, `86`, `77` | generated source를 적용 전 purity/duplicate/pattern-contract verdict로 분류 |
| PR/CI에서 의미론 regression을 빨리 잡고 싶다 | `84`, `21`, `22`, `57`, `71` | 작은 corpus를 lane receipt matrix로 만들어 값과 held reason을 고정 |
| agent/plugin/tool이 host 권한을 쓰려 한다 | `87`, `23`, `80`, `25` | 기본 deny, 승인된 effect만 witness 달고 실행 |
| formatting/refactor 뒤에도 같은 의미인지 보고 싶다 | `88`, `12`, `30`, `57` | source string 대신 AST/content hash와 fresh cross-check 사용 |
| 서비스 옵션 schema/contract가 필요하다 | `86`, `76`, `82` | Nix pattern lambda로 required/default/ellipsis policy를 코드화 |
| abstract machine/meta-circular 성장분을 보고 싶다 | `61`, `78`, `79`, `81`, `89`, `90` | evaluator와 machine 결과를 비교하고 machine report/fuel/import seam을 확인 |
| 기록, 재현, 감사 로그가 필요하다 | `14`, `16`, `29`, `49`, `84` | event hash-chain, replay witness, durable run receipt로 남김 |

예를 들어 “AI가 Nix-like 설정을 생성하고, CI에서 자동 검토한 뒤, 승인된 것만 배포”하려면 `83 -> 86 -> 84 -> 88` 순서로 보면 됩니다. config 후보를 gate로 분류하고, option contract를 확인하고, CI receipt matrix로 고정한 뒤, refactor noise는 content-address cache로 줄이는 흐름입니다.

## 산업별 응용 예

아래는 “이 기능을 어느 회사/팀 업무에 붙일 수 있나?”를 기준으로 본 적용 지도입니다. 특정 업체 이름보다 업무 도메인으로 보면 됩니다.

| 도메인 | 실제 문제 | 볼 예제 | 적용 코드 모양 |
|---|---|---|---|
| SaaS platform/SRE | tenant별 config를 AI가 생성했는데 바로 배포해도 되는지 판단 | `83`, `85`, `86` | `{:source generated-config :gate [:pure? :status :reason] :action :approve-or-review}` |
| fintech/regtech | 계산 규칙, 위험 scoring rule, 감사 로그를 나중에 재현해야 함 | `14`, `16`, `21`, `29`, `84` | `{:rule-id id :receipt receipt :snapshot snapshot :replayable? true}` |
| AI coding agent | agent가 파일/환경변수/tool을 읽으려 할 때 권한을 분리 | `01`, `23`, `80`, `87` | `{:effect :file-read :granted #{:pure} :decision (:status crossing)}` |
| DevTools/CI | parser/lowering/runtime 변경이 semantic regression인지 판정 | `22`, `57`, `71`, `84` | `{:case source :direct direct :lowered lowered :same? (= direct lowered)}` |
| Build/release engineering | compile artifact와 JVM/classfile/runtime identity를 추적 | `34`, `54`, `70`, `71` | `{:artifact artifact :hash hash :trust-boundary trust :status status}` |
| Data/ML platform | 사용자 UDF나 feature transform을 안전하게 평가 | `01`, `23`, `38`, `43` | `{:udf source :pure? pure? :fuel fuel :verdict verdict}` |
| Enterprise plugin system | plugin이 host object나 host API를 다룰 때 격리 | `31`, `80`, `87` | `{:plugin plugin-id :opaque-ref ref :witness witness}` |
| Compiler/DSL team | DSL 최적화, partial evaluation, machine lane을 검증 | `03`, `33`, `48`, `61`, `78`, `79` | `{:source source :direct direct :specialized residual :machine machine}` |
| Migration/runtime rewrite | 기존 runtime과 새 runtime이 같은 값을 내는지 비교 | `20`, `40`, `72`, `81` | `{:old old-result :new new-result :collapse? same? :frontier reason}` |
| Compliance/internal audit | 사람이 읽는 report와 코드 상태가 drift 나지 않게 유지 | `58`, `59`, `73`, `84` | `{:report-kind kind :status status :count count :hash report-hash}` |

실무에서는 각 예제의 `assert`를 그대로 복사해 테스트에 넣기보다, 아래처럼 row를 만든 뒤 CI artifact, PR comment, DB audit row로 저장하는 쪽이 자연스럽습니다.

```clojure
{:domain :ai-generated-deployment-config
 :source-id :pr-142-config-candidate
 :source generated-source
 :checks {:purity purity-row
          :eval eval-row
          :receipt receipt-row}
 :decision (if (= :ok (:status eval-row))
             :auto-approve
             :manual-review)
 :reason (:reason eval-row)}
```

읽는 순서는 `limit_clojure.clj`에서 위험한 plain pattern을 먼저 보고, `pnix_clj_way.clj`에서 그 위험을 어떤 `:status`/`:reason`/hash/receipt로 바꾸는지 보면 됩니다. 각 섹션 README의 `코드 해설`은 이 관점으로 두 파일을 읽도록 주석을 붙여 둔 것입니다.

## 핵심 대비 (한 줄 요약)

| 섹션 | plain Clojure의 한계 | pnix-clj |
|---|---|---|
| `01-pure-sandbox` | `eval`은 부작용·무한루프·자원소모를 막지 못함 | 순수성 정적판정 + fuel 한계로 **신뢰 가능한 샌드박스** (`safe-eval`) |
| `02-four-lane-receipt` | 값은 얻지만 다중 substrate receipt/collapse가 없음 | `run-source`가 evaluator/clj-meta/px-runtime/pnix-mirror receipt와 cross-lane verdict를 남김 |
| `03-specialization-futamura` | 부분입력에 특화된 잔여 프로그램을 만들 수 없음 | `specialize` = **Futamura 1차 사영**(잔여 pnix 코드 생성 + JVM bytecode 투영) |
| `04-host-interop-loss-effect` | host crossing은 효과·손실·권한 증거가 흐릿함 | `interop` = effect/loss/capability/witness가 붙은 Clojure↔pnix crossing |
| `05-witness-and-gate` | 값/예외는 있지만 gate verdict와 witness가 없음 | `safe-eval` + `interop` = held/ok verdict와 witness hash |
| `06-ast-lowering-roundtrip` | read/eval은 AST·lowering·compile receipt를 남기지 않음 | parser/lowering/clj-meta로 AST hash·form hash·host proof result를 남김 |
| `07-clojure-macro-over-pnix` | macroexpand/eval은 값은 만들지만 pnix projection/tower/witness가 없음 | macroexpand 결과를 pnix source로 synthesize하고 tower witness로 검증 |
| `08-clojure-reader-or-edn-embed-pnix` | EDN reader는 tagged literal을 데이터로 읽지만 pnix 검증/witness가 없음 | `#px` source를 데이터로 읽은 뒤 parse/purity/tower witness로 검증 |
| `09-clojure-form-fixture` | form eval은 host 값만 만들고 projection fixture receipt가 없음 | `clojure-form` fixture가 host eval, clj-meta, projection validation을 비교 |
| `10-reverse-synthesis` | Clojure form을 pnix source로 의미보존 투영하는 whitelist가 없음 | `synthesize/form->pnix`가 허용 form만 투영하고 tower로 검증 |
| `11-self-hosting-convergence` | 자기 언어 구현과의 수렴 증명 없음 | `run-tower` = 한 소스를 **4 substrate에서 평가·collapse**(자기호스팅 타워) |
| `12-content-addressed-cache` | `memoize`는 표현 기반(공백 다르면 미스) | `cached-eval` = **정본 내용주소** 캐시(공백/괄호 변형이 한 엔트리) |
| `13-canonical-term-store` | 문자열/hash identity는 alpha-equivalence를 모름 | `cas`가 canonical form, term hash, structural confirmation을 분리 |
| `14-append-only-event-log` | atom/vector 로그는 수정·오염·chain 검증을 막지 못함 | `store`가 pure EDN event만 append하고 hash-chain을 검증 |
| `15-runtime-snapshot` | cache/result가 어느 runtime에서 나온 값인지 pin이 없음 | `snapshot`이 evaluator/JVM/classpath version을 pin하고 mismatch를 held 처리 |
| `16-witness-replay` | 저장된 값은 fresh replay verdict가 아님 | `replay-witness`가 persisted source/witness를 재실행해 reproduced/diverged/missing을 구분 |
| `17-structural-search` | text search는 구조 후보와 equivalence를 섞음 | `search`가 skeleton/free-vars/distance 후보와 CAS confirmation을 분리 |
| `18-property-fuzzer` | hand-picked sample은 counterexample shrink가 없음 | `property-fuzzer`가 generated source로 lane/cache/specializer/machine property를 검사 |
| `19-lowered-compiled-runtime` | eval 값과 compiled/evidence 경로가 기본 연결되지 않음 | direct evaluator와 lowered clj-meta compiled path를 receipt로 비교 |
| `20-mirror-chain` | 반복 실행 안정성을 event chain으로 남기지 않음 | `mirror-chain!`이 temporal convergence와 drift event를 기록 |
| `21-determinism-audit` | 단일 eval 성공은 hash stability 증거가 아님 | `determinism/report`가 corpus를 반복 parse/eval해 AST/result hash 안정성을 확인 |
| `22-translation-validation` | compile/run 성공은 per-candidate validation이 아님 | validator catalog가 parse/lowering/compile/px/mirror row를 분리 |
| `23-capability-gate` | host capability 호출은 가능하지만 허용/거부 verdict가 없음 | purity/capability gate가 host-effect 요구를 실행 전 `:held` verdict로 남김 |
| `24-phase-separation` | read/eval/host-effect가 한 덩어리로 섞이기 쉬움 | parse/purity/direct-eval/lowering/compiled-path/capability-gate를 phase별 verdict로 분리 |
| `25-typed-attestation` | host call은 값은 만들지만 typed capability/witness attestation이 없음 | 허용/거부 crossing 모두에 schema/effect/loss/input/output/witness hash를 남김 |
| `26-arithmetic-proof` | finite numeric samples는 산술 동치 proof가 아님 | `arith-proof`가 canonical polynomial로 산술 fragment 동치를 증명 |
| `27-boolean-proof` | 일부 boolean sample은 전체 truth table proof가 아님 | `bool-proof`가 boolean fragment를 exhaustive truth table로 proven/refuted 처리 |
| `28-generate-and-cegis` | 후보를 손으로 고르면 refinement/proof boundary가 없음 | `generate` + `cegis`가 observational 후보, counterexample, arith proof upgrade를 연결 |
| `29-witnessed-durable-run` | 저장된 실행값은 admission된 evidence spine이 아님 | `run-witnessed-durable`이 term/snapshot/tower/mirror/purity/witness/persistence를 묶음 |
| `30-verifying-cache` | `memoize` hit를 fresh eval/purity/key와 대조하지 않음 | content-addressed cache hit를 fresh evaluation, purity verdict, cache key로 검증 |
| `31-compartment-isolation` | host object를 직접 넘기면 identity/mutation/access 경계가 없음 | host object를 opaque ref로 격리하고 release 이후 deref를 `:held` verdict로 만듦 |
| `32-value-roundtrip` | `pr-str`/`read-string`은 pnix value projection receipt가 아님 | `value-roundtrip`이 pnix value→Clojure form→value closure를 확인 |
| `33-futamura-ladder` | eval/compile은 되지만 Futamura projection ladder evidence가 없음 | direct interpreter, 1st projection, 2nd generating extension, cogen-free 3rd route를 구분해 비교 |
| `34-classfile-trust-receipt` | eval 성공은 classfile hash/trust boundary를 남기지 않음 | `classfile-receipt`와 `trust`가 JVM artifact identity와 common-mode risk를 receipt화 |
| `35-stage-tower-internals` | eval 값은 있지만 self-hosting tower layer/pair/witness가 없음 | tower 내부 layers, adjacent pairs, collapse witness, held blocking point를 전시 |
| `36-clojure-projection-report` | host 값은 만들지만 projection runtime/host crossing report가 없음 | `clojure-projection` fixture가 projection runtime, accepted rows, host crossing을 검증 |
| `37-emit-form-roundtrip` | macroexpand/eval은 analyzer emit-form 왕복 증거가 아님 | `emit-form-roundtrip`이 analyzer AST -> emitted form -> value equality를 확인 |
| `38-form-analysis-host-surface` | eval은 host call을 실행 전 분류하지 않음 | `form-analysis`가 tools.analyzer AST로 pure core와 host interop surface를 분리 |
| `39-forward-reference-lift` | Clojure `let`은 forward reference contract/lane 증거가 없음 | `forward-reference` corpus가 lift 성공과 semantic error를 lane별로 고정 |
| `40-mirror-error-alignment` | try/catch는 multi-lane error frontier alignment가 아님 | `mirror-error`가 evaluator/runtime/mirror error reason과 phase를 정렬 |
| `41-coverage-report` | sample eval은 evaluator coverage evidence가 아님 | `coverage`가 fixture corpus의 op/builtin/operator/branch coverage를 측정 |
| `42-grammar-fuzzer` | 랜덤 eval은 seed/expected verdict/lane summary가 없음 | `grammar-fuzzer`가 generated positive/error source를 `run-source` gate와 비교 |
| `43-strict-audit` | truthiness 실행값은 strict Nix typing frontier가 아님 | `strict-audit`가 behavior change 없이 strict-ok/held/gate 결과를 분리 |
| `44-purity-event-spine` | 반복 eval은 snapshot-pinned event chain이 아님 | `purity-check!`가 rerun determinism을 `:purity/run` event로 기록 |
| `45-reflection-host-lane` | raw reflection은 stable host-lane identity가 아님 | `reflect`가 JVM/classpath/Var metadata를 pure EDN snapshot/hash로 pin |
| `46-self-mod-gate` | Var mutation은 witness/owner gate 없이 즉시 바뀜 | `self-mod-gate`가 admitted witness도 기본 `:held`로 막고 event trail을 남김 |
| `47-self-improve-review-queue` | 후보 ranking은 witnessed owner-held queue가 아님 | `self-improve`가 후보를 적용하지 않고 ranked review queue와 round event로 남김 |
| `48-weval-partial-eval` | interpreter는 매번 tag dispatch를 수행함 | `weval`이 interpreted dispatch와 residual dispatch-free path를 비교 |
| `49-persistent-evidence-store` | file IO는 content-address/hash-chain integrity가 없음 | `persist`가 term/event를 저장하고 reload integrity를 검증 |
| `50-stage7-core-lockins` | `load-string`은 stage lockin fixture receipt가 아님 | `stage7-core`가 self-hosting core lockin fixture를 `pnix/report`로 고정 |
| `51-import-module-fixture` | file IO는 in-memory import fixture/lane receipt가 아님 | `import-module` fixture map을 모든 lane에 전달해 `import ./m`을 검증 |
| `52-static-oracle-corpus` | 손 expected map은 Nix oracle provenance가 없음 | captured Nix oracle resource를 `pnix/report` sample에 태워 검증 |
| `53-rust-grounded-slice` | sample eval은 Rust corpus provenance/hash가 없음 | `rust-batch` manifest/inventory와 첫 fixture cross-lane receipt를 확인 |
| `54-report-artifact-materialization` | `spit`은 report registry artifact가 아님 | `report-artifact`가 supported kind를 versioned hashed EDN 파일로 씀 |
| `55-stage15-control-plan` | command list는 backend hash/owner gate가 없음 | `stage15/control-plan`이 외부 실행 없이 held read-only plan을 전시 |
| `56-px-runtime-run-plan` | file scan은 px runtime import graph 검증이 아님 | `px-runtime/runtime-run-plan`이 boundary, entry parse, import edges를 점검 |
| `57-parser-unparse-roundtrip` | `pr-str` roundtrip은 pnix grammar/AST 동치가 아님 | parser/unparse/reparse가 위치 metadata 제외 구조 동치를 확인 |
| `58-capability-index` | 손 목록은 구현 API/report와 drift함 | `capabilities/index`가 builtins, report kinds, public API를 코드에서 읽음 |
| `59-lane-registry-policy-map` | 파일 목록은 lane policy map이 아님 | `lane-registry`가 namespace별 `lane-classification`을 registry row로 렌더 |
| `60-guest-surface-registry` | builtin 추가는 guest surface diff가 아님 | `guest_surface.edn`이 real-Nix captured surface와 pnix extensions를 고정 |
| `61-abstract-machine-lane` | hand interpreter는 derived machine lane 증거가 아님 | `machine/eval-source`가 evaluator와 수렴하고 unsupported op를 held 처리 |
| `62-hash-json-codec` | `pr-str`/`hash`는 JSON codec과 stable data hash가 아님 | `json`과 `hash`가 deterministic JSON/value hash surface를 제공 |
| `63-structured-error-envelope` | exception string은 shared error schema가 아님 | `error/held`가 phase/reason/schema가 있는 held envelope를 생성 |
| `64-receipt-verdict-frontier` | result vector는 lane verdict/frontier 규칙이 없음 | `receipt/verdict`와 `summarize`가 cross-lane 판단을 표준화 |
| `65-version-math-helpers` | lexicographic compare와 `/`는 Nix helper semantics가 아님 | `version`/`math` helper가 compareVersions/parseDrvName/int-div surface를 제공 |
| `66-repl-rendering` | `pr-str`는 pnix REPL value renderer가 아님 | `repl/render`와 `eval-print`가 pnix 값 출력 정책을 공유 |
| `67-nrepl-middleware-dry-run` | Clojure eval은 pnix nREPL lane routing이 아님 | `nrepl/wrap-pnix-eval`을 fake transport로 검증 |
| `68-stage15-execution-dry-run` | fake command map은 stage15 execution receipt가 아님 | `stage15/execute-plan`을 fake runner로 외부 실행 없이 검증 |
| `69-tiny-benchmark-report` | `time` 출력은 semantic benchmark report가 아님 | `benchmark/run-benchmark`가 preflight와 measurement lanes를 report화 |
| `70-core-compile-pipeline` | `eval`은 parse/lower/compile receipt가 아님 | `core/compile-source`가 source hash, lowered form, clj-meta receipt를 묶음 |
| `71-clj-meta-lowered-eval` | Clojure eval은 clj-meta compile/eval evidence가 없음 | lowered form을 `clj-meta/eval-lowered`로 실행해 compile receipt를 확인 |
| `72-mirror-facet-rows` | 값 하나는 mirror facet row가 아님 | `run-source` receipt에서 clojure/px/pnix/cross mirror row를 읽음 |
| `73-wiki-integrity-index` | 손 checklist는 capability wiring integrity가 없음 | `wiki` data API가 capability registry, roadmap, integrity를 확인 |
| `74-parser-lowering-cache-stats` | 반복 read/transform은 cache hit evidence가 아님 | parser/lowering cache stats가 hit/miss/entry를 노출 |
| `75-clojure-projection-host-crossing` | `read-string`/`eval`은 projection host crossing witness가 아님 | projection host API가 reader/eval crossing에 interop/capability/witness를 붙임 |
| `76-pattern-lambda-nix-parity` | Clojure destructuring은 Nix pattern lambda receipt가 아님 | D19 lazy recursive defaults, ellipsis, @as, held reasons를 3-lane receipt로 확인 |
| `77-dynamic-attr-key-strictness` | Clojure map assoc은 collision/type error를 모름 | D20 dynamic attr duplicate/non-string key를 held reason으로 고정 |
| `78-machine-default-env-builtins` | hand interpreter는 builtins env를 직접 복제해야 함 | M7c machine이 evaluator default-env/builtins boundary와 수렴 |
| `79-machine-pattern-lambda-native` | Clojure destructuring은 derived machine pattern-bind가 아님 | M7d machine이 pattern lambda를 native control로 실행하고 evaluator와 수렴 |
| `80-interop-opaque-host-ref` | host object를 직접 넘기면 capability/witness/lifecycle이 없음 | `interop`이 deny-by-default crossing과 opaque host ref release gate를 제공 |
| `81-machine-dynamic-attr-frontier` | loose interpreter는 dynamic key collision을 overwrite로 숨김 | M7e machine이 dynamic attr key를 native로 실행하고 evaluator와 수렴 |
| `82-tryeval-pattern-uncatchable` | `try/catch Throwable`은 Nix tryEval taxonomy가 아님 | throw/assert는 catch, pattern application error는 uncatchable held로 분리 |
| `83-ai-generated-config-gate` | AI config를 실행/merge 뒤에야 위험을 봄 | generated config를 purity/eval/D20 verdict로 적용 전 triage |
| `84-ci-receipt-matrix` | CI eval pass/fail은 lane/provenance가 없음 | 작은 corpus를 `run-source` receipt와 held reason matrix로 고정 |
| `85-generated-config-merge-collision` | generated merge는 key collision을 overwrite로 숨김 | dynamic attr collision을 `:duplicate-attr` review verdict로 보존 |
| `86-service-option-contract` | Clojure destructuring은 option typo를 default로 숨김 | pattern lambda로 required/default/ellipsis contract를 표현 |
| `87-plugin-capability-boundary` | plugin/tool call은 host 권한을 바로 씀 | source gate와 interop witness로 deny-by-default 권한 경계를 둠 |
| `88-refactor-cache-stability` | source-string cache는 formatting refactor에 취약함 | canonical AST content hash로 같은 의미의 refactor를 같은 cache entry로 봄 |
| `89-machine-path-import-seam` | path/import를 문자열 lookup으로 흉내 내면 resolver 경계가 없음 | M7f machine이 path literal과 import resolver seam을 명시적으로 처리 |
| `90-machine-report-fuel-witness` | 손 report는 shared corpus/stack/fuel 증거가 없음 | M7g `machine/report`와 fuel bound를 regression artifact로 확인 |
| `91-machine-report-artifact-gate` | machine report 모양의 파일은 gate capability 증거가 아님 | M7g `:machine` report artifact를 registry/gate 경로로 materialize |
| `92-machine-property-fuzzer-lane` | sample 몇 개 통과는 random machine agreement가 아님 | M7h `property-fuzzer`가 machine⇄evaluator exact agreement를 fifth property로 검사 |
| `93-live-oracle-differential` | 손 셸아웃은 구조화된 matched/mismatched 집계가 없음 | `live-oracle`이 실제 `nix-instantiate`와 소스별로 직접 값을 비교(없으면 구조화된 skip) |
| `94-mirror-pair-corpus-report` | 199개 소스를 손으로 4-레인 비교할 수 없음 | `mirror-pair`가 committed 코퍼스 전체의 4-레인(evaluator/clj-meta/px-runtime/pnix-mirror) 수렴을 집계 |

각 pnix-clj 능력은 테스트 스위트와 runnable example로 회귀 고정되어 있습니다.

## 실행법

pnix-clj 디렉터리(이 파일의 상위)에서, 예제 스크립트를 프로젝트 classpath로 실행:

```sh
cd pnix-clj                                   # deps.edn 있는 곳
clojure -M examples/01-pure-sandbox/pnix_clj_way.clj
clojure -M examples/01-pure-sandbox/limit_clojure.clj
```

또는 flake 개발셸에서 (`nix develop`; JDK + clojure + clj-meta on PATH):

```sh
nix develop
clojure -M examples/11-self-hosting-convergence/pnix_clj_way.clj
```

## pnix-hy 예제와의 매핑

pnix-hy의 35개 기본 섹션은 pnix-clj 쪽에서도 **Clojure/JVM/clj-meta 역할 대응물**로 채워져 있고,
36-50번은 pnix-clj 고유의 report/fixture/audit lane, 51-61번은 registry/runtime-plan/abstract-machine lane,
62-69번은 core utility와 developer control-surface lane, 70-74번은 compile/mirror/wiki/cache diagnostic lane,
75번은 Clojure projection host crossing lane, 76-82번은 최근 M7/D19/D20/interop 성장분,
83-88번은 AI/config/CI/plugin/cache 같은 현실 개발 적용 시나리오, 89-92번은 M7f/M7g/M7h machine 성장분을 추가로 전시합니다.
호스트-무관한 메타서큘러 기둥(sandbox·Futamura·self-hosting·cache·capabilities·phase separation·
tower internals·synthesize)은 pnix-clj 방식으로 매핑됩니다.

다만 pnix-clj는 Hy/Python식 복제가 아니라 Clojure/JVM/clj-meta 방식으로 재해석합니다.

- Hy macro / Python import-hook 방향이 아니라 Clojure namespace, Var, clj-meta, JVM host path를 기준으로 봅니다.
- host interop은 무증거 실행이 아니라 capability/gate/witness/receipt를 통해 다룹니다.
- cache는 단순 memoization이 아니라 content-address + fresh verification + purity verdict로 다룹니다.
- self-hosting은 collapse 결과뿐 아니라 tower layer/pair/witness까지 전시합니다.

Hy 고유 섹션(macro/reader/import-hook/host-introspection 등)은 이름을 복제하지 않고,
Clojure macroexpand, EDN/tagged literal, namespace/form fixture, JVM classfile receipt처럼
pnix-clj의 투영 상대(Clojure/JVM/clj-meta)에 맞는 기능으로 번역합니다.
Python/Hy interop은 pnix-clj의 길이 아닙니다.
