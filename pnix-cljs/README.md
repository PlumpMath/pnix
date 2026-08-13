# pnix-cljs

`pnix-cljs` adds a genuine ClojureScript/JavaScript host to PNIX. It is not a
textual rename of the JVM host.

```text
PNIX source
    |
    v
pnix-cljs parser/evaluator (ClojureScript)
    |
    +-- nominal Done / Failed
    +-- Node CLI
    +-- CommonJS module

ClojureScript source
    |
    v
cljs-meta (cljs.js self-host substrate)
```

### Dual-axis + library (read this)

Canonical: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md). Agent notes: [`CLAUDE.md`](CLAUDE.md).

| Axis | Command / surface |
|------|-------------------|
| **host-main** | bare `clojurescript` → `pnix-cljs`; Node with `NODE_PATH` → `share/pnix-cljs` |
| **pnix-main** | `nix run .#pnix-cljs-pnix` / `pnix-cljs --repl` |
| **library** | flake package `$out/share/pnix-cljs` — **host-bound** JS, not portable `.px` |
| **import `.px` from Node** | `require('@plumpmath/pnix-cljs')` → `evalFile*` — see [HOST_IMPORT.md](HOST_IMPORT.md) |

`shadow-cljs` may remain the **build orchestrator**; the default **runtime** host
is `pnix-cljs`, not a portable multi-host bytecode package.

## Build

The checked-in `clojurescript-r1.12.145` tree is the development compiler
source. Maven dependencies are resolved by Clojure CLI during regeneration;
the Nix product package contains only the resulting JavaScript and Node.

```sh
./bin/build-cljs
./bin/pnix-cljs-gate --rebuild
nix flake check path:. --no-write-lock-file
```

Generated artifacts:

```text
cljs-meta/dist/cljs-meta.js
cljs-meta/dist/cljs-meta-module.js
cljs-meta/dist/fixed-point/cljs-meta-fixed.js
cljs-meta/dist/fixed-point/receipt.json
pnix-cljs/dist/pnix-cljs.js
pnix-cljs/dist/pnix-cljs-module.js
pnix-cljs/dist/pnix-cljs-self-test.js
```

## Run

```sh
node pnix-cljs/dist/pnix-cljs.js -e 'let double = x: x * 2; in double 21'
node cljs-meta/dist/cljs-meta.js -e '(+ 20 22)'
node cljs-meta/dist/fixed-point/cljs-meta-fixed-cli.js -e '(+ 20 22)'
```

JavaScript:

```js
const pnix = require("./pnix-cljs/dist/pnix-cljs-module.js");

console.log(pnix.evalSource("20 + 22"));
console.log(pnix.evalValue("{ answer = 42; }.answer"));
```

Nix exposes these applications after the JavaScript artifacts are present:

```sh
nix run .#pnix-cljs -- -e '20 + 22'
nix run .#pnix-cljs-cljs -- -e '(+ 20 22)'
```

## Seed surface

The first executable surface supports integers in JavaScript's safe integer
range, booleans, null, strings, `let`, lambdas/application, `if`, attrsets,
recursive attrsets, selection, boolean operations, comparison, and checked
integer arithmetic.

Effect execution, retained continuation support, complete Nix numeric
semantics, and established-host equivalence are explicit future work. No
fallback evaluator is used when this seed encounters unsupported syntax.

## 실행 테스트해봄.
https://teu5us.github.io/nix-lib.html

builtins.zipAttrsWith (name: values: { inherit name values; }) [ { a = "x"; } { a = "y"; b = "z"; } ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"a":{"name":"a","values":["x","y"]},"b":{"name":"b","values":["z"]}}}

builtins.typeOf 1
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"int"}
builtins.typeOf true
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"bool"}
builtins.typeOf "hello"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"string"}
## 사용방법 모르는것 >> "path" 가 나오는방법은?
builtins.typeOf null
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"null"}
builtins.typeOf {a=1;}
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"set"}
builtins.typeOf [ 1 2 "a" ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"list"}
builtins.typeOf (arg: 1+arg)
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"lambda"}
builtins.typeOf 1.2
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"float"}

