# pnix-cljs 구현 문서

목적: 이 호스트에 뭐가 어떻게 구현돼 있는지 한눈에 찾아보는 문서. 같은 걸
중복해서 다시 만들지 않도록, 뭔가 다르게 동작하는 걸 발견했을 때 "진짜
버그인지 원래 그런 건지" 빨리 판단할 수 있도록, 다른 4개 호스트와 비교할
때 참고하도록 만들었다. 2026-08-19 작성 시작 — 이후 구조가 바뀌면 여기도
같이 갱신할 것. 2026-08-20에 (구)`SCOPE_LOCK.md`와 (구)`HOST_IMPORT.md`
(바깥쪽 `pnix-cljs/`에 있던 파일)를 여기로 합쳐서(§2, §3) pnix-cljs 문서를
`IMPLEMENTATION.md`/`TODO.md`/`BUGS.md`/`PLANS.md` 4개로 정리했다 — 나머지
셋은 `docs/` 아래 같이 있다.

## 1. 아키텍처 개요 — 어디서 뭘 찾아야 하는가

`src/pnix_cljs/` 아래: `tokenizer.cljs`(렉서), `parser.cljs`(재귀 하강 →
AST), `evaluator.cljs`(값 표현 + 평가기 + 빌트인, 가장 큼), `module.cljs`/
`node_loader.cljs`(Node용 import 소스 로딩 어댑터).

- **렉서**: `keywords` 맵에 예약어 등록. 경로 리터럴은 `relative-path-start?`
  (`./`, `../`만, 2026-08-19까지)와 `absolute-path-start?`(bare `/`,
  2026-08-19 추가 — pnix-clr의 `path-start?` 규칙을 그대로 참고: 숫자
  토큰 바로 뒤만 나눗셈, 그 외는 경로).
- **파서**: `atom-starts`(어떤 토큰 종류가 함수 적용 인자를 시작할 수
  있는지)에 `:path`가 있다(2026-08-20 추가, §9) — `builtins.isPath ./x`
  처럼 경로를 일반 함수 인자로 쓸 수 있다. `parse-primary`도 `:path` 토큰을
  `{:op :path :value "..."}` 식으로 변환하는 case가 있다(같은 날 추가).
  `parse-list-element`에도 별도로 `:path` 케이스가 있음 — `:integer`/
  `:string` 등과 같은 이유로, 없으면 `[ ./a ./b ]`가 기본(default) 분기인
  `parse-expression`으로 떨어져서 `(./a) (./b)` 함수 적용 하나로
  오파싱된다. `import`/`scopedImport` 문법은 이것과 **별개**다 — 그 둘은
  여전히 파서가 `:path` 토큰을 직접 소비해서 `{:op :import :path "..."}`,
  `{:op :scoped-import :scope <ast> :path "..."}`로, 문자열 그대로 AST에
  박는다(평가 시점에 다시 해석 안 함) — `parse-primary`의 일반 `:path`
  case에는 도달하지 않는, 이 두 예약어 자체의 전용 파싱 경로.
- **값 표현**: 레코드 — `AttrsetValue{fields}`, `ClosureValue{...}`,
  `ByteStringValue{bytes}`(비UTF-8 중간값), `ContextStringValue{content
  context}`(§8), `PathValue{text}`(2026-08-20 추가, §9 — `text`는 항상
  `normalize-path`로 정규화된 텍스트). `import`/`scopedImport` 밖에서도
  이제 경로를 값으로 쓸 수 있다.
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

### 관련 문서 — 여기 없는 내용은 여기서 찾을 것

이 문서는 "무엇이 어디 있는지"에 집중한다. 아래는 이미 있는 다른 문서들 —
내용을 중복하지 않고 링크만 한다. `cljs-meta/`는 이 트리(`pnix-cljs/`,
파서/평가기)와 별개인 자매 메커니즘(ClojureScript self-hosting
substrate)이라 여기서 다시 설명 안 하고 링크만 한다.

| 문서 | 다루는 것 |
|---|---|
| [`TODO.md`](TODO.md) | 지금 당장 픽업 가능한 열린 작업(없으면 "현재 활성 작업 없음"이라고 명시) |
| [`BUGS.md`](BUGS.md) | 알려진 버그/한계 + 의도적으로 안 고치는 항목(SCOPE_LOCK 제외 목록 포함) |
| [`PLANS.md`](PLANS.md) | 아직 확정 안 된 미래 설계 방향(pnixMounts/unsafeGetAttrPos 통일 등) |
| [`CAPABILITIES.md`](CAPABILITIES.md) | 코드에서 자동 생성된 능력 인덱스(`src/pnix_cljs/capabilities.cljs`, `pnix-cljs capabilities`/`capabilities-check`) — 손 편집 금지, §7 참고 |
| [`pnix-cljs/pnix-cljs/examples/README.md`](../examples/README.md), [`FOUNDATION_PATH.md`](../examples/FOUNDATION_PATH.md) | 예제 00~17 카탈로그 + 온보딩용 추천 읽기 순서 |
| [`cljs-meta/README.md`](../../cljs-meta/README.md), [`cljs-meta/STATUS.md`](../../cljs-meta/STATUS.md) | cljs-meta(자매 프로젝트, ClojureScript self-host 컴파일 substrate)의 소개 + peer-floor 상태(다른 호스트들의 meta floor와 동등한 지점이 뭔지 진술) |
| [`cljs-meta/FIXED-POINT.md`](../../cljs-meta/FIXED-POINT.md) | stage0→stage3 self-recompile fixed-point 빌드, trust root, 게이트 요구사항 |
| [`cljs-meta/todo.md`](../../cljs-meta/todo.md) | cljs-meta 자체의 남은 작업(DDC/Trusting-Trust 축, 멀티플랫폼 byte 결정성 등) — 이 호스트의 `TODO.md`와는 별개 |

## 2. 스코프 — 이 호스트가 소유하는 것

