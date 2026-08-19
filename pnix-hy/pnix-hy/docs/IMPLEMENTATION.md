# pnix-hy 구현 맵

목적: 이 호스트에 뭐가 어떻게 구현돼 있는지 한눈에 찾아보는 문서. 같은 걸
중복해서 다시 만들지 않도록, 뭔가 다르게 동작하는 걸 발견했을 때 "진짜
버그인지 원래 그런 건지" 빨리 판단할 수 있도록, 다른 4개 호스트와 비교할
때 참고하도록 만들었다. 2026-08-19 작성 시작 — 이후 구조가 바뀌면 여기도
같이 갱신할 것.

> **2026-08-20 문서 통합**: `SCOPE_LOCK.md`(범위 선언, §4로), `docs/SEPARATION.md`
> (레이어링 계획, §5로), `docs/IMPLEMENTATION_AUDIT.md`(감사 로그, §8로),
> `docs/INTEROP_ROLE_MATRIX.md`(interop 매트릭스, §6으로) 를 이 문서로 흡수하고
> 원본은 삭제했다. `todo.md`는 `docs/TODO.md`로 옮겼다(내용은 그대로, 위치만).
> 파일명 `IMPLEMENTATION_MAP.md` → `IMPLEMENTATION.md`로 개명(5개 호스트 공통
> 문서 세트: IMPLEMENTATION/TODO/BUGS/PLANS). 옛 경로를 가리키는 링크가 있으면
> 이 문서 또는 `docs/BUGS.md`/`docs/PLANS.md`를 대신 볼 것.

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
  분리된 코드베이스, 이 문서는 pnix 런타임(`pnix_hy/`)만 다룬다. 두
  레인이 왜 이렇게 나뉘었는지는 §5(레이어링) 참고.

### 관련 문서 — 여기 없는 내용은 여기서 찾을 것

이 문서는 "무엇이 어디 있는지"에 집중한다. 아래는 이미 있는 다른 문서들 —
내용을 중복하지 않고 링크만 한다.

| 문서 | 다루는 것 |
|---|---|
| 이 문서 §4(범위 선언) | 이 호스트+hy-meta의 권위 있는 경계 선언 — "scope 대비 완성"이란 말의 정의, 금지된 재구현, in/out-of-scope. 예전 `SCOPE_LOCK.md`(2026-08-20 통합, 원본 삭제) |
| [`pnix-hy/README.md`](../../README.md), [`pnix-hy/AGENTS.md`](../../AGENTS.md) | 상위 소개, 에이전트용 경계 노트(hy-meta ↔ pnix-hy 정체성 분리). `pnix-hy/CLAUDE.md`는 `AGENTS.md`로의 심볼릭 링크(다른 에이전트 툴 호환용) |
| [`docs/TODO.md`](TODO.md) | **현재 진행 중인 작업만**. 과거 완료 이력은 `docs/archive/todo-history.md` 또는 이 문서 §8, 신규 기능 제안은 `docs/proposals/NNNN-*.md`(요약 인덱스는 `docs/PLANS.md`) |
| [`docs/BUGS.md`](BUGS.md) | 알려진 버그/한계 + **의도적으로 고치지 않는** 항목(placeholder) — "이건 버그 아니라 의도된 제한"으로 명시 표시 |
| [`docs/PLANS.md`](PLANS.md) | 아직 확정 안 된 미래 방향 + `docs/proposals/` 전체 인덱스(1-2줄 요약+링크) |
| [`docs/CAPABILITIES.md`](CAPABILITIES.md) | **자동 생성**(손 편집 금지) — public API 표면. `pnix-hy-project --capabilities`로 재생성, drift가 생기면 게이트가 잡음. 이 문서(`IMPLEMENTATION.md`)의 §2 빌트인 표와는 다른 것 — CAPABILITIES.md는 코드에서 직접 파생된 진실의 원천이고, §2는 5개 호스트를 나란히 비교하기 위해 수동으로 만든 스냅샷이다(§9 참고) |
| 이 문서 §5(레이어링) | hy-meta vs pnix-hy vs interop vs mirror 레이어링이 왜 이렇게 나뉘었는지. 예전 `docs/SEPARATION.md`(2026-08-20 통합, 원본 삭제) |
| 이 문서 §6(interop 매트릭스) | interop 기능 × 소유자 × 상태 매트릭스. 예전 `docs/INTEROP_ROLE_MATRIX.md`(2026-08-20 통합, 원본 삭제) |
| 이 문서 §8(역사) | 여러 에이전트가 돌린 감사 로그 — "진짜 미구현 코드가 남아있지 않다"는 검증 기록. 예전 `docs/IMPLEMENTATION_AUDIT.md`(2026-08-20 통합, 원본 삭제) |

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
  전부 에러 스텁 — `docs/BUGS.md`/`docs/PLANS.md` "pnixMounts" 참고).
  2026-08-19에 다른 호스트들 빌트인 40개 넘게 이식하면서도 이거 하나는
  일부러 안 가져왔다 — 3개 참조 호스트끼리도 서로 동작이 다 달라서(정책
  거부/타입에러/미종결) 합의 기준이 없었기 때문(hy 에이전트가 직접 확인).
  **이건 버그가 아니라 의도된 제한** — 자세한 근거는 `docs/BUGS.md`.
