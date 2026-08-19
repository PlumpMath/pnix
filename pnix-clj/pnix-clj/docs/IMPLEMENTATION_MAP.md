# pnix-clj 구현 맵

목적: 이 호스트에 뭐가 어떻게 구현돼 있는지 한눈에 찾아보는 문서. 같은 걸
중복해서 다시 만들지 않도록, 뭔가 다르게 동작하는 걸 발견했을 때 "진짜
버그인지 원래 그런 건지" 빨리 판단할 수 있도록, 다른 4개 호스트와 비교할
때 참고하도록 만들었다. 2026-08-19 작성 시작 — 이후 구조가 바뀌면 여기도
같이 갱신할 것.

## 1. 아키텍처 개요 — 어디서 뭘 찾아야 하는가

`src/pnix_clj/` 아래: `parser.clj`(렉서+파서 같이 있음), `evaluator.clj`
(값 표현 + 평가기 + 빌트인, 5개 호스트 중 제일 방대함 — clj-meta 프로덕션
증명까지 같이 하는 저장소라 다른 호스트보다 많은 걸 검증한다), `core.clj`
(`eval-source`/`eval-file` 등 최상위 진입점, import 리졸버들), `capabilities.clj`
(빌트인 목록 자동 문서화).

- **값 표현**: 대부분 Clojure 네이티브 타입을 그대로 씀(정수는 `long`,
  attrset은 `map`) + `PxPath`/`ctx-string` 같은 소수의 전용 타입. 진짜
  Path 값 타입 있음(clr/hy와 같은 부류).
- **환경/스코프**: `default-env`가 기준. `eval-ast*`가 메인 평가 함수.
- **지연 평가**: thunk 기반, `force-value`/`realize-value`로 강제.
- **빌트인 dispatch**: `builtin` 헬퍼로 이름→arity 등록(예:
  `"import" (builtin :import 1)`), 실제 실행은 큰 `case`(evaluator.clj
  4500줄대 근방). 다른 호스트보다 훨씬 세분화돼서 여러 지점에 빌트인
  로직이 흩어져 있다(`finish-extra-builtin` 같은 헬퍼로 일부 위임).
- **import (2026-08-19 이전엔 절반만 구현돼 있었음, 지금은 완성)**:
  파서는 `import <path>`(전용 AST) **와** `builtins.import`(진짜 함수
  값, `builtin :import 1`)를 **둘 다** 지원 — 5개 호스트 중 유일하게
  import를 진짜 1급 함수로도 쓸 수 있다. 실제 파일 해석은
  `*import-resolver*`라는 **교체 가능한 dynamic var**(기본값 `nil` —
  바인딩 안 하면 evaluator는 파일시스템을 절대 안 건드림, 이게 핵심
  설계: `eval-source` 자체는 항상 순수해야 함). 구현체 두 개:
  - `in-memory-import-resolver`(core.clj) — `*import-modules*`(경로
    문자열→pnix 소스 텍스트 순수 맵)만 보고 해석, 파일시스템 접근 전혀
    없음. `eval-source-with-imports`가 이걸 씀. 테스트/샌드박스용.
  - `filesystem-import-resolver`(core.clj, **2026-08-19 신설**) — 실제
    디스크에서 읽음, canonical path로 순환참조 감지. `eval-file`(그리고
    `repl.clj`의 CLI 파일 모드)이 이걸 씀.
  둘 다 캐시를 안 한다(매번 새로 파싱+평가) — 순수 함수형이라 값은
  같아서 정합성엔 문제없음, 다만 clr처럼 canonical-path 캐시는 없다는
  뜻(scopedImport와 캐시 슬롯을 안 섞어 쓸 걱정 자체가 없음).
  `contextual-import-target`이 상대경로를 호출부 기준으로 정규화.
  **`-e`(인라인)/파일 모드 둘 다 이제 import 됨** — clr/rs와 달리 파일
  컨텍스트가 없어도 `*import-resolver*`만 바인딩돼 있으면 동작(단,
  상대경로는 origin이 없으면 cwd 기준으로 남는다).
- **여러 평가 lane**: 이 호스트는 "직접 evaluator" 말고도 clj-meta
  바이트코드 lowering, pnix self-runtime, pnix mirror까지 4개 기반에서
  같은 소스를 교차검증하는 self-hosting tower가 있다(다른 4개 호스트엔
  없는 이 저장소만의 특징). `clj-meta-eval-error-model.v1` 스키마로
  에러를 표준화.

