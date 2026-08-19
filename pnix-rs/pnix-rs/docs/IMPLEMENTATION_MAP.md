# pnix-rs 구현 맵

목적: 이 호스트에 뭐가 어떻게 구현돼 있는지 한눈에 찾아보는 문서. 같은 걸
중복해서 다시 만들지 않도록, 뭔가 다르게 동작하는 걸 발견했을 때 "진짜
버그인지 원래 그런 건지" 빨리 판단할 수 있도록, 다른 4개 호스트와 비교할
때 참고하도록 만들었다. 2026-08-19 작성 시작 — 이후 구조가 바뀌면 여기도
같이 갱신할 것.

## 1. 아키텍처 개요 — 어디서 뭘 찾아야 하는가

핵심 파일은 사실상 하나: **`src/px.rs`**(~9200줄). 렉서/파서/평가기/빌트인/
import 확장/emit(pretty-print)/normalize(정규형)가 전부 이 안에 있다.
`src/main.rs`는 CLI + corpus 목록(`px_corpus()`) + import 대상 파일을
미리 훑어 모으는 `load_px_module`/`px_import_targets`. 나머지(`bta.rs`,
`gate.rs`, `incremental.rs`, `specialize.rs`, `tower.rs`)는 각각
Futamura 이진법/이산 특화, 순도 검사, 증분 평가, 특화, 자기호스팅 타워용
AST 순회 유틸 — `PxExpr`에 새 variant를 추가하면 이 파일들도 전부 손봐야
한다(러스트 컴파일러가 빠짐없이 찾아준다, 2026-08-19 `Isolated` 추가 때
실측).

- **렉서**: `px_lex` (px.rs). 문자열 슬라이스 하나짜리 `if/else if` 체인.
  경로 리터럴은 `./`, `../`, 그리고 (2026-08-19부터) 숫자 뒤가 아니고
  공백/닫는 괄호/두 번째 `/`가 안 따라오는 bare `/`까지 인식. 나눗셈 vs
  경로 구분 규칙은 pnix-clr의 `path-start?`를 그대로 참고해서 만듦(§3
  참고).
- **파서**: `px_lex` 출력을 재귀 하강으로 소비. `PxExpr` enum이 AST.
- **값 표현**: `PxVal` enum — `Int(i64)`, `Float(f64)`, `Str(String)`,
  `Bytes(Vec<u8>)`(비UTF-8 raw bytes 중간값), `List(Rc<Vec<PxVal>>)`,
  `Closure{param,body,env}`, `Builtin{name,args}`(커링), `Attrs(Rc<Vec<(String,PxVal)>>)`.
  **경로는 별도 값 타입이 아니다** — `./x` 같은 리터럴은 렉서 단계에서만
  의미 있고(`import` 인자로만), 그 외 위치에서 쓰면 "path literal outside
  import" 에러. `builtins.typeOf ./x`는 `"path"`가 아니라 그냥 문자열처럼
  샌다(§3 다른 호스트와 차이점 참고 — clr/clj/hy는 진짜 Path 타입 있음).
- **환경/스코프**: `env: Vec<PxFrame>`, `PxFrame`은 `Rec`(재귀 let, memo
  포함) / `Bind{name,value}` / `With(PxVal)`. 조회는 innermost 프레임부터,
  `With`는 다른 프레임 다 실패한 뒤에만 확인(오라클: `let a=2; in with
  {a=1;}; a == 2`).
- **지연 평가**: call-by-need. `Rec` 프레임이 각 바인딩의 최초 평가 결과를
  memo(`RefCell<Vec<Option<Result<...>>>>`)한다.
- **빌트인 dispatch**: 이름→arity는 `px_builtin_arity`, 실제 실행은
  `px_builtin_exec(name, args)`. 커링은 `px_apply_outcome`에서
  `PxVal::Builtin`에 인자를 하나씩 채워가다 arity 도달 시 실행. 새 빌트인
  추가 시 **3곳** 다 고쳐야 함: 이름 문자열 등록(빌트인 attrset에 노출되는
  이름 목록), `px_builtin_arity`, `px_builtin_exec`.