(구 `SCOPE_LOCK.md` 내용. 제외 목록 자체는 "고쳐야 할 버그"가 아니라
"의도적으로 안 하는 것"이라 [`BUGS.md`](BUGS.md)에도 같은 목록이 "의도된
제한"으로 다시 나온다 — 여기서는 스코프 정의로서 진술.)

### 제품 소유

`pnix-cljs`는 ClojureScript/JavaScript 프로젝션 메커니즘만 소유한다:

- PNIX 소스 토큰화 및 파싱
- 네이티브 ClojureScript 값 위 평가
- 명목상 machine outcome 값
- Node 및 CommonJS interop

### 의미 소유

이 저장소에는 이식 가능한 언어 의미를 소유하는 별도 저장소-수준 트리가
없다. 이 호스트는 복사된 Clojure/JVM 런타임 트리를 유지하지 않고 자체
네이티브 seed로 파싱/평가한다. 네이티브 seed는 공유 적합 코퍼스가 연결되고
all-host gate로 비교되기 전까지 정규 크로스호스트 패리티를 주장할 수 없다.

### 이 seed에서 제외

- service policy 및 admission status
- evaluator fallback
- proof-receipt-gated execution
- JVM/Java/ASM 구현 코드
- retained effects 및 filesystem execution
- automatic application code generation
- authoritative string-encoded types
- 복사된 `stdlib`, `pnixc-pnix`, `pnix-mirror-runtime`, 또는 domain-content roots

## 3. 호스트 라이브러리로 embedding (Node / CommonJS)

(구 `HOST_IMPORT.md` 내용, 바깥쪽 `pnix-cljs/`에 있던 파일. 제품 패키지는
**호스트 바인딩** JS 라이브러리를 실어 보낸다 — 이식 가능한 멀티호스트
`.px` 패키지가 아니다. 이중 축 교리 정본은
[`../../../HOST_DEV_ENV.md`](../../../HOST_DEV_ENV.md)(monorepo 루트).)

### 레이아웃 (`nix build .#pnix-cljs` / HM install 이후)

```text
$out/share/pnix-cljs/
  package.json              # name @plumpmath/pnix-cljs, main: pnix-cljs-module.js
  pnix-cljs-module.js       # require 대상 (eval API)
  pnix-cljs.js              # CLI 진입 (bin/pnix-cljs로 래핑)

$out/lib/node_modules/@plumpmath/pnix-cljs/   # 동일 파일 (scoped require)
```

Env (HM `node` / `pnix-cljs-node` / shadow wrapper):

| 변수 | 의미 |
|------|------|
| `PNIX_CLJS_SHARE` / `PNIX_CLJS_LIBRARY` | `$out/share/pnix-cljs` |
| `PNIX_CLJS` | `pnix-cljs` CLI 경로 |
| `NODE_PATH` | `$out/lib/node_modules:$out/share/pnix-cljs:…` |

### Require + eval API

```js
// 권장 (scoped 패키지 — NODE_PATH에 lib/node_modules 필요)
const pnix = require('@plumpmath/pnix-cljs');

// flat 폴백 (NODE_PATH에 share/만 있어도 충분)
// const pnix = require('pnix-cljs-module.js');

// 인라인
pnix.evalSource('1 + 2');           // JS 프로젝션 객체
pnix.evalSourceJson('1 + 2');       // JSON 문자열
pnix.evalValueJson('1 + 2');        // value-only JSON (예: "3")

// 파일 (.px)
pnix.evalFile('prog.px');
pnix.evalFileJson('prog.px');
pnix.evalFileValueJson('prog.px');  // 스모크에 자주 사용: "3"
pnix.evalFileValue('prog.px');
```

#### 스모크 (`pnix-cljs-host` 있는 HM 프로파일)

```bash
echo '1 + 2' > /tmp/t.px
node -e "const p=require('@plumpmath/pnix-cljs'); console.log(p.evalFileValueJson('/tmp/t.px'))"
# => 3   (lib/node_modules를 싣는 flake install 이후)

# 현재 share/가 NODE_PATH에 있으면 항상 동작:
node -e "const p=require('pnix-cljs-module.js'); console.log(p.evalFileValueJson('/tmp/t.px'))"

pnix-cljs-library   # env + 경로 출력
clojurescript -e '20 + 22'   # → pnix-cljs CLI
pnix-cljs-pnix               # pnix-main REPL
```

### 명명

| 이름 | 역할 |
|------|------|
| `pnix-cljs` | 런타임 CLI (eval / `--repl`) |
| `pnix-cljs-pnix` | pnix-main 대화형 REPL |
| `clojurescript` | bare host-main 별칭 → `pnix-cljs` |
| `pnix-cljs-cljs` / `cljs-meta` | host-meta fixed-point 표면 |
| `shadow-cljs` | **빌드 오케스트레이터**만; `PNIX_CLJS` / `NODE_PATH` 주입 |

### 주장하지 않음

- 이식 가능한 멀티호스트 `.px` 패키지
- shadow-cljs 빌드 그래프 전체 대체
- npm 레지스트리 게시 (이 소유자 제품 목표 아님 — 로컬 피드만)

### 로컬 export (개인 피드)

```bash
# pnix-cljs/dist/ 필요 (이전 build-cljs / nix build)
./bin/export-pnix-cljs-library          # → pnix-cljs/target/pnix-cljs-library
./bin/pnix-cljs-library-smoke
set -a; source pnix-cljs/target/pnix-cljs-library/refs.env; set +a
```

## 4. 빌트인 구현 현황 (5개 호스트 비교, 2026-08-19 기준)

O = 등록됨(실제로 호출되는지는 별개, §5 참고). 표는 5개 호스트 소스에서
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

## 5. 다른 호스트와 알려진 차이점

- **경로가 일반 값이 아니었던 시기가 있었다(2026-08-20에 해소, §9).**
  `import`/`scopedImport` 문법 밖에서 경로 토큰을 값으로 쓰는 게 전부
  파싱 에러였다 — 이제는 `PathValue`가 생겨서 `builtins.isPath ./x`,
  `let p = ./foo; in p` 둘 다 정상 동작한다. §9에 전체 설계/포팅 근거가
  있다.
- **경로 리터럴이 현재 파일 디렉터리로 절대화되지 않는다.** 실제 Nix는
  상대 경로 리터럴을 그 파일의 디렉터리 기준 절대 경로로 만들지만, 이
  호스트는 (다른 두 호스트와 함께, 오라클로 교차검증된) 리터럴 텍스트를
  그대로 유지한다 — `./a + ./b` 같은 연산도 cur-dir을 절대 모른 채 순수
  텍스트 이어붙이기+정규화로만 처리한다. §9 참고.
- **`import`/`scopedImport`는 파서가 경로를 문자열 그대로 삼킴** — 다른
  두 호스트는 경로를 일반 AST 식으로 평가해서 얻지만(그래서 동적으로
  계산된 경로도 이론상 가능), cljs는 파서 단계에서 리터럴 경로 토큰만
  받는다(동적 경로 `import (if cond then ./a.px else ./b.px)` 같은 건 안
  됨) — `PathValue`를 값으로 쓸 수 있게 된 것과는 별개로, 이 제약은 그대로
  남아 있다.
- **환경이 프레임 체인이 아니라 평범한 맵.** clr(`[frame ...]`)이나
  rs(`Vec<PxFrame>`)보다 구조가 단순해서 스코프 주입(`load-module-scoped`)
  이 그냥 `merge` 한 줄로 끝난다 — clr처럼 새 프레임을 cons하거나 rs처럼
  전용 AST 노드를 새로 만들 필요가 없었음.

## 6. 역사 — 무엇이 언제 만들어졌는가

**git log의 한계부터**: `git log --all -- pnix-cljs/`는 41개 커밋뿐이고
(2026-08-10~08-19), 첫 커밋(`4240414`, `init`)이 tokenizer/parser/
evaluator, 빌트인, cljs-meta self-host substrate, 예제, 문서 전부를
한 스냅샷으로 들여온다. **렉서/파서/평가기가 실제로 처음 어떻게
설계됐는지는 이 repo git 이력으로 재구성이 안 된다** — `init`
한 커밋 안에 이미 다 있었다. 그 이전 서사는 `cljs-meta/STATUS.md`,
`cljs-meta/FIXED-POINT.md`에 글로만 남아있다.

`init` 이후 이 repo git 이력 안에서 있었던 주요 사건:

| 커밋 | 무엇을 |
|---|---|
| `4240414` | `init` — tokenizer/parser/evaluator, 빌트인, cljs-meta self-host substrate, 예제, 문서 전체가 한 스냅샷으로 들어옴 |
| `e848f82` | 빌트인 성숙 패스: `evaluator.cljs`에 +424줄 — math/bitwise/list/attrset 헬퍼를 clj 패리티 쪽으로 이식 |
| `4b0cbcd` | cljs-meta: 첫 독립 mini backend(Diverse Double-Compiling) — 진짜 Trusting-Trust gap을 닫음(문서만이 아니라) |
| `0c8c875`/`c9043c0`/`c13bbb0` | cljs-meta: 독립 mini backend DDC fixture 8→14→21→34로 확장(map, seq 연산, 구조 분해) |
| `8d3b1af` | cljs-meta: 독립 mini backend에 loop/recur + 진짜 클로저 — DDC 백엔드의 의미 있는 언어-커버리지 도약 |
| `75c9ec5` | `bin/export-pnix-cljs-library` + `bin/pnix-cljs-library-smoke` — 로컬 호스트-라이브러리 export 메커니즘("library" 축이 실제로 테스트 가능해짐) |
| `116f5fe` | 클로저 예제 추가 — 실제 제품 런타임 gap을 메움(클로저가 end-to-end로 진짜 동작함을 확인) |
| `9eadb52`/`b2e3ef2` | `let` 안 `inherit`/dotted-name 바인딩 지원(오늘, §6-오늘과 겹치는 항목이지만 언어-문법 확장이라 여기도 표기) |

이후 2026-08-19 하루 동안 있었던 일은 아래 §6-오늘 참고.

### 오늘(2026-08-19) 실제로 고친 것들 — 무엇을, 왜

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
| `82c4daa` | `import`가 상대경로만 되고 절대경로는 안 되던 것 — 렉서에 `absolute-path-start?` 추가(clr의 규칙 참고). 처음에 `atom-starts`도 같이 고치려다 `parse-primary`가 준비 안 돼서 되돌림(§5) |
| `8bdb4c5` | `scopedImport` 신규 구현. `load-module-scoped` 신설(캐시 안 탐, `merge`로 스코프 주입) — 첫 시도에 바로 통과, rs처럼 nested import 누수 문제 없었음(환경이 매번 `merge (builtin-environment) ...`로 완전히 새로 만들어지는 구조라 애초에 그런 버그가 생길 수 없는 설계였음) |

교차검증에서 배운 것: 오늘 고친 빌트인 5개(`removeAttrs`/`catAttrs`/
`abs`/`abort`/`intersectAttrs`/`assertMsg`/`getEnv`, 총 7개)는 전부
"이름 자체가 아예 없던" 케이스였다 — clr/rs처럼 "이름은 있는데 동작이
다른" 유형의 숨은 버그는 이 호스트에서는 덜 나왔다. import 관련 두
개(절대경로, scopedImport)는 둘 다 한 번에 잘 됐는데, 이건 환경 모델이
단순한(맵 하나) 덕분으로 보인다 — 복잡한 프레임 체인/AST 치환 방식보다
사고 날 여지가 적음.

### 오늘(2026-08-20) 실제로 고친 것들 — 무엇을, 왜

| 커밋 | 무엇을 |
|---|---|
| (커밋 전) | cross-host 빌트인 프레즌스 매트릭스에서 빠진 6개 중 `log`/`tan` 신규 구현(수학 빌트인, `ln`/`sin`/`cos` 바로 옆에 동일한 등록+dispatch 패턴으로 추가)과 `mapAttrs'` 신규 구현(유일한 참고 구현인 pnix-clj `evaluator.clj`의 알고리즘을 이식 — `f name value`로 `{ name; value; }` pair를 받아 반환된 `name`으로 결과 attrset을 재구성하는, `mapAttrs`와 달리 키를 바꿀 수 있는 변형, 중복 결과 이름은 `listToAttrs`와 동일하게 첫 항목이 우선). `nixVersion`은 이미 등록은 돼 있었지만 값이 `"2.34.7"`로 틀려 있던 걸 `"2.18.0-pnix"`(pnix-rs/pnix-hy와 일치)로 정정. `storeDir`/`langVersion`은 이미 올바른 값으로 등록돼 있어서 코드 변경 없음 — 애초에 프레즌스 매트릭스(§4)가 stale했던 케이스였다. |
| (커밋 전) | `docs/CAPABILITIES.md` 자동 생성기 신설(§7) — `src/pnix_cljs/capabilities.cljs`가 `evaluator/builtins-value`를 직접 introspect(189개 키), `pnix-cljs.main`에 `capabilities`/`capabilities-check` 서브커맨드 추가, `bin/pnix-cljs-gate`에 drift 게이트로 편입. pnix-clj/pnix-hy/pnix-rs 세 호스트가 이미 갖고 있던 패턴을 이 호스트에 맞춰 이식(rs의 단일-문서 스코프를 참고, clj의 큰 네임스페이스 reflection 방식은 이 호스트 규모에 안 맞아 채택 안 함). |
| (커밋 전) | 문자열 컨텍스트(string context) 추적 + `derivation`/`derivationStrict` 신규 구현(§8). `appendContext`/`getContext`/`hasContext` 신규, `unsafeDiscardStringContext`/`unsafeDiscardOutputDependency`를 죽은 `:identity` alias에서 실제 구현으로 교체, `+`/문자열 보간/약 20개 문자열 빌트인에 컨텍스트 전파 배선. 유일한 참고 구현(다른 JVM 호스트의 `evaluator.clj`)의 설계를 이 호스트 자체 관용구(레코드, 예외 기반 `evaluation-failure!`, `Cell`/`force-cell` 지연평가)로 이식 — JVM 전용 코드는 한 줄도 복사하지 않음(§ pnix-cljs 영구 규칙). |
| (커밋 전) | **Path 값 타입 신설**(§9) — 이 호스트가 5개 호스트 중 마지막까지 진짜 Path 값이 없던 곳이었다(`builtins.isPath`가 하드코딩 `false`). `PathValue{text}` 레코드, `normalize-path`/`make-path`(생성 시점 정규화), `path-add`(`+` 연산), `parser.cljs`의 `parse-primary`/`atom-starts`/`parse-list-element`에 `:path` 케이스 추가(경로 리터럴이 이제 `import`/`scopedImport` 밖에서도 값이 됨), `typeOf`/`isPath`/`dirOf`/`baseNameOf`/`toPath`/`==`/`<`/`toJSON`/`toXML`/`toString`/문자열 보간/`materialize`/`readFile`류(`path-string`) 전부 배선. |

## 7. 이 문서가 코드와 어긋나지 않게 유지하는 법

- **§4 빌트인 표는 자동 생성이 아니다.** 5개 호스트를 나란히 비교하기
  위해 수동으로 추출한 2026-08-19 스냅샷이고, 빌트인이 추가/삭제될 때마다
  stale해진다. 다시 뽑으려면 저장소 루트(`~/pnix`)에서:
  ```bash
  bin/gen-builtin-presence-matrix          # 새 표 출력
  bin/gen-builtin-presence-matrix --check  # 5개 문서 표가 실제 소스와 다르면 비영 종료
  ```
  `import`/`scopedImport`는 이 호스트에서 예약 키워드라 이 스크립트가
  못 잡아서 손으로 `*` 표시가 남아있다(§4 상단 각주).
- **`docs/CAPABILITIES.md`는 자동 생성이다** (2026-08-20, §1에 링크됨) —
  이 점에서는 이 문서(`IMPLEMENTATION.md`)와 다르다. 생성기는
  `src/pnix_cljs/capabilities.cljs`: `builtin-public-names`가
  `evaluator/builtins-value`의 `:fields` 키 집합을 직접 introspect하므로
  빌트인이 추가/삭제되면 재생성 결과가 자동으로 따라간다(손 typed 목록이
  아님). 재생성: `node pnix-cljs/dist/pnix-cljs.js capabilities >
  pnix-cljs/docs/CAPABILITIES.md`. drift 게이트:
  `node pnix-cljs/dist/pnix-cljs.js capabilities-check`(재생성 결과를
  메모리에서 만들어 커밋된 파일과 diff, 다르면 비영 종료) — `bin/pnix-cljs-gate`
  가 매 실행마다 이걸 돌린다. 이걸로 pnix-clj/pnix-hy/pnix-rs 세 호스트가
  이미 갖고 있던 "코드에서 자동 파생되고 drift-게이트로 보호되는 능력
  인덱스" 패턴을 이 호스트도 갖추게 됐다(pnix-clr은 별도 트리라 이 문서가
  다루는 범위 밖 — 그쪽 상태는 그쪽 문서 참고).
