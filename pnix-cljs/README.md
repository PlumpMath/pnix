# pnix-cljs

`pnix-cljs`는 PNIX에 진짜 ClojureScript/JavaScript 호스트를 추가한다.
JVM 호스트의 텍스트 rename이 아니다.

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

### 이중 축 + 라이브러리 (필독)

정본: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md). 에이전트 노트: [`AGENTS.md`](AGENTS.md).

| 축 | 명령 / 표면 |
|------|-------------------|
| **host-main** | bare `clojurescript` → `pnix-cljs`; `NODE_PATH` 있는 Node → `share/pnix-cljs` |
| **pnix-main** | `nix run .#pnix-cljs-pnix` / `pnix-cljs --repl` |
| **library** | flake 패키지 `$out/share/pnix-cljs` — **호스트 바인딩** JS, 이식 `.px` 아님 |
| **Node에서 `.px` 임포트** | `require('@plumpmath/pnix-cljs')` → `evalFile*` — [pnix-cljs/docs/IMPLEMENTATION.md](pnix-cljs/docs/IMPLEMENTATION.md) §3 |

`shadow-cljs`는 **빌드 오케스트레이터**로 남을 수 있다. 기본 **런타임** 호스트는
`pnix-cljs`이며 이식 가능한 멀티호스트 바이트코드 패키지가 아니다.

## 빌드

체크인된 `clojurescript-r1.12.145` 트리는 개발 컴파일러 소스다.
Maven 의존은 재생성 시 Clojure CLI가 resolve한다.
Nix 제품 패키지는 결과 JavaScript와 Node만 담는다.

```sh
./bin/build-cljs
./bin/pnix-cljs-gate --rebuild
nix flake check path:. --no-write-lock-file
```

생성 아티팩트:

```text
cljs-meta/dist/cljs-meta.js
cljs-meta/dist/cljs-meta-module.js
cljs-meta/dist/fixed-point/cljs-meta-fixed.js
cljs-meta/dist/fixed-point/receipt.json
pnix-cljs/dist/pnix-cljs.js
pnix-cljs/dist/pnix-cljs-module.js
pnix-cljs/dist/pnix-cljs-self-test.js
```

## 실행

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

JavaScript 아티팩트가 있으면 Nix가 다음 앱을 노출한다:

```sh
nix run .#pnix-cljs -- -e '20 + 22'
nix run .#pnix-cljs-cljs -- -e '(+ 20 22)'
```

## Seed 표면

첫 실행 가능 표면은 JavaScript safe integer 범위 정수, 불리언, null, 문자열,
`let`, 람다/적용, `if`, attrset, 재귀 attrset, selection, 불리언 연산, 비교,
checked integer 산술을 지원한다.

Effect 실행, retained continuation, 완전한 Nix 수치 의미, established-host
등가는 명시적 후속 작업이다. 이 seed가 미지원 구문을 만나도 fallback 평가기는 쓰지 않는다.


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