### 관련 문서 — 여기 없는 내용은 여기서 찾을 것

이 문서는 "무엇이 어디 있는지"에 집중한다. 아래는 이미 있는 다른 문서들 —
내용을 중복하지 않고 링크만 한다. 이 호스트는 5개 중 문서가 제일 많다
(4-substrate self-hosting tower 증명까지 같이 하는 저장소라서).

| 문서 | 다루는 것 |
|---|---|
| [`pnix-clj/pnix-clj/SCOPE_LOCK.md`](../SCOPE_LOCK.md), [`LANE_CLASSIFICATION.md`](../LANE_CLASSIFICATION.md) | 권위 있는 범위 선언 + CORE/PROOF-ONLY/EXPERIMENTAL/QUARANTINE 네임스페이스 분류 |
| [`pnix-clj/pnix-clj/clj-meta-separation.md`](../clj-meta-separation.md) | clj-meta ↔ pnix-clj 분리가 실제 `feat/clj-meta-metacircular` 브랜치 대비 BUILT/PARTIAL/TARGET/MOVED/HELD 어디에 있는지 |
| [`pnix-clj/pnix-clj/todo.md`](../todo.md)(3300+줄) | 살아있는 백로그/상태 로그 — landed 항목(D18, M7 추상 머신 등)과 `/deep-research` 판정으로 보류/기각된 항목 |
| [`docs/CAPABILITIES.md`](CAPABILITIES.md), [`docs/LANE_REGISTRY.md`](LANE_REGISTRY.md), [`docs/WIKI.md`](WIKI.md) | **셋 다 자동 생성**(손 편집 금지) — 각각 `clojure -M:capabilities`/`-M:lane-registry`/`-M:wiki`로 재생성. 이 문서(`IMPLEMENTATION_MAP.md`)의 §2 빌트인 표와는 다른 것들 — 저것들은 코드에서 직접 파생된 진실의 원천이고, §2는 5개 호스트를 나란히 비교하려고 수동으로 만든 스냅샷이다(§5 참고) |
| [`docs/META_CIRCULAR_AUDIT.md`](META_CIRCULAR_AUDIT.md) | 58-capability "Pure Meta-Circular Capability Checklist" 대비 스코어카드, evidence-store SPINE 구성 이력 |
| [`docs/HOST_IMPORT.md`](HOST_IMPORT.md) | 다른 언어가 pnix-clj를 호스트 라이브러리로 embed할 때 쓰는 공개 API(`core/eval-file` 등) |
| [`docs/SPINE_ROADMAP.md`](SPINE_ROADMAP.md), [`docs/REMAINING_DECISION.md`](REMAINING_DECISION.md), [`docs/GENERATOR_DECISION.md`](GENERATOR_DECISION.md) | evidence-spine 구축 순서 계획, `/deep-research` 판정 백로그, self-* 루프 candidate generator 설계 결정 — 전부 진행 중이거나 보류된 연구 방향 |
| [`pnix-clj/clj-meta/{README,STATUS,todo}.md`](../../clj-meta/README.md) | clj-meta(자매 프로젝트, Clojure-on-Clojure self-host 증명 레인)의 자체 문서 |

## 2. 빌트인 구현 현황 (5개 호스트 비교, 2026-08-19 기준)

O = 등록됨(실제로 호출되는지는 별개, §3 참고). 표는 5개 호스트 소스에서
직접 추출한 것 — 시간이 지나면 stale해지니 의심되면 다시 뽑아볼 것(방법:
각 호스트 evaluator 소스에서 빌트인 이름 등록 패턴을 grep, 5개를 합쳐서
diff).

`import`/`scopedImport`\*: 자동 추출 스크립트는 "평범한 빌트인 이름 등록 패턴"만 grep하는데, clr/cljs/rs는 이 둘을 예약 키워드(파서 전용 문법)로 구현해서 그 패턴에 안 잡힌다 — 실제로는 5개 다 있음(값으로 표는 수동 정정함). 새로 표를 다시 뽑을 때 이 두 줄은 자동 추출 결과를 믿지 말 것.