- **이 문서(`IMPLEMENTATION.md`)와 `TODO.md`/`BUGS.md`/`PLANS.md` 자체는
  여전히 자동 생성이 아니다** — 사람이 손으로 쓰고 갱신하므로, 코드가
  바뀌어도 자동으로는 안 어긋난다는 보장이 없다. §4 빌트인 표도 같은 이유로
  자동 생성이 아니다(바로 위 항목 참고) — `docs/CAPABILITIES.md`의
  presence 목록과는 성격이 다르다(§4는 5개 호스트 비교용 스냅샷, `CAPABILITIES.md`는
  이 호스트 단독의 항상-최신 presence 인벤토리).

## 8. 문자열 컨텍스트(string context) + `derivation` — 2026-08-20

Nix의 실제 의미론: 문자열이 **컨텍스트**(그 문자열을 쓰기 전에 realize돼야
하는 store-path 의존성의 집합)를 실어 나를 수 있다 — `derivation`의
`outPath`/`drvPath`, `builtins.appendContext`가 만들어낸다. 이 호스트에는
2026-08-20 이전까지 이 개념이 전혀 없었다(`appendContext`/`getContext`/
`hasContext`도 없었음). 다른 JVM 기반 호스트의 순수-시뮬레이션 설계(값
표현, fail-closed 게이트, derivation 계산)를 이 호스트 자체 관용구로
포팅했다 — JVM 전용 코드는 옮기지 않고, 설계만 참고했다.

