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
| builtin | - | - | - | - | O |
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
| name | - | - | - | - | O |
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
| policy | - | - | - | - | O |
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
| value | - | - | - | - | O |
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

## 4. 오늘(2026-08-19) 실제로 고친 것들 — 무엇을, 왜

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
