# pnix-clr 알려진 버그 / 제한사항 / 의도된 제외

목적: "이거 고쳐야 하나?"라는 질문에 빨리 답하기 위한 문서. 여기 적힌
항목은 두 종류로 나뉜다 — **의도적으로 admit 안 한 것**(버그 아님, 실수로
"고치지" 말 것)과 **실제로 알려진 제한/버그**(외부 의존성 등으로 아직
못 풀었지만, 진짜 제한). 각 항목이 어느 쪽인지 제목에 명시했다.

관련: [`IMPLEMENTATION.md`](IMPLEMENTATION.md)(뭐가 built됐는지),
[`TODO.md`](TODO.md)(지금 집을 수 있는 작업), [`PLANS.md`](PLANS.md)(아직
방향 미정인 것).

## 공통 규칙 — Stage15/N·Trusting-Trust 주장 금지 (의도적, 버그 아님)

여러 원본 문서(`SCOPE_LOCK.md`, `CLOJURE_CLR_ADMITTED_SURFACE.md`,
`IN_PROCESS_EVAL.md`, `IMPLEMENTATION.md` §3)에 반복해서 나오던 규칙을
여기 한 번만 정리한다: 이 호스트의 어떤 조각도 — `clr-meta` tool-eval,
`bin/clojure-clr` facade, in-process C# 평가기 스파이크, evaluator
generation 0/1/2 등 — **그 자체만으로** Compiler Stage15/N,
self-reproduction, IL fixed point, 또는 Trusting-Trust를 증명한 것으로
주장하지 않는다. 이건 각 기능의 "여전히 진행 중"인 상태를 정직하게
유지하기 위한 전역 가드레일이다. `clr-meta` 쪽 정확한 Stage 진행 상태는
`clr-meta/STATUS.md` / `clr-meta/STAGE15_N_ROADMAP.md` 참고(Stage1–N +
self-reproduction 게이트는 `promotion/allowed?=false`로 닫힘. 일반 IL
fixed point / host promotion은 open).

## 1. 의도적으로 범위 밖(버그 아님) — `SCOPE_LOCK.md` "범위 밖" 원문 기반

아래는 전부 **이건 버그 아니라 의도된 제한**이다. 필요해지면 나중에
`SCOPE_LOCK.md`(→ 이제 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §5)의
admit 절차를 따라 새로 admit해야 하는 것들이지, 지금 코드에서 놓친 게
아니다.

- JVM classfile, ASM, Java reflection, Maven/JAR execution, 또는 JVM
  fallback 없음
- 어떤 sibling corpus 트리에서든 portable PNIX semantics를 복사해오는 것
  없음
- basic execution에 대한 service admission, deployment policy, 또는
  proof receipt 없음
- Hangul/NL/dictionary/agent/domain 콘텐츠 없음
- 완전한 mature JVM-host parity, IL fixed-point self-hosting, 게이트가
  존재하기 전 established tri-host membership 주장 없음
- 이 **제품**은 Compiler Stage2–15/N / compiler self-reproduction /
  일반 CLR IL fixed point를 promotion하지 않는다. `clr-meta` 쪽 해당
  게이트는 `promotion/allowed?=false`로 닫혀 있다
  (`../clr-meta/STATUS.md`). 제품 러너가 그 사다리를 컴파일러로 쓰지는 않음
- broad ClojureCLR language/command/runtime/ecosystem compatibility 또는
  교체 없음; `bin/clojure-clr`는 현재 generation 2를 통한 focused `-e`와
  single-file profile만 admit하며 explicit bootstrap trust root가 host
- standalone source-free distribution 없음; launch validation은 여전히
  live plan과 source closure에 바인딩되고, AOT execution은 pinned
  runtime을 유지
- PNIX common compiler/PIR integration 또는 CLR host promotion 없음
- BigInt arithmetic 또는 Int64 + finite Double을 넘는 full numeric
  promotion 없음
- `pnix.primitive-abi.v1` manifest routing/enforcement, production-evaluator
  primitive-manifest enforcement, 또는 full-builtin manifest enforcement
  없음
- production effect request/resume, finite-fuel suspension,
  common-machine replacement, 또는 canonical-result/JCS completion 없음
