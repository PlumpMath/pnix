# pnix-clr 구현 맵

목적: 이 호스트에 뭐가 어떻게 구현돼 있는지 한눈에 찾아보는 문서. 같은 걸
중복해서 다시 만들지 않도록, 뭔가 다르게 동작하는 걸 발견했을 때 "진짜
버그인지 원래 그런 건지" 빨리 판단할 수 있도록, 다른 4개 호스트와 비교할
때 참고하도록 만들었다. 2026-08-19 작성 시작 — 이후 구조가 바뀌면 여기도
같이 갱신할 것.

## 1. 아키텍처 개요 — 어디서 뭘 찾아야 하는가

`src/pnix_clr/` 아래 파일 4개가 핵심: `lexer.clj`(토큰화), `parser.clj`
(재귀 하강 파서 → AST), `evaluator.clj`(값 표현 + 평가기 + 빌트인, 가장
큼), `host.clj`(파일 I/O 등 .NET 호스트 연동, import 경로 해석).

- **렉서**: `keyword-kinds` 맵에 예약어 등록(`let`/`in`/`rec`/`import`/
  `scopedImport`/...). 경로 vs 나눗셈 구분은 `path-start?`(§3에서 자주
  참고되는 함수) — "숫자 토큰 바로 뒤"만 나눗셈으로 취급, 그 외
  identifier/닫는 괄호 뒤는 경로로 취급(단, `next-ch`가 공백/닫는
  괄호/세미콜론이면 역시 나눗셈). 이 규칙을 rs와 cljs가 오늘(2026-08-19)
  그대로 참고해서 자기 버전을 만들었음 — 새 lexer 작업할 땐 여기부터
  볼 것.
- **파서**: `parse-application-term`이 `import`/`scopedImport`를
  특별 취급(예약 키워드, 일반 식별자 아님 — 진짜 Nix의 `scopedImport`는
  일반 함수지만 여기서는 문법으로 처리). 나머지는 표준 재귀 하강
  (`parse-expression` → ... → `parse-primary`).
- **값 표현**: 태그드 맵 — `{:pnix/type :closure ...}`, `{:pnix/type
  :path :value "..."}`(진짜 Path 값 타입 있음, rs/cljs와 다름),
  `{:pnix/type :raw-bytes ...}`. attrset은 `{:entries {...}}`.
- **환경/스코프**: `environment`는 **프레임 체인**(`[frame1 frame2 ...]`,
  각 frame은 `(atom {name -> value})`). 조회(`lookup-value`)는 앞에서부터
  순서대로, 못 찾으면 다음 프레임. `root-environment`가 `[frame]`(builtins/
  lib이 든 프레임 하나)을 리턴. scope 주입(`scopedImport`)은 이 체인
  맨 앞에 새 프레임을 `cons`해서 구현(§4).
- **빌트인 dispatch**: `builtins-entries`(이름→`(bi :kind arity)`)로
  등록, 실제 실행은 `exec-builtin`(`{:keys [name args]}` 받는 큰 `case`).
  **중요**: `exec-builtin`은 root/file/modules 같은 import 컨텍스트를
  전혀 못 받는다 — 그래서 `scopedImport`를 진짜 함수(빌트인)로 못 만들고
  `import`처럼 예약 키워드+전용 AST 노드로 만들 수밖에 없었음(2026-08-19,
  §4).
- **import**: `:import` AST 노드 → `evaluator.clj`의 `eval-file*`가
  `host/resolve-import`(경로 해석) + 캐시(`modules` atom, canonical path
  키) + thunk 기반 순환참조 감지까지 다 있는, 5개 호스트 중 제일 정교한
  구현(2026-08-19 교차검증에서 확인 — clj/cljs/hy/rs 다 여기서 배울 점
  있음). **`-e`(인라인) 모드에서는 root/file 컨텍스트가 없어서 import가
  항상 실패한다 — 파일 모드(`./bin/pnix-clr file.px`)에서만 동작.**
  `scopedImport`는 `eval-file-scoped*`(2026-08-19 신설) — `eval-file*`와
  거의 같지만 **캐시를 안 탄다**(scope가 다르면 같은 파일이라도 결과가
  달라지므로, `eval-file*`의 canonical-path 캐시 슬롯을 공유하면 안 됨)
  는 게 핵심 차이.