| 이름 | clj | clr | cljs | rs | hy |
|---|---|---|---|---|---|
| abort | O | O | O | O | O |
| abs | O | O | O | O | O |
| add | O | O | O | O | O |
| addDrvOutputDependencies | - | - | - | - | O |
| addErrorContext | O | O | O | O | O |
| all | O | O | O | O | O |
| and | O | O | O | O | O |
| any | O | O | O | O | O |
| append | O | O | O | O | O |
| appendContext | O | - | - | - | O |
| assert | - | - | - | - | O |
| assertMsg | O | O | O | O | O |
| atan2 | O | O | O | - | O |
| attrByPath | O | O | O | O | O |
| attrNames | O | O | O | O | O |
| attrValues | O | O | O | O | O |
| baseNameOf | O | O | O | O | O |
| bitAnd | O | O | O | O | O |
| bitOr | O | O | O | O | O |
| bitXor | O | O | O | O | O |
| boolToString | O | O | O | O | O |
| break | O | O | O | O | O |
| builtins | - | - | - | O | - |
| catAttrs | O | O | O | O | O |
| ceil | O | O | O | O | O |
| compareVersions | O | O | O | O | O |
| concatLists | O | O | O | O | O |
| concatMap | O | O | O | O | O |
| concatMapStrings | O | O | O | O | O |
| concatMapStringsSep | O | O | O | O | O |
| concatStrings | O | O | O | O | O |
| concatStringsSep | O | O | O | O | O |
| cons | O | O | O | O | O |
| const | O | O | O | O | O |
| cos | O | O | O | O | O |
| count | O | - | - | - | - |
| currentSystem | - | - | - | - | O |
| deepSeq | O | O | O | O | O |
| derivation | O | - | - | - | O |
| derivationStrict | O | - | - | - | O |
| dirOf | O | O | O | O | O |
| div | O | O | O | O | O |
| drop | O | O | O | O | O |
| elem | O | O | O | O | O |
| elemAt | O | O | O | O | O |
| eq | O | O | O | O | O |
| exp | O | O | O | O | O |
| false | - | - | - | O | - |
| fetchGit | O | O | O | O | O |
| fetchTarball | O | O | O | O | O |
| fetchurl | O | O | O | O | O |
| filter | O | O | O | O | O |
| filterAttrs | O | O | O | O | O |
| filterAttrsRecursive | O | O | O | O | O |
| find | O | O | O | O | O |
| findFirst | O | O | O | O | O |
| fix | O | O | O | O | O |
| flatten | O | O | O | O | O |
| flip | O | O | O | O | O |
| floor | O | O | O | O | O |
| fold | - | - | - | - | O |
| foldl | O | O | O | O | O |
| foldl' | O | O | O | O | O |
| foldlAttrs | O | O | O | O | O |
| foldr | O | O | O | O | O |
| fromJSON | O | O | O | O | O |
| fromTOML | - | - | - | - | O |
| functionArgs | O | O | O | O | O |
| ge | O | O | O | O | O |
| genAttrs | O | O | O | O | O |
| genList | O | O | O | O | O |
| genericClosure | O | O | O | O | O |
| get | O | O | O | O | O |
| getAttr | O | O | O | O | O |
| getAttrFromPath | O | O | O | O | O |
| getAttrFromPathOr | O | O | O | O | O |
| getAttrs | - | - | - | - | O |
| getContext | O | - | - | - | O |
| getEnv | O | O | O | O | O |
| getName | O | O | O | O | O |
| getVersion | O | O | O | O | O |
| groupBy | O | O | O | O | O |
| gt | O | O | O | O | O |
| hasAttr | O | O | O | O | O |
| hasAttrByPath | O | O | O | O | O |
| hasContext | O | - | - | - | O |
| hasInfix | O | O | O | O | O |
| hasPrefix | O | O | O | O | O |
| hasSuffix | O | O | O | O | O |
| hashFile | - | - | - | - | O |
| hashString | O | O | O | O | O |
| head | O | O | O | O | O |
| htmlEmit | - | - | - | - | O |
| htmlParse | - | - | - | - | O |
| id | O | O | O | O | O |
| imap0 | O | O | O | O | O |
| imap1 | O | O | O | O | O |
| implies | O | O | O | O | O |
| import* | O | O | O | O | O |
| init | O | O | O | O | O |
| intersectAttrs | O | O | O | O | O |
| intersectLists | O | O | O | O | O |
| isAttrs | O | O | O | O | O |
| isBool | O | O | O | O | O |
| isFinite | - | - | - | - | O |
| isFloat | O | O | O | O | O |
| isFunction | O | O | O | O | O |
| isInf | - | - | - | - | O |
| isInt | O | O | O | O | O |
| isList | O | O | O | O | O |
| isNaN | - | - | - | - | O |
| isNull | O | O | O | O | O |
| isPath | O | O | O | O | O |
| isString | O | O | O | O | O |
| keys | O | O | O | O | O |
| langVersion | - | - | - | O | O |
| last | O | O | O | O | O |
| le | O | O | O | O | O |
| length | O | O | O | O | O |
| lessThan | O | O | O | O | O |
| listToAttrs | O | O | O | O | O |
| ln | O | O | O | O | O |
| log | - | - | - | O | O |
| lt | O | O | O | O | O |
| map | O | O | O | O | O |
| mapAttrs | O | O | O | O | O |
| mapAttrs' | O | - | - | - | - |
| mapAttrsRecursive | O | O | O | O | O |
| mapAttrsToList | O | O | O | O | O |
| mapGet | - | - | - | - | O |
| mapKeys | - | - | - | - | O |
| mapMerge | - | - | - | - | O |
| mapSet | - | - | - | - | O |
| mapValues | - | - | - | - | O |
| match | O | O | O | O | O |
| max | O | O | O | O | O |
| merge | O | O | O | O | O |
| min | O | O | O | O | O |
| mod | O | O | O | O | O |
| mul | O | O | O | O | O |
| nameValuePair | O | O | O | O | O |
| neg | O | O | O | O | O |
| nixVersion | - | - | - | O | O |
| not | O | O | O | O | O |
| null | - | - | - | O | - |
| optional | O | O | O | O | O |
| optionalAttrs | O | O | O | O | O |
| optionalString | O | O | O | O | O |
| optionals | O | O | O | O | O |
| or | O | O | O | O | O |
| parseDrvName | O | O | O | O | O |
| partition | O | O | O | O | O |
| pathExists | O | O | O | O | O |
| pipe | O | O | O | O | O |
| placeholder | O | O | O | O | O |
| pnixMounts | O | O | O | - | - |
| pow | O | O | O | O | O |
| product | O | O | O | O | O |
| range | O | O | O | O | O |
| readDir | O | O | O | O | O |
| readFile | O | O | O | O | O |
| readFileType | - | - | - | - | O |
| recursiveUpdate | O | O | O | O | O |
| removeAttrs | O | O | O | O | O |
| removePrefix | O | O | O | O | O |
| removeSuffix | O | O | O | O | O |
| replaceStrings | O | O | O | O | O |
| replicate | O | O | O | O | O |
| reverse | - | - | - | - | O |
| reverseList | O | O | O | O | O |
| schemaExplain | - | - | - | - | O |
| schemaNormalize | - | - | - | - | O |
| schemaValidate | - | - | - | - | O |
| scopedImport* | O | O | O | O | O |
| seq | O | O | O | O | O |
| set | O | O | O | O | O |
| sin | O | O | O | O | O |
| sort | O | O | O | O | O |
| split | O | O | O | O | O |
| splitString | O | O | O | O | O |
| splitVersion | O | O | O | O | O |
| sqrt | O | O | O | O | O |
| storeDir | - | - | - | O | O |
| storePath | O | O | O | O | O |
| stringLength | O | O | O | O | O |
| stringToCharacters | O | O | O | O | O |
| sub | O | O | O | O | O |
| substring | O | O | O | O | O |
| subtractLists | O | O | O | O | O |
| sum | O | O | O | O | O |
| tail | O | O | O | O | O |
| take | O | O | O | O | O |
| tan | - | - | - | O | O |
| throw | O | O | O | O | O |
| toFile | O | O | O | O | O |
| toInt | O | O | O | O | O |
| toJSON | O | O | O | O | O |
| toLower | O | O | O | O | O |
| toPath | O | O | O | O | O |
| toString | O | O | O | O | O |
| toUpper | O | O | O | O | O |
| toXML | O | O | O | O | O |
| trace | O | O | O | O | O |
| traceVerbose | - | - | - | - | O |
| true | - | - | - | O | - |
| tryEval | O | O | O | O | O |
| typeOf | O | O | O | O | O |
| unique | O | O | O | O | O |
| unsafeAddOutputDependency | - | - | - | - | O |
| unsafeAddOutputName | - | - | - | - | O |
| unsafeDiscardOutputDependency | O | O | O | O | O |
| unsafeDiscardStringContext | O | O | O | O | O |
| unsafeGetAttrPos | O | O | O | - | O |
| updateManyAttrs | O | O | O | O | O |
| values | O | O | O | O | O |
| warn | O | O | O | O | O |
| when | O | O | O | O | O |
| xmlEmit | - | - | - | - | O |
| xmlParse | - | - | - | - | O |
| zip | O | O | O | O | O |
| zipAttrs | O | O | O | O | O |
| zipAttrsWith | O | O | O | O | O |
| zipLists | O | O | O | O | O |
| zipListsWith | O | O | O | O | O |