- **경로 리터럴이 clr보다 좁게 처리됨** — clr/rs/cljs는 "숫자 또는
  식별자/닫는 괄호 뒤가 아니면 경로"까지 넓게 잡는데, hy는 "숫자 뒤가
  아니면"까지만(식별자 뒤 제외는 기존 테스트를 깨뜨려서 일부러 안 가져옴,
  2026-08-19). 완전히 같은 동작을 기대하지 말 것. **이것도 의도된 제한**
  — `docs/BUGS.md` 참고.
- **import가 처음부터 제일 완성도 높았다** — 다른 4개 호스트가 오늘 각자
  다른 이유로 절대경로/scopedImport를 새로 만들거나 고쳐야 했던 것과
  대조적으로, 이 호스트는 그대로 기준(oracle) 역할을 했다.
- **`unsafeGetAttrPos`가 Nix 스펙과 일치하는 유일한 호스트** —
  `{file; line; column;}` 모양을 이미 정확히 돌려준다(다른 4개는 각자
  다른 이유로 부정확/부재). 5개 호스트를 나중에 통일할 때 이 구현이
  목표 모양이 될 가능성이 높다 — `docs/PLANS.md` "unsafeGetAttrPos 통일"
  참고.

## 4. 범위 선언 (SCOPE LOCK)

> 이 절은 예전 `pnix-hy/SCOPE_LOCK.md`(2026-07-01 수립)를 그대로 옮긴
> 것이다 — hy-meta(호스트 레인)와 pnix-hy(pnix 런타임 레인) **둘 다**
> 관장하는 권위 있는 경계 선언. 원본 파일은 2026-08-20 이 절로 통합 후
> 삭제했다. 무엇을 구현하기 전에 먼저 읽을 것.

### 4.1 Source of truth — 이력 노트

"scope 안에서 닫힘" 주장은 scope-relative일 뿐 아니라 branch/ref-relative였다.
2026-07-01에 이 저장소가 만들어지기 전 워크스페이스에서 `main`이
`374a8e4`에서 `3f0e186`로 fast-forward됐고(감사·drift-guard·stub-hunt
verdict·이 scope lock 포함), 그 시점 기준 `main`이 권위 있는 닫힌 상태로
선언됐다. **주의**: 이 문단이 인용하는 `374a8e4`/`3f0e186`/`314b89f`/
`accad7b` 같은 커밋 해시는 **이 저장소(`pnix-hy/`) git 이력에 존재하지
않는다** — §8(역사)에서 설명하듯 이 repo의 첫 커밋(`4240414`, `init`)은
이미 완성된 스냅샷을 통째로 들여왔고, 그 이전 이력은 재구성 불가능한 외부
워크스페이스의 것이다. 그대로 믿지 말고, 이 문서(및 §8)의 서술을 참고할 것.

### 4.2 Status — scope-relative (이 정확한 표현을 쓸 것)

**맞음:**
> pnix-hy / hy-meta는 현재 정의된 meta-circular projection scope 안에서 open todo와
> genuinely unimplemented stub는 0으로 수렴했다.
>
> Complete **with respect to the stated meta-circular-projection scope.**

**틀림(쓰지 말 것):** "전체 AI가 완성됐다" / "Complete overall" / "the project is finished."
`미완 0`은 **항상** scope-relative다. 여기서 완성이란 오직: 선언된 Hy/Python ↔ pnix
meta-circular projection surface(§4.5) 안에서 open todo가 없고 genuinely unimplemented stub가
없다는 뜻이다.

증거(2026-07-01 기준): 두 todo `[ ]` = 0; `--check` 44/44 all_ready; working tree clean;
origin-synced. 세 번의 독립 multi-agent adversarial sweep 수렴 — capability audit(23 agents),
follow-up re-verification(8 agents), stub-hunt(15 agents) — 모두: genuine unimplemented code
없음, 모든 placeholder는 의도적/문서화됨. 상세 검증 기록은 §8.

### 4.3 대원칙

> **의도적 placeholder를 미구현으로 재해석해서 구현하지 말 것.**
> (No new implementation may reinterpret intentional placeholders as missing work.)

미래의 주된 위험은 "미구현"이 아니라 — LLM이 문서화된 placeholder를 gap으로 오인해 이 닫힌 scope를
다시 열어젖히는 것이다. 의도적 placeholder 전체 목록은 `docs/BUGS.md` — 여기서 다시 나열하지 않는다.

### 4.4 금지된 재구현 (scope 재개봉 요소)

- `docs/BUGS.md`의 의도적 placeholder를 gap으로 재해석해 "채우기".
- pnix macro / quasiquote / reader-macro(§4.5 언급) 구현 — pnix 언어가 명시적으로 동형(homoicony)을
  채택하기로 결정하지 않는 한(proposal 수준 결정).
- `derivation` outPath/drvPath용 Nix store hashing.
- fallback이나 error-message를 "기능"으로 바꾸기.
- §4.5의 OUT-of-scope 항목을 "미구현 작업"으로 이 저장소에 끌어들이기.
- 새 기능을 proposal 대신 `docs/TODO.md [ ]`로 시작(§4.7).

### 4.5 In scope vs OUT of scope

**IN scope**(이 lock이 관장): Hy/Python ↔ pnix meta-circular projection surface(원문 §1–§24
체크리스트), hy-meta = HOST 자기컴파일/평가/재현/inspect proof 레인; pnix-hy = 그 위 pnix 런타임;
interop = 명시적 경계; SINGLETON pnix mirror; 별도 수렴 게이트로서의 4-lane parity.