### 값 표현 — `ContextStringValue` 레코드

`src/pnix_cljs/evaluator.cljs` 최상단(`ByteStringValue` 바로 옆)에
`(defrecord ContextStringValue [content context])`을 새로 추가했다.
**레코드**를 골랐다(참고 구현은 문자열 키 태그 맵을 씀) — 이 호스트는
`AttrsetValue`/`ClosureValue`/`BuiltinValue`/`ByteStringValue`가 전부
레코드라 그게 이 파일 고유의 관용구이고, 참고 구현이 태그 맵을 쓰는 이유
중 하나(Path 값 타입과 표현을 맞추려는 것)가 이 호스트엔 아예 해당 안
된다(§5 — 이 호스트엔 Path 값 타입이 없음, 경로는 그냥 문자열). 핵심
함수:

- `ctx-string content context` — 생성자. **컨텍스트가 비어 있으면
  `content`를 그대로 반환**해서 컨텍스트-free 문자열은 이 기능 이전과
  완전히 동일한 표현/비용을 유지한다(대부분의 문자열 연산이 여기 해당).
  `content`가 `ByteStringValue`(비UTF-8 원시 바이트)인데 컨텍스트가
  비어있지 않으면 거부(`type-error`/`raw-bytes-with-context`) — 두 특수
  표현이 섞이는 조합에는 의미를 부여하지 않는다.
