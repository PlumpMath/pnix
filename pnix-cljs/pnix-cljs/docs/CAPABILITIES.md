# pnix-cljs CAPABILITIES — 능력 인덱스 (중복개발 방지 조회)

> 생성: `node pnix-cljs/dist/pnix-cljs.js capabilities > pnix-cljs/docs/CAPABILITIES.md` — 손 편집 금지.
> drift 게이트: `node pnix-cljs/dist/pnix-cljs.js capabilities-check`.

## CLI 명령

| 명령 | 목적 |
|---|---|
| `-e SOURCE` / `--eval SOURCE` | 인라인 소스 평가 → canonical JSON 프로젝션 |
| `FILE` | `.px` 파일 평가 → canonical JSON 프로젝션 |
| `--repl` | 대화형 REPL (Node `readline`, `core/projection` 그대로 사용) |
| `capabilities` | 이 문서 생성 (stdout) |
| `capabilities-check` | drift 게이트 — 재생성 결과 vs 커밋된 `docs/CAPABILITIES.md` |

## 모듈 (`src/pnix_cljs/`)

| 파일 | 역할 |
|---|---|
| `tokenizer.cljs` | 렉서 |
| `parser.cljs` | 재귀 하강 파서 → AST |
| `evaluator.cljs` | 값 표현 + 평가기 + 빌트인 dispatch (가장 큼) |
| `core.cljs` | `eval-source`/`projection`/`canonical-json` 진입점 |
| `module.cljs` / `node_loader.cljs` | Node용 import 소스 로딩 어댑터 |
| `main.cljs` | CLI 진입점 (`dist/pnix-cljs.js`) |
| `capabilities.cljs` | 이 문서의 생성기 |

## 호스트 라이브러리 진입점 (Node / CommonJS)

| 대상 | 빌드 결과물 | shadow-cljs main |
|---|---|---|
| CLI | `dist/pnix-cljs.js` (`package.json` `bin.pnix-cljs`) | `pnix-cljs.main` |
| require 라이브러리 | `dist/pnix-cljs-module.js` (`package.json` `main`) | `pnix-cljs.module` |
| self-test | `dist/pnix-cljs-self-test.js` | `pnix-cljs.self-test` |

require API 상세(`evalSource`/`evalFile`/... 전체 표면): [`docs/IMPLEMENTATION.md`](IMPLEMENTATION.md) §3.

## 빌트인 표면 (presence inventory)

등록된 `builtins-value` 키 194개(콜러블 + 값 상수 `true`/`false`/`null`/`langVersion`/`nixVersion`/`storeDir` + 재귀 `builtins` 필드):

abort abs add addErrorContext all and any append appendContext assertMsg atan2 attrByPath attrNames attrValues baseNameOf bitAnd bitOr bitXor boolToString break builtins catAttrs ceil compareVersions concatLists concatMap concatMapStrings concatMapStringsSep concatStrings concatStringsSep cons const cos deepSeq derivation derivationStrict dirOf div drop elem elemAt eq exp false fetchGit fetchTarball fetchurl filter filterAttrs filterAttrsRecursive find findFirst fix flatten flip floor foldl foldl' foldlAttrs foldr fromJSON functionArgs ge genAttrs genList genericClosure get getAttr getAttrFromPath getAttrFromPathOr getContext getEnv getName getVersion groupBy gt hasAttr hasAttrByPath hasContext hasInfix hasPrefix hasSuffix hashString head id imap0 imap1 implies init intersectAttrs intersectLists isAttrs isBool isFloat isFunction isInt isList isNull isPath isString keys langVersion last le length lessThan listToAttrs ln log lt map mapAttrs mapAttrs' mapAttrsRecursive mapAttrsToList match max merge min mod mul nameValuePair neg nixVersion not null optional optionalAttrs optionalString optionals or parseDrvName partition pathExists pipe placeholder pnixMounts pow product range readDir readFile recursiveUpdate removeAttrs removePrefix removeSuffix replaceStrings replicate reverseList seq set sin sort split splitString splitVersion sqrt storeDir storePath stringLength stringToCharacters sub substring subtractLists sum tail take tan throw toFile toInt toJSON toLower toPath toString toUpper toXML trace true tryEval typeOf unique unsafeDiscardOutputDependency unsafeDiscardStringContext unsafeGetAttrPos updateManyAttrs values warn when zip zipAttrs zipAttrsWith zipLists zipListsWith

presence는 호출 parity 주장이 아니다 — 실제 실행 의미는 `evaluator/invoke-builtin`을 참고.
5개 호스트 비교표는 [`docs/IMPLEMENTATION.md`](IMPLEMENTATION.md) §4.

## 관련 문서

narrative 상세(아키텍처, 스코프, known 차이점, 역사)는 [`docs/IMPLEMENTATION.md`](IMPLEMENTATION.md) 전체 참고 — 이 문서는 코드에서 파생된 terse 인덱스일 뿐이다.
