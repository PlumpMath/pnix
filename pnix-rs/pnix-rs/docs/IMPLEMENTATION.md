# pnix-rs 구현 맵

목적: 이 호스트에 뭐가 어떻게 구현돼 있는지 한눈에 찾아보는 문서. 같은 걸
중복해서 다시 만들지 않도록, 뭔가 다르게 동작하는 걸 발견했을 때 "진짜
버그인지 원래 그런 건지" 빨리 판단할 수 있도록, 다른 4개 호스트와 비교할
때 참고하도록 만들었다. 2026-08-19 작성 시작 — 이후 구조가 바뀌면 여기도
같이 갱신할 것. 2026-08-20: `docs/IMPLEMENTATION_MAP.md`에서 이 이름으로
옮기면서 REGISTRY.md/SCOPE_LOCK.md/todo.md/docs/CARGO_HOST_IMPORT.md
4개 문서의 내용을 흡수(§5~§7 신설, §4에 압축 이력 추가) — 저 4개는
삭제됐다. 열린 작업은 `docs/TODO.md`, 알려진 제한/의도적 held는
`docs/BUGS.md`, 미확정 로드맵은 `docs/PLANS.md`로 분리.

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
  `Bytes(Vec<u8>)`(비UTF-8 raw bytes 중간값), `Path(String)`(2026-08-20부터
  — 아래 참고), `List(Rc<Vec<PxVal>>)`, `Closure{param,body,env}`,
  `Builtin{name,args}`(커링), `Attrs(Rc<Vec<(String,PxVal)>>)`.
- **경로(Path) 값 (2026-08-20)**: `./x`/`../x`/`/x` 같은 리터럴은 이제
  `import`/`scopedImport` 인자 위치 밖에서도 진짜 `PxVal::Path(String)`
  값이다 — clj/clr처럼 `PathBuf`가 아니라 이 파일이 이미 다른 값 종류에
  쓰던 대로 plain `String`을 택함(substrate-check로 조기 확인, `PathBuf`
  대신이라 문제 없었음). 렉서의 `PathLit` 토큰은 그대로 파서에서
  `Var(":path:<literal>")`로 마킹되고(변경 없음), 새로 바뀐 건 이 마킹이
  **소비되지 않고 끝까지 남았을 때**의 처리뿐이다 — 예전엔
  `px_expand_imports`(파일 모드)가 이걸 하드 에러로 처리했지만, 이제는
  마킹을 그대로 통과시켜서 평가 시점의 변수 조회 fallback이
  `px_normalize_path`로 정규화한 `PxVal::Path`를 만든다(`-c` 인라인 모드는
  애초에 `px_expand_imports`를 안 거치므로 같은 fallback을 이미 타고
  있었음 — 두 모드가 이제 완전히 통일된 경로 하나로 수렴). **cur_dir로
  절대화하지 않는다** — 상대 리터럴은 정규화된 상대 텍스트(`./a/b`)를
  그대로 유지한다. 이건 실제 Nix(파일 위치 기준 절대 경로로 만듦)와는
  다르지만, pnix-clj 오라클도 같은
  선택(리터럴 텍스트만 정규화, cur_dir 조인 없음)을 하는 걸 직접 확인하고
  따른 것 — `nix-instantiate`로 교차검증해서 실제 Nix와는 다르다는 것도
  확인함(의도적 divergence, `docs/BUGS.md`에는 안 남김 — 오라클이 합의한
  설계라 "버그"가 아니라 이 host들의 공유된 설계 선택). 정규화
  (`px_normalize_path`, `src/px.rs`)는 `.`/`..` 세그먼트를 real Nix처럼
  접는다(절대 경로는 루트 위로 못 올라가서 `..`를 버림, 상대 경로는 접을 게
  없으면 `..`를 그대로 유지). `+` 연산(path+path/path+string/string+path)은
  두 피연산자의 표시 텍스트를 **구분자 없이** 이어붙인 뒤 정규화하는데,
  이것도 오라클(pnix-clj) 같은 "구분자 없는 raw concat"
  방식이라 그대로 이식(`./a + ./b` → `./a./b`처럼 직관과 다르게 보일 수
  있지만 오라클 확인됨). `"${./p}"` 문자열 보간은 이 정규화된 텍스트가
  아니라 **가짜 store path**(`/nix/store/<sha256 앞 32자>-<basename>`)로
  치환된다 — 실제 Nix의 "store에 복사하고 store path를 보간"을 흉내낸
  것으로, `toString`/`dirOf` 등 다른 모든 경로 소비자는 원래 텍스트를 그대로
  쓰는 것과 대조적인, 의도적으로 분리된 별개 메커니즘(derivation 절이 이미
  쓰던 `px_sha256_hex`/pseudo-hash 인프라 재사용). `isPath`/`typeOf`
  (`"path"`)/`dirOf`(Path 유지)/`baseNameOf`(항상 문자열)/`toPath`(이제
  `PxVal::Path` 반환)/`==`/`<`(정규화된 문자열 직접 비교, 생성 시점에 이미
  정규화됐으므로 별도 재정규화 불필요)/`toJSON`/`toXML`/rust-mirror 투영/
  specialize의 값→AST 역투영까지 전부 반영(§4 역사 표 참고).
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
- **string-context (2026-08-20)**: Nix의 string context(파생물 추적용
  store-path 의존성 집합)를 순수 시뮬레이션한다. **`PxVal`에 새 variant를
  추가하지 않았다** — `PxVal::Bytes`가 이미 쓰던 "특수 값 모양, 지정된
  표면만 이해, 나머지는 fail-closed"라는 선례를 그대로 따라, context를
  가진 문자열을 `__pnix_value_kind = "string-context"` 센티널 키를 가진
  `PxVal::Attrs`로 태그한다(pnix-clj/pnix-clr의 태그된-맵 설계를 그대로
  이식). 대표 함수: `px_ctx_string`(생성자, context가 비면 평범한 `Str`로
  붕괴), `px_is_ctx_string`/`px_string_like_content`/`px_string_like_context`
  (접근자), `px_context_aware_builtin`(고정 allowlist, pnix-clj의
  `context-aware-builtins`를 이름 그대로 이식), `px_ctx_string_in_args`
  (얕은 fail-closed 스캔 — `px_builtin_exec`의 단일 chokepoint에서
  실행). `+`/`${...}` 문자열 보간/`==`/`<` 같은 **언어 연산자**는
  allowlist를 거치지 않고 항상 context-aware다(`px_builtin_exec`를 안
  거치므로). `builtins.derivation`/`derivationStrict`/`placeholder`도
  이 절에서 같이 구현됨(pure-simulation 의사 store path). 상세 설계·
  알려진 한계(pseudo-hash, `d.out == d` 미표현, `.` select에서의 의도적
  오라클 발산)는 [`docs/BUGS.md`](BUGS.md) §1을 볼 것.
