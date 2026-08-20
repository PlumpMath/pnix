# pnix-clr 구현 맵

목적: 이 호스트에 뭐가 어떻게 구현돼 있는지 한눈에 찾아보는 문서. 같은 걸
중복해서 다시 만들지 않도록, 뭔가 다르게 동작하는 걸 발견했을 때 "진짜
버그인지 원래 그런 건지" 빨리 판단할 수 있도록, 다른 4개 호스트와 비교할
때 참고하도록 만들었다. 2026-08-19 작성 시작 — 이후 구조가 바뀌면 여기도
같이 갱신할 것. **2026-08-20**: `SCOPE_LOCK.md`/`todo.md`(pnix-clr/pnix-clr),
`CLOJURE_CLR_ADMITTED_SURFACE.md`/`IN_PROCESS_EVAL.md`/`TFM_POLICY.md`(outer
`pnix-clr/docs/`)를 이 문서와 `TODO.md`/`BUGS.md`/`PLANS.md`로 통합 — 문서가
너무 잘게 흩어져 있던 걸 고정된 4개 파일로 정리했다. 옛 파일들은 삭제됨.

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
  `{:pnix/type :raw-bytes ...}`, `{:pnix/type :string-context :value ".."
  :context [".."]}`(2026-08-20 admit, 아래 별도 항목). attrset은
  `{:entries {...}}`.
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
  거의 같지만 **캐시를 안 탄다**(scope가 다르면 같은 파일이라도 결과이
  달라지므로, `eval-file*`의 canonical-path 캐시 슬롯을 공유하면 안 됨)
  는 게 핵심 차이.
- **string-context + derivation**(2026-08-20 admit): Nix 문자열은 context
  (실현돼야 하는 store-path 의존성 집합)를 옆에 매달고 다닐 수 있다는
  것의 순수 시뮬레이션. context가 빈 문자열은 여전히 평범한 CLR
  `String`(표현/비용 변화 없음); 비어있지 않을 때만
  `{:pnix/type :string-context :value ".." :context [".."]}`가 된다
  (`ctx-string` 생성자가 collapse — `evaluator.clj`). `exec-builtin`
  진입부에 fail-closed 게이트(`ctx-string-in-args?` — 최상위 인자 +
  벡터 인자 한 겹까지 shallow scan) — contextful string이 `context-
  aware-builtins`(고정 53개 이름, 오라클에서 이름 그대로 이식) 밖의
  빌트인에 들어가면 `string-context-frontier` type-error로 즉시 거부,
  절대 조용히 버려지거나 망가지지 않는다. `+`(`eval-binary`의 `:plus`)와
  `<`/`>`/`<=`/`>=`, 문자열 보간(`:string-interp`)은 빌트인이 아니라
  이 게이트 대상이 아니라서 항상 context-aware(양쪽 context 합집합).
  `hasContext`/`getContext`/`appendContext`/`unsafeDiscardStringContext`/
  `unsafeDiscardOutputDependency`, `derivation`/`derivationStrict`
  빌트인 신규 추가(`placeholder`/`storePath`는 기존 구현이 이미
  오라클과 일치해서 무변경). CLI JSON 출력 경계(`realize-value`)에서는
  context를 content로 벗긴다(진짜 Nix `--json`도 마찬가지). 알려진
  pure-simulation 한계(`d.out == d` 없음, pseudo-hash, shallow scan 등)는
  [`BUGS.md`](BUGS.md) §6 참고.
- **제품 실행 방식이 독특함**: `pnix-clr`는 다른 4개 호스트처럼 소스를
  즉석 인터프리트하는 게 아니라, **미리 빌드된 정확한 8-DLL 아티팩트만
  신뢰하고 로드**한다(`bin/pnix-clr`가 root/file/artifact 해시를 다
  검증). 소스만 고치고 `./bin/build-pnix-clr-artifact`로 재빌드 안 하면
  변경사항이 반영 안 된다 — 오늘 몇 번 이걸 깜빡해서 "source-stale"
  에러를 봤다.

### 관련 문서 — 여기 없는 내용은 여기서 찾을 것

이 문서는 "무엇이 어디 있는지"에 집중한다. 아래는 이미 있는 다른 문서들 —
내용을 중복하지 않고 링크만 한다.