- **import**: `import <path>`는 파서에서 특별 취급되는 게 아니라 그냥
  `Apply{func: Var("import"), arg: Var(":path:...")}` 모양으로 파싱된다
  (`:path:` 접두사가 붙은 마킹 변수). 실제 파일을 읽고 대상 파일의 AST를
  **그 호출부 자리에 통째로 치환**하는 건 별도 사전 처리 패스
  `px_expand_imports`(evaluator가 아니라 evaluate 직전에 한 번 도는
  AST→AST 변환)가 담당. `scopedImport scope path`도 같은 패스에서
  `Apply{Apply{Var("scopedImport"), scope}, :path:...}` 모양을 감지해서
  처리. 2026-08-19 이전에는 이 "AST 치환"이 곧이곧대로였어서, import된
  파일이 호출부의 지역 변수를 실수로 캡처하는 버그가 있었음 — 지금은
  `PxExpr::Isolated{with_scope, body}`라는 내부 전용 AST 노드로
  감싸서, body가 항상 빈 환경에서 시작하도록 고쳐짐(§4 참고).
  파일을 실제로 읽어 `modules: Vec<(String,String)>`(경로→소스 텍스트
  맵)에 채우는 건 `main.rs`의 `load_px_module`(진입 파일에서부터 재귀적으로
  모든 import 대상을 미리 훑음) — **`-c`(인라인) 모드는 이 사전 스캔을
  안 해서 import가 항상 실패한다. `-f`(파일) 모드만 import가 된다.**
- **오류 모델**: `PxError{class, diagnostic}` + `px_error_into_diagnostic`
  로 최종 문자열화. `builtins.tryEval`은 `throw`/`assert` 실패만 잡고
  나머지(0으로 나누기, 오버플로, 타입에러 등)는 그대로 전파.
- **substrate-check 제약**: `px.rs`는 자매 프로젝트 `rs-meta`(이 저장소
  안의 순수 Rust-in-Rust 인터프리터)가 직접 해석 가능해야 한다. 즉
  `char` 리터럴을 받는 `starts_with('/')` 같은 건 안 되고 `starts_with("/")`
  써야 함, 슬라이스 패턴 대신 `.first()` 쓰기 등 — 새 코드 짤 때마다
  `./target/release/pnix-rs substrate-check`로 실제 확인해야 한다(다른
  전체 게이트 통과해도 이거 하나만 따로 깨질 수 있음, 2026-08-19 실측).

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

- **경로(path) 값 타입이 없다.** clr/clj/hy는 `builtins.typeOf ./x`가
  `"path"`를 돌려주는 진짜 Path 값 타입이 있다. rs는 경로 리터럴이
  `import`/`scopedImport` 인자 위치에서만 의미 있고, 그 외에서 쓰면
  "path literal outside import" 에러 — 일반 값으로 저장/전달 불가.
  cljs도 마찬가지로 없음(단, cljs는 `import`에서조차 파서가 경로 토큰을
  직접 소비하는 방식이라 rs와는 다른 이유로 없음). 언젠가 일반 Path
  타입을 만들 거면 이게 제일 큰 선행 작업.
- **`import`가 "AST를 그 자리에 붙여넣는" 방식.** clr/clj/cljs/hy는
  import마다 독립된 평가 호출(그 파일만의 새 환경에서 시작)을 한다. rs는
  대상 파일의 파싱된 AST를 호출부에 구조적으로 치환한다 — 2026-08-19
  `Isolated` 노드를 넣기 전까지는 이 때문에 호출부의 지역 변수를 실수로
  캡처하는 버그가 있었다(지금은 고쳐짐, `Isolated`가 매 import마다 빈
  환경으로 리셋). 겉보기 동작은 이제 동일하지만, 내부 구현 방식 자체가
  다르다는 건 기억해둘 것 — 예를 들어 `builtins.unsafeGetAttrPos`처럼
  "소스 파일이 무엇인지" 알아야 하는 기능을 나중에 만들 때 rs는 AST
  치환 흔적을 추적해야 해서 다른 호스트보다 손이 더 갈 수 있다.
