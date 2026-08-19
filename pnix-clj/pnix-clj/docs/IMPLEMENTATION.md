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
| §5–§11 아래 | 스코프 경계, 네임스페이스 레인 분류, clj-meta/pnix-clj 분리, 증거-저장소 spine, rec/let 전방 참조, self-* 생성기, 호스트 라이브러리 embed — 예전엔 별도 파일(`SCOPE_LOCK.md`/`LANE_CLASSIFICATION.md`/`clj-meta-separation.md`/`docs/SPINE_ROADMAP.md`/`docs/META_CIRCULAR_AUDIT.md`/`rec-forward-reference-taxonomy.md`/`docs/GENERATOR_DECISION.md`/`docs/HOST_IMPORT.md`)로 흩어져 있었으나 2026-08-20에 이 문서로 통합했다 |
| [`docs/TODO.md`](TODO.md) | 지금 집어서 진행할 수 있는, 아직 안 끝난 작업 |
| [`docs/BUGS.md`](BUGS.md) | 알려진 버그·한계, 그리고 의도적으로 안 고치는 항목 |
| [`docs/PLANS.md`](PLANS.md) | 아직 착수가 확정되지 않은 미래 설계 방향 |
| [`docs/CAPABILITIES.md`](CAPABILITIES.md), [`docs/LANE_REGISTRY.md`](LANE_REGISTRY.md), [`docs/WIKI.md`](WIKI.md) | **셋 다 자동 생성**(손 편집 금지) — 각각 `clojure -M:capabilities`/`-M:lane-registry`/`-M:wiki`로 재생성. 이 문서의 §2 빌트인 표와는 다른 것들 — 저것들은 코드에서 직접 파생된 진실의 원천이고, §2는 5개 호스트를 나란히 비교하려고 수동으로 만든 스냅샷이다(§12 참고) |
| [`pnix-clj/clj-meta/{README,STATUS,todo}.md`](../../clj-meta/README.md) | clj-meta(자매 프로젝트, Clojure-on-Clojure self-host 증명 레인)의 자체 문서 |

## 2. 빌트인 구현 현황 (5개 호스트 비교, 2026-08-19 기준)

O = 등록됨(실제로 호출되는지는 별개, §3 참고). 표는 5개 호스트 소스에서
직접 추출한 것 — 시간이 지나면 stale해지니 의심되면 다시 뽑아볼 것(방법:
각 호스트 evaluator 소스에서 빌트인 이름 등록 패턴을 grep, 5개를 합쳐서
diff).

`import`/`scopedImport`\*: 자동 추출 스크립트는 "평범한 빌트인 이름 등록 패턴"만 grep하는데, clr/cljs/rs는 이 둘을 예약 키워드(파서 전용 문법)로 구현해서 그 패턴에 안 잡힌다 — 실제로는 5개 다 있음(값으로 표는 수동 정정함).

`langVersion`/`nixVersion`/`storeDir`\*: 같은 이유로 또 다른 blind spot(2026-08-20 발견) — clj/clr/cljs에서 이 셋은 콜러블이 아니라 zero-arg 상수 값으로 등록돼 있어서 `(builtin ...)`/`(bi ...)`/`(->BuiltinValue ...)` 패턴에 안 잡힌다. 직접 소스 확인 후 수동 정정함(실제로는 5개 다 있음).

새로 표를 다시 뽑을 때 이 5줄(`*` 표시)은 자동 추출 결과를 믿지 말 것 — `bin/gen-builtin-presence-matrix`가 이미 이 5줄을 손으로 보정한다.

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
| atan2 | O | O | O | O | O |
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
| langVersion* | O | O | O | O | O |
| last | O | O | O | O | O |
| le | O | O | O | O | O |
| length | O | O | O | O | O |
| lessThan | O | O | O | O | O |
| listToAttrs | O | O | O | O | O |
| ln | O | O | O | O | O |
| log | O | O | O | O | O |
| lt | O | O | O | O | O |
| map | O | O | O | O | O |
| mapAttrs | O | O | O | O | O |
| mapAttrs' | O | O | O | O | O |
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
| nixVersion* | O | O | O | O | O |
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
| storeDir* | O | O | O | O | O |
| storePath | O | O | O | O | O |
| stringLength | O | O | O | O | O |
| stringToCharacters | O | O | O | O | O |
| sub | O | O | O | O | O |
| substring | O | O | O | O | O |
| subtractLists | O | O | O | O | O |
| sum | O | O | O | O | O |
| tail | O | O | O | O | O |
| take | O | O | O | O | O |
| tan | O | O | O | O | O |
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
WIKI 생성기, 심지어 예전 `todo.md`의 과거 이력(2026-07-04 날짜 항목 포함)까지
450개 파일·약 90,000줄을 통째로 한 번에 들여온다. **58-capability
checklist 작업, evidence-store SPINE 구축, 407 커밋 앞서 있었다는
`feat/clj-meta-metacircular` 브랜치 — 이 전부가 이 repo git 이력으로
재구성이 안 된다.** `git log`가 완전한 기록이라고 착각하지 말 것; 그
시기의 서사는 이제 이 문서의 §5–§11(2026-08-20에 예전 `todo.md`/
`docs/META_CIRCULAR_AUDIT.md`/`clj-meta-separation.md`에서 옮겨옴) 안에
글로만 남아있다.

