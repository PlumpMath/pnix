# pnix-cljs 구현 맵

목적: 이 호스트에 뭐가 어떻게 구현돼 있는지 한눈에 찾아보는 문서. 같은 걸
중복해서 다시 만들지 않도록, 뭔가 다르게 동작하는 걸 발견했을 때 "진짜
버그인지 원래 그런 건지" 빨리 판단할 수 있도록, 다른 4개 호스트와 비교할
때 참고하도록 만들었다. 2026-08-19 작성 시작 — 이후 구조가 바뀌면 여기도
같이 갱신할 것.

## 1. 아키텍처 개요 — 어디서 뭘 찾아야 하는가

`src/pnix_cljs/` 아래: `tokenizer.cljs`(렉서), `parser.cljs`(재귀 하강 →
AST), `evaluator.cljs`(값 표현 + 평가기 + 빌트인, 가장 큼), `module.cljs`/
`node_loader.cljs`(Node용 import 소스 로딩 어댑터).

- **렉서**: `keywords` 맵에 예약어 등록. 경로 리터럴은 `relative-path-start?`
  (`./`, `../`만, 2026-08-19까지)와 `absolute-path-start?`(bare `/`,
  2026-08-19 추가 — pnix-clr의 `path-start?` 규칙을 그대로 참고: 숫자
  토큰 바로 뒤만 나눗셈, 그 외는 경로).
- **파서**: `atom-starts`(어떤 토큰 종류가 함수 적용 인자를 시작할 수
  있는지)에 `:path`가 **없다** — 그래서 `builtins.isPath ./x`처럼 경로를
  일반 함수 인자로 못 쓴다(§3). `parse-primary`도 `:path` 토큰을 일반
  식(expression)으로 변환하는 case가 아예 없음 — 경로 토큰은 **오직
  `import`/`scopedImport` 문법 안에서만** 파서가 직접 소비한다(각각
  `{:op :import :path "..."}`, `{:op :scoped-import :scope <ast> :path
  "..."}`로, 문자열 그대로 AST에 박힘 — 평가 시점에 다시 해석 안 함).
- **값 표현**: 레코드 — `AttrsetValue{fields}`, `ClosureValue{...}`,
  `ByteStringValue{bytes}`(비UTF-8 중간값). **경로 값 타입 없음**(rs와
  같은 부류, clr/clj/hy와 다름) — `import`/`scopedImport` 밖에서 경로를
  값으로 쓸 방법이 아예 없다(§3).
- **환경/스코프**: `environment`는 **평범한 Clojure 맵**(`{name -> cell}`,
  clr의 프레임 체인이나 rs의 `Vec<PxFrame>`보다 단순). 지연 평가는
  `Cell`/`force-cell`(레코드 + memo). `module-context-key`라는 특수 키를
  환경에 심어서 import 로더(`:load-source`, `:cache`, `:source-id`)를
  전달 — Node의 `module.cljs`가 실제 파일 읽기를 담당.
- **빌트인 dispatch**: `builtins-value`(이름→`(->BuiltinValue :kind [])`
  맵, 알파벳순 정렬 관례 있음 — 새로 추가할 때 순서 지킬 것)로 등록,
  실제 실행은 `invoke-builtin`(`{:keys [name args]}` 받는 큰 `case`,
  `arguments`는 이미 강제평가된 값들). 커링은 `(< (count arguments) N)`
  이면 `->BuiltinValue`로 부분적용 리턴하는 패턴 반복.
- **import**: `load-module`(일반 import)과 `load-module-scoped`
  (2026-08-19 신설, scopedImport용)가 나란히 있음. 둘 다 `environment`의
  `module-context-key`에서 로더 얻어서 소스 읽고, `module-id`로 캐시
  확인. **`load-module-scoped`는 캐시를 안 탄다**(scope 다르면 같은
  파일도 결과가 다르므로) — `load-module`처럼 `(:cache context)`에
  절대 쓰지 않는 게 핵심. 두 함수 다 `(merge (builtin-environment)
  ...)`로 모듈의 시작 환경을 완전히 새로 만들어서 호출부 지역변수가
  안 새게 돼 있음(clr과 같은 패턴 — rs는 원래 이게 안 됐다가 2026-08-19
  에 `PxExpr::Isolated`로 뒤늦게 맞춘 것과 비교해볼 것).

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

