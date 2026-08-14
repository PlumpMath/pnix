# pnix-clj + clj-meta

**Clojure(clj-meta) ↔ pnix** 메타순환 도구 모음입니다. `pnix-clj`는 Clojure/JVM에서
호스팅되는 **pnix 런타임**이고, `clj-meta`는 Clojure-on-Clojure **호스트 증명 레인**
(바이트코드 self-host 컴파일러)입니다. 모든 능력은 네 개의 독립 기판 — 직접
evaluator, JVM 바이트코드 lowering(clj-meta), pnix self-runtime(`.px`), pnix
mirror — 에서 교차 검증되며, self-hosting **tower**가 하나의 값으로 수렴시킵니다.

> 이 트리는 [`pnix-hy`](../pnix-hy)(Hy/Python ↔ pnix)의 Clojure/JVM 형제입니다.
> 여기서의 투영 대상은 **오직** Clojure/JVM(clj-meta)이며, Python/Hy는 없습니다.

### 이중 축 + 라이브러리 (필독)

정본: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md). 에이전트 메모: [`CLAUDE.md`](CLAUDE.md).

| 축 | 명령 / 표면 |
|------|-------------------|
| **host-main** | `pnix-clj-clj` / bare `clojure` — 이 트리를 `-Sdeps`로 주입 |
| **pnix-main** | `nix run .#pnix-clj-pnix` |
| **library** | classpath의 `pnix-clj` 소스; `PNIX_CLJ_ROOT` — **호스트 바인딩**, 이식 가능 `.px` 아님 |
| **Clojure에서 `.px` import** | `(pnix-clj.core/eval-file "x.px")` — [docs/HOST_IMPORT.md](pnix-clj/docs/HOST_IMPORT.md) |

## 레이아웃

```
pnix-clj/     pnix 런타임 (parser, evaluator, lowering, tower, safe-eval,
              specialize, cached-eval, capabilities, mirror, examples, tests)
clj-meta/     Clojure/JVM 호스트 증명 레인 (바이트코드 self-host 컴파일러)
```

## 설치 / 실행 (Nix flake)

flake가 툴체인을 고정합니다(Temurin JDK 21 + Clojure CLI). 런타임과 호스트 증명은
저장소 안의 `./pnix-clj`, `./clj-meta` 트리입니다. **저장소 루트**에서 실행하세요.

### 세 개의 독립 러너

Clojure 호스팅 Clojure 확장 — 각각 나중에 자체 `deps.edn`을 두고 네트워크/nREPL
서버로 기동할 수 있습니다.

```sh
nix run .#pnix-clj-pnix -- -e '1 + 2'   # pnix 언어 레인 (→ 3)
nix run .#pnix-clj-pnix -- file.px       # .px 파일 평가
nix run .#pnix-clj-pnix                  # ./default.px가 있으면 eval, 없으면 REPL
nix run .#pnix-clj-clj                   # pnix-clj의 Clojure 호스트 REPL
nix run .#clj-meta -- -e '(+ 1 2)'       # clj-meta 메타순환 Clojure (→ 3)

# 서버 이음새 (에디터 / 툴링)
nix run .#pnix-clj-pnix-server           # pnix 네트워크 REPL   (소켓, 포트 7888)
nix run .#pnix-clj-nrepl                 # Clojure 호스트 nREPL  (포트 7888)
nix run .#pnix-clj-pnix-nrepl            # pnix 언어 nREPL, eval via pnix (7890)
nix run .#clj-meta-nrepl                 # clj-meta nREPL, eval via clj-meta backend (7889)
```

`pnix-clj-pnix`는 Nix의 `default.nix` 관례를 따릅니다. 디렉터리에서 bare 호출 시
`./default.px`(그다음 `./default.nix`)가 있으면 평가합니다.

### 게이트 / 리포트

```sh
nix run .#gate                 # 전체 테스트 게이트 (164 tests / 3250 assertions)
nix run .#tower                # self-hosting 4-substrate tower 리포트
nix run .#capabilities-check   # 기계 생성 능력 인덱스 drift 게이트
nix run .#clj-meta-gate        # clj-meta 자체 메타순환 self-host 게이트
nix run .#examples             # 모든 examples/*/pnix_clj_way.clj 실행
nix run .#safe-eval            # pure resource-bounded sandbox 리포트
nix run .#specialize           # Futamura specialize 리포트

nix develop                    # PATH에 JDK + clojure, 그다음:
  cd pnix-clj && clojure -M:test
  cd pnix-clj && clojure -M examples/11-self-hosting-convergence/pnix_clj_way.clj
```