- `ctx-string?`, `string-content`(언랩, 다른 값엔 그대로 통과 — 아무 값에나
  걸어도 안전), `string-ctx`(컨텍스트 벡터, 컨텍스트-free면 `[]`).
- 기존 `string-value?`/`string-bytes`/`string-text` 세 함수를 확장해서
  `ContextStringValue`를 투명하게 인식/언랩하게 만들었다 — 이 세 함수가
  파일 전체에서 "이게 문자열인가/바이트를 달라/텍스트를 달라"의 공용
  창구라서, 여기 세 곳만 넓히면 **fail-closed 게이트를 통과한** 빌트인
  대부분이 별도 코드 없이 컨텍스트를 올바르게 다룬다(아래 참고).

### fail-closed 게이트 — `context-aware-builtins` / `ctx-string-in-args?`

`invoke-builtin`(`{:keys [...]}` 아닌, `[builtin argument]` 받는 큰
`case`) 진입부에 게이트를 심었다: 누적된 `arguments`에 컨텍스트 있는
문자열이 하나라도 있는데 그 빌트인 이름이 `context-aware-builtins`
집합에 없으면 즉시 `type-error`(`detail_class`
`"string-context-frontier"`)로 거부한다 — **컨텍스트를 아직 배우지 않은
빌트인이 조용히 컨텍스트를 버리거나 망가뜨리는 일이 없게** 하는 것이 이
설계 전체의 핵심. `context-aware-builtins`는 참고 구현의 동일 집합을
그대로 이름만 옮긴 고정 목록(그 목록 중 `count`처럼 이 호스트에 아직 없는
빌트인 이름은 그냥 빠짐). `ctx-string-in-args?`도 참고 구현과 똑같이
**얕은** 스캔이다(최상위 + 벡터 인자 한 단계, 아직 강제평가 안 된 `Cell`
뒤에 숨은 원소는 못 봄) — 강제로 더 깊이 스캔하지 않은 이유는 §"검증
결과"의 `sort`/`filter` 항목 참고.

### 전파 지점

- **문자열 보간** (`evaluate-string-segments`/`evaluate-indented-string`):
  각 조각을 평가해 `string?`면 그대로, `ctx-string?`면 컨텐츠를 꺼내고
  컨텍스트를 모으고, 그 외 타입은 여전히 `type-error`. 최종 결과는
  `(ctx-string 합친텍스트 모은컨텍스트)` — 컨텍스트가 하나도 없으면 이전과
  바이트 단위로 동일. indented-string은 컨텐츠에만 들여쓰기 정규화를 적용한
  뒤 컨텍스트를 다시 씌운다.
- **문자열 `+`** (`numeric-binary`의 `:add` 분기): 언어 연산자라서
  `invoke-builtin` 게이트를 아예 거치지 않는다 — `+`는 항상 컨텍스트를
  이해한다(오른쪽/왼쪽 컨텍스트 합집합).
- **컨텍스트-보존 빌트인 약 20개**: `toString`/`toJSON`(컬렉트용 `volatile!`
  누산기를 재귀 호출에 실어 나름), `substring`/`toUpper`/`toLower`/
  `stringToCharacters`/`removePrefix`/`removeSuffix`(원본 문자열의 전체
  컨텍스트를 그대로 유지 — substring 기반 의미론이라 컨텍스트를 자르지
  않음), `concatStrings`/`concatMapStrings`/`concatStringsSep`/
  `replaceStrings`(컨텍스트 합집합, replaceStrings는 실제 **사용된**
  교체 문자열의 컨텍스트만), `match`/`split`(정규식 인자가 컨텍스트 있으면
  거부, 대상 문자열은 컨텍스트 있어도 되지만 결과는 컨텍스트-free),
  `hashString`(다이제스트는 컨텍스트-free), `toPath`(정규화된 경로에
  컨텍스트 유지), `stringLength`/`hasPrefix`/`hasSuffix`/`hasInfix`/
  `splitString`/`toInt`(이미 컨텍스트-aware해진 `string-bytes`/
  `string-text` 덕분에 별도 코드 변경 없이 자동으로 맞게 동작).
- `nix-to-string`(범용 coerceMore 강제변환 유틸, `toString`/`trace`/
  `concatStrings`류가 공유)은 **1-arity 호출에서는 여전히
  `ContextStringValue`를 거부**한다 — 아직 컨텍스트-aware하게 안 바뀐
  호출부(`warn`, `optionalString`, `concatMapStringsSep`)를 위한 심층
  백스톱. 컨텍스트-aware 호출부는 `collected` volatile을 명시적으로 넘기는
  2-arity로 부른다.

### 4개 직접 조작 빌트인

`appendContext`/`getContext`/`hasContext`/`unsafeDiscardStringContext`를
신규 등록(`unsafeDiscardStringContext`와 `unsafeDiscardOutputDependency`는
전부터 이름은 등록돼 있었지만 죽은 `:identity` alias였다 — 이번에 실제
구현으로 교체).

- `appendContext s ctxAttrs`(문자열이 먼저, 컨텍스트 attrset이 둘째 — 오라클
  확인됨) — 각 경로 키의 info attrset을 Nix 인코딩 컨텍스트 원소로 해석:
  `path = true` → `"<p>"`, `allOutputs = true` → `"=<p>"`, `outputs =
  [o..]` → `"!o!<p>"`. **빈 info attrset(`{}`)은 아무 것도 안 붙인다**
  (오라클 확인).
- `getContext s` — Nix 인코딩 컨텍스트 원소를 경로별 info attrset으로
  역-디코딩(`"<p>"` → `{path=true;}`, `"!o!<p>"` → `{outputs=[o..];}`,
  `"=<p>"` → `{allOutputs=true;}`, 같은 경로에 여러 종류가 섞이면 한 키로
  합침).