| 문서 | 다루는 것 |
|---|---|
| [`docs/CAPABILITIES.md`](CAPABILITIES.md) | **자동 생성**(손 편집 금지) — CLI 명령 표면/188개 빌트인 presence 인벤토리. `bin/pnix-clr capabilities`로 재생성, drift 게이트 `bin/pnix-clr capabilities-check`가 어긋나면 잡음(`bin/pnix-clr-gate`에 연결됨). 이 문서 §2 빌트인 표는 5개 호스트를 나란히 비교하려고 수동으로 만든 별개의 스냅샷이다(§9 참고) |
| [`TODO.md`](TODO.md) | 지금 당장 손댈 수 있는, 확정된 개별 작업 항목 |
| [`BUGS.md`](BUGS.md) | 알려진 버그/제한사항, 그리고 의도적으로 admit 안 한 것(버그 아님) |
| [`PLANS.md`](PLANS.md) | 아직 방향이 확정 안 된 미래 아이디어/로드맵 |
| [`pnix-clr/csharp/Pnix.Clr/README.md`](../../csharp/Pnix.Clr/README.md) | C# 호스트 라이브러리 표면(`Eval.Source/File`, guest AOT DLL 연결) |
| [`clr-meta/STATUS.md`](../../clr-meta/STATUS.md) | clr-meta의 peer-floor 상태표(JVM/Hy/Rust host-meta 대비) |
| [`clr-meta/STAGE15_N_ROADMAP.md`](../../clr-meta/STAGE15_N_ROADMAP.md) | Stage1→15/N 목표 정의 + closure 상태 — `AGENTS.md`가 이걸 직접 가리킴 |
| `clr-meta/STAGE{3..15,N}_DESIGN.md`(14개), `SELF_REPRODUCTION_DESIGN.md`, `INDEPENDENT_MINI_INTERPRETER_DESIGN.md`, `CLR_BOOTSTRAP.md`, `RESIDUAL_SURFACE.md` | 닫힌 self-host 컴파일러 단계별 설계 문서 + B==C 자기재생 + 2번째 독립 인터프리터(DDC) 설계 + evaluator-generation 0→1→2 자기해석 주장 + 원칙 레벨의 남은/닫힌 것 지도 |

`SCOPE_LOCK.md`, `todo.md`, `docs/CLOJURE_CLR_ADMITTED_SURFACE.md`,
`docs/IN_PROCESS_EVAL.md`, `docs/TFM_POLICY.md`는 2026-08-20에 이 문서
§5~§8과 `TODO.md`/`BUGS.md`/`PLANS.md`로 통합되고 삭제됐다 — 옛 경로로
링크를 걸지 말 것.

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
| appendContext | O | O | O | O | O |
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
| derivation | O | O | O | O | O |
| derivationStrict | O | O | O | O | O |
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
| getContext | O | O | O | O | O |
| getEnv | O | O | O | O | O |
| getName | O | O | O | O | O |
| getVersion | O | O | O | O | O |
| groupBy | O | O | O | O | O |
| gt | O | O | O | O | O |
| hasAttr | O | O | O | O | O |
| hasAttrByPath | O | O | O | O | O |
| hasContext | O | O | O | O | O |
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
| unsafeGetAttrPos | O | O | O | O | O |
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
  로드한다"는 이 호스트의 핵심 설계 원칙(§5) 때문.
- **`.NET AOT 컴파일러 Stage1~15/N`** — `clr-meta`(이 호스트 언어 자체
  증명 레인)는 evaluator generation(0/1/2, 중첩 인터프리터)과 compiler
  stage(Stage1~15/N, 실제 컴파일러)가 별개 축이라는 점을 문서 여러 곳에서
  강조한다(`AGENTS.md`) — 헷갈리지 말 것.

## 4. 역사 — 무엇이 언제 만들어졌는가

**git log의 한계부터**: `git log --all -- pnix-clr/`는 64개 커밋뿐이고
(2026-08-10~08-19), 첫 커밋(`4240414`, `init`)이 lexer.clj/parser.clj/
evaluator.clj(당시 이미 2529줄)/host.clj/json.clj/outcome.clj,
checked-I64 산술, `import`, clr-meta의 Stage1~7 self-host 스캐폴딩까지
전부 첫날부터 이미 존재하는 채로 들어온다. **Stage1~15/N 각각이 실제로
"언제" 닫혔는지는 git 커밋 시점으로 오해하기 쉽지만, Stage1~7은 이미
`init` 안에 완성돼 있었다** — git 이력만 보고 단계별 완성 시점을
재구성하려 하지 말 것. 그 시기의 서사는 `clr-meta/STATUS.md`,
`STAGE15_N_ROADMAP.md`, 개별 `STAGE*_DESIGN.md`(§1 관련 문서)에 있다.

`init` 이후 이 repo git 이력 안에서 있었던 주요 사건:

| 커밋 | 날짜 | 무엇을 |
|---|---|---|
| `4240414` | 08-10 | `init` — lexer/parser/evaluator(2529줄)/host.clj, checked-I64, import, clr-meta Stage1~7 스캐폴딩까지 전체가 한 스냅샷으로 들어옴 |
| `e848f82` | 08-11 | pnix-clj 패리티를 향한 빌트인 성숙 패스: list/attrset 구조적 동등성, float 리터럴, 확장 math/bitwise/list/attrset 빌트인 |
| `d173826` | 08-11 | 독립 mini backend(2번째 from-scratch DynamicMethod 컴파일러)를 19 fixture로 확장: nested if, 추가 arity, checked overflow |
| `1c38118` | 08-12 | 컴파일러 Stage8 닫음: 재현 가능/byte-identical 어셈블리 아티팩트 |
| `c510d9b` | 08-12 | 컴파일러 Stage9 닫음: clean-process 컴파일러/런타임 replay |
| `3ff0824` | 08-12 | 컴파일러 Stage10~15/N 닫음(session/sandbox, adapter, quarantine, horizon, cross-impl, evidence-federation) |
| `119417a` | 08-13 | 컴파일러 자기재생(B==C fixed point) 닫음 |
| `8eac081` | 08-13 | 독립 인터프리터 DDC 트랙 닫음(2번째 from-scratch tree-walking 인터프리터) |
| `6b33951` | 08-13 | `pnix-clr-library`: C# `Pnix.Clr` 호스트 + guest AOT export, `.px` eval-file 헬퍼 |
| `22c4f33` | 08-14 | 실험적 in-process C# evaluator spike + parity 게이트 |
| `9b8156b`/`9b362da`/`c8e378f`/`4637561` | 08-17 | 독립 mini backend에 `let`, `loop`/`recur`, nested fn/closure(beta-reduction desugar), 진짜 1급 클로저 순차 추가 |
| `feb7a51` | 08-17 | 클로저 슬라이스용 Stage1~15/N 전체 체인 재확인 |