- Nix UTF-8 byte-string model, pattern lambda, 또는 store purity 게이트
  없음 — 단, **float literal, `with`, list/attrset structural `==`,
  language `assert`, `inherit`/`inherit (expr)`, string-context
  propagation(`appendContext`/`getContext`/`hasContext`/
  `unsafeDiscardStringContext`/`unsafeDiscardOutputDependency`),
  `derivation`/`derivationStrict`/`placeholder`는 이후 admit되어 이미
  동작한다**(위 목록에 넣지 말 것 — `IMPLEMENTATION.md` §5 admit된 것
  목록 참고; string-context/derivation의 pure-simulation 범위는 §6 참고)

## 2. CLI 허용 표면 — 금지된 지름길(의도적, 버그 아님)

`bin/clojure-clr` / `bin/clr-meta` 관련해서 하지 않기로 한 것들
(`CLOJURE_CLR_ADMITTED_SURFACE.md` 원문). 자세한 admitted 표면은
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §6 참고.

- 전체 ClojureCLR를 암시하도록 `clojure-clr` 이름을 바꾸지 않는다.
- facade `-e`만으로 Stage15/N 또는 Trusting-Trust를 주장하지 않는다
  (위 "공통 규칙" 참고).
- 명시되지 않은 profile에서 Rhino sdk_8과 pnix-clr net10을 혼합하지
  않는다(§7 TFM 정책 참고).
- `bin/clojure-clr`는 REPL, `-i`, `-M`, deps.edn, clojure CLI 패리티를
  admit하지 않는다 — 요청하면 stderr + exit 2로 fail closed(우회하려고
  하지 말 것; `clojure-clr-bootstrap`가 그 용도의 별도 entrypoint다).

## 3. 프로세스 내 평가기 스파이크 — 비목표(의도적, 버그 아님)

`IN_PROCESS_EVAL.md` 원문. 자세한 내용은
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §8 참고.

- 첫 스파이크에 nuget.org 요구하지 않는다(로컬 export 레이아웃으로
  충분).
- `clojure-clr` facade 또는 bootstrap multi-ns 스토리를 교체하지 않는다.
- 임의 사용자 Clojure 프로젝트를 프로세스 내에 로드하지 않는다.
- (옵션 C) ClojureCLR 없이 CLR에서 pnix를 pure managed로 재구현하지
  않는다 — 두 번째 의미 소스가 되고 host-bound 제품 교리에 위배되기
  때문에 **현재 거부**.

## 4. 프로세스 내 평가기 스파이크 — 알려진 제한(진짜 제한, blocked)

이 항목은 "의도된 제외"가 아니라 **외부 의존성 때문에 지금은 못 푸는
진짜 제한**이다. 나중에 조건이 바뀌면(아래 참고) 다시 볼 것.

- **Collectible isolated ALC 언로드 안 됨 — 현재 blocked.** ClojureCLR
  guest AOT가 `Assembly.Load`로 **기본** 컨텍스트에 초기화된다;
  collectible ALC는 이미 로드된 substrate 타입을 dual Resolving 없이 볼
  수 없고, dual Resolving은 Default로 붕괴한다. 문서화된 tradeoff다 —
  **ALC-aware load를 지원하는 substrate가 나오기 전까지는 재검토하지
  않는다.** (`csharp/Pnix.Clr/InProcessEval.cs`)
- **net8 호스트에서는 프로세스 스폰만 유지된다** — in-process API는
  net10.0+ 전용이라 net8 전용 host-main C#은 `Eval.Source`/`Eval.File`
  (process-spawn)만 쓸 수 있다. 이건 TFM 정책(§7)상 당연한 결과이자 현재
  제한이다.
- **Reentrancy: 직렬화, multi-threaded 아님** — `eval-source` 주변에
  global lock이 걸려 있다(ClojureCLR RT가 process-wide이기 때문). 동시
  호출자는 대기하며 `*Async` 헬퍼도 같은 lock을 공유한다. 병렬 처리량이
  필요한 워크로드에는 안 맞는다 — 의도된 설계(ClojureCLR RT 특성)이지만,
  "여러 스레드에서 in-process eval을 동시에 빠르게 돌릴 수 있다"고
  기대하면 안 된다는 뜻에서 여기 기록해둔다.

## 5. 알려진 코드 차이점(설계 특성, 버그 아님)

