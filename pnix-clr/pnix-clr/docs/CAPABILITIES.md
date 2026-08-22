# pnix-clr CAPABILITIES — 생성물 (손 편집 금지 / GENERATED — do not hand-edit)

> 재생성: `bin/pnix-clr capabilities > pnix-clr/docs/CAPABILITIES.md` (저장소 루트 `pnix-clr/`에서 실행).
> drift 게이트: `bin/pnix-clr capabilities-check` (`bin/pnix-clr-gate`에 연결됨).

코드에서만 파생된 인덱스 -- 손으로 쓴 텍스트나 타임스탬프 없음. 서술형 설명(아키텍처, 5개 호스트 비교, 범위 경계)은 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) 참고 -- 이 문서는 조회용 인덱스일 뿐이다.

## CLI 명령 (`bin/pnix-clr`)

| 형태 | 동작 |
|---|---|
| `-e SOURCE` | 인라인 SOURCE 평가, CLI JSON projection 출력 |
| `FILE.px` | 파일 평가, CLI JSON projection 출력 |
| `--call-json FILE.px ENTRY ARGS_JSON` | `.px` attrset의 curried ENTRY를 JSON 배열 인자로 호출 |
| `--repl` | 대화형 pnix REPL (개발자 진입점, evaluation authority 아님) |
| `--production-outcome-self-check` | 내장 production-outcome self-check 실행 |
| `--production-outcome CASES.tsv` | TSV 파일의 production-outcome 케이스 실행 |
| `capabilities` | 생성된 능력 인덱스를 stdout에 출력 (docs/CAPABILITIES.md 소스) |
| `capabilities-check` | 커밋된 docs/CAPABILITIES.md를 방금 렌더링한 것과 비교하는 drift 게이트 |

## 빌트인 presence (193종)

`pnix-clr.evaluator/builtin-names`가 root `builtins-entries` 등록 테이블(이름 -> 빌트인/상수)에서 직접 뽑은 목록 -- 손으로 옮겨 적지 않았으므로 빌트인이 추가/삭제되면 재생성 시 자동으로 따라온다. presence는 등록된 이름과 arity만 볼 뿐 호출 시 실제 semantics/parity를 주장하지 않는다; 5개 호스트 비교표는 [`IMPLEMENTATION.md`](IMPLEMENTATION.md) §2 참고.

  abort abs add addErrorContext all and any append
  appendContext assertMsg atan2 attrByPath attrNames attrValues baseNameOf bitAnd
  bitOr bitXor boolToString break catAttrs ceil compareVersions concatLists
  concatMap concatMapStrings concatMapStringsSep concatStrings concatStringsSep cons const cos
  deepSeq derivation derivationStrict dirOf div drop elem elemAt
  eq exp false fetchGit fetchTarball fetchurl filter filterAttrs
  filterAttrsRecursive find findFirst fix flatten flip floor foldl
  foldl' foldlAttrs foldr fromJSON functionArgs ge genAttrs genericClosure
  genList get getAttr getAttrFromPath getAttrFromPathOr getContext getEnv getName
  getVersion groupBy gt hasAttr hasAttrByPath hasContext hashString hasInfix
  hasPrefix hasSuffix head id imap0 imap1 implies init
  intersectAttrs intersectLists isAttrs isBool isFloat isFunction isInt isList
  isNull isPath isString keys langVersion last le length
  lessThan listToAttrs ln log lt map mapAttrs mapAttrs'
  mapAttrsRecursive mapAttrsToList match max merge min mod mul
  nameValuePair neg nixVersion not null optional optionalAttrs optionals
  optionalString or parseDrvName partition pathExists pipe placeholder pnixMounts
  pow product range readDir readFile recursiveUpdate removeAttrs removePrefix
  removeSuffix replaceStrings replicate reverseList seq set sin sort
  split splitString splitVersion sqrt storeDir storePath stringLength stringToCharacters
  sub substring subtractLists sum tail take tan throw
  toFile toInt toJSON toLower toPath toString toUpper toXML
  trace true tryEval typeOf unique unsafeDiscardOutputDependency unsafeDiscardStringContext unsafeGetAttrPos
  updateManyAttrs values warn when zip zipAttrs zipAttrsWith zipLists
  zipListsWith

`import`/`scopedImport`는 이 호스트에서 예약 키워드(파서 전용 문법)로 구현돼 있어 `builtins-entries` 등록 패턴에 안 잡힌다 -- 실제로는 둘 다 있다(`IMPLEMENTATION.md` §1). `builtins`(재귀 자기참조)는 `builtins-entries`가 아니라 `make-builtins`가 별도로 붙이므로 위 목록에는 없다.