이후 2026-08-19 하루 동안 있었던 일은 아래 §4-오늘 참고.

### 오늘(2026-08-19) 실제로 고친 것들 — 무엇을, 왜

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

### 오늘(2026-08-20) 실제로 고친 것들 — 무엇을, 왜

| 커밋 | 무엇을 |
|---|---|
| (미커밋, 리뷰 대기) | 크로스호스트 빌트인 presence matrix diff에서 빠진 6개 추가: `log`(`ln`과 동일하게 자연로그, `Math/Log`), `tan`(`sin`/`cos`와 같은 모양, `Math/Tan`), `mapAttrs'`(pnix-clj가 유일한 레퍼런스 — `f name value`가 `{ name; value; }` 쌍을 반환, 반환된 name으로 새 attrset을 키잉하고 중복 name은 first-wins(`listToAttrs`와 동일한 tie-break); name은 즉시 force, value는 thunk로 lazy 유지). 그리고 이미 등록돼 있었지만 매트릭스가 놓친 것 2개도 정정: `nixVersion` 값이 `"2.34.7"`로 잘못돼 있던 걸 rs/hy와 맞춰 `"2.18.0-pnix"`로 고침(`storeDir`/`langVersion`은 값도 이미 맞았음 — 매트릭스 자동추출 스크립트가 `(bi :name arity)` 패턴만 잡고 plain-value 등록은 못 잡아서 `-`로 잘못 표시됐던 것, §2 표 자체가 stale). |
| (미커밋, 리뷰 대기) | `bin/pnix-clr-identity-gate`가 이 문서(§4/§9)의 정당한 크로스호스트 인용("pnix-clj" 문자열)을 "stale JVM-host identity 누수"로 오탐지하던 것 수정 — 2026-08-20 문서 통합 때 §4 역사 표/§9 백로그 항목이 pnix-clj를 이름으로 인용하게 되면서 생긴 회귀. `clr-meta/STATUS.md`/`todo.md`에 이미 있던 것과 같은 파일 allowlist 패턴을 `docs/{IMPLEMENTATION,BUGS,PLANS,TODO}.md`/`AGENTS.md`에도 적용해서 게이트가 다시 PASS하도록 고침. |
| (미커밋, 리뷰 대기) | `docs/CAPABILITIES.md` 자동 생성기 신설(PLANS.md §2 해결) — `pnix-clr.evaluator/builtin-names`(신규 public, `builtins-entries` 등록 테이블을 직접 introspect)와 `pnix-clr.main`의 신규 `capabilities`/`capabilities-check` 서브커맨드 + `capabilities-doc`/`cli-commands`; `bin/pnix-clr`의 인자 재작성 로직이 `-e`/`--production-outcome`처럼 이 두 bare 서브커맨드도 특별 취급하도록 수정(안 그러면 caller-relative 파일 경로로 잘못 재작성됨); `bin/pnix-clr-gate`에 drift 게이트로 연결. clj/hy/rs 패턴 참고, pnix-clr 스코프(namespace 8개 고정)에 맞춰 새 namespace 없이 기존 evaluator.clj/main.clj에 얹음. |
| (미커밋, 리뷰 대기) | string-context + derivation 이식(§1 "string-context" 문단, BUGS.md §6). pnix-clj를 1차 오라클, 같은 Clojure 계열이라 이 호스트 idiom에 더 가까운 pnix-cljs의 이미 끝난 포트를 2차 참고로 삼음. 새 `{:pnix/type :string-context :value ".." :context [..]}` 값(`ctx-string`/`ctx-string?`/`string-content`/`string-ctx`), `exec-builtin` 진입부의 fail-closed `context-aware-builtins` 게이트(`ctx-string-in-args?`, shallow scan), `+`/`<`/문자열 보간의 context 합집합 전파, `hasContext`/`getContext`/`appendContext`/`unsafeDiscardStringContext`/`unsafeDiscardOutputDependency` 5개 빌트인(2개는 동작 재작성, 3개는 신규), `derivation`/`derivationStrict`(신규, `placeholder`/`storePath`는 기존 구현이 이미 오라클과 맞아서 무변경), 그리고 `realize-value`(CLI JSON 출력 경계)에서 context를 content로 벗기는 스트립. 오라클 비교 배틀리 + fail-closed 확인 + 기존 게이트/테스트 스위트 전부 통과. |

## 5. 범위 — 뭐가 admit됐고 뭐가 아직 open인가

(원본: `SCOPE_LOCK.md`. 2026-08-20 이 문서로 통합, 옛 파일은 삭제됨.)