**OUT of scope**(별도 문제 — 여기서 "미구현" 아님, 이 lock 아래 추가 금지): 더 큰 제품 scope ·
pnix 전체 언어 완성 · cross-repo ABI 통합 · nix-msv-template 레이어 · pnix-clj feature branch ·
rs-meta stageN · pnix-hs · 실서비스 런타임. 각각 자기 scope + proposal이 필요하며, pnix-hy /
hy-meta의 gap이 아니다.

**2026-07-08 OWNER AMENDMENT로 이 펜스가 일부 해제됨** — §4.8 참고.

### 4.6 ABI 경계 — 유일한 공유 envelope

두 레인(hy-meta ↔ pnix-hy) 사이의 유일한 공유 계약:
- **witness FIELD SCHEMA**(in_hash/out_hash/env_hash/status/loss + InteropRecord 필드명) —
  런타임 drift-guard(`gate.gate_report:witness_schema_ok`) 포함.
- **opaque-ref shape** — `__hy_meta_opaque__`(호스트) / `__pnix_opaque__`(pnix fallback).

그 외는 전부 레인-로컬. hy-meta는 호스트 Python/Hy artifact·import hook·clean replay·
introspection·실행 floor를 소유; pnix-hy는 pnix reader/parser/AST/IR/eval/value/builtins/mirror/
stage-ladder/gate를 소유(§5에 상세). 공유 envelope 변경은 두 레인 + drift-guard를 함께 갱신하고
proposal에 기록.

### 4.7 변경 프로세스

- 새 기능은 **proposal 문서**(`docs/proposals/NNNN-<slug>.md`)로 시작, `docs/TODO.md [ ]`가
  아니라. `docs/TODO.md`는 in-scope·이미 합의된 작업만.
- proposal은 반드시 밝힌다: 어느 scope인가, 의도적 placeholder나 OUT-of-scope 항목을 건드리는가,
  그렇다면 이를 승인하는 명시적 human decision.
- proposal이 수락된 뒤에만 작업이 `docs/TODO.md`로 들어간다.
- proposal 전체 인덱스(1-2줄 요약 + 상태)는 `docs/PLANS.md`.

### 4.8 OWNER AMENDMENT 2026-07-08 — shared common-.px core admitted IN scope (B6)

Owner-authorized proposal per §4.5 (which requires OUT-of-scope items to be
admitted by an explicit scope + proposal). The **shared common-`.px` core**
track is now IN scope for this repo:

- loading common `.px` from `../pnix-meta` through the pnix-hy runtime;
- the cross-repo canonical-result + effect/capability ABI (blockers B1–B3,
  originally tracked in the `pnix-zero` predecessor repo's project-wiki — this
  self-contained tree has no such sibling tree to load);
- the "full pnix language" growth **only as needed to run the shared corpus**.

Bound by the constitution (`./AGENTS.md`, symlinked as `CLAUDE.md`):

1. **Non-regression** — the existing closed scope stays closed and its
   gate stays green; the shared-core track is ADDITIVE, never a rewrite.
