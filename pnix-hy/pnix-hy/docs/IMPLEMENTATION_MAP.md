# pnix-hy 구현 맵

목적: 이 호스트에 뭐가 어떻게 구현돼 있는지 한눈에 찾아보는 문서. 같은 걸
중복해서 다시 만들지 않도록, 뭔가 다르게 동작하는 걸 발견했을 때 "진짜
버그인지 원래 그런 건지" 빨리 판단할 수 있도록, 다른 4개 호스트와 비교할
때 참고하도록 만들었다. 2026-08-19 작성 시작 — 이후 구조가 바뀌면 여기도
같이 갱신할 것.

## 1. 아키텍처 개요 — 어디서 뭘 찾아야 하는가

핵심 파일은 사실상 하나: **`pnix_hy/pnix_runtime.py`**(16000줄 이상,
5개 호스트 중 가장 큼). 렉서/파서/평가기/빌트인/self-test corpus가 전부
이 안에 있다.

- **빌트인이 3중으로 구현돼 있다** — 이게 이 호스트의 제일 큰 특징.
  같은 기능이 세 군데 따로 있고, 새 빌트인/버그 수정은 **셋 다** 고쳐야
  한다(2026-08-19 `storePath` 수정 때 이걸 놓쳐서 게이트가 두 번 깨짐):
  1. **인터프리터 lane** — `native_builtins()` 안 `builtins = {...}`
     dict(순수 Python 함수, 약 4750~5030줄). 일반 `--pnix` 평가가 씀.
  2. **compile backend lane** — 큰 문자열로 박혀있는 Hy s-표현식
     안(약 10300~10600줄 부근). Hy 코드 생성 경로가 씀. 여기 안에서는
     `pnix_error` 같은 파이썬 최상위 이름이 스코프에 없다 —
     `(pnix-error "...")`(Hy 커널함수) 쓸 것.
  3. **`_C(...)` minimal 백엔드 lane** — 별도의 축약형 Python 딕셔너리
     (`b["name"]=_C(lambda ...)` 패턴, 약 12200~12800줄 부근). 이 lane도
     `pnix_error`가 스코프에 없어서 `raise Exception(...)`을 직접 쓴다
     (기존 관례: `_topathstr` 같은 함수가 이미 이렇게 함).
  세 lane이 다른 값을 내면 self-test의 `lanes_agree` 필드가 `False`로
  뜬다 — 이건 항상 "버그"는 아니고, `mapAttrsToList`처럼 잘 알려진
  library-style 빌트인들 상당수가 이미 원래부터 `lanes_agree: False`다
  (compile lane이 그 이름을 모르는 pre-existing 패턴, 2026-08-19
  hy-builtins 에이전트가 확인). 새로 추가한 빌트인이 `lanes_agree: False`
  면 "새로 생긴 문제"인지 "이미 있던 패턴과 같은 것"인지 먼저 확인할 것.
- **렉서/파서**: 같은 파일 안, 함수명으로 검색(`tokenize`/`parse` 계열).
  경로 vs 나눗셈 구분 — 2026-08-19에 clr의 "숫자 토큰 뒤만 나눗셈" 규칙
  중 **숫자 제외 부분만** 이식(식별자/닫는 괄호 제외는 안 가져옴 — 기존
  테스트를 깨뜨려서). clr/rs/cljs보다 좁은 버전.
- **환경/스코프**: Python 표준 방식(클로저 + dict 체인 추정 — 자세한
  변수 조회 함수는 `lookup_env` 근방).
- **import**: `import <path>`가 **처음부터** 절대/상대경로 둘 다,
  파일 모드/인라인(`--pnix`) 모드 둘 다에서 잘 됐다 — 2026-08-19 교차
  검증에서 5개 호스트 중 **유일하게 손댈 필요가 없었던** 호스트(다른
  4개는 각자 다른 이유로 절대경로 또는 scopedImport 또는 둘 다 안 됐음).
  상대경로는 import하는 파일 자기 자신의 디렉터리 기준으로 정확히
  해석됨(호출 시점 cwd 기준 아님, 중첩 import도 확인함). `scopedImport`
  도 원래부터 있었음.