`init`(08-10) 이전, git 이력으로 재구성 안 되는 시기의 주요 마일스톤(예전
`todo.md` 서사에서 압축, 날짜는 그 문서에 적힌 작업일 기준):

| 시기 | 무엇을 |
|---|---|
| 2026-07-01 | Nix-conformance 하드닝 라운드: `rec`/`let` 전방 참조 lift(§9), 연산자 strict-audit Phase A-D, lazy attrset/list 값 코어 슬라이스, `run-mirror` 싱글톤화, interop deny-by-default 게이팅 — 13개 빌트인/의미론 버그 + rec 전방참조 수정 |
| 2026-07-02 | 🗼 메타서큘러 로드맵 M1–M6 전체 완료: `specialize`(Futamura 1차 투영, JVM 바이트코드까지), `tower/run-tower`(4-기판 단일 진입점), `synthesize`(역방향 투영), `capabilities`(능력 인덱스+drift 게이트), `safe-eval`(순도 샌드박스), `cached-eval`(content-addressed 캐시) |
| 2026-07-03 | F1·F2 랜딩: Futamura 2차 투영(cogen-free, `pnix-clj.futamura`) + Jones-optimality 측정 증인; string-context/derivation/import를 전 레인(evaluator/clj-meta lowering/`.px`)으로 확장 |
| 2026-07-04 | 증거-저장소 spine(§8) 전체 구축 — `docs/SPINE_ROADMAP.md`의 계획(§3 CAS term store부터 §15 witness capstone까지)을 clean-rewrite로 실행, `cas.clj`/`store.clj`/`reflect.clj`/`snapshot.clj`/`purity.clj`/`search.clj`/`mirror_chain.clj`/`witness.clj`/`witnessed_run.clj`/`self_mod_gate.clj`/`persist.clj` 랜딩 |
| 2026-07-07~08 | F8 weval-스타일 IR-level PE 스파이크 랜딩(`pnix-clj.weval`); 게이트 리포트 캐시 랜딩(13종 리포트 1-JVM 통합); `/deep-research`로 남은 백로그 판정 — splice leniency 기각(현재 동작이 정답), D1c 연기, conformance Phase D 연기, F7b 보류(§10, PLANS.md 참고) |
| 2026-07-01~08 | `pnix-clj.generate`(observational-equivalence 후보 생성기, §10) + `self-improve`/`self-mod-gate`/`synthesize` EXPERIMENTAL 레인 랜딩 — self-* 루프의 첫 정직한 벽돌 |
| 2026-08-13~14 | 호스트 언어 임포트 제품화(§11): `core/eval-file`, `docs/HOST_IMPORT.md`, `bin/export-pnix-clj-library` 로컬 라이브러리 export |

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

### 오늘(2026-08-20) 실제로 고친 것들 — 무엇을, 왜

| 커밋 | 무엇을 |
|---|---|
| (미커밋) | §2 표에서 clj가 빠져 있던 5개 빌트인 중 `log`/`tan`을 hy 기준 시맨틱으로 신규 구현(`Math/log`·`Math/tan`, sin/cos/ln과 동일한 패턴); `nixVersion`/`storeDir`/`langVersion`은 조사해보니 `init` 커밋부터 이미 구현돼 있었고 §2 표만 stale했던 것으로 판명 — 표만 정정. `log`/`tan`은 `finish-builtin`의 메인 `case`(3400줄대)에 넣으면 JVM 메서드 바이트코드 64KB 한도(`Method code too large!`)를 넘겨 컴파일이 깨져서, 정확히 이 문제를 피하려고 이미 존재하던 `finish-extra-builtin`(2831줄, 별도 메서드로 분리된 오버플로 빌트인 처리) 쪽에 대신 추가함 |

이번에 배운 것: `finish-builtin`의 메인 `case`는 이미 JVM 메서드 크기
한도에 바짝 붙어 있다 — 새 산술/문자열류 빌트인을 여기 늘리기 전에 반드시
컴파일부터 해볼 것(`clojure -e "(require 'pnix-clj.evaluator)"`), 안
들어가면 `finish-extra-builtin`(또는 `finish-context-builtin`) 같은
분리된 헬퍼로 보낼 것 — 이 파일 자체가 이미 그 분산 패턴을 쓰고 있다(§1
"여러 지점에 빌트인 로직이 흩어져 있다" 참고).

## 5. 스코프 경계

pnix-clj는 **Clojure 호스팅 pnix 런타임 및 메타원형 증인 substrate**다.
clj-meta는 **호스트 언어 증명 레인**이다. (이 절은 예전 `SCOPE_LOCK.md`를
흡수한 것 — 2026-08-20 문서 통합.)

### 범위 안

- pnix 소스
- tokenizer / parser
- pnix AST
- canonical form / lowering
- content hash / CAS
- store / snapshot
- eval-source / eval-from-ast
- mirror / mirror-chain
- purity / determinism
- tower / stage closure
- witness / receipt / replay
- clj-meta host reflection / compiler proof lane

