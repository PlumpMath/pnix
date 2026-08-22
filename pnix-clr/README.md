# pnix-clr

`pnix-clr`는 PNIX의 ClojureCLR/.NET 호스트다.

```text
pinned ClojureCLR bootstrap trust root
              |                         pnix-clr/runtime-artifact.edn
              v                                      |
      clr-meta evaluator + generic AOT builder <-----+
              |  exact hash-bound 8-DLL artifact
              v
          pnix-clr runtime
```

두 레이어는 의도적으로 분리된다. `clr-meta`는 PNIX 비의존 호스트 기계다.
제품 소유 `pnix-clr/runtime-artifact.edn` 플랜을 받아 정확한 소스 클로저를 검증하고
선언된 CLR AOT 아티팩트를 만든다; PNIX 제품 네임스페이스를 하드코딩하지 않는다.
`pnix-clr`는 그 아티팩트를 검증·로드한다. `clr-meta`의 selfhost compiler는
Stage15/N과 scoped fixed point까지 닫혔지만, 현재 제품 backend는 별도
`host-clojureclr-aot`다. 직접 compiler acceleration과 common-compiler 배선은
후속 작업이다. pin된 CLR substrate는 업스트림 `Clojure` NuGet 패키지
(`1.12.3-alpha8`)이며 `clr-bootstrap/`에서 `bin/build-clr`가 게시한다.
업스트림 컴파일러 소스는 여기에 벤더하지 않는다.

### 이중 축 + 라이브러리 (필독)

정본: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md). 에이전트 노트: [`CLAUDE.md`](CLAUDE.md).  
C# 패키지: [`csharp/Pnix.Clr/README.md`](csharp/Pnix.Clr/README.md).  
**`clojure-clr` 허용 표면 (fail-closed) + TFM 정책:** [`pnix-clr/docs/IMPLEMENTATION.md`](pnix-clr/docs/IMPLEMENTATION.md) §6, §7.  
**멀티-ns bootstrap 템플릿:** [`examples/clojure-clr-project/`](examples/clojure-clr-project/).  
**프로파일 스모크:** `./bin/clojure-clr-profiles-smoke` (tool-eval + multi-ns).