- **hy-meta / stage1 / stage2**: 이 저장소는 pnix 런타임 말고 Hy/Python
  자기컴파일 증명 레인(`hy-meta/`)도 같이 있다 — `pnix_runtime.py`와는
  분리된 코드베이스, 이 문서는 pnix 런타임(`pnix_hy/`)만 다룬다.

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

- **빌트인이 3중 구현(§1)** — 이게 다른 4개 호스트엔 없는 이 호스트만의
  구조. 새 빌트인 만들 때 한 lane만 고치고 끝냈다고 착각하기 쉽다.
- **`pnixMounts`가 없다**(clj/clr/cljs엔 등록만이라도 있음, 다만 그쪽도
  전부 에러 스텁 — §3/`todo.md` "미래 아이디어" 참고). 2026-08-19에
  다른 호스트들 빌트인 40개 넘게 이식하면서도 이거 하나는 일부러 안
  가져왔다 — 3개 참조 호스트끼리도 서로 동작이 다 달라서(정책 거부/
  타입에러/미종결) 합의 기준이 없었기 때문(hy 에이전트가 직접 확인).
- **경로 리터럴이 clr보다 좁게 처리됨** — clr/rs/cljs는 "숫자 또는
  식별자/닫는 괄호 뒤가 아니면 경로"까지 넓게 잡는데, hy는 "숫자 뒤가
  아니면"까지만(식별자 뒤 제외는 기존 테스트를 깨뜨려서 일부러 안 가져옴,
  2026-08-19). 완전히 같은 동작을 기대하지 말 것.
- **import가 처음부터 제일 완성도 높았다** — 다른 4개 호스트가 오늘 각자
  다른 이유로 절대경로/scopedImport를 새로 만들거나 고쳐야 했던 것과
  대조적으로, 이 호스트는 그대로 기준(oracle) 역할을 했다.

## 4. 오늘(2026-08-19) 실제로 고친 것들 — 무엇을, 왜

| 커밋 | 무엇을 |
|---|---|
| `e78dfe4` | `builtins.length`가 문자열을 받아들이던 버그(Nix는 리스트/attrset만 허용) — 인터프리터/compile backend/Hy-codegen 3곳 다 고침 |
| `ad65caf` | `N/M`이 나눗셈이 아니라 경로로 잘못 렉싱되던 버그 — clr 규칙 중 숫자 제외 부분만 이식(§3) |
| `c501cbb` | 다른 호스트엔 있고 hy만 없던 빌트인 12개 추가(`pnixMounts`는 합의 없어서 의도적으로 제외, §3). 4/4 자체 게이트(`--check`/`--gate`) 통과 확인, 세 lane 다 확인 |
| `722a124` | `builtins.storePath`가 순수 평가기인데 실제 파일 경로를 계산해버리던 버그(다른 4개 호스트는 "store 없음" 에러) — 3개 lane 다 있는 자체 구현을 다 고쳐야 했고, 그 과정에서 self-test 3개가 예전(틀린) 동작을 기준으로 고정돼 있던 것도 같이 고침, compile lane은 `pnix_error`가 스코프에 없어서 별도 헬퍼 필요했음(§1) |

교차검증에서 배운 것: 이 호스트는 "3중 구현"이라는 특수성 때문에, 겉보기
게이트가 초록불이어도 **lane 하나를 빠뜨린 채 커밋했다가 나중에 게이트
전체 재실행에서 발견**되는 패턴이 실제로 있었다(`storePath` 수정 때
`--check`는 바로 통과했는데 `--gate`의 `rust_corpus`/`toolkit_self_checks`
에서 뒤늦게 컴파일 lane 에러가 잡힘). 이 호스트에서 빌트인을 고칠 땐
`--check` 하나만 보고 끝내지 말고 반드시 `--gate`까지 돌릴 것.