- `hasContext s` — `(ctx-string? s)`.
- `unsafeDiscardStringContext s` — 컨텍스트를 버리고 순수 컨텐츠만 반환.
- `unsafeDiscardOutputDependency s` — `"!"`/`"="`로 시작하는 (출력 의존)
  컨텍스트 원소만 걸러내고 순수 경로 원소는 남긴다.

이 넷 다 "어떤 의존성인지"만 추적하는 순수-시뮬레이션 스코프다 — 실제
Nix의 `path`/`allOutputs`/`outputs` 종류 구분보다는 얕지만, `getContext`가
디코딩한 결과 shape는 그 세 종류를 다 구분해서 보여준다(오라클과 구조
일치 확인됨).

### `derivation` / `derivationStrict` / `placeholder`

`derivation-core`(검증 + realize) → `derivation-paths`(결정적 **의사**
해시 — 진짜 Nix 해시 아님, forced 속성의 canonical JSON 텍스트를 sha256)
→ 두 빌트인:

- `derivationStrict attrs` → `drvPath`(컨텍스트 `["=<drvPath>"]`) + 출력당
  하나씩(컨텍스트 `["!<output>!<drvPath>"]`)인 attrset.
- `derivation attrs` → 입력 attrs 전체 + `type`/`name`/`drvPath`/
  `outPath`/`outputName`(첫 출력 기준) + 출력당 하나씩 "축소된"
  하위-derivation attrset(`type`/`name`/`drvPath`/`outPath`/`outputName`만
  — 실제 Nix의 `d.out == d` 자기참조는 순수 레코드/맵 값 모델로는 표현이
  안 됨, 의도된 시뮬레이션 한계, `BUGS.md` 참고).
- `placeholder output` — 컨텍스트 없는 결정적 의사-해시 문자열(출력 이름은
  반드시 순수 `string?`여야 함 — 컨텍스트 있는 인자는 그냥 거부).
- `storePath` — 순수 평가기라 store 접근 불가, 항상 거부(기존 동작
  그대로, 이번에 allowlist에만 추가).

검증(§ 아래)에서 clj 오라클과 실제 값(경로 텍스트 자체는 의사-해시라
다름) 구조를 대조 확인.

### 출력 경계 — `materialize`

`evaluator/materialize`(canonical JSON 투영의 입구)에 `ctx-string?` 분기를
추가: 컨텍스트 있는 문자열은 **순수 컨텐츠로 materialize**된다(컨텍스트는
버려짐) — 이건 "조용히 컨텍스트를 버리는 버그"가 아니라, 진짜 Nix의
`--json` 출력이 `derivation`의 `outPath` 등을 그냥 평범한 JSON 문자열로
찍는 것과 같은, **평가 도중이 아니라 canonical 출력 경계 한 곳**에서의
의도된 설계다. 평가 중에는 이 문서에 적은 모든 전파/게이트가 그대로
적용된다 — 버려지는 건 오직 값이 canonical JSON으로 나가는 마지막
순간뿐.

### 검증 결과 (2026-08-20)

동일 클래스의 JVM 기반 참고 구현(문자열 컨텍스트를 이미 갖춘 유일한
호스트)에 같은 질의를 돌려 구조/값을 대조하는 방식으로 검증했다:
`appendContext`/`getContext`/`hasContext`/`unsafeDiscardStringContext`의
기본 왕복, `getContext (a + b)` 컨텍스트 합집합, 문자열 보간의 컨텍스트
전파, `derivation`/`derivationStrict`의 `outPath`/`drvPath`/`outputName`
shape(다중 출력 포함), `toJSON`/`concatStringsSep`/`replaceStrings`의
컨텍스트 합집합, `match`/`fromJSON`의 컨텍스트 거부, `stringToCharacters`의
문자당 전체 컨텍스트 유지 — 전부 오라클과 구조 일치(경로 텍스트 자체는
의사-해시라 바이트 단위로는 다름, 의도된 차이).

**fail-closed 실제 동작(오라클로 확인, 처음 예상과 다름)**: 컨텍스트 있는
문자열이 `sort`/`filter` 같은 non-allowlisted 빌트인의 **리스트 인자 안에
중첩**돼 있으면 — 얕은 스캔이 강제평가 안 된 리스트 원소를 못 보기 때문에
— 실제로는 거부되지 **않고 그냥 통과**한다(오라클도 동일하게 통과시킴,
직접 대조 확인). 게이트가 신뢰성 있게 잡는 건 **바로 그 자리에 스칼라로
온** 컨텍스트 문자열이다(`parseDrvName`/`getEnv`에 직접 넘기면 확실히
거부됨, 오라클과 일치). 이 호스트는 오라클의 이 얕은-스캔 동작을 정확히
그대로 재현했다 — 오라클보다 더 엄격하게 만들지 않았다(§ 위
`ctx-string-in-args?`).

알려진 한계(고칠 버그 아님, 의도된 시뮬레이션 범위)는 `BUGS.md`에
정리했다.

## 9. Path 값 타입 — 2026-08-20

2026-08-20 이전까지 이 호스트는 5개 호스트 중 유일하게 진짜 Path 값이
없었다: `builtins.isPath`가 하드코딩된 `false`였고, 경로 리터럴 토큰은
`import`/`scopedImport` 문법에서만 파서가 직접 소비했다(§1/§5 옛 서술).
바로 앞서 같은 날 이식된 문자열 컨텍스트 작업(§8)의 값-표현/전파 관용구를
그대로 재사용해서 같은 자리에 Path 값 타입을 채웠다.

### 값 표현 — `PathValue` 레코드

`src/pnix_cljs/evaluator.cljs` 최상단(다른 레코드들 옆)에
`(defrecord PathValue [text])`를 추가했다. **plain-string-backed**를
골랐다(다른 두 호스트 중 한쪽은 순수 문자열, 다른 쪽은 문자열 키 태그
맵) — 이 파일 고유의 관용구가 이미 새 태그 값 종류엔 레코드
(`ContextStringValue`, §8)라서 그걸 그대로 따랐고, 태그 맵과 달리
`PathValue`는 진짜 attrset과 절대 혼동되지 않는다(맵이 아니므로 `map?`류
검사에 안 걸림). `text` 필드는 **항상** `normalize-path`로 정규화된 텍스트
— 생성 지점(리터럴, `+` 연결, `toPath`, `dirOf`)마다 매번 정규화해서
`==`/`<`가 저장된 텍스트를 재정규화 없이 바로 비교할 수 있게 했다.