builtins.tryEval (1 + 2)
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"success":true,"value":3}}

builtins.trace "여기까지 실행됨" 42
>> trace: 여기까지 실행됨
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":42}

builtins.toXML {a=1;}
>> "<?xml version='1.0' encoding='utf-8'?>\n<expr><attrs><attr name=\"a\"><int>1</int></attr></attrs></expr>\n"

builtins.toString [ "foo" "bar" ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"foo bar"}

builtins.toJSON { a = 1; b = true; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"{\"a\":1,\"b\":true}"}

builtins.toFile "hello.txt" "안녕하세요"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"/var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/pnix-tofile-0xYo7I/hello.txt"}

# builtins.throw "에러 발생!"
>> {"error":{"class":"explicit-throw","evidence":{},"phase":"eval"},"outcome_kind":"failed","schema":"pnix.machine.host-outcome.v1"}

builtins.substring 0 3 "abcdef"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"abc"}

builtins.readFile "/path/to/hello.txt"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"helo\n"}

builtins.readDir "/path/to/pnix-cljs"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"CLAUDE.md":"regular","README.md":"regular","bin":"directory","cljs-meta":"directory","deps-lock.json":"regular","flake.nix":"regular","package.json":"regular","pnix-cljs":"directory","scripts":"directory"}}

builtins.pathExists "/path/to/hello.txt"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":true}

builtins.fetchurl "https://bootstrap.pypa.io/get-pip.py"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"/var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/pnix-fetch-1786258190619-0.bz0my7lf4hv.out"}

builtins.fetchTarball { url = "https://www.svp-team.com/files/svp4-latest.php?mac"; sha256 = "04phzhyw0haiz77j494s1rz0as5yg70gb33i864riylfj776h27v"; }
>> "/var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/pnix-fetch-1786371799185-0.gpz7glkazot.out"

builtins.fetchGit { url = "https://github.com/NixOS/nixpkgs.git"; rev = "abcdef1234567890"; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"narHash":"","outPath":"/var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/pnix-fetch-1786258245010-0.yr0e605y8vl.out","rev":"abcdef1234567890","revCount":0,"shortRev":"abcdef1","submodules":false}}

builtins.attrNames { a = 1; b = 2; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":["a","b"]}

builtins.attrValues { a = 1; b = 2; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":[1,2]}

builtins.hasAttr "a" { a = 1; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":true}

builtins.getAttr "a" { a = 1; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":1}

builtins.getAttrFromPath [ "foo" "bar" ] { foo = { bar = 42; }; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":42}

builtins.mapAttrs (name: value: value + 1) { a = 1; b = 2; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"a":2,"b":3}}

builtins.filterAttrs (name: value: value > 1) { a = 1; b = 2; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"b":2}}

builtins.listToAttrs [ { name="a"; value=1; }  { name="b"; value=2; } ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"a":1,"b":2}}

builtins.length [1 2 3]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":3}

builtins.head [ "a" "b" "c" ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"a"}

builtins.tail [ "a" "b" "c" ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":["b","c"]}

builtins.last [ "a" "b" "c" ] 
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"c"}

builtins.init [ "a" "b" "c" ] 
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":["a","b"]}

builtins.elem "b" [ "a" "b" "c" ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":true}

builtins.concatLists [ [1 2] [3 4] ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":[1,2,3,4]}

builtins.flatten [ [1 2] [3 [4 5]] ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":[1,2,3,4,5]}

builtins.concatStringsSep ", " [ "a" "b" "c" ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"a, b, c"}

builtins.concatMapStringsSep "-" (x: toString x) [1 2 3]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"1-2-3"}

builtins.removePrefix "foo" "foobar"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"bar"}

builtins.removeSuffix ".txt" "hello.txt" 
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"hello"}

builtins.hasPrefix "foo" "foobar"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":true}

builtins.hasSuffix ".txt" "hello.txt" 
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":true}

builtins.splitString ":" "a:b:c"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":["a","b","c"]}

builtins.toLower "Hello"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"hello"}

builtins.toUpper "Hello"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"HELLO"}