| 축 | 명령 / 표면 |
|------|-------------------|
| **host-main (C#)** | `export-pnix-clr-library` 이후 `Pnix.Clr.Eval`; MSBuild props |
| **host-main (CLR)** | `bin/clojure-clr` / flake `clojure-clr` (focused `-e` / 단일 파일) |
| **pnix-main** | `./bin/pnix-clr --repl` / `nix run .#pnix-clr-pnix` |
| **library** | `./bin/export-pnix-clr-library` → guest AOT + managed DLL + props |
| **C#에서 `.px` 임포트** | `Eval.File("x.px")` / `Eval.Source("1+2")` |

Env 계약: `PNIX_CLR`, `PNIX_CLR_ROOT`, `PNIX_CLR_ARTIFACT` (레거시 별칭
`PNIX_CLR_RUNTIME_ARTIFACT`), `PNIX_CLR_LIBRARY`. Guest `*.clj.dll`은
**ClojureCLR 바인딩** — 이식 가능한 멀티호스트 `.px` 패키지 아님.

```sh
./bin/build-pnix-clr-artifact
./bin/export-pnix-clr-library
# → pnix-clr/target/pnix-clr-library/{lib,build,share}
nix run .#pnix-clr-library
nix run .#pnix-clr-refs
```

## Bootstrap

직접 런타임 스크립트 요구사항: .NET SDK 10과 `jq`. aggregate 게이트는
추가로 common outcome 계약을 `nix eval`로 소비한다. Nix 러너가 셋 모두 제공한다.

```sh
./bin/build-clr
./bin/clr-meta -e '(+ 20 22)'
./bin/build-pnix-clr-artifact
./bin/pnix-clr -e 'true && !false'
./bin/pnix-clr -e '(-7) * (-6)'
./bin/pnix-clr-gate
```

첫 빌드는 중앙 pin NuGet 의존을 restore하고 framework-dependent `net10.0`
Clojure.Main 타겟과 런타임 어셈블리만 게시한다. 전체 업스트림 솔루션은 bootstrap
게이트가 아니다: .NET Framework, net11, Unix/Mono 빌드 레인도 포함되어 모든
개발 호스트에서 가능하지 않다.

`clr-meta -e`와 파일 모드는 reader evaluation 비활성, 비활성 tagged reader,
EOF 검사, 재귀 portable-value-domain 검사 후 physical evaluator generation 2로
정확히 하나의 focused Clojure 폼을 파싱·실행한다. map/set/regex, tagged/conditional
reader 값, trailing form, 허용 도메인 밖 값은 평가 전 실패한다. 툴 경로는
`load-string`을 쓰지 않는다. evaluator generation 0, 1, 2는 작은 중첩
self-interpretation 레인이며 컴파일러 스테이지가 아니다. 그 중첩 인터프리터를
15회 self-extension으로 늘리려는 시도는 CLR 스택을 소진한다. 따라서 generation
수나 그 실험은 컴파일러 Stage15/N 증거가 아니다.

별도로 `clr-meta`에는 두 compiler family가 있다. 좁고 frozen된 checked-Int64
Compiler Stage1과, macro-free 정규 compiler kernel을 같은 언어로 재컴파일하는
selfhost family다. 후자는 C0/C1 admission, 실행 가능한 Stage1/2, Stage3--7
same-source chain, Stage8 재현 artifact, Stage9 clean-process replay,
Stage10--15/N과 StageN closure를 live gate로 닫았다. Stage1--7 assembly output의
scoped self-reproduction/fixed point도 별도 receipt로 닫혔다. 현재 정본은
[`clr-meta/STATUS.md`](clr-meta/STATUS.md)다.

이 증거는 general CLR IL fixed point나 ClojureCLR 전체 대체가 아니다. 모든
stage receipt는 `promotion/allowed?=false`이고, `pnix-clr` 제품 아티팩트는 여전히
별도 선언 `host-clojureclr-aot` backend를 쓰며 selfhost StageN compiler를 직접
소비하지 않는다.

아티팩트 빌드는 정확한 8-namespace 제품 플랜을 소비하고 8개의 `.clj.dll`과
`manifest.json`을 낸다. 매니페스트는 `host-clojureclr-aot` 백엔드, `net10.0` 타겟,
entry namespace, plan digest, ordered source/output rows, 두 closure digests를 기록한다.
모든 제품 기동에서 `bin/pnix-clr`는 이 정체성을 live plan, source tree, artifact
bytes에 대해 검사하고, exact manifest keys와 artifact tree를 요구하며, pinned
runtime lookup roots의 제품 네임스페이스 shadow를 거부하고, cwd를 검증된 아티팩트로
바꾼 뒤 `CLOJURE_LOAD_PATH`를 그 디렉터리로 교체한다. 그다음 AOT 제품 entry만
로드하며 없는 아티팩트를 빌드하거나 제품 소스로 fallback하지 않는다. pin된
`Clojure.Main.dll`이 런타임 substrate로 남는다.
`bin/clojure-clr`는 generation-2 툴을 통해 `-e`와 단일 파일만 받는 focused 호환
파사드다; `bin/clojure-clr-bootstrap`는 명시적 업스트림 컴파일러/런타임 명령을 이름 붙인다.
파사드는 여전히 그 substrate 위에서 돌며 광범위 대체 주장이 아니다.

flake는 pin된 .NET SDK와 source-closure 러너를 공급한다. 이 디렉터리에서는 live
checkout을 재사용하고, 다른 곳에서는 체크인 CLR 소스와 정규 seed 파일들의 쓰기 가능
content-keyed 캐시를 materialize한 뒤 빌드한다. NuGet restore는 중앙 버전 pin이지만
아직 lock-file 기반 완전 hermetic Nix 패키지는 아니다.

```sh
# 새로 만든 파일이 추적되기 전:
./bin/pnix-clr-gate

# 추적된 monorepo Git 트리에서:
nix flake check --no-build .
nix run .#gate
```

기대 fixture는 JSON 텍스트 레코드다. CLR 러너는 정규 JSON 비교 전 말단 LF/CRLF
레코드 구분자 최대 하나를 제거한다; 그 외 leading/trailing 바이트는 모두 유의미하다.

## 정직한 경계

이 레인은 코드가 .NET 위 ClojureCLR로 실행되고, dead `if` 분기·미사용 인자·미선택
attr 필드의 missing import가 resolve되지 않으며, application이 static attr-path `?`보다
강하게 묶이고, checked signed-I64 산술이 lazy overflow 회피와 함께 재현됨을 증명한다.
추가로 운영 아티팩트 의존을 증명한다: generic `clr-meta` 빌더가 plan-bound eight-DLL
아티팩트를 만들고, 제품 러너는 missing/stale/malformed/extra source/output 상태를
source/bootstrap 빌드 fallback 없이 거부한다.

checked 산술 경계는 호스트 수치 API보다 의도적으로 좁다: operand는 허용된 PNIX
소스 lexer/evaluator 경로에서 나와 `System.Int64`로 남는다. 이 증거는 임의
ClojureCLR 정수용 ABI 경계를 세우지 않는다. 아직 주장하지 않는 것:

- 완전한 mature JVM-host 언어 또는 연구 표면;
- 일반 CLR IL fixed point, host promotion, 또는 광범위 ClojureCLR 대체.
  `clr-meta` Compiler Stage1–N + self-reproduction 게이트는
  `promotion/allowed?=false`로 닫혀 있다(`clr-meta/STATUS.md`) — 제품 러너가
  그 사다리를 컴파일러로 소비하지는 않는다;
- 독립 빌드 간 byte-identical raw AOT 재현;
- 광범위 ClojureCLR 명령/언어/런타임/생태계 호환 또는 ClojureCLR 대체;
- 단독 source-free 배포 (기동 검증이 현재 live plan과 source closure에 묶이고
  실행은 pin된 런타임을 유지);
- PNIX common compiler/PIR 통합;
- 전체 common conformance corpus;
- dynamic (`${...}`) attr paths;
- BigInt 산술 또는 일반 수치 promotion;
- `pnix.primitive-abi.v1` 매니페스트 경유 라우팅/강제;
- production-evaluator 또는 full-builtin primitive-manifest 강제;
- production `Requested` / `Suspended` 평가기 통합;
- 미래 canonical-result/JCS wire 완성;
- 다른 pnix 호스트와의 행동 패리티 주장.

미지원 PNIX 구성은 structured `Failed`로 fail-closed.
`Done`, `Failed`, `Requested`, `Suspended`는 common host-outcome 스키마를 구현하는
명목 ClojureCLR 타입이며, 이 평가기 슬라이스에 통합된 것은 `Done`과 `Failed`뿐이다.
guest attrset이 이를 위조할 수 없다. JVM 평가기 fallback은 없다.

`clr-meta` Stage 설계 기록은 `clr-meta/STAGE15_N_ROADMAP.md`, 게이트 정본은
`clr-meta/STATUS.md`다. 남은 열린 주장은 host promotion / broad ClojureCLR
대체 / 일반 IL fixed point다.

## 레이아웃

```text
clojure-clr-clojure-1.12.3-alpha8/  NuGet-restored ClojureCLR publish 출력
clr-meta/                            PNIX-agnostic CLR meta bootstrap
pnix-clr/                            CLR-native PNIX host + artifact plan
bin/                                 build, runners, focused gate
```

복제된 JVM/domain 트리는 CLR 포트로 텍스트 재라벨하지 않고 가지치기했다.
여기에는 CLR 소유 메커니즘만 속한다.


## 실행 테스트해봄.
https://teu5us.github.io/nix-lib.html

builtins.zipAttrsWith (name: values: { inherit name values; }) [ { a = "x"; } { a = "y"; b = "z"; } ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"a":{"name":"a","values":["x","y"]},"b":{"name":"b","values":["z"]}}}