- **오류 모델**: `PxError{class, diagnostic}` + `px_error_into_diagnostic`
  로 최종 문자열화. `builtins.tryEval`은 `throw`/`assert` 실패만 잡고
  나머지(0으로 나누기, 오버플로, 타입에러 등)는 그대로 전파.
- **substrate-check 제약**: `px.rs`는 자매 프로젝트 `rs-meta`(이 저장소
  안의 순수 Rust-in-Rust 인터프리터)가 직접 해석 가능해야 한다. 즉
  `char` 리터럴을 받는 `starts_with('/')` 같은 건 안 되고 `starts_with("/")`
  써야 함, 슬라이스 패턴 대신 `.first()` 쓰기 등 — 새 코드 짤 때마다
  `./target/release/pnix-rs substrate-check`로 실제 확인해야 한다(다른
  전체 게이트 통과해도 이거 하나만 따로 깨질 수 있음, 2026-08-19 실측).

### 관련 문서 — 여기 없는 내용은 여기서 찾을 것

이 문서는 "무엇이 어디 있는지"에 집중한다. 아래는 이미 있는 다른 문서들 —
내용을 중복하지 않고 링크만 한다. **2026-08-20 문서 정리**: 예전에는
REGISTRY.md/SCOPE_LOCK.md/todo.md/docs/CARGO_HOST_IMPORT.md 4개로 흩어져
있던 내용을 이 문서(구현/게이트/원칙)와 `docs/TODO.md`(열린 작업)/
`docs/BUGS.md`(알려진 제한)/`docs/PLANS.md`(미확정 방향)로 모았다 — 저
4개 파일은 삭제됨, 아래 표는 새 위치 기준.

| 문서 | 다루는 것 |
|---|---|
| §6(아래) 게이트 레지스트리 | pnix-rs/rs-meta 전체 게이트 목록(truth = code, 옛 REGISTRY.md §1) — 새 능력을 만들기 전엔 여기부터 grep할 것 |
| [`docs/TODO.md`](TODO.md) | 지금 당장 손댈 수 있는 열린 작업만(옛 todo.md의 미완료분 — 사실상 거의 없음, 대부분 이미 §4 역사로 편입됨) |
| [`docs/BUGS.md`](BUGS.md) | 알려진 버그·제한, 그리고 **의도적으로 안 고치는 것**(옛 SCOPE_LOCK.md의 held 목록) |
| [`docs/PLANS.md`](PLANS.md) | 미확정 로드맵 — proposal별 1~2줄 요약 + 링크(옛 REGISTRY.md §2) |
| [`docs/CAPABILITIES.md`](CAPABILITIES.md) | **자동 생성**(손 편집 금지) — CLI 명령/모듈/px 표면/192개 빌트인 인벤토리. `pnix-rs capabilities`로 재생성, drift 게이트 `capabilities-check`가 어긋나면 잡음. 이 문서 §2 빌트인 표는 5개 호스트를 나란히 비교하려고 수동으로 만든 별개의 스냅샷이다(§5 참고) |
| [`rs-meta/STATUS.md`](../../rs-meta/STATUS.md) | rs-meta(자매 프로젝트, pnix을 전혀 모르는 순수 Rust-in-Rust 메타순환 엔진)의 peer-floor 상태 |
| `docs/proposals/000N-*.md`(10개), `docs/research/2026-07-03-metacircular-frontier.md` | 미구현 아이디어의 근거/설계 노트 — 원본은 그대로 두고 `docs/PLANS.md`가 요약+링크만 건다 |

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