공유 common-`.px` 코어 로딩(외부 `../pnix-meta` 루트에서 로드, 공유 정규
결과 + held reason 방출, 실제 호스트 IO effect/capability 브리지)도 범위
안이다 — 기존 `eval-source`/`import`/`tower`/`mirror` 레인의 직접 확장으로
취급한다(소유자 수정, 2026-07-08). 단 이 공유 `.px` core 자체(`pnix-meta`)는
사람(저장소 소유자)이 직접 작성할 몫이고, 에이전트가 대신 만들 작업이 아니다.

### 범위 밖

다음 레인은 pnix-clj 코어에 들어오면 안 된다 — §6의 QUARANTINE 표와
겹치며, "아직 구현 안 됨"이 아니라 애초에 이 저장소의 범위 밖으로 못박아둔
것이다. 왜 버그가 아닌지는 [`docs/BUGS.md`](BUGS.md) "의도된 제한" 절 참고:

- Hangul codec
- MSV / meaning sentence variants
- Korean dictionary / Korean mirror
- domain token matching
- graph-gate / gate-graph
- multi-language emit registry
- behavior-atom coding-agent emit
- puck-cli executor bridge
- autonomous tick runner
- redb ingest brain
- NL corpus / meaning graph / answer composer

규칙: 기능이 메타원형 Clojure 호스팅 pnix 증명에 속하지 않으면 코어 게이트에
추가하면 안 된다.

예전 `todo.md`에는 이 경계와 별개로 프로젝트 정체성을 못박는 Non-Goals
목록(20여 항목)도 있었다 — "AI 에이전트/코딩 에이전트/자율 planner를 만들지
말 것", "pnix-hy를 베끼지 말고 Python/Hy 의미론을 이해하려 하지 말 것"(이
저장소가 투영하는 대상은 오직 Clojure/JVM), "공통 pnix 브레인·semantic
ABI를 두 언어 사이 투영이 강해지기 전에 조급하게 설계하지 말 것",
"`~/pnix-old`/새 `~/pnix`를 런타임·테스트·게이트 의존성으로 쓰지 말 것"
등. 전부 위 QUARANTINE 목록과 "이 저장소는 Clojure/JVM ↔ pnix 언어 투영
연구소이지 자율 에이전트 프로젝트가 아니다"라는 하나의 원칙으로 수렴한다.

## 6. 네임스페이스 레인 분류

범위 잠금(§5) 아래 실제 네임스페이스/기능 표면을 분류한 것 — 새 네임스페이스,
테스트, 별칭, 앱 러너를 추가하기 전에 여기서 분류할 것. 불확실하면
QUARANTINE으로 분류한다. (이 절은 예전 `LANE_CLASSIFICATION.md`를 흡수한 것 —
2026-08-20 문서 통합. 생성된 진실의 원천은 [`docs/LANE_REGISTRY.md`](LANE_REGISTRY.md).)

### 분류 라벨

- **CORE** — pnix-clj 코어 게이트에 허용. Clojure 호스팅 pnix 메타순환 증명
  레인의 일부.
- **PROOF-ONLY** — 경계 있는 증명/동등성/증인 증거를 생성할 때만 허용. 제품
  동작, 자율 행동, NL 라우팅, coding-agent 실행이 되어서는 안 됨.
- **EXPERIMENTAL** — 경계 있는 연구/증명 실험으로만 허용. 게이트되고,
  문서화되며, 비권위적이어야 함.
- **QUARANTINE** — pnix-clj 코어 밖. 사이드 저장소로 분리하거나 명시적으로
  재분류하지 않는 한 `src`, 테스트 게이트, 코어 런타임에 들어오면 안 됨.

### CORE 레인

| 레인 | 이유 |
|---|---|
| parser | pnix 소스 → AST |
| lowering | AST → 정규/lower form |
| core evaluator | pnix eval-source / eval-from-ast |
| px-runtime | pnix 평가용 런타임 레인 |
| CAS | content-addressed 정체성 |
| store | append-only 증거 / term 저장 |
| snapshot | 결정적 핀된 상태 |
| persist | 내구성 있는 재연 지원 |
| mirror | 런타임 mirror 증거 |
| mirror-chain | 반복 mirror 수렴 |
| mirror-pair | mirror 경로 간 동등성 비교 |
| mirror-error | 구조화된 mirror 실패 증거 |
| determinism | 반복 실행 안정성 |
| purity | effect와 결정성 규율 |
| replay | 증인 재검증 |
| witness | 증명/증거 객체 표면 |
| witnessed-run | 실행 + 증인 바인딩 |
| receipt | content-bound receipt |
| safe-eval | 경계 있는 eval 표면 |
| capabilities | effect/capability 규율 |
| trust | trust 경계 증거 |
| classfile-receipt | JVM/class 아티팩트 증인 |
| version | 런타임/컴파일러 버전 바인딩 |
| clj-meta host reflection | 호스트 언어 증명 레인 |
| interop | Clojure 런타임 ↔ pnix 런타임 메타순환 교차 경계 |
| nREPL | 메타순환 대화형 제어 표면; eval은 코어만 경유 |
| wiki | 자기 문서화 능력 및 로드맵 기판 |
| lane-registry | 생성된 레인 분류 레지스트리 |