- **제품 실행 방식이 독특함**: `pnix-clr`는 다른 4개 호스트처럼 소스를
  즉석 인터프리트하는 게 아니라, **미리 빌드된 정확한 8-DLL 아티팩트만
  신뢰하고 로드**한다(`bin/pnix-clr`가 root/file/artifact 해시를 다
  검증). 소스만 고치고 `./bin/build-pnix-clr-artifact`로 재빌드 안 하면
  변경사항이 반영 안 된다 — 오늘 몇 번 이걸 깜빡해서 "source-stale"
  에러를 봤다.

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

- **경로(path) 값 타입 있음** — clj/hy와 같은 부류(rs/cljs는 없음).
  `builtins.typeOf /tmp`가 `"path"`.
- **`import`/`scopedImport`는 예약 키워드**(Nix 실제 스펙에서 `import`는
  이런 특수 문법이 아니라 `builtins.import`라는 진짜 함수값이지만, 여기
  선 파서 레벨 키워드로 단순화돼 있다 — clj도 마찬가지, rs/cljs/hy도
  대부분 비슷한 단순화). `let f = import; in f ./x`처럼 import를 값으로
  떼어내 쓰는 건 안 됨.
- **아티팩트 기반 배포** — 소스를 고쳐도 `./bin/build-pnix-clr-artifact`
  로 재빌드하기 전까지 `./bin/pnix-clr`는 예전 동작 그대로다(다른 4개
  호스트는 대부분 즉석 인터프리트라 이런 단계가 없거나, 있어도 이 정도로
  엄격하게 검증하지 않음). 이건 버그가 아니라 "미리 선언된 정확한 소스셋만
  로드한다"는 이 호스트의 핵심 설계 원칙(`SCOPE_LOCK.md`) 때문.
- **`.NET AOT 컴파일러 Stage1~15/N`** — `clr-meta`(이 호스트 언어 자체
  증명 레인)는 evaluator generation(0/1/2, 중첩 인터프리터)과 compiler
  stage(Stage1~15/N, 실제 컴파일러)가 별개 축이라는 점을 문서 여러 곳에서
  강조한다(`README.md`) — 헷갈리지 말 것.

## 4. 오늘(2026-08-19) 실제로 고친 것들 — 무엇을, 왜

빠른 참고용 요약. 각 커밋 메시지에 훨씬 자세한 설명이 있다.

| 커밋 | 무엇을 |
|---|---|
| `15740c9` | `i64::MIN` 소스 리터럴 표현 문제 고침 |
| `ce28f69` | `N/M`이 나눗셈이 아니라 경로 리터럴로 잘못 렉싱되던 버그 — 이때 만든 `path-start?` 규칙(§1)이 나중에 rs/cljs 수정의 참고 자료가 됨 |
| `f1bfcf3` | `let a.b = 1;` 같은 dotted 이름 바인딩 지원 |
| `32ac5ad` | `builtins.replaceStrings`가 빈 패턴의 문자열 끝 매치를 빠뜨리던 버그 |
| `61bf484` | `attrs.${expr}`(따옴표 없는 동적 attr 선택) — 렉서에 `${` 처리 추가, 파서의 `parse-attr-name`에 새 분기 |
| `412c6f5` | `scopedImport` 신규 구현. 첫 아티팩트 빌드가 forward-declare 에러로 실패(AOT 컴파일은 인터프리트 evaluate와 달리 앞쪽에서 뒤쪽 함수 참조를 허용 안 함) — `(declare eval-file-scoped* ...)` 추가해서 해결 |

교차검증에서 배운 것: 이 호스트의 `import` 구현(캐시 + 순환참조 감지 +
root/file 컨텍스트 추적)이 5개 중 제일 견고했다. rs가 나중에 자기
`import`를 고칠 때도(scope 누수 버그) 결국 "각 import가 독립된 환경에서
평가돼야 한다"는 이 호스트의 기본 전제를 따라가는 방향으로 고쳐졌다 —
새 호스트에서 import 관련 기능을 만들 일이 있으면 여기부터 참고할 것.
