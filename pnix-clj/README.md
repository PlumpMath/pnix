# pnix-clj + clj-meta

A **Clojure(clj-meta) ↔ pnix** meta-circular toolkit. `pnix-clj` is a
Clojure/JVM-hosted **pnix runtime**; `clj-meta` is the Clojure-on-Clojure
**host-proof lane** (a bytecode self-host compiler). Every capability is
cross-checked across four independent substrates — the direct evaluator, a
JVM-bytecode lowering (clj-meta), the pnix self-runtime (`.px`), and the pnix
mirror — and collapsed to a single value by the self-hosting **tower**.

> This is the Clojure/JVM sibling of [`pnix-hy`](../pnix-hy) (Hy/Python ↔ pnix).
> The projection target here is **only** Clojure/JVM (clj-meta) — no Python/Hy.

## Layout

```
pnix-clj/     the pnix runtime (parser, evaluator, lowering, tower, safe-eval,
              specialize, cached-eval, capabilities, mirror, examples, tests)
clj-meta/     the Clojure/JVM host-proof lane (bytecode self-host compiler)
```

## Install / run (Nix flake)

The flake pins the toolchain (Temurin JDK 21 + Clojure CLI); the runtime and
host-proof are the in-repo `./pnix-clj` and `./clj-meta` trees. Run from the
**repo root**:

### Three independent runners

Clojure-hosted Clojure extensions — each can later carry its own `deps.edn` and
be started as a network/nREPL server:

```sh
nix run .#pnix-clj-pnix -- -e '1 + 2'   # the pnix language lane (→ 3)
nix run .#pnix-clj-pnix -- file.px       # evaluate a .px file
nix run .#pnix-clj-pnix                  # eval ./default.px if present, else REPL
nix run .#pnix-clj-clj                   # pnix-clj's Clojure host REPL
nix run .#clj-meta -- -e '(+ 1 2)'       # clj-meta meta-circular Clojure (→ 3)

# server seams (editors / tooling)
nix run .#pnix-clj-pnix-server           # pnix network REPL   (socket, port 7888)
nix run .#pnix-clj-nrepl                 # Clojure-host nREPL  (port 7888)
nix run .#pnix-clj-pnix-nrepl            # pnix-language nREPL, eval via pnix (7890)
nix run .#clj-meta-nrepl                 # clj-meta nREPL, eval via clj-meta backend (7889)
```

`pnix-clj-pnix` follows Nix's `default.nix` convention: a bare invocation in a
directory evaluates `./default.px` (then `./default.nix`) if present.

### Gates / reports

```sh
nix run .#gate                 # full test gate (164 tests / 3250 assertions)
nix run .#tower                # self-hosting 4-substrate tower report
nix run .#capabilities-check   # machine-generated capability index drift gate
nix run .#clj-meta-gate        # clj-meta's own meta-circular self-host gate
nix run .#examples             # run every examples/*/pnix_clj_way.clj
nix run .#safe-eval            # pure resource-bounded sandbox report
nix run .#specialize           # Futamura specialize report

nix develop                    # JDK + clojure on PATH, then:
  cd pnix-clj && clojure -M:test
  cd pnix-clj && clojure -M examples/11-self-hosting-convergence/pnix_clj_way.clj
```

Directly (no Nix), the runners map to deps.edn aliases:
`cd pnix-clj && clojure -M:repl-pnix -e '1 + 2'`,
`cd clj-meta && clojure -M:repl -e '(+ 1 2)'`.

Without Nix, from `./pnix-clj` with a JDK + [Clojure CLI](https://clojure.org/guides/install_clojure):

```sh
cd pnix-clj
clojure -M:test
clojure -M examples/01-pure-sandbox/pnix_clj_way.clj
```

## Examples

`pnix-clj/examples/` shows **plain-Clojure limits vs the pnix-clj way**, one
pair per meta-circular pillar (Korean-commented, each runnable + self-asserting):

- `01-pure-sandbox` — `safe-eval`: purity static-check + fuel bounds
- `03-specialization-futamura` — `specialize`: residual source + JVM-bytecode projection
- `04-host-interop-loss-effect` — host crossing loss/effect/capability witness
- `05-witness-and-gate` — held/ok verdicts and witness hashes
- `06-ast-lowering-roundtrip` — parser/lowering/clj-meta structural receipt
- `07-clojure-macro-over-pnix` — macroexpand result projected to pnix and tower-verified
- `08-clojure-reader-or-edn-embed-pnix` — EDN tagged pnix source verified by parse/purity/tower witness
- `11-self-hosting-convergence` — `run-tower`: one source, four substrates, one value
- `12-content-addressed-cache` — `cached-eval`: canonical content-addressed key
- `19-lowered-compiled-runtime` — direct evaluator vs lowered compiled runtime equivalence
- `23-capability-gate` — host-effect requirements become purity/capability gate verdicts
- `24-phase-separation` — parse/purity/eval/lowering/compile/gate verdict separation
- `25-typed-attestation` — capability crossing verdicts carry typed interop witness hashes
- `30-verifying-cache` — cache hit verified against fresh evaluation, purity, and content key
- `31-compartment-isolation` — host objects isolated as opaque refs with release/held boundaries
- `33-futamura-ladder` — 1st/2nd Futamura projections verified, 3rd stated-not-built
- `35-stage-tower-internals` — tower layers, adjacent pairs, collapse witness, held blocking point

See [`pnix-clj/examples/README.md`](pnix-clj/examples/README.md) for the index.


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

# builtins.fetchTarball { url = "https://www.svp-team.com/files/svp4-latest.php?mac"; sha256 = "04phzhyw0haiz77j494s1rz0as5yg70gb33i864riylfj776h27v"; }
>> error: {:schema :pnix.machine.eval-error-model.v1, :phase :eval, :class :type-error, :evidence {:url "{:kind :thunk, :label [:attr \"url\"], :state #object[clojure.lang.Atom 0x418911d3 {:status :ready, :val {:phase :pending}}], :compute #object[pnix_clj.evaluator$eval_attrs$fn__914 0x4f4ef792 \"pnix_clj.evaluator$eval_attrs$fn__914@4f4ef792\"]}"}}

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

