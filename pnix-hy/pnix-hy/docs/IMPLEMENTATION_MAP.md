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

### 관련 문서 — 여기 없는 내용은 여기서 찾을 것

이 문서는 "무엇이 어디 있는지"에 집중한다. 아래는 이미 있는 다른 문서들 —
내용을 중복하지 않고 링크만 한다.

| 문서 | 다루는 것 |
|---|---|
| [`pnix-hy/SCOPE_LOCK.md`](../../SCOPE_LOCK.md) | 이 호스트+hy-meta의 권위 있는 범위 선언 — "scope 대비 완성"이란 말의 정의, 의도적으로 남겨둔 placeholder 목록(고치면 안 됨), 금지된 재구현 |
| [`pnix-hy/README.md`](../../README.md), [`pnix-hy/CLAUDE.md`](../../CLAUDE.md) | 상위 소개, 에이전트용 경계 노트(hy-meta ↔ pnix-hy 정체성 분리) |
| [`pnix-hy/pnix-hy/todo.md`](../todo.md) | **현재 진행 중인 작업만** — 과거 완료 이력은 `docs/archive/todo-history.md`, 신규 기능 제안은 `docs/proposals/NNNN-*.md` 참고(각각 별도 문서, 여기서 다시 나열 안 함) |
| [`docs/CAPABILITIES.md`](CAPABILITIES.md) | **자동 생성**(손 편집 금지) — public API 표면. `pnix-hy-project --capabilities`로 재생성, drift가 생기면 게이트가 잡음. 이 문서(`IMPLEMENTATION_MAP.md`)의 §2 빌트인 표와는 다른 것 — CAPABILITIES.md는 코드에서 직접 파생된 진실의 원천이고, §2는 5개 호스트를 나란히 비교하기 위해 수동으로 만든 스냅샷이다(§5 참고) |
| [`docs/IMPLEMENTATION_AUDIT.md`](IMPLEMENTATION_AUDIT.md) | 여러 에이전트가 돌린 감사 로그(23-에이전트 capability audit, 15-에이전트 mistake-hunt, 15-에이전트 stub-hunt) — "진짜 미구현 코드가 남아있지 않다"는 검증 기록 |
| [`docs/SEPARATION.md`](SEPARATION.md) | hy-meta vs pnix-hy vs interop vs mirror 레이어링이 왜 이렇게 나뉘었는지의 역사적 인벤토리/계획 |
| [`docs/INTEROP_ROLE_MATRIX.md`](INTEROP_ROLE_MATRIX.md) | interop 기능 × 소유자 × 상태 × proposal-ID 매트릭스 — 의도적 gap을 다시 열지 않도록 표시 |

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

## 4. 역사 — 무엇이 언제 만들어졌는가

**git log의 한계부터**: 이 저장소의 `pnix-hy/` 전체 이력은
`git log --oneline --all -- pnix-hy/`로 봐도 37개 커밋뿐이고, 그마저
첫 커밋(`4240414`, `init`, 2026-08-10)이 이미 완성된 17,292줄짜리
`pnix_runtime.py`를 통째로 들여온다. 3중 lane 빌트인 구조, hy-meta
self-host 부트스트랩, 렉서/파서/평가기 최초 작성 같은 진짜 "탄생" 사건은
이 repo git 이력으로 재구성이 **안 된다** — 그 이전 세션/작업공간에서
이미 끝나 있었던 스냅샷이 통째로 들어온 것. (`SCOPE_LOCK.md`가 인용하는
`374a8e4`/`3f0e186`/`314b89f`/`accad7b` 같은 커밋 해시는 이 저장소에
아예 존재하지 않는다 — 외부의, 지금은 사라진 이력을 가리키는 것이니
그대로 믿지 말 것.) 그 시기의 서사는 커밋이 아니라
[`docs/IMPLEMENTATION_AUDIT.md`](IMPLEMENTATION_AUDIT.md)와
[`docs/SEPARATION.md`](SEPARATION.md), `SCOPE_LOCK.md`에 글로 남아있다 —
"언제 만들어졌나"가 궁금하면 거기부터 볼 것.