builtins.typeOf 1
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"int"}

builtins.typeOf true
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"bool"}

builtins.typeOf "hello"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"string"}

## 사용방법 모르는것 >> "path" 가 나오는방법은?
builtins.typeOf null
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"null"}

builtins.typeOf {a=1;}
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"set"}

builtins.typeOf [ 1 2 "a" ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"list"}

builtins.typeOf (arg: 1+arg)
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"lambda"}

builtins.typeOf 1.2
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"float"}

builtins.tryEval (1 + 2)
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"success":true,"value":3}}

builtins.trace "여기까지 실행됨" 42
>> trace: 여기까지 실행됨
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":42}

## 안되는것/사용방법?: builtins.toXML

builtins.toString [ "foo" "bar" ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"foo bar"}

builtins.toJSON { a = 1; b = true; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"{\"a\":1,\"b\":true}"}

builtins.toFile "hello.txt" "안녕하세요"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"/var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/pnix-nix-store/2c68318e352971113645cbc72861e1ec23f48d5baa5f9b405fed9dddca893eb4-hello.txt"}

# builtins.throw "에러 발생!"
>> {"error":{"class":"throw","evidence":{"message":"에러 발생!"},"phase":"eval"},"host":"pnix-clr","outcome_kind":"failed","schema":"pnix-clr.cli-result.v1"}