- **`import`/`scopedImport`는 `-f`(파일) 모드에서만 동작.** `-c`(인라인
  소스) 모드는 `load_px_module`의 사전 스캔을 안 거쳐서 `import` 자체가
  "unbound variable import"로 실패한다. 다른 호스트는(적어도 clr은)
  `-e` 인라인 모드에서도 import가 실패하는 게 똑같지만 이유가 다르다
  (clr은 파일 컨텍스트가 없어서 명시적으로 거부; rs는 애초에 모듈 맵이
  안 채워짐).
- **수학 확장 빌트인 다수가 의도적으로 held.** `sin cos tan sqrt exp ln
  log abs pow mod`(그리고 아마 `atan2`도 같은 부류) — `docs/CAPABILITIES.md`
  와 `SCOPE_LOCK.md`에 "B1 numeric model 미결정"이라고 명시. 표에서
  O로 보여도 **호출하면 에러 나는 게 정상**이니 "왜 안 되지" 하고 새로
  만들려 하지 말 것 — 이건 오늘(2026-08-19) `functionArgs`/`abs`처럼
  개별적으로 un-hold한 것들과 다르게, 숫자 모델 전체를 어떻게 할지
  아직 결정이 안 나서 의도적으로 묶어놓은 것.
- **`pnixMounts`, `unsafeGetAttrPos`**: 5개 호스트 다 설계가 안 끝난
  상태. 자세한 내용과 방향 아이디어는 `todo.md`의 "미래 아이디어" 절.

## 4. 오늘(2026-08-19) 실제로 고친 것들 — 무엇을, 왜

빠른 참고용 요약. 각 커밋 메시지에 훨씬 자세한 설명이 있다(`git log
--oneline -- src/`로 목록, 커밋 해시로 `git show`).

| 커밋 | 무엇을 |
|---|---|
| `9bbd6a8` | `i64::MIN`을 소스 리터럴로 못 쓰던 것 고침 |
| `7efd8f4` | `let` 안 `inherit` 지원 추가, `functionArgs` 등록(당시 held) |
| `da12b12` | `let a.b = 1;` 같은 dotted 이름 바인딩 지원 |
| `d4f5fa8` | `builtins.replaceStrings`가 문자열 끝의 빈 패턴 매치를 빠뜨리던 버그 |
| `f97b536` | `functionArgs`를 패턴 람다 desugar 흔적으로 실제 구현, `substring` 음수 길이 처리 |
| `b6faca1` | 다른 4개 호스트엔 있고 rs만 없던 빌트인 40개 한 번에 추가(교차 배터리 테스트로 발견) |
| `2407dad` | `import`가 상대경로(`./x`)만 되고 절대경로는 안 되던 것 — 렉서 + `px_path_join` 수정. 처음 고친 버전이 `a / b`(공백 있는 나눗셈)와 `//`(attrset update) 연산자를 깨뜨려서 두 번 더 고침(공백 뒤따름 체크, 두 번째 `/` 배제) |
| `8268bb5` | `scopedImport` 신규 구현(다른 4개 호스트엔 있었음). 대상 파일을 `with <scope>; <모듈>`로 감싸는 방식 — 이때는 "scope가 대상 파일 안의 또 다른 nested import까지 새는" 알려진 한계를 문서화만 하고 넘어감 |
| `b3e0453` | 위 한계를 제대로 고침 — 알고 보니 scopedImport만의 문제가 아니라 **일반 `import`에도 있던 더 근본적인 캡처 버그**(호출부 지역변수를 실수로 캡처)였음. `PxExpr::Isolated` 노드 신설해서 모든 import가 항상 빈 환경에서 시작하도록 구조 자체를 고침. `PxExpr`에 variant 하나 추가한 여파로 크레이트 전역의 exhaustive match 8곳을 전부 손봄(러스트 컴파일러가 위치를 다 짚어줌) |

이 흐름에서 배운 것: **cross-host 교차 배터리 테스트**(같은 pnix 소스를
5개 호스트에 다 돌려서 값 비교)가 "이름은 있는데 동작이 다른" 유형의
버그를 제일 잘 찾아냈다. 단순히 "빌트인 이름이 등록됐나"만 보면 놓치는
버그가 많다 — 예: `functionArgs`는 등록은 돼 있었지만 항상 held 에러였고,
`import`는 동작은 했지만 캡처 버그가 있었다. 이름 존재 여부(§2 표)와
"실제로 올바르게 동작하는가"는 다른 질문이다.