`pnix-clr`는 ClojureCLR 호스팅 PNIX 메커니즘이다. `clr-meta`는 별도의,
PNIX-agnostic ClojureCLR meta/bootstrap lane이다. Artifact dependency는
layer identity를 병합하지 않는다 — `pnix-clr`는 namespace plan과 PNIX
메커니즘을 소유하고, `clr-meta`는 generic validation과 CLR artifact
production을 소유한다. 이 저장소에는 portable/cross-host PNIX meaning을
소유하는 별도 sibling 트리가 없다 — `pnix-clr`는 그 소유권을 주장하지
않으며, 어떤 sibling corpus에도 실행 시점에 의존하지 않는다. pin된
ClojureCLR compiler/runtime은 explicit bootstrap 및 host-AOT trust root로
남는다.

### admit된 것 (bootstrap 범위)

- 제품 소유 `runtime-artifact.edn` plan과 exact source closure를 소비하는
  PNIX-agnostic `clr-meta` artifact builder
- 별도 버전된 `clr-meta` selfhost compiler family: closed C0/C1 source
  admission 및 exact low-level support ABI를 구현하고 explicit
  pinned-host B0 trust root를 통해 canonical kernel에서 executable
  Compiler Stage1 PE를 생성하는 소유자 승인 C2 slice; C2는 Stage2와
  self-reproduction을 false로 유지하면서 fresh-process unseen-target
  compilation과 semantic mutation propagation을 증명해야 함
- 그 plan이 선언한 정확히 여덟 DLL을 담은 `host-clojureclr-aot` manifest,
  plan/source/output hash와 explicit entrypoint 포함
- artifact-only PNIX product loading: live plan, source set, output set,
  exact manifest/tree shape, 기록된 모든 digest 검증; pinned-runtime 및
  cwd namespace shadow 거부, cwd와 load path를 artifact로 교체, product
  source를 compile하거나 load하는 대신 fail closed
- physical evaluator generation 2를 통한 `clr-meta` focused tool
  evaluation, non-evaluating, exact-one-form portable-domain reader,
  `load-string` tool path 없음
- CLR-native PNIX tokenization, parsing, evaluation mechanism
- `pnix.machine.host-outcome.v1`을 구현하는 nominal CLR `Done | Failed |
  Requested | Suspended` carrier와 observer(guest map으로 위조 불가);
  `Done` 및 structured `Failed`용 production evaluator 통합, `Requested`와
  `Suspended`는 carrier/observer shape만
- 공통 11-case basic-outcome contract에 필요한 exact integer/string/
  `if`/checked-`+`/integer-`/` 메커니즘
- deterministic seed JSON projection(canonical, sorted-key,
  control-character safe — `pnix-clr.test-runner`의
  `canonical-json-is-sorted-and-valid-for-control-characters`로 검증)
- dead `if` branch, unused argument, unselected attr field가 import
  expression을 resolve하거나 read하지 않는다는 로컬 test-suite 증거
  (`dead-import-mechanisms-never-resolve-or-read`)
- null 및 bool/int/string scalar equality와 static identifier attr-path
  `?`, application binding이 `?`보다 더 tight
- source-originated `System.Int64` unary negation 및 checked add,
  subtract, multiply, truncating division, structured overflow 및 lazy
  dead-overflow 동작 포함(`checked-i64-errors-are-structured-and-left-strict`,
  `checked-i64-overflow-remains-lazy`)
- **README corpus language surface**(clj/hy/rs/cljs와의 peer parity
  의도): builtins + `lib`(core/attrs/lists/strings/predicates/math/
  combinators/FS/best-effort fetch), nested attr path(`foo.bar = expr`),
  partial builtin application, `root-environment` frame. 기존 여덟
  namespace 안에서 구현(`evaluator.clj`/`host.clj`); 새 artifact
  namespace 없음
- float literal, `with`, list/attrset structural `==`, language
  `assert`, `inherit`/`inherit (expr)`(이 넷은 나중에 추가 admit됨 — §1
  언어 표면 요약과 함께 볼 것)
- ClojureCLR/.NET host adapter
- JVM host로 fallback할 수 없는 focused net10 게이트

런타임은 그 surface 너머로 의도적으로 좁다. 추가 syntax 또는 ABI claim은
oracle 증거와 common-corpus 합의로만 admit된다. README surface 확장은
tri-host promotion을 **확립하지 않는다**.

Evaluator generation 번호와 compiler stage 번호는 분리된다. 현재 evaluator
generation 0, 1, 2는 focused nested interpreter를 증명한다. Compiler
Stage1, Stage2, 또는 Stage15/N을 증명하지 않는다. 그 nested interpreter를
15 self-extension으로 확장하는 것은 현재 CLR 스택을 소진하며, open host
resource limitation이지 `Held` 결과 또는 stage receipt가 아니다.
**clr-meta Compiler Stage1–N + self-reproduction 게이트는
`promotion/allowed?=false`로 닫힘** (`../clr-meta/STATUS.md`). 이 제품은
그 사다리를 컴파일러로 소비하지 않으며, 일반 IL fixed point / host
promotion은 주장하지 않는다.