builtins.substring 0 3 "abcdef"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"abc"}

# builtins.readFile "${builtins.getEnv "HOME"}/hello.txt"
builtins.readFile "/path/to/hello.txt"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"helo\n"}

builtins.readDir "/path/to/pnix/pnix-clr"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"CLAUDE.md":"regular","README.md":"regular","bin":"directory","clr-bootstrap":"directory","clr-meta":"directory","flake.lock":"regular","flake.nix":"regular","pnix-clr":"directory"}}

# builtins.pathExists "${builtins.getEnv "HOME"}/hello.txt"
>> error

builtins.pathExists "/path/to/hello.txt"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":true}

builtins.fetchurl "https://bootstrap.pypa.io/get-pip.py"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"/var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/pnix-nix-store/fb24e693bab954209a063d90953621412ccad4a500905a726286e038f508ddf6-source"}

# builtins.fetchTarball { url = "https://www.svp-team.com/files/svp4-mac.4.5.210-4.dmg"; sha256 = "04phzhyw0haiz77j494s1rz0as5yg70gb33i864riylfj776h27v"; }
>> {"error":{"class":"type-error","evidence":{"message":"structured pnix-clr failure","operation":"fetchurl","reason":"download-failed","url":"https://www.svp-team.com/files/svp4-mac.4.5.210-4.dmg"},"phase":"eval"},"host":"pnix-clr","outcome_kind":"failed","schema":"pnix-clr.cli-result.v1"}

builtins.fetchGit { url = "https://github.com/NixOS/nixpkgs.git"; rev = "abcdef1234567890"; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"outPath":"/var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/pnix-nix-store/f81c9901138e5afe20de6b3a38b328609a505a51d0504c8ccd5d3012b607a8b2-git-src","rev":"abcdef1234567890","shortRev":"abcdef1","url":"https://github.com/NixOS/nixpkgs.git"}}

builtins.attrNames { a = 1; b = 2; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":["a","b"]}

builtins.attrValues { a = 1; b = 2; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[1,2]}

builtins.hasAttr "a" { a = 1; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":true}

builtins.getAttr "a" { a = 1; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":1}

builtins.getAttrFromPath [ "foo" "bar" ] { foo = { bar = 42; }; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":42}

builtins.mapAttrs (name: value: value + 1) { a = 1; b = 2; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"a":2,"b":3}}

builtins.filterAttrs (name: value: value > 1) { a = 1; b = 2; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"b":2}}

builtins.listToAttrs [ { name="a"; value=1; }  { name="b"; value=2; } ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"a":1,"b":2}}

builtins.length [1 2 3]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":3}

builtins.head [ "a" "b" "c" ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"a"}

builtins.tail [ "a" "b" "c" ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":["b","c"]}

builtins.last [ "a" "b" "c" ] 
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"c"}

builtins.init [ "a" "b" "c" ] 
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":["a","b"]}

builtins.elem "b" [ "a" "b" "c" ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":true}

builtins.concatLists [ [1 2] [3 4] ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[1,2,3,4]}

builtins.flatten [ [1 2] [3 [4 5]] ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[1,2,3,4,5]}

builtins.concatStringsSep ", " [ "a" "b" "c" ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"a, b, c"}

builtins.concatMapStringsSep "-" (x: toString x) [1 2 3]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"1-2-3"}

builtins.removePrefix "foo" "foobar"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"bar"}