### PROOF-ONLY 레인

| 레인 | 규칙 |
|---|---|
| Futamura / specialize | 투영/동등성 증거로만 허용 |
| translation-validation | 동등성 검증으로만 허용 |
| stage7-core | staged 클로저 증명으로만 허용 |
| stage15 | 경계 있는 tower/self-hosting 증명으로만 허용 |
| oracle / live-oracle | 경계 있는 비교 oracle로만 허용 |
| coverage | 증명 표면 커버리지로만 허용 |
| grammar-fuzzer | parser/런타임 견고성 증거로만 허용 |
| property-fuzzer | 경계 있는 property 증거로만 허용 |
| arith-proof | 산술 증명 fixture로만 허용 |
| bool-proof | 불리언 증명 fixture로만 허용 |
| value-roundtrip | 값 브리지 증거로만 허용 |
| emit-form-roundtrip | Clojure form roundtrip 증거로만 허용, 다언어 codegen 아님 |

### EXPERIMENTAL 레인

| 레인 | 필수 절제 |
|---|---|
| synthesize | 경계 있는 후보 생성만; 자율 허가 없음 |
| generate | 경계 있는 생성만; NL/coding-agent 확장 없음 |
| self-improve | held/candidate/gated로 유지; 자율 mutation 없음 |
| self-mod-gate | 게이트만; 직접 mutation 허가 없음 |
| rust-batch | 증명/동등성 배치로 남을 때만, Rust 제품 레인 아님 |
| clojure-projection | Clojure 호스트 투영 증거로만 |
| clojure-form | 호스트 form 분석/roundtrip 증거로만 |
| form-analysis | Clojure form 증명 분석으로만 |
| benchmark | 측정만; 의미 권위 아님 |
| wiki | 문서/인덱스만; 런타임 진리 아님(위 CORE 표에도 wiki가 있다 — 자기 문서화 능력/로드맵 기판이라는 CORE 정체성 표면과, 개별 문서 생성 결과물의 비권위성은 별개 축이다) |

### QUARANTINE 레인

| 레인 | 이유 |
|---|---|
| Hangul codec | NL/의미 레인, pnix 메타순환 증명 아님 |
| MSV / meaning sentence variants | NL 의미 생성 레인 |
| Korean dictionary | 언어 지식 레인 |
| Korean mirror | NL mirror 레인 |
| domain-token / domain-generic matching | 의미 라우팅/매칭 레인 |
| graph-gate / gate-graph | agent graph/emit 레인 |
| multi-language emit registry | coding-agent/codegen 레인 |
| behavior-atom emit | coding-agent 동작 표면 |
| puck-cli bridge | 외부 executor 브리지 |
| tick-runner | 자율 루프/스케줄러 |
| redb ingest brain | 외부 지식/메모리 수집 |
| NL corpus / meaning graph | 자연어 의미 메모리 |
| answer composer | NL 응답 생성 레인 |

### 현재 정체성 잠금 부록

생성된 진실의 원천은 [`docs/LANE_REGISTRY.md`](LANE_REGISTRY.md)다. 현재
최상위 레지스트리 개수:

- CORE: 43
- EXPERIMENTAL: 7
- PROOF-ONLY: 28
- TOTAL: 78

다음 표면은 CORE 정체성 표면이다:

- interop: Clojure 런타임 ↔ pnix 런타임 메타순환 교차 경계
- nREPL: 메타순환 대화형 제어 표면; eval은 코어만 경유
- wiki: 자기 문서화 능력 및 로드맵 기판
- lane-registry: 생성된 레인 분류 레지스트리

`nrepl`, `wiki`, `interop`는 버릴 수 있는 개발 전용 표면이 아닙니다.

다음은 QUARANTINE으로 남으며 pnix-clj 코어에 들어오면 안 된다.

- Hangul codec
- MSV / meaning sentence variants
- graph-gate / gate-graph
- multi-language emit registry
- puck-cli bridge
- tick runner
- redb ingest brain

## 7. clj-meta / pnix-clj 아키텍처 분리

(이 절은 예전 `clj-meta-separation.md`를 압축 흡수한 것 — 2026-08-20 문서
통합. 원문은 `feat/clj-meta-metacircular` 브랜치의 실제 파일을 근거로 한
"stays / moves / interop" 배정이었다.)

### 세 계층, 하나의 브리지

```
clj-meta = Clojure/JVM 메타순환 컴파일러/evaluator PROOF 레인 (호스트 하한, pnix-agnostic, 성숙 — 소비만)
pnix-clj = clj-meta 위 순수 pnix 계층 (그 "호스트"가 곧 clj-meta)
interop  = 경계의 명시적 양방향 브리지 (mirror 아님)
```