**admit 안 된 항목(범위 밖, 의도된 제외) 전체 목록은 [`BUGS.md`](BUGS.md)
§1 참고** — 이것들은 버그가 아니라 여기 넣지 않기로 한 것들이다.

### 빌드/게이트 순서

먼저 `clr-meta`를 build하고 gate한다. aggregate 게이트는 그다음 exact AOT
artifact를 build하고 negative matrix를 검사하며, seed `pnix-clr` runtime을
그 artifact를 통해서만 admit한다. Missing 또는 stale artifact 상태는
infrastructure/configuration 실패이며, source 또는 bootstrap fallback을
허가하지 않는다. `pnix-clr`는 common `.px`를 load한다. Unsupported language
input은 nominal structured `Failed` outcome을 반환; `Held`로 안전하게
만들지 않는다.

목표 순서는 compiler Stage1, self-reproducing Stage2, 반복 Stage3–7
convergence, Stage8–15/N hardening, 개별 admit ClojureCLR compatibility
profile, 그다음에서야 bootstrap-hosted focused facade에서 generated
compiler tool로 더 넓은 compatibility command 이전이다. PNIX
common-compiler integration과 CLR host promotion은 그 이후 독립적으로
닫힌다. `clr-meta/STAGE15_N_ROADMAP.md` 참조. 현재 CLR artifact/adoption
게이트 통과는 증거이지, established host로서의 자동 교체 또는 admission이
아니다.

## 6. CLI 허용 표면 — `bin/clojure-clr` / `bin/clr-meta`가 오늘 admit하는 것

(원본: `docs/CLOJURE_CLR_ADMITTED_SURFACE.md`, 2026-08-14 작성. 2026-08-20
이 문서로 통합, 옛 파일은 삭제됨. `bin/clojure-clr`, `bin/clr-meta`,
upstream bootstrap이 오늘 admit하는 것의 정직한 지도 — 전체 ClojureCLR
교체를 주장하지 않는다. 이후 작업이 facade를 조용히 늘리지 않고 named
profile로 확장할 수 있게 하기 위한 것.)

### Named profile(혼동하지 말 것)

| Profile | Entrypoint | 역할 |
|---------|------------|------|
| **`tool-eval`** | `bin/clojure-clr` | 집중 facade: `-e` / **단일 폼** 파일 하나 |
| **`tool-eval-multi`** | `--multi-form FILE\|-` / `--multi-e FORM` | 옵트인: 여러 top-level form L→R, 마지막 값(named gate) |
| **`bootstrap`** | `bin/clojure-clr-bootstrap` | Upstream Clojure.Main(substrate가 admit하는 전체 CLI 플래그) |
| **`bootstrap-project`** | `examples/clojure-clr-project/` | bootstrap + `CLOJURE_LOAD_PATH` 위 multi-ns 샘플 |
| **`meta`** | `bin/clr-meta` | Selfhost builder, gate, runtime-artifact, tool-eval family |

Named profile용 게이트(`bin/pnix-clr-gate`에도 연결됨):

```bash
./bin/clojure-clr-profiles-smoke
# tool-eval + tool-eval-multi + bootstrap-project → 42 (5 checks)
```

TFM: **net10.0** 제품 경로; Rhino **sdk_8**은 별도 — §7 TFM 정책 참고.

### `bin/clojure-clr` — admitted CLI

진실 소스: `bin/clojure-clr`(fail-closed).

| Admitted | Form | 동작 |
|----------|------|----------|
| Yes | `-e FORM` 또는 `--eval FORM`(정확히 2 argv) | `exec bin/clr-meta "$@"`(단일 폼) |
| Yes | 존재하는 파일인 단일 path(정확히 1 argv) | `exec bin/clr-meta FILE`(단일 폼; trailing 실패) |
| Yes | `-`(정확히 1 argv) | stdin에서 단일 폼(trailing 실패) |
| Yes | `--multi-form FILE`(정확히 2 argv, 파일 존재) | `tool-eval-multi` — 모든 top-level form, 마지막 값 |
| Yes | `--multi-form -` | stdin에서 `tool-eval-multi` |
| Yes | `--multi-e FORM` / `--multi-eval FORM` | 인라인 문자열에서 `tool-eval-multi` |
| No | REPL, `-i`, `-M`, deps.edn, clojure CLI 패리티 | stderr + exit 2 |

에러 텍스트(facade exit 2, non-admitted argv):

```text
clojure-clr compatibility: admitted surface is -e FORM, one FORM file, '-',
--multi-form FILE|-, or --multi-e FORM; use clojure-clr-bootstrap …
```

Surface matrix 게이트(fail-closed 인벤토리):

```bash
./clr-meta/scripts/clr-meta-tool-surface-gate
# also wired into clr-meta-gate after tool-eval-multi
```

`pnix.clr-meta.main`(tool profile)에 위임, **전체** Clojure 아님:

- 정확히 **하나의** 폼(reader evaluation 비활성; tagged/conditional reader
  거부), 단 `--multi-form` / `--multi-e`(tool-eval-multi profile) 제외.
- 값 도메인은 **admitted portable form domain**으로 제한(밖이면 eval 전
  fail closed).
- 평가는 **physical evaluator generation 2** 경유(nested interpreter
  lane; Compiler Stage1–15/N **아님**).