- ~~경로(path) 값 타입이 없다~~ — **정정(2026-08-20): 더 이상 사실이
  아니다.** clj/clr처럼 진짜 `PxVal::Path` 값 타입이 생겼다(§1 "경로(Path)
  값" 절 참고). cljs는 여전히 없음(별개 프로젝트, rs가 상관할 일 아님).
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
- **`pnixMounts`**: 아직 설계가 안 끝난 상태. `unsafeGetAttrPos`는
  2026-08-20에 hy와 같은 `{file; line; column}`으로 구현됨. 인라인/`px_parse`
  의 `file`은 `"<pnix-px>"`; `-f` 파일 eval은 `px_parse_in`이 모듈 키에서
  실제 경로를 굽는다. `pnixMounts` 방향은 [`docs/PLANS.md`](PLANS.md).
- **context 있는 문자열에 `.` select를 쓰면 clj 오라클과 다르게 동작한다
  (의도적).** clj는 `eval-select`가 `attrset-value?`가 아니라 맨 `map?`을
  써서 ctx-string의 내부 표현이 `a.string` 같은 select로 그대로 샌다
  (`?`/`//`는 같은 오라클에서도 올바르게 막혀 있어 `eval-select` 한
  함수만의 우연한 누락으로 보임). rs는 pnix-cljs 포트(애초에 별개 레코드
  타입이라 이 누락을 재현할 수 없었던 선례)의 판단을 따라 `.` select에서
  ctx-string을 명시적으로 거부한다(타입 에러 — 실제 Nix도 문자열에 `.`를
  쓰면 타입 에러). 상세는 [`docs/BUGS.md`](BUGS.md) §1.

## 4. 역사 — 무엇이 언제 만들어졌는가

**git log의 한계부터**: `git log --oneline --all -- pnix-rs/`는 48개
커밋뿐이고(2026-08-10~08-19), 첫 커밋(`4240414`, `init`)이 `px.rs`(당시
이미 8379줄), `gate.rs`, `tower.rs`, `bta.rs`, `specialize.rs`,
`incremental.rs`, `stage.rs`, `ir.rs`, `mirror.rs`, `rust_mirror.rs`,
`compartment.rs`, rs-meta의 interp/typeck/check(8291줄) 전체를 한
스냅샷(2만+5.6만 줄)으로 들여온다. **tower/specialize/bta/gate.rs/
substrate-check/self-hosting이 "언제, 어떤 순서로" 만들어졌는지는 이
repo git 이력으로 재구성이 안 된다** — 전부 `init` 한 커밋 안에 이미
있었다. 그 이전 서사가 궁금하면 아래 §4-이전 압축 요약, `rs-meta/STATUS.md`,
`docs/proposals/*.md`의 날짜/버전 표기를 참고할 것(git 커밋이 아니라
문서 자체의 날짜가 유일한 단서인 경우가 많다).

### §4-이전 (2026-07-02~07-10, git 이력 밖) — 옛 todo.md 압축 요약

옛 `todo.md`(795줄)는 이 기간의 작업을 P0~P13 단위로 하나하나 기록했다.
전부 `[x]` DONE이고 커밋 해시가 없어(위 git log 한계 참고) 날짜만 남는다.
아래는 그 내용을 날짜 단위로 압축한 것 — 상세 설계/수용 기준이 궁금하면
`docs/proposals/000N-*.md`와 `docs/research/2026-07-03-metacircular-frontier.md`를
볼 것([`docs/PLANS.md`](PLANS.md) §2가 각 proposal을 1~2줄로 요약+링크).

| 날짜 | 무엇을 |
|---|---|
| 07-02 | **P0~P10을 한 날에 완주**: px 런타임 기판(P0) · singleton mirror_run(P1, roundtrip 어휘 lossless/lossy-ok/held/rejected 확정) · pnix 런타임 stage ladder(P2) · canonical IR + in-house SHA-256(P3) · gate/witness 13필드(P4) · interop 경계(P5, host 접촉을 서브프로세스 호출·파일 읽기 두 가지로 한정) · rust-mirror v0 값 축(P6) · check 집계+`docs/CAPABILITIES.md` 자동생성(P7) · specialize 부분평가(P8) · incremental(P9, Unison식 의존성-치환 해시 + realisation 캐시) · compartment SES 격리(P10). 12 reports all_ready. |
| 07-02~07-03 | **P11 tower** — 문헌 기반(Amin&Rompf POPL'18, Jones/Gomard/Sestoft, 3-Lisp) milestone 사다리: reify/reflect+self-interpreter(m1)→재귀 let 인코딩(m2)→**Rc 공유로 성능벽 해소**(m3a, 원 스케일 재귀 프로브 5분+ 타임아웃→19.2s)→str/list/attrs 인코딩(m3b)→고차 builtins 브리지(m4)→px로 쓴 specializer(m5)→**1차 Futamura 사영**(m6a, 인터프리터-free residual)→mix 자기언어 커버리지(m6b)→**2차 사영은 6라운드(1R~6R) 미종결**(20분~1h40m+, memo/lid/gid/정렬 이진탐색/축소객체까지 다 시도해도 안 풀림)→계측으로 진범 확정: **call-by-name 재귀 let의 지수 재평가**→**thunk-memo call-by-need 도입**(proposal 0003)으로 **1h40m+ → ~0.1초**(m6f, 2차 Futamura 사영 완주). 3차 사영(자기적용/cogen)은 **polyvariance가 의미적으로 폭발**해 연구 지평으로 확인(m7 fv-제한 coarsening은 5% 개선이 한계 — Jones-optimality를 못 올리는 강도 천장, deep-research finding 5). |
| 07-03 | 같은 날: BTA 오프라인 분석기(m8, mix 폴딩의 상한임을 명시) · **Jones-optimality 게이트**(m9, jones-check) · 손으로 쓴 cogen bounded(proposal 0004) · 잘-타입된 residual 게이트(proposal 0005, rs-meta에 `typecheck` 커맨드 추가) · Rust AST projection v1a~v8(proposal 0001, 제네릭 struct/impl까지) · peer-engine adapter(proposal 0008, `src/engine.rs`) · canonical Rust IR + FNV hash(proposal 0009) · attest/reflect-tower/verifying-cache/phase/assumption/ir-diff/attenuate/explain 게이트 전부 이 날 편입(deep-research 프론티어 인덱스, proposal 0007). P12 action, P13 cross-host(TSV export)도 이 날. |
| 07-08 | **owner amendment**: import/module system이 처음 범위 안으로(이전엔 예약/held 취급) — `SCOPE_LOCK.md`(이제 이 문서 §6)에 기록. 이 repo git 이력보다 이전이라 커밋 없음(init 스냅샷에 이미 반영돼 있었음); 실제 구현/`Isolated` 캡처 버그 수정은 08-19(§4-오늘). |
| 07-10 | **proposal 0010 builtin surface convergence phase 1-2**: discovery baseline Nix 118종 대비 rs 77→phase 1 후 91종. checked i64 산술, 혼합 int/float 산술·비교(**Nix와 합치 — 이전에 있던 "int↔float 승격 없음" divergence가 이때 해소됨**), 부동소수점 `toString`(6자리, NaN/Infinity 스펠링), `hashString`(md5/sha1/sha256/sha512) 전부 `nix-instantiate 2.34.7`을 로컬 오라클로 pin해 검증. rs/hy/clj 3개 호스트 + shared conformance 148→182케이스 교차 게이트 PASS. raw-surface 전체 수렴·path/string-context 값·canonical JSON float(지수 표기/무한대 인코딩)는 명시적으로 open으로 남김(`docs/PLANS.md` 참고). |
| 08-13~08-14 | **호스트 언어 임포트(Cargo)**: flake `packages.pnix-rs-library` + C 헤더, `PNIX_RS_LIB_DIR`/`PNIX_RS_INCLUDE_DIR` env, `pnix_rs::eval_file`/C `pnix_rs_eval`(§7 참고). crates.io 공개 배포는 owner가 "이 owner에겐 product goal 아님(로컬 전용)"으로 명시 결정. |

`init` 이후 이 repo git 이력 안에서 있었던 주요 사건(rs-meta 쪽 —
pnix-rs 자체는 대부분 오늘(§4-오늘) 있었음):

| 커밋 | 날짜 | 무엇을 |
|---|---|---|
| `4240414` | 08-10 | `init` — px.rs/gate.rs/tower.rs/bta.rs/specialize.rs/incremental.rs/ir.rs/mirror.rs/rust_mirror.rs/compartment.rs + rs-meta interp/typeck/check 전체가 한 스냅샷으로 들어옴 |
| `272ccef` | 08-11 | rs-meta: 독립 mini backend(Diverse Double-Compiling)로 진짜 Trusting-Trust DDC gap을 닫음 |
| `34f8099` | 08-11 | rs-meta: `stage9-aggregate-replay-check`로 찾은 실제 self-hosting 버그 수정 |
| `ae170e0` | 08-11 | rs-meta: `source-bundle-check`를 PASS로(5개 레이어 수정) |
| `79464d6` | 08-13 | rs-meta: A4 제네릭 추론 tail 닫음, mini-backend 13 fixture로 확장, trait-boundary-check 회귀 수정 |
| `d367c3c`/`f3074ad`/`f47edd6`/`2519605` | 08-17 | rs-meta: 독립 mini backend에 `let`/`while`/대입, 클로저, `loop`/`break`/`%`, `!=`+고차함수 매개변수 순차 추가 |
| `c9e35a9` | 08-18 | pnix-rs: `check`의 34개 게이트 대비 예제 gap 13개 채움 |

이후 2026-08-19 하루 동안 pnix-rs 쪽에서 있었던 일은 아래 §4-오늘 참고
(같은 날 안에서도 `import`/`scopedImport`/40개 빌트인처럼 큰 일이 몰려
있었다).

### 오늘(2026-08-19) 실제로 고친 것들 — 무엇을, 왜

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

### 오늘(2026-08-20) 실제로 고친 것들 — 무엇을, 왜

| 커밋 | 무엇을 |
|---|---|
| (미커밋 — 검토 대기) | **string-context + derivation 구현** (proposal 0006/0010이 명시적으로 open으로 남겨뒀던 두 갭 중 string-context 쪽 해소). pnix-clj(오라클)/pnix-cljs(이식 선례)의 설계를 pnix-rs 관용구로 이식: `PxVal::Bytes` 선례를 따라 새 variant 없이 태그된 `PxVal::Attrs`(`__pnix_value_kind = "string-context"`)로 표현, `px_builtin_exec` 단일 chokepoint에서 고정 allowlist 기반 얕은 fail-closed 게이트, `+`/`${...}`/`==`/`<`는 언어 연산자라 항상 context-aware. 신규 빌트인 5개(`hasContext`/`getContext`/`appendContext`/`derivation`/`derivationStrict`) + 기존 25개 이상 빌트인(`toString`/`toJSON`/`stringLength`/`substring`/`concatStrings(Sep)`/`concatMapStrings`/`replaceStrings`/`match`/`split`/`toUpper`/`toLower`/`hasPrefix`/`hasSuffix`/`hasInfix`/`removePrefix`/`removeSuffix`/`toInt`/`stringToCharacters`/`splitString`/`toPath`/`hashString`/`unsafeDiscardStringContext`/`unsafeDiscardOutputDependency`/`typeOf`/`isString`/`isAttrs`)에 context 전파/인식 로직 추가. 캐노니컬 출력 경계(`px_print`/`px_to_json`)는 context를 벗겨 content만 방출(실제 Nix `--json`과 동일 — context는 텍스트에 표현되지 않는 메타데이터). `pnix-clj` 라이브 오라클 대비 30여 항목 교차검증 배터리 전부 일치; 유일한 의도적 발산은 `.` select(오라클의 `eval-select`가 `attrset-value?`가 아니라 맨 `map?`을 써서 ctx-string 내부 표현이 새는 것으로 확인됐는데, 이건 오라클 자신의 대상함수 하나짜리 누락으로 보여 pnix-cljs의 별개-레코드-타입 판단을 따라 재현하지 않음 — `docs/BUGS.md` §1 참고). `rs-meta` 인터프리트 서브셋에서 새로 걸린 제약: `Option::unwrap_or_default` 미지원(명시 `match`로 대체), `char` 리터럴 `starts_with`(문자열 리터럴로 대체 — 08-19에 이미 알려진 제약과 동일 종류), 패턴 안 `mut` 바인딩(`Some((x, mut y))`) 미지원, 인덱싱을 거친 튜플 필드 대입(`acc[idx].1 = v`)과 그 필드에 대한 메서드 호출(`acc[idx].3.push(v)`) 둘 다 미지원(둘 다 "로컬 변수로 꺼내 수정 후 되쓰기"로 우회) — 전부 `substrate-check`로 실제 잡아냈고, 이 파일이 이미 쓰던 명시-루프 관용구를 그대로 따라 우회함. `capabilities-check`/`registry-check`/전체 `check` 34+1개 게이트 재검증 PASS(`capabilities-check`는 재생성 전 1회만 FAIL — 드리프트 게이트가 실제로 작동함을 확인한 것). |
| (미커밋 — 검토 대기) | 08-19에 "B1 숫자 모델 미결정"으로 held 묶었던 확장 수학 빌트인 10개(`sin cos tan sqrt exp ln log abs pow mod`)를 실구현. 다른 4개 호스트(clj/clr/cljs/hy)가 이미 전부 동작하는 구현을 갖고 있던 4/5 합의 사례였음이 재확인되어 hold 해제 — "B1 숫자 모델" 우려는 실제로는 언어 전체 int/float 승격 정책에 관한 것이었지, 이 단순 단항/이항 float 함수들을 막을 이유는 아니었다. rs-meta의 인터프리트 Rust 부분집합은 f64 메서드 디스패치가 아예 없어서(`substrate-check`가 `src/px.rs` 전체를 rs-meta bootstrap으로 해석하는데, `interp.rs`의 `call_method`는 i64 계열만 숫자 메서드 타깃으로 인식) `.sin()`/`.sqrt()`/`.exp()`/`.ln()`/`.powf()` 같은 표준 라이브러리 호출을 못 쓴다 — 이 파일이 이미 같은 이유로 쓰던 관례(`px_bit_op`의 bit-by-bit AND/OR/XOR, `px_round_to_int`의 cast-and-adjust ceil/floor)를 그대로 따라 순수 산술(Newton's method 제곱근, 2*ln2/2*pi 범위축소 + Taylor 급수)로 직접 구현(`px_math_sqrt`/`px_math_exp`/`px_math_ln`/`px_math_sin`/`px_math_cos`/`px_math_tan`/`px_math_atan`/`px_math_atan2`, `px.rs`의 `px_num_f64` 옆). `abs`/`pow`/`mod`는 기존 `add`/`sub`/`mul`/`div` 관례(int⊕int는 checked 정수 유지, 오버플로우 에러; 그 외는 float)를 그대로 따름. 같은 변경에서 신규 `atan2`(오라클: pnix-hy, 커링 순서 `atan2 y x`)와 `builtins.mapAttrs'`(오라클: pnix-clj — `f name value`가 `{ name; value; }` 쌍을 돌려주고 결과 이름으로 재-키잉, 충돌 시 first-name-wins는 `listToAttrs`와 동일 규칙)도 추가. |
| (미커밋 — 검토 대기) | **중첩 동적 attr 경로(D21)** — `a.${x}.c = 1;`처럼 dotted attrset-binding 경로 중간(또는 첫 세그먼트 포함 어디든)에 동적 세그먼트가 오는 형태를 새로 지원(`docs/BUGS.md` §1 "중첩 동적 attr 경로가 없다" 해소). pnix-clj의 `parse-attr-path`/`path->nested`에서 이식한 SEMANTICS를 이 파일의 기존 `PxStrPart`/단일-동적-키 `listToAttrs` 데슈가 머신에 얹었다(pnix-clj의 `:attrset` AST 노드는 리터럴 안에 동적 키를 직접 담을 수 있지만, 이 파일의 `PxExpr::Attrs(Vec<(String, PxExpr)>)`는 정적 키만 표현 가능 — 그래서 데이터 구조를 그대로 베끼지 않고 세그먼트별로 "정적이면 `PxExpr::Attrs` 중첩, 동적이면 `builtins.listToAttrs` 단일-entry 호출 중첩"을 재귀적으로 선택하는 새 `parse_attr_path_segment`/`px_wrap_dynamic_attr` 헬퍼로 변환). 가장 까다로웠던 부분은 정적 세그먼트만으로 이뤄진 형제 바인딩이 동적-세그먼트-포함 형제와 여전히 병합돼야 하는 경우(`{ a.b = 1; a.${x}.c = 2; }`의 `a`)였는데, `merge_attr_field`에 새 폴백(`px_dynamic_pairs`/`px_attrs_to_dynamic_pairs`)을 추가해 리터럴<->리터럴 재귀 병합이 안 통할 때 한쪽(또는 양쪽)이 `builtins.listToAttrs` 데슈가 모양인지 인식하고 두 pair 리스트를 이어붙이는 방식으로 해결(진짜 병합 불가능한 값은 여전히 파스-타임 에러). 실제 이름 충돌은 새 divergence를 만들지 않고 기존 §3 "동적 attrset 키 중복 = first-wins"를 그대로 상속. 부수 효과로 동적 **첫** 세그먼트 뒤 점 이어짐도 열림(`${x}.c = 1;`). `pnix-clj` 라이브 오라클 대비 10여 개 표현식 교차검증 전부 일치. `let` 바인딩(`parse_let`)의 동일 형태는 이번 스코프 밖(별도 후속 과제, `docs/BUGS.md` §1 참고). 신규 corpus fixture `seed_nested_dynamic_attr`(`px_corpus()`) + `proof/oracles-rs.tsv` 재생성. `substrate-check`/`px-check`/전체 `check` 재검증 PASS. |
| (미커밋 — 검토 대기) | **경로(Path) 값 타입 + JSON float 메시지 + POSIX ERE 확장** (proposal 0006/0010이 open으로 남겨뒀던 나머지 갭들 대부분 해소). (1) `PxVal::Path(String)` 신설 — §1 "경로(Path) 값" 절 참고, `isPath`/`typeOf`/`dirOf`/`baseNameOf`/`toPath`/`==`/`<`/`+`/`toJSON`/`toXML`/`${...}` 보간/rust-mirror/specialize 전부 갱신. (2) URI 리터럴은 손대 보니 **이미 완전히 구현돼 있었다** — `px_uri_scheme_char`/`px_uri_body_char`/`px_uri_end`(실제 Nix 렉서 규칙 `[A-Za-z][A-Za-z0-9+.-]*:[A-Za-z0-9%/?:@&=+$,_.!~*'-]+`와 정확히 일치)와 `PxTok::Uri`가 이미 있었고 `phase3_uri_literals`(substrate-check)/URI 관련 px-check 케이스가 이미 통과 중이었음 — `docs/BUGS.md`/`docs/IMPLEMENTATION.md` §3의 "URI 리터럴이 없다"는 낡은 기록이었을 뿐, 코드 변경 없음. (3) `toJSON`의 비유한 float(NaN/+inf/-inf) 처리는 이미 에러를 내고 있었다(`x - x == 0.0` 유한성 체크가 이미 있었음) — 다만 메시지가 뭉뚱그려져 있어서 `px_json_float_text` 공용 헬퍼로 빼고 NaN/+inf/-inf를 구분하는 메시지로 교체. 유한 float의 지수 표기(`{:?}`)는 이미 유효한 JSON 숫자 문법(지수에 `+` 안 붙임, 소수부 없는 지수 표기도 JSON 문법상 유효)이었음을 실측 확인, 코드 변경 불필요. (4) POSIX ERE 엔진(`rx_compile`/`rx_at`, `src/px.rs`)의 실제 갭 2개를 닫음 — `*`/`+`가 단일 문자 노드에서만 허용되던 제약을 그룹(괄호 하위표현식)까지 확장(새 `rx_repeat_group_try`: 그리디 전진 후 반복 횟수를 하나씩 백오프, 중첩 캡처는 마지막 성공 반복의 값을 유지 — 백오프가 실제로 일어나는 드문 경우의 캡처 정확성은 의도적 미해결, 진짜 POSIX 정확성은 leftmost-longest 오토마톤이 필요해서 범위 밖), 그리고 구간 반복 `{m}`/`{m,}`/`{m,n}`을 파싱 시점에 필수 복사본 + `?`/`*`로 desugar하는 방식으로 신규 추가(`try_parse_interval`). `nix-instantiate`를 라이브 오라클로 교차검증해서 중요한 설계 결정 2개를 뒤집음 — 처음엔 "불완전한 `{...}`는 그냥 리터럴 `{`로 취급"(GNU grep -E 스타일)으로 짰다가, 실제 Nix가 `a{`/`a{,3}`/`a{x}`/`a{}`/무피연산자 `{3}` 전부를 하드 에러로 낸다는 걸 확인하고 리터럴 폴백을 전부 제거함(이스케이프한 `\{`는 여전히 리터럴로 동작); 구간 카운트 상한도 처음엔 4096으로 뒀다가 실제 Nix가 `a{1000000}`은 받아주고(즉시 매치 시도, `null`) `a{4294967296}`은 거부하는 걸 확인하고 100만으로 올림(이 엔진은 구간을 실제 AST 노드 복사본 개수로 desugar하므로 진짜 O(1) counted-repeat인 실제 엔진보다는 낮은 상한이 여전히 필요). 같은 오라클로 `(a\|ab)(c\|bcd)(d*)`류 패턴에서 실제 Nix 자체도 POSIX-leftmost-longest가 아니라 backtracking-첫-성공-승 방식이라는 것도 확인 — 이 엔진의 기존 "알려진 한계" 주석이 실은 오라클과 이미 일치하는 동작이었음이 드러나 주석을 정정. `substrate-check`에서 `let x: T;` 뒤늦은 대입, 함수 안 `const`, `.parse::<usize>()`(i64/f64만 지원) 셋 다 새로 걸림 — 전부 이 파일이 이미 쓰던 관용구(값-생성 `if`/`match` 식, `let` 상수 대용, i64 파싱 후 캐스팅)로 우회. `substrate-check`/`px-check`/전체 `check` 재검증 PASS. |

## 5. 게이트 레지스트리 (중복개발 방지용, 옛 REGISTRY.md §1 편입)

**새 기능을 만들기 전에 여기부터 grep할 것.** 이미 구현된 것과 아직 안
만든 것을 한 곳에 모아 누락·중복개발을 막는다는 옛 REGISTRY.md의 원칙을
그대로 가져왔다 — 로드맵(아직 안 만든 것) 쪽은 [`docs/PLANS.md`](PLANS.md)로
옮겨졌다. 두 lane 모두 crates.io 의존 0(std만); rs-meta는 네이티브 tier용으로
rustc만 호출.

### 5.1 pnix-rs (Rust↔px 프론트엔드)

`pnix-rs check`가 all_ready로 집계하는 게이트 이름 목록, **소스 =
`check_commands()`(`src/main.rs`)** — 2026-08-20 실측 **35개**:

```text
px · mirror · stage · ir · gate · interop · rust-mirror · specialize ·
incremental · compartment · tower · bta · jones · welltyped · certify ·
cogen · attest · reflect-tower · verifying-cache · phase · assumption ·
ir-diff · attenuate · explain · engine-verdict · engine-artifact ·
engine-request · engine-attestation · engine-verify · engine-batch ·
action · cross-host · substrate · capabilities · registry
```

(각 게이트에 `-check` CLI 서브커맨드가 대응. 옛 REGISTRY.md는 "18 게이트"라고
적어뒀었는데 실제로 세보니 35개였다 — REGISTRY.md 자신도 자기가 경고하던
drift를 피하지 못한 사례. 정확한 개수가 궁금하면 이 표를 다시 믿지 말고
`check_commands()`를 직접 셀 것.)

무엇을 증명하는지 한 줄 요약(이름만으로 안 드러나는 것만):

- **incremental** — demand-driven 변경 전파: 독립 변경→그것만, 피의존→전이적
  의존자까지만 재평가.
- **tower** — reify/reflect + px 자기해석기 == 네이티브 + **1·2차 Futamura
  사영**(§4 역사 07-02~07-03 참고).
- **bta** — 오프라인 binding-time 분석 + mix 교차검증(mix 폴딩의 상한).
- **jones** — Jones-optimality(해석 계층이 실제로 제거됐는가, bloat-불변 게이트).
- **certify** — proof-carrying residual(differential testing 기반 재검증 가능 인증서).
- **cogen** — 손으로 쓴 generating extension(3차 사영을 자기적용 없이).
- **attest** — typed attestation(predicate 타입 URI + subject content hash).
- **reflect-tower** — 3-Lisp 유한 반영 타워(2-레벨 coherent).
- **verifying-cache** — 캐시 무결성.
- **phase** — phase 관측적 분리.
- **assumption** — assumed specialization.
- **ir-diff** — canonical IR 의미 diff.
- **attenuate** — SES capability 생명주기(grant→감쇠→회수, 재확대 불가).
- **welltyped** — px→Rust residual이 플로어 typeck(rs-meta `typecheck`)로
  well-typed(Rust 정적 강점, proposal 0005).
- **explain** — 사람이 읽는 진단 한 방(px 값+gate+ir+mirror+witness 조합,
  새 기계 없음 — `main.rs`의 `explain_report`).
- **engine-\***(verdict/artifact/request/attestation/verify/batch) —
  `src/engine.rs`, proposal 0008(peer-engine adapter). rs-meta의 Rust
  translation-validation 결과를 pnix-hy/pnix-clj류 peer engine이 이해할
  공통 `.px` verdict 봉투로 매핑(`pnix.engine.verdict.v0` 등) — rs-meta는
  여전히 pnix를 모르고 프로세스 경계(bootstrap CLI) 너머로만 호출된다.
- **substrate** — rs-meta interp==rustc==native 3-way(px.rs 자체가
  rs-meta subset 안에서 해석 가능함의 증거).
- **capabilities/registry** — 이 절 자체의 drift 게이트(§8 참고).

생성 원천: `pnix-rs capabilities` → `docs/CAPABILITIES.md`(drift 게이트
`capabilities-check`, 정합 게이트 `registry-check`).

### 5.2 rs-meta (Rust-in-Rust meta-circular, 자매 프로젝트)

**out of scope 리마인더**: rs-meta 자체는 이 문서에서 건드리지 않는다
(자세한 내용은 `rs-meta/STATUS.md`). 여기 적힌 건 REGISTRY.md가 남겨둔
공개 요약뿐 — pnix-rs가 왜/어떻게 그걸 쓰는지 이해하는 데만 필요한 정도.

`bootstrap check` PASS 기준 57 게이트(게이트 원천: `rs-meta/proofs/stage-manifest.tsv`
의 status 열). rs-meta는 pnix를 전혀 모르는 독립 Rust meta-circular engine —
pnix-rs는 CLI 프로세스 경계로만 호출한다:

self · tv(interp==rustc) · typeck · roundtrip · emit-tv(310/310) ·
emit-self-host(방출 번들이 corpus 재생) · ast-canonical(제네릭 faithful) ·
ast-diff(정본 AST 의미 diff) · rust-ir(content-addressed canonical Rust IR +
format-invariant ir_hash) · borrow-boundary(ownership 경계: rustc reason
code 보존, interp≠borrow checker) · trait-boundary(supported vs held:
assoc-type/dyn/where/blanket) · macro-boundary(fixed vs macro_rules!/proc
held) · source-ast/bundle · stage2/stage3 mirror·fixedpoint·core ·
stage8~stageN 사다리 · witness/hash · cap · trace · diag · manifest ·
isolation · constitution.

### 5.3 배포 (실제 설치 작동)

`flake.nix`(저장소 루트 `pnix-rs/`): packages(rs-meta/pnix-rs) ·
apps(pnix-rs/rs-meta/rs-meta-check/pnix-rs-check/substrate-check) ·
devShell. `nix build`·`nix run` 검증됨(래퍼가 rustc/RS_META_BOOTSTRAP 배선,
substrate-check 3-way PASS). 예제: `pnix-rs/examples/` 12섹션(각
`limit_rust.rs` + `pnix_rs_way.sh`, 전량 실행/컴파일 — examples는 이 정리
작업의 범위 밖, 손대지 않았다).

## 6. 이 lane의 개발 원칙 (옛 SCOPE_LOCK.md §0/§2/§3/§4 편입)

권위 있는 범위 선언이었던 `SCOPE_LOCK.md`(2026-07-02 수립, 형식은
pnix-hy SCOPE_LOCK을 따르되 이 lane은 Rust/rs-meta뿐)의 내용. 새 기능을
구현하기 전에 먼저 읽을 것. 의도적으로 보류된 기능 목록(경로/string-context
값, 수학 확장 빌트인 등) 자체는 [`docs/BUGS.md`](BUGS.md)로 옮겨졌다 —
여기 남은 건 "어떻게 개발하는가"에 대한 원칙.

### 6.1 source of truth / 완성 문구

`main` 브랜치가 권위 상태. 완성/닫힘 주장은 이 브랜치의 커밋과 `pnix-rs
check`(all_ready) receipt만 근거로 한다. **"전체 완성"/"Complete overall"
같은 문구는 쓰지 말 것** — 항상 scope-relative로만 표현한다: "pnix-rs는
현재 선언된 Rust↔pnix meta-circular projection scope(P0~P13 milestone-1)
안에서 open todo 0으로 수렴했다. Complete **with respect to the stated
Rust↔pnix projection scope.**"

### 6.2 대원칙

> **의도적 placeholder를 미구현으로 재해석해서 구현하지 말 것.**

[`docs/BUGS.md`](BUGS.md)에 적힌 held 항목들이 바로 이 원칙의 대상이다 —
"왜 이거 안 되지"가 보이면 먼저 BUGS.md를 확인하고, 거기 없으면 그때
진짜 버그로 다룰 것.

### 6.3 절차 (변경 불가 규칙)

- 새 기능/경계 이동은 `docs/proposals/NNNN-*.md`로 시작한다.
- **스키마 동결**: witness 13필드(이름·순서, `gate.rs`의 `WITNESS_FIELDS`가
  정의) 변경 금지. roundtrip 어휘(lossless/lossy-ok/held/rejected), effect
  어휘(file-read/file-write/host-call/import/network) 변경 금지. 기존
  receipt 스키마(`pnix-rs.*.v0`)는 필드 추가 시 v1로 올리고 마이그레이션을
  명시한다.
- **두 번째 평가기/mirror/gate 금지** — 모든 평가는 `src/px.rs`(sacred
  runtime) 경유.
- **zero crates.io dependency.** Python/Hy 불가촉(pnix-hy는 구조 모범일 뿐,
  코드를 가져오거나 이식하지 않는다).
- **rs-meta에 pnix 코드 금지** — 필요한 기능은 pnix와 무관한 범용 기능으로
  rs-meta 쪽에 제안만 한다.
- `px.rs`는 rs-meta evaluated subset 안에 머물러야 한다(`substrate-check`가
  게이트) — §1의 substrate-check 제약 참고.

### 6.4 걷지 않는 길

에이전트/coding-agent 런타임 ❌, task routing/plan synthesis/autonomous
실행 ❌, MSV/gate-graph 실험 ❌, corpus 표면(문장처리) 갈기 자체가 목적이
되는 것 ❌. pnix-rs는 **human-operated meta-circular language projection
lab**이다 — 모든 기능은 "Rust↔pnix projection과 mirror evidence를
개선하는가"로만 판단하고, 에이전트를 굴리는가/작업을 라우팅하는가로
판단하지 않는다.

**호스트 언어 경계**: pnix-hy는 길의 모범(구조·수준)일 뿐이다. 이 lane은
Python도 Hy도 다루지 않는다 — 호스트는 오직 Rust/rs-meta, projection은
오직 Rust↔px. pnix-hy/pnix-clj와의 접점은 cross-host(§5.1의 `cross-host`
게이트) 하나뿐이며, 그것도 `.px` 결과물/witness의 **비교**이지 그쪽
호스트를 만지는 게 아니다.

## 7. Cargo host-main에서 `pnix-rs-library` 임포트 (옛 docs/CARGO_HOST_IMPORT.md 편입)

**교리:** `../../HOST_DEV_ENV.md` · `../../HOST_IMPORT.md`(저장소 루트 문서,
이 정리 작업 범위 밖).

`pnix-rs`는 `publish = false`이며 **crates.io 의존 0개**. 호스트 crate는
오늘 crates.io 좌표를 쓰지 않는다(§4 역사 08-14: crates.io 공개 배포는
owner가 "이 owner에겐 product goal 아님"으로 명시 결정, 로컬 전용). 지원
패턴 세 가지:

### A. 시스템 라이브러리 (nix / home-manager) — 일상용 권장

`pnix-rs-refs` 실행 시 `PNIX_RS_LIB_DIR`/`PNIX_RS_INCLUDE_DIR`가 나온다.
`build.rs`에서 `cargo:rustc-link-search`/`cargo:rustc-link-lib=static=pnix_rs`로
연결하거나, C FFI로 `#include "pnix_rs.h"`
(`-I$PNIX_RS_INCLUDE_DIR -L$PNIX_RS_LIB_DIR -lpnix_rs`). 로컬 export(개인
피드, crates.io 아님):

```bash
cd pnix-rs/pnix-rs
./bin/export-pnix-rs-library          # → target/pnix-rs-library/{lib,include}
./bin/pnix-rs-library-smoke
set -a; source target/pnix-rs-library/refs.env; set +a
```

### B. Path dependency (monorepo 체크아웃) — `~/pnix` 안에서 개발할 때 우선

```toml
# Cargo.toml
[dependencies]
pnix-rs = { path = "../../../../pnix-rs/pnix-rs", package = "pnix-rs" }
```

```rust
fn main() {
    println!("{}", pnix_rs::eval("1 + 2").unwrap());
    println!("{}", pnix_rs::eval_file("prog.px").unwrap());
}
```

참고: crate 이름은 `pnix-rs`이고 lib 이름은 `pnix_rs`(`[lib] name =
"pnix_rs"`, `src/lib.rs` — "Embeddable PNIX runtime library", ABI 버전
`PNIX_RS_ABI_VERSION = 1`)이므로 `package = "pnix-rs"`가 필요. 인트리 미니
데모: `examples/host-import/rs/pnix-rs-smoke`(`cargo run -q -- ../../hello.px`
→ `3`).

### C. `nix build` 아티팩트만

```bash
cd pnix-rs
nix build .#pnix-rs-library
ls result/lib result/include
export PNIX_RS_LIB_DIR=$PWD/result/lib
export PNIX_RS_INCLUDE_DIR=$PWD/result/include
```

### 하지 말 것

- 전체 `pnix-rs` 패키지(dylib 포함)와 `pnix-rs-library`를 하나의
  home-manager `buildEnv`에 혼합(파일 충돌).
- crates.io `pnix-rs` 기대 — 미게시(`publish = false`).
- 이식 가능한 멀티호스트 `.px` 패키지 주장; 이것은 **Rust/rs 호스트
  바인딩**이다(공통 이식 가능 `.px` 라이브러리 트랙은 별도 미래 작업이며
  이 owner의 손으로 직접 쓸 예정 — pnix-meta 스타일, 여기서 진행하지 않음).
- C ABI 버전 정책: 깨지는 변경 시 `include/pnix_rs.h`에서
  `PNIX_RS_ABI_VERSION`을 올린다(현재 1, 아직 깬 적 없음).

## 8. 이 문서가 코드와 어긋나지 않게 유지하는 법

이 문서는 두 성격이 섞여 있다 — 어느 쪽인지 구분해서 신뢰할 것.

- **§2 빌트인 표는 자동 생성이 아니다.** 5개 호스트를 나란히 비교하기
  위해 수동으로 추출한 2026-08-19 스냅샷이고, 빌트인이 추가/삭제될 때마다
  stale해진다. 다시 뽑으려면 저장소 루트(`~/pnix`)에서:
  ```bash
  bin/gen-builtin-presence-matrix          # 새 표 출력
  bin/gen-builtin-presence-matrix --check  # 5개 문서 표가 실제 소스와 다르면 비영 종료
  ```
  이 스크립트는 rs 쪽엔 `docs/CAPABILITIES.md`(아래 참고)의 192개 빌트인
  인벤토리 줄을 그대로 읽는다 — rs 자체 소스를 다시 grep하지 않는다.
  `import`/`scopedImport`는 예약 키워드라 어차피 못 잡아서 손으로 `*`
  표시가 남아있다(§2 상단 각주). 이 스크립트는 5개 호스트를 가로지르는
  monorepo 레벨 도구라 어느 한 호스트의 self-contained 게이트에는
  넣지 않았다.
- **`docs/CAPABILITIES.md`는 진짜 자동 생성+drift-게이트다** — `pnix-rs
  capabilities`로 재생성, `capabilities-check` 게이트가 코드와의 drift를
  잡는다(생성 원천 = 코드). CLI 명령/모듈/px 표면/빌트인 인벤토리가
  궁금하면 이 문서의 §1/§2보다 그쪽을 먼저 믿을 것. §5(게이트 레지스트리)는
  그 위에 게이트 이름 전체(pnix-rs 35개 + rs-meta 57개)를 얹은 상위
  인덱스(옛 `REGISTRY.md`, 2026-08-20에 이 문서로 흡수) — "이미
  구현됐는지" 확인의 1차 소스는 이제 §5다.
- clj/hy도 각자 CAPABILITIES.md(+clj는 LANE_REGISTRY.md/WIKI.md까지)를
  가진 같은 패턴이다(각 호스트 IMPLEMENTATION.md/IMPLEMENTATION_MAP.md §5
  참고). clr/cljs는 아직 이런 자동 drift-게이트 문서가 없다 — clr/cljs 쪽
  문서에 실제 미해결 gap으로 적어뒀다.