builtins.boolToString true
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"true"}

lib.implies true false
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":false}

builtins.optional true "foo"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":["foo"]}

builtins.optionals false [1 2 3] 
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":[]}

lib.optionalAttrs true { a = 1; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"a":1}}

lib.when false "foo"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":null}

builtins.id 42
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":42}

lib.const "foo" "bar"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"foo"}

builtins.flip (a: b: a - b) 3 10
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":7}

builtins.pipe 2 [ (x: x + 3) (x: x * 2) ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":10}

builtins.foldl (acc: x: acc + x) 0 [1 2 3]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":6}

builtins.foldr (x: acc: x + acc) 0 [1 2 3]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":6}

lib.fix (self: { a = 1; b = self.a + 1; })
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"a":1,"b":2}}

builtins.min 3 7
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":3}

builtins.max 3 7
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":7}

builtins.range 1 5
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":[1,2,3,4,5]}

builtins.genList (x: x * 2) 4
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":[0,2,4,6]}

lib.sum [1 2 3 4]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":10}

lib.product [1 2 3 4] 
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":24}

builtins.recursiveUpdate { a = { b = 1; }; } { a = { c = 2; }; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"a":{"b":1,"c":2}}}

lib.updateManyAttrs [ { a = 1; } { b = 2; } ] { c = 3; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"a":1,"b":2,"c":3}}

builtins.attrByPath [ "foo" "bar" ] 0 { foo.bar = 42; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":42}

lib.attrsets.isAttrs { a = 1; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":true}

lib.attrsets.mapAttrsToList (n: v: "${n}=${toString v}") { a = 1; b = 2; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":["a=1","b=2"]}

lib.attrsets.zipAttrs [ { a = 1; } { a = 2; b = 3; } ]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"a":[1,2],"b":[3]}}

lib.getName { name = "hello-1.0"; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"hello"}

lib.getVersion { version = "1.0"; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"1.0"}

lib.getAttrFromPathOr { meta = { description = "테스트"; }; } [ "meta" "description" ] "없음"
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"테스트"}

builtins.hasAttrByPath [ "meta" "license" ] { meta.license = "MIT"; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":true}

lib.filterAttrsRecursive (name: value: name == "license") { meta = { license = "MIT"; }; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{}}

lib.mapAttrsRecursive (path: value: toString value) { a = { b = 1; }; }
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"a":{"b":"1"}}}

builtins.unique [1 2 2 3 1]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":[1,2,3]}

lib.intersectLists [1 2 3] [2 3 4]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":[2,3]}

lib.subtractLists [1 2 3] [2]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":[1,3]}

builtins.concatMap (x: [x x]) [1 2 3]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":[1,1,2,2,3,3]}

builtins.partition (x: x > 2) [1 2 3 4]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":{"right":[3,4],"wrong":[1,2]}}

lib.zipLists [1 2] ["a" "b"]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":[{"fst":1,"snd":"a"},{"fst":2,"snd":"b"}]}

builtins.zipListsWith (a: b: "${toString a}${b}") [1 2] ["a" "b"]
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":["1a","2b"]}

builtins.warn "경고: deprecated 함수 사용" "foo"
>> warning: 경고: deprecated 함수 사용
>> {"outcome_kind":"done","schema":"pnix.machine.host-outcome.v1","value":"foo"}

# lib.assert (1 + 1 == 2) "수학이 잘못됨!"
>> {"error":{"class":"syntax-error","evidence":{"detail_class":"invalid-selection-name","offset":4.0,"token":"assert"},"phase":"parse"},"outcome_kind":"failed","schema":"pnix.machine.host-outcome.v1"}

# builtins.assert (1 + 1 == 2) "수학이 잘못됨!"
>> {"error":{"class":"syntax-error","evidence":{"detail_class":"invalid-selection-name","offset":9.0,"token":"assert"},"phase":"parse"},"outcome_kind":"failed","schema":"pnix.machine.host-outcome.v1"}