- 이 tool surface에 `load-string` 경로 없음.

**결과 맵(테스트/Stage 게이트):** 성공·실패 tool-eval 결과는 최소한 다음을
포함: `:profile`(예: `:tool-eval` 또는 multi-form profile),
`:form-count`(평가된 top-level form 수, 단일 `-e`는 1). pre-multi-form
형태에 대한 정확한 EDN/map 동등성을 assert하지 말 것; admitted key +
value를 고정하거나 named surface/multi 게이트를 사용.

따라서 `clojure-clr`는 **이름 호환 슬라이버**이지, "임의 프로젝트를 위한
CLR 위 Clojure"가 아니다.

### `bin/clr-meta` — 더 넓지만 여전히 profiled

| Profile | 예 | 비고 |
|---------|----------|--------|
| Tool-eval | `-e`, 단일 파일, `--gate`(eval-family) | form eval에 대해 위와 같은 reader/domain 규칙 |
| Runtime artifact | `--build-runtime PLAN OUT SRC` | **pnix-clr** product namespace용 hash-bound AOT |
| Compiler selfhost | `--build-compiler-selfhost-stageN …` | Stage ladder; `clr-meta/STATUS.md` / design 문서 참조 |
| Aggregate | `bin/clr-meta-gate` | 전체 family; promotion 주장하지 말 것 |

닫힌 compiler/selfhost claim은 `clr-meta/STATUS.md`와
`STAGE15_N_ROADMAP.md` Open claims에 나열(정직히 남은 것: 일반 IL fixed
point, broad ClojureCLR compatibility, host promotion, …).

### Upstream substrate(trust root)

| 조각 | 위치 |
|-------|----------|
| NuGet pin | `clr-bootstrap/` 경유 `Clojure` 1.12.3-alpha8 |
| Publish | `bin/build-clr` → `clojure-clr-clojure-…/…/publish/` |
| Main assembly | `Clojure.Main.dll`(net10.0) |

더 넓은 upstream compiler/runtime 작업: **`clojure-clr-bootstrap`**,
`clojure-clr` facade 아님.

### 확장 로드맵(`clr-meta/todo.md` Post host-env / P3.2, 전부 완료됨)

1. **[x] 인벤토리**(이 섹션의 원 출처).
2. **[x] TFM 정책 정리** — §7(net10 제품 vs net8 Rhino / multi-target
   Pnix.Clr).
3. **[x] 프로젝트 템플릿 + smoke(bootstrap profile)** —
   `examples/clojure-clr-project/`가 **clojure-clr-bootstrap** +
   `CLOJURE_LOAD_PATH`로 **두 namespace** 로드(facade 아님). `./smoke`는
   `42` 기대. 여전히 **`clojure-clr` multi-file 아님**, deps.edn 패리티
   아님.
4. **[x] Named profile + dual smoke** — `tool-eval` / `bootstrap` /
   `bootstrap-project` 문서화; `bin/clojure-clr-profiles-smoke` +
   `clojure-clr --help`(2026-08-14).
5. **[x] tool-eval-multi** — `--multi-form FILE` +
   `scripts/clr-meta-tool-eval-multi-gate`(`clr-meta-gate`에 연결); 기본
   단일 폼 trailing 거부 유지(2026-08-14).
6. **[x] product aggregate에 profiles-smoke** — `bin/pnix-clr-gate`가
   `clojure-clr-profiles-smoke` 실행(~17s, 2026-08-14).
7. **[x] 로컬 nupkg pack smoke** — `bin/pnix-clr-nupkg-smoke`(export
   layout + dual-TFM pack; 로컬 feed only, 2026-08-14).
8. **[x] nuget.org** — **하지 않음**(소유자: personal/local feed only,
   2026-08-14).

**admitted CLI와 관련해 하지 않기로 한 것("금지된 지름길")은
[`BUGS.md`](BUGS.md) §2 참고.**

### 빠른 smoke(facade only)

```bash
cd pnix-clr
./bin/build-clr                 # if substrate missing
./bin/clojure-clr -e '(+ 20 22)'   # => 42 via clr-meta gen2
echo '(+ 1 2)' > /tmp/t.clj
./bin/clojure-clr /tmp/t.clj
./bin/clojure-clr -M -e 1         # must fail closed (exit 2)
```

## 7. TFM / SDK 정책(pnix-clr vs Rhino)

(원본: `docs/TFM_POLICY.md`, 2026-08-14 작성. 2026-08-20 이 문서로 통합,
옛 파일은 삭제됨.)

| 경로 | TFM / SDK | 위치 |
|------|-----------|------|
| **pnix-clr 제품**(AOT guest, bootstrap, gates) | **net10.0** / `dotnet-sdk_10` | `pnix-clr/`, HM `dev/cs` runners |
| **Pnix.Clr managed Eval API** | multi-target **net8.0 + net10.0** | `csharp/Pnix.Clr/` — Rhino 측 net8이 Eval을 Reference 가능 |
| **Rhino / Grasshopper 플러그인**(Kimchi) | **net8.0** / sdk_8 cask 또는 pin | `dot-nix` Rhino 플러그인 경로 — pnix-clr AOT **아님** |

규칙:

1. Rhino 플러그인 빌드를 조용히 sdk_10 / net10에 연결하지 **말 것**.
2. pnix-clr runtime-artifact를 sdk_8로 빌드하지 **말 것**.
3. `Pnix.Clr.Eval`만 필요한 host-main C#은 net8 사용 가능(`pnix-clr`로의
   프로세스 스폰은 내부적으로 net10 호스트 런타임 사용).
4. 멀티-ns **ClojureCLR** 프로젝트 템플릿은 bootstrap **net10**만
   (`examples/clojure-clr-project`).

참고: monorepo `HOST_DEV_ENV.md`, §6(CLI 허용 표면).

## 8. 프로세스 내(in-process) C# 평가기 — 실험적 스파이크

(원본: `docs/IN_PROCESS_EVAL.md`, 2026-08-14 착수. 2026-08-20 이 문서로
통합, 옛 파일은 삭제됨. **제품 기본값이 아님** — 기본값은 여전히 프로세스
스폰이다.)

**지원 기본값:** `Pnix.Clr.Eval.Source` / `Eval.File` — **프로세스 스폰**
`pnix-clr`, JSON CLI 계약.
**옵트인:** `Eval.SourceInProcess` / `FileInProcess` — **net10.0+** 전용,
기본값은 여전히 Process.

관련 코드: `csharp/Pnix.Clr/InProcessEval.cs`.

### 왜 프로세스 스폰이 제품 기본값인가

| 관심사 | 프로세스 스폰(현재) | 프로세스 내(목표) |
|---------|----------------------|-------------------|
| 격리 | 자식 프로세스; 크래시 ≠ 호스트 크래시 | 공유 AppDomain / ALC |
| TFM 혼합 | 호스트 C# net8이 net10 CLI 호출 가능 | TFM / 로드 컨텍스트 정렬 필요 |
| Guest AOT | props를 통한 선택적 Reference | `*.clj.dll` + ClojureCLR 런타임 로드 |
| 배포 크기 | PATH/env에 `pnix-clr` 필요 | 런타임 + artifact 번들 |
| 결정성 | CLI JSON 스키마 이미 게이트됨 | 동일 스키마 + 조용한 드리프트 없음 |

프로세스 스폰은 프로세스 내 경로가 도입된 뒤에도 **기본값**으로 유지된다
(옵트인).

### 목표와 범위

C# host-main 코드가 프로세스를 스폰하지 **않고** `.px` / 인라인 소스를
평가하고, CLI 경로와 **동일한** `EvalResult` 형태(`schema`,
`outcome-kind`, `value`/`error`)를 반환하도록 한다.

- **포함:** clr limb에서 host-bound pnix의 pure eval(`pnix-clr -e` / 파일과
  동일 의미).
- **제외:** 전체 ClojureCLR REPL, 임의 multi-ns Clojure 프로젝트, "모든
  배포에서 프로세스 스폰 대체".

### 임베딩 옵션 검토(정직성 비용 순)

- **A. 기존 CLI 프로토콜 위 managed host API**(얇은 계층) — 평가를 호출마다
  `Process.Start` 하지 않고 장기 생존 helper 프로세스 / named pipe에서
  유지. JSON 계약을 재사용하지만 여전히 프로세스라 "프로세스 내"가
  아니다 — 스폰 비용이 고통이고 임베딩이 아닐 때만 쓸 중간 단계.
- **B. AssemblyLoadContext에서 guest AOT + ClojureCLR 로드(선호 경로,
  채택됨)** — ship/export가 이미 제공하는 `runtime-artifact/*.clj.dll` +
  `Pnix.Clr` multi-TFM을 호스트가 **격리된** ALC에 로드하고, CLI가 `-e` /
  파일에 쓰는 것과 같은 엔트리(또는 CLI 형태 JSON/EDN을 내는 전용 managed
  entrypoint)를 셸 아웃 없이 호출해 `EvalResult`로 매핑. 코드 전 블로커였던
  것: substrate 패키지(어셈블리를 호스트 옆에 둬야 함, 버전 핀
  1.12.3-alpha8), ALC 격리(언로드/중복 타입 identity/기본 컨텍스트로
  누수 없음 — 이 부분은 아직 완전히 안 풀림, 아래 "알려진 제한" 참고),
  TFM(제품 guest AOT는 net10; net8 전용 호스트는 프로세스 스폰만 유지),
  스레드/apartment/statics(ClojureCLR init은 ALC당 한 번), 패리티
  게이트(고정 코퍼스에 대해 프로세스 경로와 JSON 동등). 아래 "착륙한
  스파이크"가 이 경로다.
- **C. ClojureCLR 없이 CLR에서 pnix의 pure managed 재구현** — **현재
  거부** — 두 번째 의미 소스가 되고 host-bound 제품 교리에 위배되기
  때문.

### 착륙한 스파이크(2026-08-14)

| 조각 | 위치 |
|-------|----------|
| 구현 | `csharp/Pnix.Clr/InProcessEval.cs`(net10 `#if`) |
| API | `Eval.SourceInProcess` / `FileInProcess`(옵트인; 기본값은 여전히 Process) |
| 패리티 예제 | `csharp/examples/InProcessParity/` |
| 게이트 | `bin/pnix-clr-inprocess-eval-gate`(substrate+artifact 있으면 `pnix-clr-gate`에 자동 연결; `PNIX_CLR_INPROCESS_GATE=0`이면 스킵) |