## 3. 다른 호스트와 알려진 차이점

- **`import`가 진짜 1급 함수로도 쓸 수 있는 유일한 호스트.**
  `builtins.import somePath`가 되고, `let i = builtins.import; in i ./x`
  같은 것도 이론상 가능(다른 4개 호스트는 `import`가 파서 예약 키워드라
  이런 게 안 됨). Nix 실제 스펙에 제일 가까운 것도 이 호스트.
- **import가 캐시를 안 한다** — clr은 canonical path로 캐시하고 rs는
  AST를 통째로 스플라이스, 여기는 매번 파싱+평가를 새로 한다. 순수
  함수라 결과값은 항상 같아서 정합성 문제는 없지만, 같은 파일을 여러
  번 import하면 다른 호스트보다 느릴 수 있다(성능 이슈일 뿐, 정확성
  이슈 아님).
- **`eval-source`는 기본적으로 파일시스템을 절대 안 건드린다** —
  `*import-resolver*`가 `nil`이면 `import`는 무조건
  `:import-evaluation-not-wired`로 실패한다. 이건 버그가 아니라 순도
  보장을 위한 설계(안전 샌드박스 용도로 이 호스트를 라이브러리처럼 쓰는
  임베더들을 위한 것) — CLI(`eval-file`/`repl.clj`)만 실제 리졸버를
  바인딩한다. 2026-08-19 전에는 이 리졸버 구현체 자체가(파일시스템
  버전은) 아예 없어서 CLI로 돌려도 import가 항상 실패했었다.