**pnix-clj는 호스트 증명을 다시 하면 안 된다.** "호스트 Clojure가
충실한가?" 작업은 전부 Clojure↔Clojure이며 clj-meta 영역(interop를 통해
도달). pnix-clj의 실제 코어는 `parser`/`evaluator`/`lowering`/`px_runtime`/
`mirror`/`receipt`(pnix 언어) 뿐이다.

### 호스트 기계는 정확히 세 파일에 국한

`requiring-resolve` / 호스트 `(eval ...)` / reflection을 쓰는 파일은
`clj_meta.clj`(interop seam), `clojure_form.clj`(호스트 `eval` vs clj-meta
컴파일 동의 — Clojure-about-Clojure 호스트 도메인, pnix 코어 아님),
`clojure_projection.clj`(가장 큰 오배치 — 순수 Clojure/JVM introspection은
호스트 도메인, `project-reader-value`/`validate-term`만 pnix 측으로 남음)
셋 뿐이다. 나머지는 전부 pnix-native.

### `origin/main`은 별개 설계 라인 (참조 자산일 뿐)

이 저장소가 서 있는 브랜치는 `origin/main`과 0 behind / 407 ahead로
갈라진 깨끗한 병렬 재작성이다. `origin/main`은 다른, 더 오래된 설계 라인
(`cas.clj`/`store.clj`/`stage.clj`/`purity.clj`/`term.clj`/`stm.clj`/
`resolve.clj`, gate-graph, 67-language emit, Korean modules)을 가진다.
그 모듈들은 **MAIN-ONLY 참조 자산**이다 — 로드맵이 필요로 할 때 이식에
유용하지만, "이미 있음"으로도 "채워야 할 부재 기둥"으로도 판단하면 안 된다.
**§8에서 보듯 이 브랜치는 실제로 필요했던 spine(cas/store/…)을 origin/main
에서 포트하지 않고 독자적으로 clean-rewrite했다** — 그래서 아래 Phase F는
사실상 더 이상 필요 없다(모두 이미 네이티브로 존재).

### 리팩터 단계 (전부 게이트 그린 상태로 완료)

| 단계 | 무엇을 | 상태 |
|---|---|---|
| A | `clj_meta.clj`를 명시적 clj-meta interop 클라이언트로(loss/effect/capability/witness 태깅) | ✅ 완료 |
| B | `clojure_projection.clj`에서 호스트 reflection을 `pnix-clj.clojure-projection.host` 뒤로 추출; pnix-clj는 `project-reader-value`/`validate-term`만 소유 | ✅ 완료 |
| C | `clojure_form.clj`의 호스트 eval/macroexpand를 `pnix-clj.interop`(`host-eval-form`) 경유로 | ✅ 완료 |
| D | 런타임 mirror를 트레이스 facet 있는 singleton `run-mirror`로 통합(`core/run-source`가 한 번만 호출) | ✅ 완료 |
| E | compile proof(determinism/verified/bytecode)를 clj-meta의 `determinism-policy`/`verified-compile`/`bytecode-witness`에 위임, 재유도 금지 | ✅ 완료 |
| F | `origin/main`에서 CAS/event-store/snapshot-resolve 포트(로드맵이 필요로 할 때만) | **모호(moot)** — §8의 spine이 이미 네이티브로 독자 구축됨. origin/main 포트는 더 이상 필요 없음 |

### Interop 경계 (Clojure/JVM ↔ pnix)

**Interop는 mirror가 아니다.** 값/함수/모듈/effect를 변환하며 mirror가
꺼져 있어도 동작해야 한다. 모든 교차에 붙는 공유 필드:

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

값 매핑(pnix ↔ Clojure/JVM): `null↔nil` `bool↔Boolean` `int↔integer`
`float↔floating` `string↔String` `list/vector↔vector` `attrset↔map`
`symbol↔symbol` `keyword↔keyword` `function↔IFn wrapper`
`module↔namespace/module wrapper` `error↔ExceptionInfo`
`opaque JVM object↔pnix opaque ref`.

**규칙**: Clojure/JVM 객체는 pnix 정규 term에 직접 들어가면 안 된다 — 순수
pnix 값으로 명시 변환하지 않는 한 opaque ref로 래핑(`java-object-term`
임베딩 규칙).

연구 근거 원칙(`/deep-research`, 2026-07-01, 94 agents): **deny-by-default**
(GraalVM Truffle — 호스트 하한에서 명시 export되기 전 아무것도 pnix
런타임에서 도달 불가), **모든 교차를 effect class로 분류**, **opaque
handle, 값 직렬화 금지**(object-capability 규율 — authority는 handle
전달로만 이동), **content-addressed cross-layer trust**(호스트 하한을
content-addressed 버전 id로 guest 증거에 바인딩, 이름은 별도 메타데이터 —
Unison 모델). 정직한 주의: cross-layer 동의(receipt/verdict N-version 검사)는
**휴리스틱, 형식 증명 아님** — 호스트 "하한 증명"이 pnix에 의미론/건전성을
넘겨주지 않는다. Singleton-mirror 선호는 우리 설계 선택이지 외부 증명된
법칙이 아니다.

## 8. 증거-저장소 spine