동작 방식:

1. **substrate**(`PNIX_CLR_SUBSTRATE` 또는 checkout `clojure-clr-…/net10.0/publish`)와 **artifact**(`PNIX_CLR_ARTIFACT`)를 resolve.
2. `AssemblyLoadContext.Default.Resolving`을 훅하여 guest AOT DLL이
   `Clojure.dll`을 찾도록 함.
3. substrate 어셈블리 preload; `pnix-clr.evaluator` / `main` / `json`을
   `require`.
4. reflection으로 `eval-source`(또는 `eval-file`) + `projection` +
   `write-json` 호출 — `-main`의 `Environment.Exit` **없음**.
5. 프로세스 경로와 같은 `EvalResult` 형태로 파싱.

Env 계약:

| 변수 | 역할 |
|----------|------|
| `PNIX_CLR_ARTIFACT` | Guest AOT 디렉터리(`manifest.json` + `*.clj.dll`) |
| `PNIX_CLR_SUBSTRATE` | ClojureCLR net10 publish 디렉터리(`Clojure.dll`) |
| `PNIX_CLR_ROOT` | 호스트 루트(import confinement) |
| `PNIX_CLR` | 패리티 비교에 여전히 쓰이는 프로세스 경로 |

검증된 코퍼스(게이트, 14 source case + file + 2 negatives):

- `1 + 2` → 3
- `true && !false` → true
- `if true then 40 + 2 else 0` → 42
- `1 / 0` → failed / division-by-zero(패리티)
- Substrate 누락 → `NotSupportedException`(fail closed)

Reentrancy 정책은 **직렬화**다 — `eval-source` 주변 global lock(ClojureCLR
RT는 process-wide이기 때문). 동시 호출자는 대기하며, `*Async` 헬퍼는
존재하나 같은 lock을 공유한다(multi-threaded RT 아님).

nuget.org는 이 스파이크의 제품 목표가 아니다 — 로컬 `pack-pnix-clr-nupkg` /
file feed만(소유자 결정).

### 실행

```bash
export PNIX_CLR_ROOT=$PWD
export PNIX_CLR_ARTIFACT=$PWD/pnix-clr/target/runtime-artifact
export PNIX_CLR_SUBSTRATE=$PWD/clojure-clr-clojure-1.12.3-alpha8/Clojure/Clojure.Main/bin/Release/net10.0/publish
export PNIX_CLR=$PWD/bin/pnix-clr
./bin/pnix-clr-inprocess-eval-gate

# Product aggregate: auto-runs when substrate+artifact exist.
# Skip: PNIX_CLR_INPROCESS_GATE=0 ./bin/pnix-clr-gate
./bin/pnix-clr-gate

# HelloPnix demo (net10, same env):
dotnet run --project csharp/examples/HelloPnix -c Release -f net10.0 -- --inprocess '1 + 2'
```

**아직 안 풀린 collectible ALC 문제 같은 알려진 제한은
[`BUGS.md`](BUGS.md) §4, "언제 experimental 딱지를 뗄지" 같은 미정 방향은
[`PLANS.md`](PLANS.md) 참고.**

## 9. 이 문서가 코드와 어긋나지 않게 유지하는 법

- **§2 빌트인 표는 자동 생성이 아니다.** 5개 호스트를 나란히 비교하기
  위해 수동으로 추출한 2026-08-19 스냅샷이고, 빌트인이 추가/삭제될 때마다
  stale해진다. 다시 뽑으려면 저장소 루트(`~/pnix`)에서:
  ```bash
  bin/gen-builtin-presence-matrix          # 새 표 출력
  bin/gen-builtin-presence-matrix --check  # 5개 문서 표가 실제 소스와 다르면 비영 종료
  ```
  `import`/`scopedImport`는 이 호스트에서 예약 키워드라 이 스크립트가
  못 잡아서 손으로 `*` 표시가 남아있다(§2 상단 각주).
- **§1이 가리키는 `docs/CAPABILITIES.md`는 진짜 자동 생성+drift-게이트다
  (2026-08-20 해결).** clj/hy/rs 세 호스트가 갖고 있던, 코드에서 자동
  생성되고 drift-게이트로 보호되는 능력 인덱스를 이 호스트도 이제 갖고
  있다: `bin/pnix-clr capabilities`가 `pnix-clr.evaluator/builtin-names`
  (root `builtins-entries` 등록 테이블 직접 introspect)와
  `pnix-clr.main/cli-commands`에서 [`CAPABILITIES.md`](CAPABILITIES.md)를
  렌더링하고, `bin/pnix-clr capabilities-check`가 커밋된 파일과 diff해서
  어긋나면 비영 종료한다(`bin/pnix-clr-gate`에 연결). §6(CLI 허용 표면)이
  다루는 건 `bin/clojure-clr`/`bin/clr-meta`의 표면이고, 이건 별개로
  여전히 사람이 손으로 쓰는 문서다 — 혼동하지 말 것. **pnix-cljs는 아직
  이 문서가 없다.** 자세한 경위는 [`PLANS.md`](PLANS.md) §2.