`init` 이후, 즉 이 repo git 이력 안에서 실제로 있었던 주요 사건들:

| 커밋 | 날짜 | 무엇을 |
|---|---|---|
| `4240414` | 08-10 | `init` — pnix_runtime.py(17k줄) 등 전체가 완성된 스냅샷으로 한 번에 들어옴. 이전 이력 없음 |
| `0a513ae` | - | 호스트별 Trusting-Trust(DDC) 로드맵 추가, clj-meta/hy-meta의 실제 DDC gap을 닫음 |
| `7fb2ae2` | - | clr-meta Trusting-Trust DDC gap을 닫음; pnix-hy에 없던 native test corpus를 고침 |
| `f685007` | - | 독립 mini backend(hy-meta의 DDC 대조군)에 진짜 클로저 추가 |
| `fcc7671` | - | 독립 mini backend에 get/dot-method-call/keyword dict 추가 |
| `bc98ffa` | - | 독립 mini backend에 defmacro + quasiquote/unquote 추가 |
| `56439ca` | - | stale해진 `docs/CAPABILITIES.md` 재생성(§5의 drift 게이트가 원래 하는 일) |
| `0c5ee44` | - | `rust_corpus`의 cwd 버그, `tower_ladder`의 RecursionError 수정 |
| `71e911e` | - | import 순서에 따른 순환-import 취약성의 근본 원인 수정 |

이후 2026-08-19 하루 동안 있었던 일은 아래 §4-오늘 참고.

### 오늘(2026-08-19) 실제로 고친 것들 — 무엇을, 왜

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

## 5. 이 문서가 코드와 어긋나지 않게 유지하는 법

이 문서는 두 성격이 섞여 있다 — 어느 쪽인지 구분해서 신뢰할 것.

- **§2 빌트인 표는 자동 생성이 아니다.** 5개 호스트를 나란히 비교하기
  위해 수동으로 추출해 만든 2026-08-19 스냅샷이고, 빌트인이 추가/삭제될
  때마다 stale해진다. 다시 뽑으려면 저장소 루트에서:
  ```bash
  bin/gen-builtin-presence-matrix          # 새 표 출력
  bin/gen-builtin-presence-matrix --check  # 5개 문서 표가 실제 소스와 다르면 비영(non-zero) 종료
  ```
  이 스크립트는 grep 기반 휴리스틱이라(각 호스트 소스 안 등록 패턴을
  찾는 것) `import`/`scopedImport`처럼 예약 키워드로 구현된 것들은 못
  잡는다 — 그 두 줄은 여전히 손으로 `*` 표시가 돼 있다(§2 상단 각주
  참고). 표가 드리프트를 일으켰다고 나오면 다시 읽어보고 손으로
  갱신할 것 — 이 스크립트 자체를 게이트에 넣지는 않았다(5개 호스트를
  가로지르는 도구라 어느 한 호스트의 self-contained 게이트에 넣으면
  그 호스트의 자기완결 원칙을 깬다).
- **§1이 가리키는 `docs/CAPABILITIES.md`는 진짜 자동 생성+drift-게이트다.**
  `pnix-hy-project --capabilities`로 재생성되고, 그 결과가 코드와
  다르면 게이트가 실패한다(생성 원천 = 코드, "진실"은 코드 쪽에 있음).
  이 호스트의 public API 표면이 궁금하면 이 문서의 §1/§2보다
  `CAPABILITIES.md`를 먼저 믿을 것 — 이 문서는 "어디서 찾는지"와
  "다른 호스트와 어떻게 다른지"를 설명하는 내레이션이고, 실시간 진실은
  `CAPABILITIES.md` 쪽이다.
- clj/rs도 각자 CAPABILITIES.md(+clj는 LANE_REGISTRY.md/WIKI.md까지)를
  가진 같은 패턴이다(각 호스트 IMPLEMENTATION_MAP.md §5 참고). clr/cljs는
  아직 이런 자동 drift-게이트 문서가 없다 — clr/cljs 쪽 IMPLEMENTATION_MAP.md
  §5에 실제 미해결 gap으로 적어뒀다.