다른 4개 호스트와 다르게 동작하는 부분들은 이미
[`IMPLEMENTATION.md`](IMPLEMENTATION.md) §3에 정리돼 있다 — path 값
타입, `import`/`scopedImport` 예약 키워드, 아티팩트 기반 배포, evaluator
generation vs compiler stage 축 분리. "clr에서만 다르게 동작하네?"
싶으면 여기부터 확인할 것 — 대부분 버그가 아니라 이 호스트의 설계
선택이다.

## 6. string-context / derivation — pure-simulation 범위(의도적, 버그 아님)

2026-08-20 admit. `builtins.appendContext`/`getContext`/`hasContext`/
`unsafeDiscardStringContext`/`unsafeDiscardOutputDependency`와
`builtins.derivation`/`derivationStrict`/`placeholder`(pnix-clj를
1차 오라클로, 같은 Clojure 계열인 pnix-cljs의 이미 끝난 포트를 2차
참고로 삼아 이 호스트의 `{:pnix/type ...}` 태그드 맵 관례로 이식함 —
`{:pnix/type :string-context :value "..." :context [...]}`, `evaluator.clj`
`ctx-string`/`ctx-string?` 근방). 아래는 전부 오라클(pnix-clj)도 똑같이
갖고 있거나, 값 모델 자체의 근본적 한계라 여기서 풀 수 없는 것들이다 —
"고쳐야 하나?"의 답은 "아니다".

- **`d.out == d` self-reference 없음.** 진짜 Nix에서 derivation 값의
  `d.out`은 `d` 자기 자신(순환 참조)이지만, 이 호스트를 포함한 모든
  Clojure 계열 host의 값 모델은 순수 불변 맵이라 순환을 표현할 수
  없다. `derivation`이 반환하는 `d.<output>` 서브 attrset은 `type`/
  `name`/`drvPath`/`outPath`/`outputName`만 담은 축약된, 비순환
  attrset이다(오라클과 동일한 타협).
- **pseudo-hash, 진짜 Nix 스토어 해시 아님.** `derivation-hash-hex`(및
  `derivation-paths`)가 만드는 `/nix/store/<32-hex>-<name>` 경로는
  deep-forced 입력 attrset의 정렬된 canonical 표현을 SHA-256으로 해싱한
  것 — 결정적이고 이 호스트 안에서 내부 일관성은 있지만, 실제 Nix의
  ATerm 기반 store-path 해싱 알고리즘과 byte-compatible하지 않다(순수
  시뮬레이션, 처음부터 그렇게 설계됨). 다른 호스트와도 바이트 단위로
  같을 필요 없음 — 실제로 우연히 일치하는 경우가 있는데(단순 입력
  attrset일 때 정렬된 맵의 `pr-str` 표현이 같아서) 이건 우연이지 보장이
  아니다.
- **`appendContext`/`getContext`는 WHICH 의존성 + 어떤 kind(path/
  allOutputs/outputs)까지만 추적한다.** 진짜 store-derivation 그래프
  (실제 빌드 依존관계, output 유효성 등)는 없음 — 순수 값 레벨 시뮬레이션.
- **fail-closed 게이트(`ctx-string-in-args?`)는 shallow scan이다** —
  최상위 인자 + 벡터 인자 한 겹까지만 검사(`exec-builtin`의 인자
  리스트). unforced thunk 뒤에 숨은 contextful string은 이 스캔을
  통과한다 — 예를 들어 `sort`/`filter`에 넘긴, 아직 강제되지 않은 list
  element 안의 contextful string은 안 잡힌다. 이건 버그가 아니라
  **오라클(pnix-clj)의 실제 동작을 그대로 재현한 것**이다(오라클도
  shallow scan) — 더 엄격한 recursive scan을 만들면 오히려 오라클과
  달라진다. 자세한 근거는 `evaluator.clj`의 `ctx-string-in-args?`
  docstring 참고.
- **canonical 출력 경계(`realize-value`, CLI JSON)에서 context는 항상
  버려진다** — content만 남는다. 이건 시뮬레이션 한계가 아니라 진짜
  Nix `--json` 출력도 마찬가지(문자열 context는 애초에 JSON으로 표현할
  방법이 없다). 평가 도중에는 context가 계속 추적/게이트되고, 이 경계
  에서만 의도적으로 벗겨진다.