2. **Meta-first / no cram** — grow through `hy-meta`; do not race the product
   surface ahead of the substrate (this repo is attempt #3, not `clj-msv`).
3. The §4.5 fence REMAINS for everything NOT part of this shared-core track.

This amendment lifts the §4.5 fence for the shared-core track only.

Note: `docs/TODO.md`의 "Host-language import of pnix product library" 절이 이
트랙의 현재 진행 상태(dot-nix 통합, `PNIX_HY_HOME`/`PNIX_HY_LIBRARY` 등)를
다룬다 — §7(호스트 라이브러리로 임베드하기) 참고. `pnix-meta`(공용 `.px`
라이브러리 본체)는 사용자 본인이 직접 손으로 작성하는 별도 작업이라 이
저장소의 AI 작업 대상이 아니다.

### 4.9 한 줄 요약

> pnix-hy / hy-meta는 현재 scope 안에서 닫혀 있다. 다음 위험은 "미구현"이 아니라 — 에이전트가 닫힌
> scope를 다시 여는 것. 경계를 지켜라; 재해석하지 말고 proposal하라.

## 5. 레이어링 — hy-meta ↔ pnix-hy ↔ interop ↔ mirror

> 이 절은 예전 `docs/SEPARATION.md`(2026-07-01 작성, 분리 실행 완료 후
> 재검증됨)를 압축한 것이다. 원문은 point-in-time 코드 인벤토리(파일:줄번호
> 포함, "심볼 이름이 권위이고 줄 번호는 drift" 라고 원문 스스로 경고함)였고
> 거기 적힌 마이그레이션은 **전부 실행 완료**(SEP1-5, IB1-4, hy-meta SR1-6).
> 아래는 그 결과로 도달한 최종 레이어링만 남긴다 — 상세 파일:줄번호 인벤토리가
> 필요하면 git 이력(`docs/SEPARATION.md`가 있던 커밋)을 볼 것.

### 5.1 왜 이렇게 나뉘었나

이전에 프로젝트는 **mirror**가 있어야만 meta-circular 능력이 존재한다고
취급했다. 너무 좁다. Meta-circular 능력은 전체 집합이다: reader · parser ·
form-as-data · AST-as-data · IR-as-data · compiler-as-data · eval/apply ·
quote/quasiquote · macro expansion · stage bootstrap · artifact reproduction ·
import hook · module loading · environment replay · bytecode/code-object
inspection · roundtrip · drift detection · witness/proof · gate/capability ·
interop · self-hosting ladder. **Mirror는 하나의 관찰 표면이지 전체가
아니다.**

### 5.2 최종 아키텍처 (도달한 상태)

```
hy-meta   = Hy/Python self-compile/evaluate/reproduce proof lane (stage chain, kernel,
            import hook, Python AST/code/pyc/marshal artifacts, mirror/drift, stage8/9,
            clean replay, host introspection 소유)
pnix-hy   = pnix runtime on top of hy-meta (pnix reader/parser/AST/eval/value/builtins/
            env, pnix-in-Hy self-hosting kernel source, sandbox/cache/diagnose/receipt,
            singleton pnix mirror, pnix stage ladder + gates/witnesses 소유)
interop   = explicit bidirectional bridge; host objects ↔ pnix values only through
            loss-marked, effect-classified, capability-checked adapters
mirror    = NOT the source of meta-circularity; ONE pnix-side observation entrypoint with
            many trace facets (`pnix_mirror.singleton_mirror_run`)
```

### 5.3 각 레인이 실제로 소유하는 것

- **hy-meta** (`hy-meta/bootstrap.py` 등) — Hy 커널 로드/평가, Python import
  hook(`KernelHyLoader`/`KernelHyFinder`/`KernelHyImportHook`), artifact/hash/
  pyc/marshal(`artifact_from_ast`, `stable_code_payload` 등), mirror/drift
  checks(`run_mirror_check`, `run_stage7_check`, DDC 등), stage8/9 proof,
  clean-env subprocess, host introspection/boundary checks, stage10-16
  governance overlay. pnix을 **import하지 않음** — pnix-agnostic.
- **`pnix_hy/hy_mirror.py`** — hy-meta로 가는 interop bridge(stage7 worker,
  projection worker) + Hy→pnix projection 함수들(`hy_form_projection`,
  `hy_quasiquote_projection`, `hy_defmacro_projection`,
  `hy_reader_macro_projection`, `hy_import_projection` 등). 호스트
  introspection 기계 자체는 `hy-meta/host_introspect.py`로 이전 완료,
  `hy_mirror.py`는 path-import + re-export로 하위호환만 유지.
- **`pnix_hy/pnix_runtime.py`** — 진짜 pnix 런타임(reader/parser/AST/eval/
  apply/value model/builtins/env, §1 참고) + pnix-in-Hy self-hosting kernel
  source(`HY_AST_EVALUATOR_SOURCE`/`HY_AST_COMPILER_SOURCE`/
  `COMPILER_PRELUDE`, host-direct emitter `_px_*`) — **이건 pnix 자체의
  self-hosting ladder이지 Hy의 meta-circular이 아니다**, pnix-hy 소유가
  맞다. 단 host Python 실행(`compile()`/`exec()`)은 hy-meta API
  (`hy-meta/host_exec.py:run_python_source`)로 위임돼 있다(§5.4).
- **`pnix_hy/pnix_mirror.py` + `pnix_hy/cli.py`** — pnix self-mirror
  runners, interop = projection/synthesis toolkit(§6), production/runtime
  layer(`safe_eval`/`static_purity_check`/`cached_eval`/`diagnose`/
  `eval_receipt`/`specialize_pnix`/`meta_circular_tower`), 31+개 `*_report`
  self-check가 `cli.py:_toolkit_reports()`에 등록돼 `--check`/`--gate`를
  구성.

### 5.4 마이그레이션 완료 기록

- **호스트 introspection**: `hy-meta/host_introspect.py`가 정본, `hy_mirror.py`
  는 호환 재노출만.
- **emitted 코드의 호스트 실행**: `pnix_runtime.py`의 `compile()`/`exec()`가
  `hy-meta/host_exec.py:run_python_source`를 거치도록 라우팅됨(hy-meta 부재
  시엔 byte-identical inline fallback으로 standalone 유지). External-oracle
  subprocess harness(`_run_original_px`/`original_oracle_report`)는 여전히
  optional, core 런타임 밖.
- **import hook**: pnix-hy는 raw `importlib`를 소유하지 않는다 — hook은
  hy-meta(`KernelHyLoader` 등). `.px` import 의미(SEMANTICS)만 pnix-hy가
  정의하고, 실제 `sys.meta_path` 통합은
  `hy-meta/import_hook.py:install_pnix_import_hook`를 interop이 wiring.
- **mirror singleton화**: `pnix_mirror.singleton_mirror_run`이 유일한 정본
  경로 — parse/lower/eval을 한 번만 하고 facet
  (`:mirror/source|token|ast|ir|eval-step|value|effect|interop|error|witness`)
  으로 emit, 하나의 result hash + witness 생성. `run_once`/`mirror_chain`/
  `run_mirror`/`stage_tower`는 이제 이 singleton 위의 legacy 뷰일 뿐.
- **pnix stage ladder**: `pnix_hy/stage.py:pnix_stage_ladder` — 7단계
  (stage1 direct eval → stage2 normalized-AST eval → stage3 content-addressed
  store eval → stage4 AST roundtrip integrity → stage5 singleton mirror route
  → stage6 deterministic replay → stage7 interpreter==compiler closure), 전부
  stage1과 값 일치를 검증. hy-meta stage8/9(호스트 컴파일러 안정성 증명)와는
  별개 — 이쪽은 **pnix 런타임** 안정성 증명.
- **witnesses/gates**: `pnix_hy/gate.py` — `EFFECT_OF`(impure 빌트인 →
  effect class), `gate_check(source, granted=)`(effect별 capability grant),
  `make_witness`(결정적 content-hash witness, 타임스탬프 없음 → 재현 가능).
- **IR layer**: `pnix_hy/ir.py` — 정규화된(위치-무관, 구조적으로 canonical)
  AST이자 직접 평가 가능한 canonical runtime 표현(`lower_to_ir`/`ir_of`/
  `eval_ir`/`ir_roundtrip`). Host Python emission(`_px_*`)은 IR이 아니라
  실행 아티팩트.

## 6. Interop 기능 매트릭스

> 이 절은 예전 `docs/INTEROP_ROLE_MATRIX.md`(D4)를 그대로 옮긴 것이다.
> 기능 × 소유자 × 상태 × proposal — 에이전트가 의도적 gap을 다시 열지
> 않도록. 소유 레인은 §4.5/§4.6 기준: **hy-meta** = 호스트 자기컴파일/평가/
> 재현 proof 레인; **pnix-hy** = pnix 런타임 + Hy↔pnix 투영; **interop** =
> 명시적 경계(pnix-hy 소유).

| 기능 | 소유 | 상태 | Proposal / 심볼 |
|---|---|---|---|
| Hy 매크로 확장 기계 | hy-meta 호스트 레인 (Hy) | 존재 / 증명됨 | `hy_mirror.hy_macroexpand_projection` |
| Hy 매크로 투영 → pnix | pnix-hy | 존재 | `hy_mirror.hy_macro_step_trace` (투영; witness 필드 없음) |
| **pnix-쪽 매크로 / quasiquote / reader-macro** | — | **의도적 GAP — `docs/BUGS.md` 참고, 다시 열지 말 것** | §4.3/§4.4; `hy_mirror._QUASIQUOTE_PNIX_NOTE` |
| Hy 매크로를 pnix-투영 폼 위에 | pnix-hy | shipped 0003 | `pnix_mirror.hy_macro_over_pnix` (C1) |
| pnix 값을 Hy quasiquote 구멍에 | pnix-hy | shipped 0003 | `pnix_mirror.hy_quasiquote_over_pnix` (C2) |
| quasiquote ↔ specialize staging 대응 | pnix-hy | shipped 0003 | `pnix_mirror.quasiquote_specialize_correspondence` (C3) |
| Hy `#px` reader macro가 pnix 임베드 (read-time) | pnix-hy | shipped 0005 | `hy_mirror.hy_read_with_pnix_reader` (C4) |
| Python AST / code-object / pyc / marshal artifact | hy-meta 호스트 레인 | 존재 | `hy-meta/host_exec.py`, `host_introspect.py` |
| Python AST 투영 → pnix | pnix-hy | 존재 | `pnix_mirror.synthesize_pnix_from_hy` |
| 값 interop (to_host / from_host) | interop | 존재 + loss-fidelity 0001 | `interop.to_host/from_host`, `roundtrip_host_value` (A1–A6) |
| pnix 소스에서 host callable 호출 | interop | shipped 0002 | `interop.host_callable_to_pnix` (B1, host-call 게이트) |
| host callable / method 호출 | interop | 존재 + kwargs 0002 | `interop.call_host(kwargs=)`, `call_host_method` |
| host-callable arity → functionArgs | interop | shipped 0002 | `interop.host_callable_arity` (B3) |
| pnix callable 래퍼 (host-facing) | interop | 존재 + typed error 0006 | `interop.wrap_pnix_callable` → `InteropError` |
| module 투영 (+ callable) | interop | 존재 + wrap_callables 0002 | `interop.host_module_to_pnix(wrap_callables=)` (B5) |
| opaque-ref 레지스트리 + method-level | interop | 존재 | `interop.make_opaque_ref/resolve_opaque/call_host_method` |
| capability / effect 게이트 | pnix-hy | 존재 | `gate.gate_check`, `interop.check_capability` |
| 결정적 witness | 공유 §4.6 envelope | 존재 (drift-guarded) | `gate.make_witness`, `gate.gate_report:witness_schema_ok` |
| 투영-drift 분류기 | pnix-hy | shipped 0004 | `pnix_mirror.classify_drift` (C5) |
| Hy-쪽 reification (대칭) | pnix-hy | shipped 0004 | `pnix_mirror.reify_hy` (C7) |
| mirror OFF에서도 interop 동작 | interop | shipped 0004 (불변식) | `interop.no_mirror_report` (C8) |
| cross-boundary 에러 계약 | interop | shipped 0006 | `interop.is_interop_error/try_call_host` (D1) |
| 공유 shape의 opaque-ref lifecycle (D2) | 공유 envelope | 후보 — 양-레인 + drift-guard | `0000` |
| versioned correspondence ABI (D3) | 공유 envelope | 후보 — 양-레인 + drift-guard | `0000` |
| host artifact interop envelope (codex P9) | hy-meta 호스트 레인 | pnix-hy interop scope 밖 | 자체 hy-meta proposal |

규칙: **의도적 GAP** 또는 **scope 밖**으로 표시된 행은 "미구현 작업"이 아니다. 새 행은
`docs/proposals/NNNN-*.md`(§4.7)로 들어오지, 절대 `docs/TODO.md [ ]`로 들어오지 않는다.

## 7. 호스트 라이브러리로 임베드하기

pnix-hy는 두 축으로 쓸 수 있다: **pnix-main**(pnix 언어 REPL/`.px` 평가)과
**host-main**(호스트 Python/Hy 코드에서 `import pnix_hy`로 pnix를 라이브러리로
불러쓰기). 정본 문서는 monorepo 루트의 `HOST_DEV_ENV.md`
(`~/dot-nix/dev/PNIX-HOSTS.md`가 HM 미러) — 이 절은 pnix-hy 쪽 요약이다.

### 7.1 dot-nix (홈매니저) 통합 — 2026-08-13 이후 상태

dot-nix가 `pnix-hy-python` / `pnix-hy-hy`를 노출하고 `pnix-hy-host`로
`python`/`python3`/`hy`에 조인한다. `pkgs.python311` 전역 오버라이드는
**의도적으로 안 함**(nixpkgs 빌더가 깨짐) — PATH/env join만(classpath,
PYTHONPATH, link path, NODE_PATH, DLL HintPath 같은 것).

- `pnix-<host>-pnix` = 이 호스트에서 pnix 언어 표면(REPL/`.px` 평가).
- `pnix-<host>-<lang>` = 일상 호스트 개발용 호스트-언어 인터프리터/컴파일러.
- pnix **제품** 절반이 만드는 라이브러리는 **호스트-언어** 라이브러리다 —
  이 호스트 언어에서 로드되도록 보장되며, 다른 호스트로 옮길 수 있는
  portable 공용 바이트코드로 **가정하지 않는다**.
- 공용 portable `.px` 라이브러리(예전 pnix-meta 스타일) 트랙은 §4.8 참고 —
  본체는 사용자가 직접 작성하는 미래 작업, 여기 host-local import 작업을
  막지 않는다.

### 7.2 hy 호스트 — 랜딩된 상태 (2026-08-14)

1. **이중 축 문서**: `HOST_DEV_ENV.md`, 호스트 `AGENTS.md`/`README.md`.
2. **Host-main**: `PYTHONPATH` + `pnix_hy`가 HM `pnix-hy-host`
   (`python`/`hy`)를 통해 잡힘.
3. **호스트-언어 `.px` import**: `pnix_hy.eval_file`(= `run_px`); 패키지 설치.
4. **환경변수**: `PNIX_HY_HOME`, `PNIX_HY_LIBRARY`, `PNIX_HY_PYTHON`.
5. **공개 API**: `import pnix_hy` — `__all__` + `HOST_IMPORT.md`가 현재
   계약(어떤 서브모듈이 host-library API로 안정적인지). `pnix_hy/py.typed`
   + setuptools package-data로 타입 힌트 배포(2026-08-14). 더 상세한 stub은
   외부 소비자가 필요해지면 그때 추가.

호스트 dual-axis + 라이브러리 import는 일상 작업 기준 **닫힘**(post
host-env, 2026-08-14). 잔여 선택 작업(P2/P3, product 잔여)은 monorepo
`HOST_ENV_P2_P3.md` — env 계약이 깨지지 않는 한 primary gate로 재열 안 함.

### 7.3 로컬 라이브러리 export (2026-08-14)

- `bin/export-pnix-hy-library` → `target/pnix-hy-library/site` + `py.typed`.
- `bin/pnix-hy-library-smoke` (`eval_file` → 3으로 스모크 테스트).
- PyPI 아님 — 개인/로컬 `PYTHONPATH` feed 전용.

### 7.4 pip 설치 (설치 티어)

```sh
pip install .                    # CORE: import pnix_hy; safe_eval / gate / action / ir / witness / explain
pip install '.[projection]'      # + Hy 1.3.1  -> Hy<->pnix 투영 / mirror-over-Hy
pip install '.[full]'            # + proof ladder; 추가로: export PNIX_HY_HOME=/path/to/pnix-hy 체크아웃
```

CORE 티어는 어떤 pip 설치에서도 Hy 없이·트리 없이 동작한다. 투영/full
티어는 `PNIX_HY_HOME`을 `hy-meta/`+`hy` 포함 체크아웃으로 지정해야 한다.
자세한 CLI/REPL/Nix 사용법은 `pnix-hy/README.md`(변경 안 함, 최신 상태).

## 8. 역사 — 무엇이 언제 만들어졌는가

**git log의 한계부터**: 이 저장소의 `pnix-hy/` 전체 이력은
`git log --oneline --all -- pnix-hy/`로 봐도 37개 커밋뿐이고, 그마저
첫 커밋(`4240414`, `init`, 2026-08-10)이 이미 완성된 17,292줄짜리
`pnix_runtime.py`를 통째로 들여온다. 3중 lane 빌트인 구조, hy-meta
self-host 부트스트랩, 렉서/파서/평가기 최초 작성 같은 진짜 "탄생" 사건은
이 repo git 이력으로 재구성이 **안 된다** — 그 이전 세션/작업공간에서
이미 끝나 있었던 스냅샷이 통째로 들어온 것. (§4.1이 인용하는
`374a8e4`/`3f0e186`/`314b89f`/`accad7b` 같은 커밋 해시는 이 저장소에
아예 존재하지 않는다 — 외부의, 지금은 사라진 이력을 가리키는 것이니
그대로 믿지 말 것.) 그 시기의 서사는 커밋이 아니라 §5·§8.1·§4에 글로
남아있다 — "언제 만들어졌나"가 궁금하면 거기부터 볼 것.

`init` 이후, 즉 이 repo git 이력 안에서 실제로 있었던 주요 사건들:

| 커밋 | 날짜 | 무엇을 |
|---|---|---|
| `4240414` | 08-10 | `init` — pnix_runtime.py(17k줄) 등 전체가 완성된 스냅샷으로 한 번에 들어옴. 이전 이력 없음 |
| `0a513ae` | - | 호스트별 Trusting-Trust(DDC) 로드맵 추가, clj-meta/hy-meta의 실제 DDC gap을 닫음 |
| `7fb2ae2` | - | clr-meta Trusting-Trust DDC gap을 닫음; pnix-hy에 없던 native test corpus를 고침 |
| `f685007` | - | 독립 mini backend(hy-meta의 DDC 대조군)에 진짜 클로저 추가 |
| `fcc7671` | - | 독립 mini backend에 get/dot-method-call/keyword dict 추가 |
| `bc98ffa` | - | 독립 mini backend에 defmacro + quasiquote/unquote 추가 |
| `56439ca` | - | stale해진 `docs/CAPABILITIES.md` 재생성(§9의 drift 게이트가 원래 하는 일) |
| `0c5ee44` | - | `rust_corpus`의 cwd 버그, `tower_ladder`의 RecursionError 수정 |
| `71e911e` | - | import 순서에 따른 순환-import 취약성의 근본 원인 수정 |

### 8.1 검증 기록 — 다중 에이전트 감사 (예전 `docs/IMPLEMENTATION_AUDIT.md`)

2026-07-01, §1-§24 "Pure Meta-Circular Capability Checklist" 기준으로 세 번의
독립 adversarial multi-agent sweep이 codebase 전체를 검증했다(원본 문서는
`file:symbol` 인용까지 포함한 읽기 전용 감사 로그였다 — 여기는 verdict만
압축):

- **Capability audit** (23 agents, 11 dimensions, ~1.06M tokens) — §1-§24
  표면이 실질적으로 완전, host/pnix separation 유지, pnix mirror는 진짜
  singleton, 경쟁 second runtime/mirror/introspection 없음. 발견된 구조
  리스크(witness schema 3중 분할, import-hook trio 중복, effect/purity
  vocab 두 곳 유지)는 이후 전부 "Duplication to reconcile"로 닫힘(§5.4에
  반영된 것들 포함).
- **Mistake-hunt** (15 agents, 7 dimensions, ~1.13M tokens) — idempotency/
  hidden-state, hollow report, failure-hiding, scope/sacred 위반, doc↔code
  drift, flaky gate, interop edge case를 전수 스윕. **Verdict: sacred-surface
  또는 SCOPE_LOCK 위반 없음, hidden global state 없음, 결과 정확.** 18개
  실제 결함 확인(high 3/medium 10/low 5), 17개 그 자리에서 수정, **1개
  deferred**(`hy_mirror._proj_worker_run`/`_stage7_worker_eval`의
  `readline()`에 deadline 없음 — 이후 2026-07-02 Phase A의 A10에서 최종
  수정됨, `PNIX_HY_WORKER_TIMEOUT` 도입. §8.2 참고, 지금은 없는 이슈).
- **Stub-hunt** (15 agents, 6 areas, ~0.79M tokens) — `NotImplementedError`,
  hollow `pass`/`...`, hard-coded `ready=True`, non-delegating facade,
  registered-but-empty builtin, `TODO/FIXME`를 전체 codebase에서 수색.
  **Verdict: 진정 미구현 없음.** 모든 후보가 의도적 documented placeholder
  또는 real-but-demoted logic으로 해소.
- **Re-verification** (codex가 모든 follow-up 닫은 후) — 모든 follow-up이
  진정 구현·기능(`--check` 44/44), separation 유지 재확인.

### 8.2 2026-07-02~07-03 — Phase A/B/C + 연구 백로그 (`docs/TODO.md`에서 이관)

`docs/audits/2026-07-02-deep-research-audit.md` 등 딥리서치 감사가 찾은
항목을 Phase 단위로 구현한 이력. 상세 repro/수정 내역은 각 audit 문서와
git log에 있음 — 여기는 "무엇이 언제 SHIPPED됐는지"만.

| Phase | 무엇을 | 결과 |
|---|---|---|
| Phase A | 딥리서치 감사가 찾은 확정 버그 26건(A1-A26) 수정 — `pnix_mirror.py`의 `_pe`(specialize) 의미 오류 3건, select 투영 오류, `hy_mirror.py` stage7 워커 desync/timeout/재빌드 3건, `interop.py` 경계 오류 6건, `action.py` 층 2건, 배포/발견 2건, `capabilities.py` 관리도구 3건, `cli.py` 2건, `repl.py` 2건, 빌드 글루 2건 | `--check` 57/57, `--gate` PASS |
| Phase B (proposals 0014-0019) | Jones-optimality 게이트, 경계 수치 무손실 술어, opaque own/borrow 수명 규율, hygiene self-check, IR 구조 diff+패스 물화, 해시-키 검사 캐시 | `--check` 61/61, `--gate` PASS |
| Phase C (proposals 0020-0029) | interop 하드닝, compartment 격리, phase 분리 게이트, 증분 평가, typed witness, PE 어노테이션+재특화, 타워 사다리 마일스톤(Futamura 1·2·3차 사영), efficient cogen, host artifact 잔여 gap(hy-meta 레인) | `--check` 68/68, `--gate` PASS |
| 0028 compiled runtime | P1(`pnix_hy/compiled.py`)+P3(`--ceval` fast-path) SHIPPED. P2(optimal cogen, 3차 사영 실행 성능벽)는 4개 실험으로 "런타임/스케일로 해결 불가, optimal-cogen 연구 필요"까지 확인 후 **중단** | `--check` 69 (P1) |
| 연구 백로그 Q1(laziness×PE) | sharing-safe unfolding, eta-expansion "The Trick", bounded static variation(I1), let-insertion(I4), commuting conversion(0030) 전부 SHIPPED — specializer 잔여 크기/공유 개선 | `pe_size_report`/`cogen_report`에 회귀 케이스로 고정 |
| 연구 백로그 Q2(stage-polymorphic maybe-lift) | **미추진 결정(WON'T-DO)** — 별도 hand-maintained 병행 평가기는 문헌이 반대(drift), 검증 방법 근거 0. 대신 이미 shipped된 0029 cogen derive 경로(`compiler_from_interpreter`)가 같은 목표를 충족한다고 판단 | 상세는 `docs/BUGS.md`(재시도 금지 항목) |

미착수로 남은 항목(Q1-3 CPS specializer, R5 scheduler×rebuilder 분류,
0028 P2 optimal cogen 재도전)은 `docs/PLANS.md` 참고.

### 8.3 오늘(2026-08-19) 실제로 고친 것들 — 무엇을, 왜

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

### 8.4 2026-08-20 — 문서 통합

문서가 여러 작은 파일로 흩어져 있던 걸 4개 고정 문서로 통폐합: 이 문서
(`docs/IMPLEMENTATION.md`, 예전 `IMPLEMENTATION_MAP.md`), `docs/TODO.md`
(예전 루트 `todo.md`), `docs/BUGS.md`(신규), `docs/PLANS.md`(신규). 흡수돼
삭제된 파일: `SCOPE_LOCK.md`, `docs/SEPARATION.md`,
`docs/IMPLEMENTATION_AUDIT.md`, `docs/INTEROP_ROLE_MATRIX.md`. 에이전트
지침 파일도 `CLAUDE.md` → `AGENTS.md`로 개명(Claude 전용이 아닌 다른 도구도
찾도록), `CLAUDE.md`는 `AGENTS.md`로의 심볼릭 링크로 유지.
`docs/CAPABILITIES.md`, `docs/archive/todo-history.md`, `docs/proposals/`,
`docs/audits/` 는 그대로 손대지 않았다.

## 9. 이 문서가 코드와 어긋나지 않게 유지하는 법

이 문서는 여러 성격이 섞여 있다 — 어느 쪽인지 구분해서 신뢰할 것.

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
- **§4/§5/§6/§8은 2026-08-20에 다른 문서에서 통합해 온 것이라, 통합 시점
  기준으로 정확하다.** 그 이후 코드가 바뀌면 이 절들도 stale해질 수 있다
  — 특히 §6(interop 매트릭스)과 §5.3(레인 소유권)은 새 interop 기능이나
  새 proposal이 SHIPPED될 때마다 같이 갱신할 것.
- **§1이 가리키는 `docs/CAPABILITIES.md`는 진짜 자동 생성+drift-게이트다.**
  `pnix-hy-project --capabilities`로 재생성되고, 그 결과가 코드와
  다르면 게이트가 실패한다(생성 원천 = 코드, "진실"은 코드 쪽에 있음).
  이 호스트의 public API 표면이 궁금하면 이 문서의 §1/§2보다
  `CAPABILITIES.md`를 먼저 믿을 것 — 이 문서는 "어디서 찾는지"와
  "다른 호스트와 어떻게 다른지"를 설명하는 내레이션이고, 실시간 진실은
  `CAPABILITIES.md` 쪽이다.
- clj/rs도 각자 CAPABILITIES.md(+clj는 LANE_REGISTRY.md/WIKI.md까지)를
  가진 같은 패턴이다(각 호스트 IMPLEMENTATION.md §9 참고). clr/cljs는
  아직 이런 자동 drift-게이트 문서가 없다 — clr/cljs 쪽 IMPLEMENTATION.md
  §9에 실제 미해결 gap으로 적어뒀다.
- **알아둘 것**: `pnix_hy/capabilities.py`의 `_scan_docs()`가 여전히
  `_REPO_ROOT / "SCOPE_LOCK.md"`와 `_PKG_ROOT / "todo.md"`를 문서
  drift 스캔 대상 extra 파일로 하드코딩하고 있다(둘 다 이번 통합으로
  사라짐/이동함). `if extra.is_file()` 가드 때문에 존재하지 않아도
  게이트가 깨지지는 않지만(조용히 skip), 실제로는 `docs/TODO.md`/이
  문서를 가리키도록 그 목록을 갱신하는 게 맞다 — 이건 소스 코드
  변경이라 이번 문서 통합 패스의 범위 밖으로 남겨뒀다(별도 후속 작업).