(이 절은 예전 `docs/SPINE_ROADMAP.md` + `docs/META_CIRCULAR_AUDIT.md`를
압축 흡수한 것 — 2026-08-20 문서 통합. **두 문서 모두 계획 문서였지만, 이
저장소에서는 그 계획이 이미 전부 landed됐다** — `src/pnix_clj/`에 아래
모듈이 실존한다. 그래서 "미래 계획"이 아니라 "이미 구현된 것"으로서 여기
IMPLEMENTATION.md에 둔다.)

58-capability "Pure Meta-Circular Capability Checklist"(2026-07-04) 대비
체크리스트의 **증거-저장소 spine**(§3 CAS term store, §5 event log, §8
snapshot, §9 purity-as-events, §10/§13.1 reflection 스냅샷, §17 search,
§6.6-6.7 mirror drift, §15 witness — 원래 `origin/main`의 오래된 설계
라인에만 있던 부분)이 2026-07-04에 **clean-rewrite(옵션 C)**로 이 브랜치에
독자 구축됐다:

| 체크리스트 § | 능력 | 모듈 |
|---|---|---|
| §3 | content-addressed TERM store; α-canonical(de Bruijn + 올바른 shadowing); hash = propose filter | `cas.clj` |
| §5 | append-only 변조 탐지 EVENT log(verifying trace + hermeticity guard) | `store.clj` |
| §10/§13.1 | Clojure/JVM reflection 스냅샷(결정적, 순수 EDN) | `reflect.clj` |
| §8 | snapshot 런타임 pin + fail-closed match 게이트 | `snapshot.clj` |
| §9 | purity/determinism as EVENTS(재실행으로 증인, first-divergent anchor) | `purity.clj` |
| §17 | content-address + event + structural-similarity 검색 | `search.clj` |
| §6.6-6.7 | mirror chain 수렴 + drift events | `mirror_chain.clj` |
| §15 | 증인 스키마 + admission lattice(CAPSTONE) | `witness.clj` |
| 통합 | **witnessed-run** — 한 실행이 spine을 term-keyed/snapshot-pinned/§5-log/§15-admitted로 묶음 | `witnessed_run.clj` |
| §14.3 | self-modification 게이트 — NO-AUTO-PROMOTION을 런타임 게이트로(admitted 증인은 소유자 승인까지 HELD) | `self_mod_gate.clj` |
| durability | persist — §3 term + §5 event용 content-addressed 디스크 백킹, load 시 재검증 | `persist.clj` |

핵심 원칙(§0, 이 spine 전체가 서 있는 전제): **content hash는 동등성 증명이
아니라 PROPOSE 필터다.** α-equivalence modulo 해싱은 한 방향으로만 보장되고
(Maziarz et al., PLDI 2021), 역방향(same-hash ⇒ α-동등)은 낮은 충돌 확률로만
성립한다 — 그래서 hash-hit은 빠른 경로(스킵/dedup/cache)를 허가하지만, 진리로
다루기 전 정확한 구조/α 검사(그리고 결정성에 대해 실제 재실행)로 CONFIRMED
되어야 한다. 구축 순서는 의존성 순서였다: `§3 term store → §5 event log →
§10+§13.1 reflection → §8 snapshot determinism → §9 purity-as-events → §17
search → §6.6-6.7 mirror drift → §15 witness(capstone)` — §3 term 해시가
전부의 키이고, §15가 전부를 통합.

Spine은 이제 실행 경로에 LIVE(`witnessed-run`), self-* 게이트 아래(자동
승격 없음), 내구성 있다(`persist`). 열린 follow-up 하나만 남는다: §11
pnix-macro/reader 레인 — Nix에 Lisp 스타일 macro가 없어 적합도가 낮고,
clj-meta에 이미 macroexpand가 있다(낮은 우선순위, `resources/pnix_clj/roadmap.edn`
에 `:planned`로 등록).

메타순환 **능력 기둥**(투영, self-hosting collapse, 증명, 교차 검사,
interop)은 체크리스트 항목을 넘어서는 것도 있다: Futamura 2차 투영
(`futamura.clj`), 측정 Jones-optimality 증인, PROVEN 산술/불리언 동등성
(`arith_proof.clj`/`bool_proof.clj`), shrinking 있는 property-based
differential fuzzer, capability index + drift gate, 역방향 투영
Clojure→pnix(`synthesize.clj` × `form_analysis.clj`).

## 9. rec / let 전방 참조 재귀 바인딩

(이 절은 예전 `rec-forward-reference-taxonomy.md`를 압축 흡수한 것 —
2026-08-20 문서 통합. 원문은 "감사 + 증거만" 문서였지만, 그 감사가 제안한
수정은 **2026-07-01에 실제로 랜딩했다**(R1, IMPLEMENTATION.md §4 역사 표
참고) — 그래서 여기서는 "지금 이렇게 동작한다"는 설명으로 다룬다.)

`pnix-clj.core/run-source`는 모든 소스를 네 레인으로 돌린다: 직접 의미
evaluator(런타임 그 자체), clj-meta lowering(pnix AST → clj-meta form →
호스트에서 eval), stage15 mirror(clj-meta lowering 위), `.px` 내부 런타임
mirror. `mirror-error` corpus는 레인이 **오류 경계에 동의**할 때 행을
"수락"한다 — 이는 *동의* 주장이지 *Nix 의미론* 주장이 아니다.