- **경로가 일반 값이 아니다.** `import`/`scopedImport` 문법 안에서만
  파서가 경로 토큰을 직접 소비한다 — `builtins.isPath ./x`,
  `let p = ./foo; in p` 같은 건 전부 파싱 에러. 언젠가 일반 Path 값
  타입을 만들려면 `tokenizer.cljs`의 `:path` 토큰을 `parser.cljs`의
  `parse-primary`에서도 받아들이게 하고, `evaluator.cljs`에 Path 값
  표현을 새로 만들어야 함 — 2026-08-19에 `scopedImport`를 붙일 때 이
  경로를 시도했다가(atom-starts에 `:path` 추가) 되돌린 적 있음(정확히는
  `parse-primary`가 `:path`를 처리할 준비가 안 돼서 "expected-expression"
  에러로 이어짐 — 절반만 고치면 오히려 더 헷갈리는 상태가 됨을 확인).
- **`import`/`scopedImport`는 파서가 경로를 문자열 그대로 삼킴** — clr/clj
  는 경로를 일반 AST 식으로 평가해서 얻지만(그래서 동적으로 계산된 경로도
  이론상 가능), cljs는 파서 단계에서 리터럴 경로 토큰만 받는다(동적 경로
  `import (if cond then ./a.px else ./b.px)` 같은 건 안 됨).
- **환경이 프레임 체인이 아니라 평범한 맵.** clr(`[frame ...]`)이나
  rs(`Vec<PxFrame>`)보다 구조가 단순해서 스코프 주입(`load-module-scoped`)
  이 그냥 `merge` 한 줄로 끝난다 — clr처럼 새 프레임을 cons하거나 rs처럼
  전용 AST 노드를 새로 만들 필요가 없었음.

## 4. 오늘(2026-08-19) 실제로 고친 것들 — 무엇을, 왜

빠른 참고용 요약. 각 커밋 메시지에 훨씬 자세한 설명이 있다.

| 커밋 | 무엇을 |
|---|---|
| `b2e3ef2` | `let a.b = 1;` 같은 dotted 이름 바인딩 지원 |
| `9eadb52` | `let` 안 `inherit` 지원 |
| `eb930fd` | `throw`가 메시지를 안 담던 버그 |
| `8857891` | 빠져있던 `builtins.mod` 추가 |
| `52ee37e` | `removeAttrs`/`catAttrs` 신규 구현(4개 호스트엔 있었음) |
| `c4d9d76` | `abs`/`abort`/`intersectAttrs` 신규 구현 |
| `cd996f2` | `assertMsg`/`getEnv` 신규 구현 |
| `82c4daa` | `import`가 상대경로만 되고 절대경로는 안 되던 것 — 렉서에 `absolute-path-start?` 추가(clr의 규칙 참고). 처음에 `atom-starts`도 같이 고치려다 `parse-primary`가 준비 안 돼서 되돌림(§3) |
| `8bdb4c5` | `scopedImport` 신규 구현. `load-module-scoped` 신설(캐시 안 탐, `merge`로 스코프 주입) — 첫 시도에 바로 통과, rs처럼 nested import 누수 문제 없었음(환경이 매번 `merge (builtin-environment) ...`로 완전히 새로 만들어지는 구조라 애초에 그런 버그가 생길 수 없는 설계였음) |

교차검증에서 배운 것: 오늘 고친 빌트인 5개(`removeAttrs`/`catAttrs`/
`abs`/`abort`/`intersectAttrs`/`assertMsg`/`getEnv`, 총 7개)는 전부
"이름 자체가 아예 없던" 케이스였다 — clr/rs처럼 "이름은 있는데 동작이
다른" 유형의 숨은 버그는 이 호스트에서는 덜 나왔다. import 관련 두
개(절대경로, scopedImport)는 둘 다 한 번에 잘 됐는데, 이건 환경 모델이
단순한(맵 하나) 덕분으로 보인다 — 복잡한 프레임 체인/AST 치환 방식보다
사고 날 여지가 적음.