- **4-기판 self-hosting tower**가 있는 유일한 호스트(clj-meta 바이트코드
  lowering + pnix self-runtime + pnix mirror까지 교차검증). 다른
  호스트에서 "왜 이 저장소만 이렇게 검증 인프라가 방대하지" 싶으면
  이것 때문.

## 4. 역사 — 무엇이 언제 만들어졌는가

**git log의 한계부터**: `git log --oneline --all -- pnix-clj/`는 48개
커밋뿐이고(2026-08-10~08-19), 첫 커밋(`4240414`, `init`)이 evaluator/
parser/core, self-hosting tower/spine 전체, capabilities/lane-registry/
WIKI 생성기, 심지어 `todo.md`의 과거 이력(2026-07-04 날짜 항목 포함)까지
450개 파일·약 90,000줄을 통째로 한 번에 들여온다. **58-capability
checklist 작업, evidence-store SPINE 구축, 407 커밋 앞서 있었다는
`feat/clj-meta-metacircular` 브랜치 — 이 전부가 이 repo git 이력으로
재구성이 안 된다.** `git log`가 완전한 기록이라고 착각하지 말 것; 그
시기의 서사는 `todo.md`, `docs/META_CIRCULAR_AUDIT.md`,
`clj-meta-separation.md` 안에 글로만 남아있다.

`init` 이후 이 repo git 이력 안에서 있었던 주요 사건:

| 커밋 | 날짜 | 무엇을 |
|---|---|---|
| `4240414` | 08-10 | `init` — evaluator/parser/core, 4-substrate self-hosting tower/spine, capabilities/lane-registry/WIKI 생성기, todo.md 과거 이력까지 전체가 한 스냅샷으로 들어옴 |
| `e848f82` | 08-11 | pnix-clj 패리티를 향한 빌트인 성숙 패스: 리스트/attrset 구조적 동등성, float 리터럴, 확장 math/bitwise/list/attrset 빌트인 |
| `6b33951` | 08-13 | 호스트 임베딩 표면 확장: `pnix-clj.core` 호스트 `.px` import 편의 함수 + CLR/C# guest 라이브러리 작업 |
| `3f16ea7` | 08-14 | D22 dotted-let 미분 행 — `todo.md`의 M7 추상 머신(Krivine/CESK 유도) measured-parity 항목을 닫음 |
| `64dbc9a` | 08-14 | `run-witnessed`가 `verify-source`로 전환 — cross-mirror collapse 정합성 수정 |
| `8249344` | 08-14 | classfile proof-lane 영수증, forward-ref semantic-fail 처리, mirror-error 분류 — clj-meta 컴파일 아티팩트 증명 능력 |
| `b0fb5ce` | 08-14 | WIKI + LANE_REGISTRY 재생성 — 지금 §5에서 설명하는 생성된-문서 drift-게이트 패턴이 여기서 확립됨 |
| `6cee253` | 08-18 | 한 번도 안 돌아가던 `-M:test` 게이트를 고쳐서 기존에 숨어있던 51개 실패를 드러내고 수정 — 상당한 정합성/커버리지 마일스톤 |