- `path-value?` — 술어.
- `normalize-path` — `.`/`..` 세그먼트를 real Nix처럼 접는다: `.`은
  사라지고, `..`는 접을 대상이 있으면 이전 세그먼트를 지운다. 접을 게
  없으면 상대 경로는 `..`를 그대로 유지(뭘 기준으로 하는지 모르니 못
  풀어냄), 절대 경로는 그냥 버림(루트 위로는 못 올라감). **cur-dir로
  절대화하지 않는다** — 상대 리터럴은 정규화된 상대 텍스트(`./a/b`)를
  그대로 유지한다. 이건 실제 Nix(파일 위치 기준 절대 경로로 만듦)와는
  다르지만, 다른 두 호스트도 같은 선택(리터럴 텍스트만 정규화, cur_dir
  조인 없음)을 하는 걸 오라클로 직접 확인하고 따른 것 — 의도적
  divergence, `BUGS.md`에는 이미 다른 두 호스트가 공유하는 설계 선택으로
  기록돼 있으므로 이 호스트 몫만 추가.
- `make-path` — 모든 `PathValue` 생성이 거쳐야 하는 단일 관문(`(->PathValue
  (normalize-path s))`).

### 파서 — 경로 리터럴이 이제 진짜 식(expression)이다

- `parser.cljs`의 `parse-primary`에 `:path` case를 추가:
  `{:op :path :value (:value token)}`.
- `atom-starts`에 `:path`를 추가 — `builtins.isPath ./x`처럼 함수 적용
  인자로 쓸 수 있게 됐다.
- `parse-list-element`에도 **별도로** `:path (parse-selection parser)`를
  추가해야 했다 — 이게 없으면 `:path`는 이 함수의 default 분기인
  `parse-expression`(완전한 식 문법, 이항 연산자 포함)으로 떨어지는데,
  `atom-starts`에 `:path`가 들어간 뒤라서 `[ ./a ./b ]`의 첫 원소를 파싱할
  때 `./a`를 `./b`에 함수 적용하는 것으로 오파싱해버린다(2원소 리스트가
  아니라 1원소 리스트가 됨). `:integer`/`:string` 등 다른 원자 토큰들이
  이미 이 함수 안에서 `parse-selection`(원자 + `.` 선택만, 연산자 없음)을
  쓰는 것과 같은 이유로 `:path`도 나란히 추가.
- `import`/`scopedImport`는 **영향 없음** — 그 둘은 여전히 자기 case 안에서
  `:path` 토큰을 직접 소비해 `{:op :import ...}`/`{:op :scoped-import
  ...}`를 만들고, `parse-primary`의 새 일반 `:path` case에는 도달하지
  않는다(오프-경로 순서: `:import`/`:scopedImport`가 별개 case로 먼저
  매치됨). `evaluator.cljs`도 마찬가지로 `evaluate-expression`에 새
  `:path` case를 추가했을 뿐, `:import`/`:scoped-import` case는 손대지
  않았다.

### 전파 지점

- **`+`** (`numeric-binary`, `path-add`로 분리): Path+Path/Path+String은
  두 피연산자의 **표시 텍스트를 구분자 없이** 이어붙인 뒤
  `normalize-path`로 정규화(`./a + ./../b`가 `./a./../b`라는 리터럴이
  아니라 `./b`로 접힘). String+Path는 반대 방향으로 코어스하지만 결과가
  **평범한 문자열**(Path 아님). 오직 **컨텍스트 없는 plain 문자열**만
  Path와 섞일 수 있다 — `ByteStringValue`나 컨텍스트 있는
  `ContextStringValue`가 섞이면 Path에 컨텍스트를 실을 자리가 없으므로
  `toPath`와 같은 fail-closed 태도로 거부(`"unsupported-path-operand"`).
- **`${...}` 문자열 보간** (`evaluate-string-segments`): 경로 자신의
  리터럴 텍스트를 그대로 삽입하지 **않는다** — 실제 Nix가 "경로를 store에
  복사하고 그 store path를 보간"하는 걸 흉내내서, `fake-store-path-for`가
  §8의 `derivation-paths`/`hash-bytes`와 같은 의사-해시 관용구
  (`sha256` 앞 32자 + basename)로 `/nix/store/<hash>-<basename>` 문자열을
  만들어 삽입한다. **주의**: 이 대체 텍스트는 `collected`(문자열
  컨텍스트)에 아무것도 얹지 않는다 — 이건 참고로 삼은 다른 호스트의
  최근 구현을 그대로 옮긴 것이고, 실제로는 "이 문자열이 이 경로에
  의존한다"는 진짜 의존성 정보가 여기서 빠지는 셈이라 잠재적 gap으로 보인다
  (아래 "판단 콜" 참고 — 일부러 고치지 않고 참고 구현 그대로 포팅함).
- **`==`/`<`**(`equal-values*`/`ordered-less`): `PathValue`는 생성
  시점에 이미 정규화돼 있으므로 저장된 `text`를 바이트 단위로 비교하는
  것만으로 충분(`equal-bytes?`/`compare-bytes`, 이 파일이 문자열 비교에
  이미 쓰던 것과 동일한 헬퍼). Path와 다른 타입을 섞으면(`==`는 false,
  `<`는 type-error) — 문자열/숫자/리스트/attrset과 같은 기존 패턴 그대로.
- **`typeOf`/`isPath`**: `typeOf`가 `"path"`를 반환하도록, `isPath`가
  하드코딩 `false`이던 걸 `(path-value? argument)`로 교체.
- **`dirOf`/`baseNameOf`**: `dirOf`는 Path 입력이면 Path를 반환(헤드
  텍스트를 다시 `make-path`로 정규화), 문자열 입력이면 기존 문자열 전용
  경로를 그대로 유지(둘 다 오라클 검증). `baseNameOf`는 입력이 Path든
  문자열이든 **항상 평범한 문자열**을 반환(오라클 확인, 두 호스트 모두
  동일).