builtins.removeSuffix ".txt" "hello.txt" 
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"hello"}

builtins.hasPrefix "foo" "foobar"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":true}

builtins.hasSuffix ".txt" "hello.txt" 
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":true}

builtins.splitString ":" "a:b:c"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":["a","b","c"]}

builtins.toLower "Hello"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"hello"}

builtins.toUpper "Hello"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"HELLO"}

builtins.boolToString true
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"true"}

lib.implies true false
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":false}

builtins.optional true "foo"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":["foo"]}

builtins.optionals false [1 2 3] 
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[]}

lib.optionalAttrs true { a = 1; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"a":1}}

lib.when false "foo"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":null}

builtins.id 42
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":42}

lib.const "foo" "bar"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"foo"}

builtins.flip (a: b: a - b) 3 10
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":7}

builtins.pipe 2 [ (x: x + 3) (x: x * 2) ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":10}

builtins.foldl (acc: x: acc + x) 0 [1 2 3]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":6}

builtins.foldr (x: acc: x + acc) 0 [1 2 3]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":6}

lib.fix (self: { a = 1; b = self.a + 1; })
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"a":1,"b":2}}

builtins.min 3 7
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":3}

builtins.max 3 7
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":7}

builtins.range 1 5
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[1,2,3,4,5]}

builtins.genList (x: x * 2) 4
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[0,2,4,6]}

lib.sum [1 2 3 4]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":10}

lib.product [1 2 3 4] 
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":24}

builtins.recursiveUpdate { a = { b = 1; }; } { a = { c = 2; }; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"a":{"b":1,"c":2}}}

lib.updateManyAttrs [ { a = 1; } { b = 2; } ] { c = 3; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"a":1,"b":2,"c":3}}

builtins.attrByPath [ "foo" "bar" ] 0 { foo.bar = 42; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":42}

lib.attrsets.isAttrs { a = 1; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":true}

builtins.mapAttrsToList (n: v: "${n}=${toString v}") { a = 1; b = 2; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":["a=1","b=2"]}

builtins.zipAttrs [ { a = 1; } { a = 2; b = 3; } ]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"a":[1,2],"b":[3]}}

lib.getName { name = "hello-1.0"; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"hello"}

lib.getVersion { version = "1.0"; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"1.0"}

lib.getAttrFromPathOr { meta = { description = "테스트"; }; } [ "meta" "description" ] "없음"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"테스트"}

builtins.hasAttrByPath [ "meta" "license" ] { meta.license = "MIT"; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":true}

lib.filterAttrsRecursive (name: value: name == "license") { meta = { license = "MIT"; }; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{}}

lib.mapAttrsRecursive (path: value: toString value) { a = { b = 1; }; }
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"a":{"b":"1"}}}

builtins.unique [1 2 2 3 1]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[1,2,3]}

lib.intersectLists [1 2 3] [2 3 4]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[2,3]}

lib.subtractLists [1 2 3] [2]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[1,3]}

builtins.concatMap (x: [x x]) [1 2 3]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[1,1,2,2,3,3]}

builtins.partition (x: x > 2) [1 2 3 4]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":{"right":[3,4],"wrong":[1,2]}}

lib.zipLists [1 2] ["a" "b"]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":[{"fst":1,"snd":"a"},{"fst":2,"snd":"b"}]}

builtins.zipListsWith (a: b: "${toString a}${b}") [1 2] ["a" "b"]
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":["1a","2b"]}

builtins.warn "경고: deprecated 함수 사용" "foo"
>> evaluation warning: 경고: deprecated 함수 사용
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"foo"}

# lib.assert (1 + 1 == 2) "수학이 잘못됨!"
>> {"error":{"class":"syntax-error","evidence":{"actual":"assert","expected":"ident","offset":4,"reason":"unexpected-token"},"phase":"parse"},"host":"pnix-clr","outcome_kind":"failed","schema":"pnix-clr.cli-result.v1"}

# builtins.assert (1 + 1 == 2) "수학이 잘못됨!"
>> {"error":{"class":"syntax-error","evidence":{"actual":"assert","expected":"ident","offset":9,"reason":"unexpected-token"},"phase":"parse"},"host":"pnix-clr","outcome_kind":"failed","schema":"pnix-clr.cli-result.v1"}