이후 2026-08-19 하루 동안 있었던 일은 아래 §4-오늘 참고.

### 오늘(2026-08-19) 실제로 고친 것들 — 무엇을, 왜

| 커밋 | 무엇을 |
|---|---|
| `1c17f9d` | `i64::MIN` 소스 리터럴 표현 문제 |
| `d1fe267` | `div`/`mod`가 0으로 나누기를 `type-error`로 잘못 보고하던 버그(정확히는 `division-by-zero`여야 함) |
| `caff954` | `import`가 **어떤 모드에서도 전혀 동작하지 않던 것** 완전히 고침 — 순환참조 감지/상대경로 해석 같은 인프라(`*import-resolver*`, `contextual-import-target`)는 이미 다 있었는데 실제로 디스크를 읽는 구현체(`filesystem-import-resolver`)가 아예 없어서 매번 "not-wired" 에러만 나던 상태였음. 기존 `in-memory-import-resolver`와 완전히 같은 모양으로 새로 만들어서 `eval-file`/CLI 파일 모드에만 연결 |

교차검증에서 배운 것: `*import-resolver*` 같은 "플러그인 가능한 확장점"
패턴은 잘 설계돼 있어도 **실제 구현체를 아무도 안 만들어서 그냥 안
쓰이던 채로 오래 남아있을 수 있다** — 껍데기(dynamic var, 캐시 로직,
경로 정규화 헬퍼)는 다 있는데 알맹이(파일 읽기)가 없는 상태였다. 새
확장점을 볼 때마다 "이거 진짜로 뭔가 바인딩하는 코드가 있나"까지
확인할 것 — 인프라 존재 여부와 실제로 동작하는지는 다른 질문이다(rs의
`functionArgs`가 등록은 됐지만 항상 held였던 것과 같은 패턴).

## 5. 이 문서가 코드와 어긋나지 않게 유지하는 법

이 문서는 두 성격이 섞여 있다 — 어느 쪽인지 구분해서 신뢰할 것.

- **§2 빌트인 표는 자동 생성이 아니다.** 5개 호스트를 나란히 비교하기
  위해 수동으로 추출한 2026-08-19 스냅샷이고, 빌트인이 추가/삭제될 때마다
  stale해진다. 다시 뽑으려면 저장소 루트(`~/pnix`)에서:
  ```bash
  bin/gen-builtin-presence-matrix          # 새 표 출력
  bin/gen-builtin-presence-matrix --check  # 5개 문서 표가 실제 소스와 다르면 비영 종료
  ```
  `import`/`scopedImport`는 clr/cljs/rs에서 예약 키워드라 이 스크립트가
  못 잡아서 손으로 `*` 표시가 남아있다(§2 상단 각주). 5개 호스트를
  가로지르는 monorepo 레벨 도구라 이 저장소의 self-contained 게이트에는
  넣지 않았다.
- **이 호스트는 자동 생성+drift-게이트 문서가 5개 중 제일 많다** —
  `docs/CAPABILITIES.md`(`clojure -M:capabilities`), `docs/LANE_REGISTRY.md`
  (`clojure -M:lane-registry`), `docs/WIKI.md`(`clojure -M:wiki`) 셋 다
  코드에서 파생되고 drift가 나면 각자 게이트가 잡는다(생성 원천 = 코드).
  네임스페이스 분류/공개 API/능력·로드맵 인덱스가 궁금하면 이 문서의
  §1/§2보다 그쪽 셋을 먼저 믿을 것 — 이 문서는 "어디서 찾는지"와 "다른
  호스트와 어떻게 다른지"를 설명하는 내레이션이고, 실시간 진실은
  저 셋에 있다.
- hy/rs도 각자 CAPABILITIES.md(+rs는 REGISTRY.md까지)를 가진 같은
  패턴이다(각 호스트 IMPLEMENTATION_MAP.md §5 참고). clr/cljs는 아직
  이런 자동 drift-게이트 문서가 없다 — clr/cljs 쪽 IMPLEMENTATION_MAP.md
  §5에 실제 미해결 gap으로 적어뒀다.