- **`toPath`**: 이제 평범한 문자열이 아니라 진짜 `PathValue`를 반환한다.
  **`string -> path`라는 실제 Nix 시그니처를 그대로 유지** — Path 인자
  자체를 넣으면 type-error(다른 두 호스트 중 하나가 더 관대하게 Path
  패스스루도 받아주지만, 나머지 한쪽 오라클로 직접 확인한 결과
  `builtins.toPath ./x`가 거기서도 에러였다 — 그래서 실제 Nix 시그니처
  + 오라클 둘 다에 맞춰 **거부**를 선택). 컨텍스트 있는 문자열 인자도
  거부(전엔 컨텍스트를 그대로 유지한 채 통과시켰는데, 이제 결과가
  Path라서 컨텍스트를 실을 자리가 없어졌다 — 조용히 버리는 대신
  `"string-context-frontier"`로 명시적 거부).
- **`toString`**(`nix-to-string`) / **`toJSON`**(`to-json-value`) /
  **`toXML`**(`to-xml-value`) / **`materialize`**(canonical 출력 경계):
  전부 Path를 **자기 자신의 정규화된 텍스트**로 직렬화한다(`${...}` 보간과
  달리 의사 store path를 안 만듦). `toXML`만 `<string>`을 재사용하지 않고
  **전용 `<path value="..." />` 태그**를 쓴다(다른 두 호스트 중 하나도
  전용 태그를 쓰는 걸 오라클로 확인, 이 파일 자체의 `value="..."` leaf
  속성 관용구와도 맞음).
- **`readFile`/`readDir`/`pathExists`**(`path-string`): 이제 Path 인자를
  받을 수 있다 — 전엔 경로 리터럴이 애초에 함수 인자가 될 수 없어서 도달
  불가능했던 코드 경로.

### 판단 콜 — 참고 구현들이 서로 다른 지점

여러 참고 구현을 대조하다 서로 갈리는 지점이 몇 군데 있었다. 여기서
내린 선택과 근거:

1. **`${...}` 보간의 컨텍스트 미추적.** 위에 적었듯, 가장 최근에 참고한
   구현은 의사 store path로 치환하되 컨텍스트에 아무것도 안 얹는다. 더
   오래된 다른 참고 구현은 반대로 경로의 리터럴 텍스트 자체를 그대로
   삽입하면서 그 텍스트 자체를 컨텍스트 원소로 얹는다(진짜 의존성 추적).
   이 호스트는 **가장 최근 구현을 그대로 포팅**했다 — 이번 포팅이 그
   구현을 "이걸 그대로 이식하라"는 명시적 지시로 받았고, 스스로 판단해
   설계를 하나 더 얹는 대신 명시된 참고를 충실히 재현하는 쪽을 택했다.
   다만 이건 실제로 gap일 수 있어 보인다(문자열 컨텍스트 기능 전체의
   존재 이유가 의존성 추적인데, 의사 store path 치환은 그 추적에서
   빠짐) — 나중에 다시 볼 가치가 있는 지점으로 남겨둔다.
2. **`toPath`의 Path 인자 처리.** 위에 적었듯 두 참고 구현이 갈려서,
   실제 Nix 시그니처(`string -> path`)와 오라클로 실측 확인한 쪽 모두에
   맞춰 거부를 선택했다.
3. **`toJSON`의 출력 모양.** 오래된 쪽 참고 구현은 Path를 JSON으로 찍을
   때 내부 태그 맵 표현이 그대로 새어나가는 것처럼 보였다(경로 값이
   `{"__pnix_value_kind":"path","path":"./a/b"}`로 직렬화됨 — 실제 Nix의
   `--json`이 경로를 평범한 문자열로 찍는 것과 다름, 아마 그쪽의 버그).
   이 호스트는 실제 Nix 동작 + 최근 참고 구현 둘 다와 일치하는 **평범한
   JSON 문자열**(`"./a/b"`)을 택했다.

### 검증 결과 (2026-08-20)

`node pnix-cljs/dist/pnix-cljs.js -e 'EXPR'`로 아래 전부 확인(오라클과 구조
일치, 위 "판단 콜" 3곳만 의도된 차이):

```
builtins.typeOf ./foo                          -> "path"
builtins.isPath ./foo                           -> true
builtins.isPath "not-a-path"                     -> false
(./a) + (./b)                                    -> "./a./b"
./a + "b"                                        -> "./ab"
"a" + ./b                                        -> "a./b"
builtins.dirOf ./a/b/c                           -> "./a/b"
builtins.baseNameOf ./a/b/c                      -> "c"
./a == ./a                                       -> true
./a == ./../x/a                                  -> false
let p = ./world; in "hello ${p}"                 -> "hello /nix/store/<hash>-world"
builtins.toPath "/x/y"                           -> "/x/y" (typeOf "path")
builtins.toPath ./x/y                            -> 실패(type-error)
builtins.toJSON ./a/b                            -> "\"./a/b\""
builtins.toXML ./a/b                             -> <path value="./a/b" />
builtins.toString ./a/b                          -> "./a/b"
./a/../../b                                      -> "../b"
/a/../../b                                       -> "/b"
[ ./a ./b ]                                      -> 2원소 리스트 (오파싱 아님)
./a - ./b, ./a + 1                               -> 둘 다 type-error
```

기존 게이트(`bin/pnix-cljs-gate --rebuild`, `bin/pnix-cljs-identity-gate`)
전부 그린 — 특히 `import`/`scopedImport`(직접 재현 테스트로 추가 확인,
회귀 없음)와 `derivation`/`placeholder`의 의사-해시 경로(§8의
`hash-bytes`/`derivation-paths`를 `fake-store-path-for`가 그대로 재사용,
회귀 없음). `test/pnix_cljs/self_test.cljs`에 위 검증 항목 상당수를
회귀 방지용으로 추가.

### 이 seed에서 새로 발생한 제한

`BUGS.md`에 정리(아래는 요약):

- 경로 리터럴 안 `${...}` 동적 보간(`./a/${x}`)은 지원 안 함(렉서가 경로
  문자로 `{`/`}`를 애초에 인정 안 함) — 다른 참고 구현들도 마찬가지로 이
  구문에서 에러가 나는 걸 오라클로 확인, 이 호스트만의 새 제한이 아님.
- `${...}` 보간의 의사 store path가 문자열 컨텍스트에 안 얹힘(위 "판단
  콜" 1번) — 참고 구현을 충실히 재현한 결과지만 잠재적 gap.