수정 전 발견(F1–F5): Nix에서 `let ... in`과 `rec { ... }`는 같은 하나의
상호 재귀 스코프다. `eval-let`은 knot-tied 메모이즈 thunk로 이미 정확했지만
(forward=11, cycle=infinite-recursion, unbound=unbound-var), `eval-attrs`
(rec)는 환경을 점진적으로 만들어 전방 이름이 스코프에 없었다 — 모든 rec
forward/cycle 케이스가 `:unbound-var`로 붕괴했다. 게다가 clj-meta/px-runtime
레인은 전방 참조의 FRONTIER일 뿐 의미 판사가 아니었다(유효한
`let a = b + 1; b = 10; in a`에도 `held`를 줬다) — 그래서 rec-forward-reference에
대한 "레인 동의"는 우연이었다(evaluator held = 진짜 버그, clj-meta/px held =
frontier).

**착지한 분류** (evaluator = Nix 판정, frontier 레인은 지원할 때까지 held):

| 클래스 | 예 | evaluator | clj-meta/stage15/px |
|---|---|---|---|
| `forward-ok` | `rec { x = y; y = 1; }.x`, `let a = b+1; b=10; in a` | **ok** | held @ frontier |
| `cycle-error` | `rec { a = a + 1; }.a`, `let a = a+1; in a` | held `:infinite-recursion` | held @ frontier |
| `unbound-error` | `rec { a = z + 1; }.a`, `let a = z+1; in a` | held `:unbound-var` | held @ frontier |

`eval-attrs`가 `eval-let`과 같은 knot-tied-thunk 스코프를 갖도록 고쳐졌고,
`rec-forward-reference`는 `mirror-error` corpus(오류-동의 corpus)에서 빠져
`resources/pnix_clj/forward_reference/cases.edn`(명시적 frontier 마커가
있는 전방 참조 corpus)로 재분류됐다 — `forward-ok` 행은 evaluator가
Nix-정답을 내고 frontier 레인(clj-meta/stage15/px)은 아직 held임을
검증한다. clj-meta/px-runtime의 재귀 바인딩 지원 자체는 별도의 더 큰
frontier-lift 작업으로 남아 있다(R1 스코프 밖).

## 10. self-* 루프 candidate generator

(이 절은 예전 `docs/GENERATOR_DECISION.md`를 흡수한 것 — 2026-08-20 문서
통합. 결정은 `/deep-research`로 합성됐고, 1번 항목은 이미 `pnix-clj.generate`
/`pnix-clj.cegis`로 랜딩해 §6의 EXPERIMENTAL 레인에 있다. 나머지 순서
항목(CEGIS refinement, Knuth-Bendix 가지치기, refinement-type 레인,
library-learning)은 아직 안 지어졌다 — [`docs/PLANS.md`](PLANS.md) 참고.)

self-* 루프에서 빠져 있던 조각은 이미 증인(`run-witnessed`) + 게이트
(`self-mod-gate`) + 순위화하는 `self-improve`에 공급하는 후보 GENERATOR였다.
`/deep-research`(16 claims 3-0 확인)가 6개 후보 기법(Escher
observational-equivalence reduction, Myth/λ² evaluate-during-enumeration,
Smyth live bidirectional eval, Burst bottom-up+angelic+FTA, Synquid
refinement types, Knuth-Bendix equivalence reduction)을 비교해 **첫 번째로
observational-equivalence-reduced bottom-up enumerative synthesizer**
(Escher 메커니즘)를 택했다:

1. **Dedup oracle이 이미 존재** — Escher value-vector 축소는 예제 입력에
   대해 후보 행동을 계산할 evaluator가 필요한데, pnix-clj는 이미
   `core/eval-source`(+ 전체 `run-witnessed` verifier)를 가진다.
2. **Lazy pure functional 언어에 직접 적합** — Escher/Burst가 정확히 이
   클래스용, 호스트 interop/mutation 없이 값이 순수 EDN.
3. **헌법의 proven-vs-heuristic 경계에 깨끗이 착륙** — 유한 예제
   value-vector 매치는 observational equivalence이며 **heuristic
   PROPOSE**, 증명 아님. generator는 PROPOSED 후보만 방출하고,
   `run-witnessed`/`arith-proof`/`bool-proof`가 적용 가능한 곳에서 PROVEN으로
   승격한다. 모두 HELD(self-mod-gate 아래).
4. Synquid의 수동 논리 명세(자율 공급 불가)와 Myth의 trace-completeness
   요구는 의도적으로 피했다.

랜딩한 것: `pnix-clj.generate`(작은 pnix 문법 위 bottom-up enumerator,
`core/eval-source`로 value-vector 평가, observational-equivalence dedup)가
`synthesize-and-propose`를 통해 `self-improve/evaluate-round`에 후보를
넘기고, HELD 검토 큐로 지속된다. §6 EXPERIMENTAL 레인 규율 그대로 —
경계 있는 생성만, 자율 mutation 없음.