Nix 없이 직접 쓸 때 러너는 deps.edn 별칭에 대응합니다.
`cd pnix-clj && clojure -M:repl-pnix -e '1 + 2'`,
`cd clj-meta && clojure -M:repl -e '(+ 1 2)'`.

Nix 없이 `./pnix-clj`에서 JDK + [Clojure CLI](https://clojure.org/guides/install_clojure)만으로:

```sh
cd pnix-clj
clojure -M:test
clojure -M examples/01-pure-sandbox/pnix_clj_way.clj
```

## 예제

`pnix-clj/examples/`는 **일반 Clojure의 한계 vs pnix-clj 방식**을 메타순환 기둥마다
한 쌍씩 보여 줍니다(한국어 주석, 각각 실행 가능 + 자체 assert).

- `01-pure-sandbox` — `safe-eval`: purity 정적 검사 + fuel 한도
- `03-specialization-futamura` — `specialize`: residual 소스 + JVM 바이트코드 투영
- `04-host-interop-loss-effect` — 호스트 교차 loss/effect/capability 증인
- `05-witness-and-gate` — held/ok 판정과 증인 해시
- `06-ast-lowering-roundtrip` — parser/lowering/clj-meta 구조적 receipt
- `07-clojure-macro-over-pnix` — macroexpand 결과를 pnix로 투영하고 tower 검증
- `08-clojure-reader-or-edn-embed-pnix` — EDN 태그 pnix 소스를 parse/purity/tower 증인으로 검증
- `11-self-hosting-convergence` — `run-tower`: 한 소스, 네 기판, 한 값
- `12-content-addressed-cache` — `cached-eval`: 정규 content-addressed 키
- `19-lowered-compiled-runtime` — 직접 evaluator vs lowered compiled 런타임 동등성
- `23-capability-gate` — 호스트 effect 요구가 purity/capability 게이트 판정이 됨
- `24-phase-separation` — parse/purity/eval/lowering/compile/gate 판정 분리
- `25-typed-attestation` — capability 교차 판정이 typed interop 증인 해시를 운반
- `30-verifying-cache` — 캐시 히트를 새 평가·purity·content 키에 대해 검증
- `31-compartment-isolation` — 호스트 객체를 opaque ref로 격리, release/held 경계
- `33-futamura-ladder` — 1차/2차 Futamura 투영 검증, 3차는 서술-미구현
- `35-stage-tower-internals` — tower 계층, 인접 쌍, collapse 증인, held 차단점

인덱스는 [`pnix-clj/examples/README.md`](pnix-clj/examples/README.md)를 보세요.


## 실행 테스트해봄.
https://teu5us.github.io/nix-lib.html

builtins.zipAttrsWith (name: values: { inherit name values; }) [ { a = "x"; } { a = "y"; b = "z"; } ]
>> { a = { name = "a"; values = [ "x" "y" ]; }; b = { name = "b"; values = [ "z" ]; }; }

builtins.typeOf 1
>> "int"
builtins.typeOf true
>> "bool"
builtins.typeOf "hello"
>> "string"
## 사용방법 모르는것 >> "path" 가 나오는방법은?
builtins.typeOf null
>> "null"
builtins.typeOf {a=1;}
>> "set"
builtins.typeOf [ 1 2 "a" ]
>> "list"
builtins.typeOf (arg: 1+arg)
>> "lambda"
builtins.typeOf 1.2
>> "float"

builtins.tryEval (1 + 2)
>> { success = true; value = 3; }

builtins.trace "여기까지 실행됨" 42
>> 42

builtins.toXML {a=1;}
>> "<?xml version='1.0' encoding='utf-8'?>\n<expr>\n  <attrs>\n    <attr name=\"a\">\n      <int value=\"1\" />\n    </attr>\n  </attrs>\n</expr>\n"

builtins.toString [ "foo" "bar" ]
>> "foo bar"

builtins.toJSON { a = 1; b = true; }
>> "{\"a\":1,\"b\":true}"

builtins.toFile "hello.txt" "안녕하세요"
>> /tmp/pnix-nix-store/21de696a91f850987248a591f1253406-hello.txt

# builtins.throw "에러 발생!"
>> error: {:schema :pnix.machine.eval-error-model.v1, :phase :eval, :class :throw-builtin-called, :evidence {:builtin :throw}}

builtins.substring 0 3 "abcdef"
>> "abc"

builtins.readFile "${builtins.getEnv "HOME"}/hello.txt"
>> "helo\n"

builtins.readDir "/path/to/pnix"
>> { .github = "directory"; .gitignore = "regular"; LICENSE = "regular"; README.md = "regular"; pnix-clj = "directory"; pnix-cljs = "directory"; pnix-clr = "directory"; pnix-hy = "directory"; pnix-rs = "directory"; }

builtins.pathExists "${builtins.getEnv "HOME"}/hello.txt"
>> true

builtins.fetchurl "https://bootstrap.pypa.io/get-pip.py"
>> "/tmp/pnix-nix-store/d3ae4644cf5ce68b5c97485ba6fbca31-get-pip.py"

builtins.fetchTarball { url = "https://www.svp-team.com/files/svp4-latest.php?mac"; sha256 = "04phzhyw0haiz77j494s1rz0as5yg70gb33i864riylfj776h27v"; }
>> "/tmp/pnix-nix-store/10b6d0baaf062b024ab29e68898e27a2-tarball"

# builtins.fetchGit { url = "https://github.com/NixOS/nixpkgs.git"; rev = "abcdef1234567890"; }
>> error: {:schema :pnix.machine.eval-error-model.v1, :phase :eval, :class :type-error, :evidence {:builtin :fetchGit, :url "{:kind :thunk, :label [:attr \"url\"], :state #object[clojure.lang.Atom 0x538f381a {:status :ready, :val {:phase :pending}}], :compute #object[pnix_clj.evaluator$eval_attrs$fn__914 0x5d48cc1b \"pnix_clj.evaluator$eval_attrs$fn__914@5d48cc1b\"]}", :rev "{:kind :thunk, :label [:attr \"rev\"], :state #object[clojure.lang.Atom 0x5977536a {:status :ready, :val {:phase :pending}}], :compute #object[pnix_clj.evaluator$eval_attrs$fn__914 0x1c75c620 \"pnix_clj.evaluator$eval_attrs$fn__914@1c75c620\"]}", :policy :network-or-git-unavailable}}

builtins.attrNames { a = 1; b = 2; }
>> [ "a" "b" ]

builtins.attrValues { a = 1; b = 2; }
>> [ 1 2 ]

builtins.hasAttr "a" { a = 1; }
>> true

builtins.getAttr "a" { a = 1; }
>> 1

builtins.getAttrFromPath [ "foo" "bar" ] { foo = { bar = 42; }; }
>> 42

builtins.mapAttrs (name: value: value + 1) { a = 1; b = 2; }
>> { a = 2; b = 3; }

builtins.filterAttrs (name: value: value > 1) { a = 1; b = 2; }
>> { b = 2; }

builtins.listToAttrs [ { name="a"; value=1; }  { name="b"; value=2; } ]
>> { a = 1; b = 2; }

builtins.length [1 2 3]
>> 3

builtins.head [ "a" "b" "c" ]
>> "a"

builtins.tail [ "a" "b" "c" ]
>> [ "b" "c" ]

builtins.last [ "a" "b" "c" ] 
>> "c"

builtins.init [ "a" "b" "c" ] 
>> [ "a" "b" ]

builtins.elem "b" [ "a" "b" "c" ]
>> true

builtins.concatLists [ [1 2] [3 4] ]
>> [ 1 2 3 4 ]

builtins.flatten [ [1 2] [3 [4 5]] ]
>> [ 1 2 3 4 5 ]

builtins.concatStringsSep ", " [ "a" "b" "c" ]
>> "a, b, c"

builtins.concatMapStringsSep "-" (x: toString x) [1 2 3]
>> "1-2-3"

builtins.removePrefix "foo" "foobar"
>> "bar"

builtins.removeSuffix ".txt" "hello.txt" 
>> "hello"

builtins.hasPrefix "foo" "foobar"
>> true

builtins.hasSuffix ".txt" "hello.txt" 
>> true

builtins.splitString ":" "a:b:c"
>> [ "a" "b" "c" ]

builtins.toLower "Hello"
>> "hello"

builtins.toUpper "Hello"
>> "HELLO"

builtins.boolToString true
"true"

lib.implies true false
>> false

builtins.optional true "foo"
>> [ "foo" ]

builtins.optionals false [1 2 3] 
>> [ ]

lib.optionalAttrs true { a = 1; }
>> { a = 1; }

lib.when false "foo"
>> null

builtins.id 42
>> 42

lib.const "foo" "bar"
>> "foo"

builtins.flip (a: b: a - b) 3 10
>> 7

builtins.pipe 2 [ (x: x + 3) (x: x * 2) ]
>> 10

builtins.foldl (acc: x: acc + x) 0 [1 2 3]
>> 6

builtins.foldr (x: acc: x + acc) 0 [1 2 3]
>> 6

lib.fix (self: { a = 1; b = self.a + 1; })
>> { a = 1; b = 2; }

builtins.min 3 7
>> 3

builtins.max 3 7
>> 7

builtins.range 1 5
>> [ 1 2 3 4 5 ]

builtins.genList (x: x * 2) 4
>> [ 0 2 4 6 ]

lib.sum [1 2 3 4]
>> 10

lib.product [1 2 3 4] 
>> 24

builtins.recursiveUpdate { a = { b = 1; }; } { a = { c = 2; }; }
>> { a = { b = 1; c = 2; }; }

lib.updateManyAttrs [ { a = 1; } { b = 2; } ] { c = 3; }
>> { a = 1; b = 2; c = 3; }

builtins.attrByPath [ "foo" "bar" ] 0 { foo.bar = 42; }
>> 42

lib.attrsets.isAttrs { a = 1; }
>> true

lib.attrsets.mapAttrsToList (n: v: "${n}=${toString v}") { a = 1; b = 2; }
>> [ "a=1" "b=2" ]

lib.attrsets.zipAttrs [ { a = 1; } { a = 2; b = 3; } ]
>> { a = [ 1 2 ]; b = [ 3 ]; }

lib.getName { name = "hello-1.0"; }
>> "hello"

lib.getVersion { version = "1.0"; }
>> "1.0"

lib.getAttrFromPathOr { meta = { description = "테스트"; }; } [ "meta" "description" ] "없음"
>> "테스트"

builtins.hasAttrByPath [ "meta" "license" ] { meta.license = "MIT"; }
>> true

lib.filterAttrsRecursive (name: value: name == "license") { meta = { license = "MIT"; }; }
>> { }

lib.mapAttrsRecursive (path: value: toString value) { a = { b = 1; }; }
>> { a = { b = "1"; }; }

builtins.unique [1 2 2 3 1]
>> [ 1 2 3 ]

lib.intersectLists [1 2 3] [2 3 4]
>> [ 2 3 ]

lib.subtractLists [1 2 3] [2]
>> [ 1 3 ]

builtins.concatMap (x: [x x]) [1 2 3]
>> [ 1 1 2 2 3 3 ]

builtins.partition (x: x > 2) [1 2 3 4]
>> { right = [ 3 4 ]; wrong = [ 1 2 ]; }

lib.zipLists [1 2] ["a" "b"]
>> [ { fst = 1; snd = "a"; } { fst = 2; snd = "b"; } ]

builtins.zipListsWith (a: b: "${toString a}${b}") [1 2] ["a" "b"]
>> [ "1a" "2b" ]

builtins.warn "경고: deprecated 함수 사용" "foo"
>> evaluation warning: 경고: deprecated 함수 사용
>> "foo"

# lib.assert (1 + 1 == 2) "수학이 잘못됨!"
>> error: {:schema :pnix.machine.eval-error-model.v1, :phase :eval, :class :attribute-missing, :evidence {:attr "assert", :available-attrs ["abort" "abs" "add" "addErrorContext" "all" "and" "any" "append" "appendContext" "assertMsg" "atan2" "attrByPath" "attrNames" "attrValues" "attrsets" "baseNameOf" "bitAnd" "bitOr" "bitXor" "boolToString" "break" "builtins" "catAttrs" "ceil" "compareVersions" "concatLists" "concatMap" "concatMapStrings" "concatMapStringsSep" "concatStrings" "concatStringsSep" "cons" "const" "cos" "count" "currentSystem" "deepSeq" "derivation" "derivationStrict" "dirOf" "div" "drop" "elem" "elemAt" "eq" "exp" "false" "fetchGit" "fetchTarball" "fetchurl" "filter" "filterAttrs" "filterAttrsRecursive" "find" "findFirst" "fix" "flatten" "flip" "floor" "foldl" "foldl'" "foldlAttrs" "foldr" "fromJSON" "functionArgs" "ge" "genAttrs" "genList" "genericClosure" "get" "getAttr" "getAttrFromPath" "getAttrFromPathOr" "getContext" "getEnv" "getName" "getVersion" "groupBy" "gt" "hasAttr" "hasAttrByPath" "hasContext" "hasInfix" "hasPrefix" "hasSuffix" "hashString" "head" "id" "imap0" "imap1" "implies" "init" "intersectAttrs" "intersectLists" "isAttrs" "isBool" "isFloat" "isFunction" "isInt" "isList" "isNull" "isPath" "isString" "keys" "langVersion" "last" "le" "length" "lessThan" "listToAttrs" "ln" "lt" "map" "mapAttrs" "mapAttrs'" "mapAttrsRecursive" "mapAttrsToList" "match" "max" "merge" "min" "mod" "mul" "nameValuePair" "neg" "nixVersion" "not" "null" "optional" "optionalAttrs" "optionalString" "optionals" "or" "parseDrvName" "partition" "pathExists" "pipe" "placeholder" "pnixMounts" "pow" "product" "range" "readDir" "readFile" "recursiveUpdate" "removeAttrs" "removePrefix" "removeSuffix" "replaceStrings" "replicate" "reverseList" "seq" "set" "sin" "sort" "split" "splitString" "splitVersion" "sqrt" "storeDir" "storePath" "stringLength" "stringToCharacters" "sub" "substring" "subtractLists" "sum" "tail" "take" "throw" "toFile" "toInt" "toJSON" "toLower" "toPath" "toString" "toUpper" "toXML" "trace" "true" "tryEval" "typeOf" "unique" "unsafeDiscardOutputDependency" "unsafeDiscardStringContext" "unsafeGetAttrPos" "updateManyAttrs" "values" "warn" "when" "zip" "zipAttrs" "zipAttrsWith" "zipLists" "zipListsWith"]}}

# builtins.assert (1 + 1 == 2) "수학이 잘못됨!"
>> error: {:schema :pnix.machine.eval-error-model.v1, :phase :eval, :class :attribute-missing, :evidence {:attr "assert", :available-attrs ["abort" "abs" "add" "addErrorContext" "all" "and" "any" "append" "appendContext" "assertMsg" "atan2" "attrByPath" "attrNames" "attrValues" "baseNameOf" "bitAnd" "bitOr" "bitXor" "boolToString" "break" "builtins" "catAttrs" "ceil" "compareVersions" "concatLists" "concatMap" "concatMapStrings" "concatMapStringsSep" "concatStrings" "concatStringsSep" "cons" "const" "cos" "count" "currentSystem" "deepSeq" "derivation" "derivationStrict" "dirOf" "div" "drop" "elem" "elemAt" "eq" "exp" "false" "fetchGit" "fetchTarball" "fetchurl" "filter" "filterAttrs" "filterAttrsRecursive" "find" "findFirst" "fix" "flatten" "flip" "floor" "foldl" "foldl'" "foldlAttrs" "foldr" "fromJSON" "functionArgs" "ge" "genAttrs" "genList" "genericClosure" "get" "getAttr" "getAttrFromPath" "getAttrFromPathOr" "getContext" "getEnv" "getName" "getVersion" "groupBy" "gt" "hasAttr" "hasAttrByPath" "hasContext" "hasInfix" "hasPrefix" "hasSuffix" "hashString" "head" "id" "imap0" "imap1" "implies" "init" "intersectAttrs" "intersectLists" "isAttrs" "isBool" "isFloat" "isFunction" "isInt" "isList" "isNull" "isPath" "isString" "keys" "langVersion" "last" "le" "length" "lessThan" "listToAttrs" "ln" "lt" "map" "mapAttrs" "mapAttrs'" "mapAttrsRecursive" "mapAttrsToList" "match" "max" "merge" "min" "mod" "mul" "nameValuePair" "neg" "nixVersion" "not" "null" "optional" "optionalAttrs" "optionalString" "optionals" "or" "parseDrvName" "partition" "pathExists" "pipe" "placeholder" "pnixMounts" "pow" "product" "range" "readDir" "readFile" "recursiveUpdate" "removeAttrs" "removePrefix" "removeSuffix" "replaceStrings" "replicate" "reverseList" "seq" "set" "sin" "sort" "split" "splitString" "splitVersion" "sqrt" "storeDir" "storePath" "stringLength" "stringToCharacters" "sub" "substring" "subtractLists" "sum" "tail" "take" "throw" "toFile" "toInt" "toJSON" "toLower" "toPath" "toString" "toUpper" "toXML" "trace" "true" "tryEval" "typeOf" "unique" "unsafeDiscardOutputDependency" "unsafeDiscardStringContext" "unsafeGetAttrPos" "updateManyAttrs" "values" "warn" "when" "zip" "zipAttrs" "zipAttrsWith" "zipLists" "zipListsWith"]}}