## 11. 호스트 라이브러리로 embed하기

(이 절은 예전 `docs/HOST_IMPORT.md`를 흡수한 것 — 2026-08-20 문서 통합.
정본 이중 축 교리: [`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md).)

`clojure` / `pnix-clj-clj`가 아래를 주입한 뒤 호출 프로젝트가 `pnix-clj`를
호스트 라이브러리로 로드할 때의 공개 API 표면:

```clojure
{:deps {pnix/pnix-clj {:local/root "…/pnix-clj"}}}
```

Env(HM / 래퍼): `PNIX_CLJ_ROOT`, `PNIX_CLJ_LIBRARY`(같은 트리 루트).

### 지원(host-main에 안정)

애플리케이션 코드는 이쪽을 우선. 그 외 네임스페이스는 tower/gate용이며
deprecation 주기 없이 바뀔 수 있다.

| 네임스페이스 | 진입점 | 역할 |
|--------------|--------|------|
| **`pnix-clj.core`** | `parse-source`, `eval-source`, **`eval-file`**, `eval-source-with-imports`, `eval-source-strict`, `eval-source-strict-audit`, `lower-source` | `.px` 파싱 / 평가(1차 표면) |
| **`pnix-clj.machine-outcome`** | `eval-source-outcome` | 구조화 Done/Failed/Suspended 프로젝션 |
| **`pnix-clj.convenience`** | 예제용 헬퍼 | core 위 얇은 설탕(신규 코드는 core 우선) |

최소 예제:

```clojure
(require '[pnix-clj.core :as c])

;; 인라인 소스
(c/eval-source "1 + 2")
;; => {:status :ok, :value 3, …}

;; 파일(호스트 언어에서 .px 프로그램 임포트)
(c/eval-file "path/to/prog.px")

;; 메모리 임포트만(FS 없음): target-string -> source 맵
(c/eval-source-with-imports "import ./lib.px" {"./lib.px" "1 + 1"})
```

미니 멀티모듈 프로젝트: `examples/host-import/clj-imports/`(`main.px`가
`./lib.px` import). 결과 형태: `:status`(`:ok`/`:failed`/`:suspended` …),
`:value` 또는 `:error`, 파싱 메타를 담은 런타임 맵. `:ok`가 아니면 실패로
취급.

### 사용 가능하나 2차

tower/mirror 레인에 이미 익숙할 때만: `pnix-clj.mirror`/`pnix-clj.mirror-pair`
(크로스-substrate mirror 리포트), `pnix-clj.interop`(호스트 interop/opaque
refs), `pnix-clj.capabilities`(capability 인덱스), `pnix-clj.lowering`
(lowering 레인), `pnix-clj.parser`/`pnix-clj.evaluator`(내부 — `core` 우선).
증명/생성기/fuzzer 네임스페이스(`generate`, `grammar-fuzzer`, `arith-proof`,
…)는 애플리케이션용 호스트 라이브러리 API가 **아니다**.

### 이것이 아닌 것

- 이식 가능한 멀티호스트 `.px` 바이트코드 패키지 아님.
- jar / `libexec`가 필요할 때 inject 래퍼 없는 stock `clojure` 대체 아님 —
  그때는 nix의 `clojure-stock` 사용.
- **로컬 라이브러리 export**(개인 피드, Maven Central 아님):
  ```bash
  ./bin/export-pnix-clj-library          # → target/pnix-clj-library/
  ./bin/pnix-clj-library-smoke
  # then: {:deps {pnix/pnix-clj {:local/root "…/pnix-clj-library/pnix-clj"}}}
  ```
- Maven Central / 공개 레지스트리 게시는 이 소유자의 제품 목표 **아님**
  (clr 로컬 nupkg와 동일 정책).

### 스모크

```bash
# HM clojure(= pnix-clj-clj) 기준
echo '1 + 2' > /tmp/t.px
clojure -M -e "(require '[pnix-clj.core :as c]) (println (:value (c/eval-file \"/tmp/t.px\")))"
# => 3

pnix-clj-library   # PNIX_CLJ_ROOT / local/root 힌트 출력
pnix-clj-pnix      # pnix-main REPL
```

## 12. 이 문서가 코드와 어긋나지 않게 유지하는 법

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
  §1/§2/§6(네임스페이스 레인 분류, 수동 통합본)보다 그쪽 셋을 먼저 믿을
  것 — §6은 2026-08-20에 `LANE_CLASSIFICATION.md`를 그대로 흡수한
  스냅샷이고, 실시간 진실은 `docs/LANE_REGISTRY.md`에 있다. 이 문서
  자체는 "어디서 찾는지"와 "다른 호스트와 어떻게 다른지"를 설명하는
  내레이션이다.
- hy/rs도 각자 CAPABILITIES.md(+rs는 REGISTRY.md까지)를 가진 같은
  패턴이다(각 호스트 `docs/IMPLEMENTATION.md`의 해당 절 참고). clr/cljs는
  아직 이런 자동 drift-게이트 문서가 없다 — clr/cljs 쪽
  `docs/IMPLEMENTATION.md`에 실제 미해결 gap으로 적어뒀다.
