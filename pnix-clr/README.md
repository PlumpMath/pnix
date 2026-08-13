# pnix-clr

`pnix-clr` is the experimental ClojureCLR/.NET host for PNIX.

```text
pinned ClojureCLR bootstrap trust root
              |                         pnix-clr/runtime-artifact.edn
              v                                      |
      clr-meta evaluator + generic AOT builder <-----+
              |  exact hash-bound 8-DLL artifact
              v
          pnix-clr runtime
```

The two layers are deliberately separate. `clr-meta` is PNIX-agnostic host
machinery. It accepts the product-owned `pnix-clr/runtime-artifact.edn` plan,
validates its exact source closure, and produces the declared CLR AOT artifact;
it does not hardcode PNIX product namespaces. `pnix-clr` validates and loads
that artifact. Direct compiler acceleration and common-compiler wiring remain
future work. The pinned CLR substrate is the upstream `Clojure` NuGet package
(`1.12.3-alpha8`), published by `bin/build-clr` from `clr-bootstrap/`; no
upstream compiler sources are vendored here.

### Dual-axis + library (read this)

Canonical: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md). Agent notes: [`CLAUDE.md`](CLAUDE.md).  
C# package: [`csharp/Pnix.Clr/README.md`](csharp/Pnix.Clr/README.md).  
**`clojure-clr` admitted surface (fail-closed):** [`docs/CLOJURE_CLR_ADMITTED_SURFACE.md`](docs/CLOJURE_CLR_ADMITTED_SURFACE.md).  
**TFM policy:** [`docs/TFM_POLICY.md`](docs/TFM_POLICY.md).  
**Multi-ns bootstrap template:** [`examples/clojure-clr-project/`](examples/clojure-clr-project/).

| Axis | Command / surface |
|------|-------------------|
| **host-main (C#)** | `Pnix.Clr.Eval` after `export-pnix-clr-library`; MSBuild props |
| **host-main (CLR)** | `bin/clojure-clr` / flake `clojure-clr` (focused `-e` / single file) |
| **pnix-main** | `./bin/pnix-clr --repl` / `nix run .#pnix-clr-pnix` |
| **library** | `./bin/export-pnix-clr-library` → guest AOT + managed DLL + props |
| **import `.px` from C#** | `Eval.File("x.px")` / `Eval.Source("1+2")` |

Env contract: `PNIX_CLR`, `PNIX_CLR_ROOT`, `PNIX_CLR_ARTIFACT` (legacy alias
`PNIX_CLR_RUNTIME_ARTIFACT`), `PNIX_CLR_LIBRARY`. Guest `*.clj.dll` are
**ClojureCLR-bound** — not a portable multi-host `.px` package.

```sh
./bin/build-pnix-clr-artifact
./bin/export-pnix-clr-library
# → pnix-clr/target/pnix-clr-library/{lib,build,share}
nix run .#pnix-clr-library
nix run .#pnix-clr-refs
```

## Bootstrap

Requirements for direct runtime scripts: .NET SDK 10 and `jq`. The aggregate
gate also consumes the common outcome contracts through `nix eval`. The Nix
runners supply all three.

```sh
./bin/build-clr
./bin/clr-meta -e '(+ 20 22)'
./bin/build-pnix-clr-artifact
./bin/pnix-clr -e 'true && !false'
./bin/pnix-clr -e '(-7) * (-6)'
./bin/pnix-clr-gate
```

The first build restores the centrally pinned NuGet dependencies and publishes
only the framework-dependent `net10.0` Clojure.Main target with its runtime
assemblies. The full upstream solution is not the bootstrap gate: it also
contains .NET Framework, net11, and Unix/Mono build lanes not available on
every development host.

`clr-meta -e` and file mode now parse exactly one focused Clojure form with
reader evaluation disabled, inert tagged readers, an EOF check, and a recursive
portable-value-domain check before executing it through physical evaluator
generation 2. Maps, sets, regexes, tagged/conditional reader values, trailing
forms, and values outside the admitted domain fail before evaluation. The tool
path does not use `load-string`. Evaluator generations 0, 1, and 2 are the small nested
self-interpretation lane; they are not compiler stages. A live attempt to
extend that nested interpreter through 15 self-extensions exhausts the CLR
stack. Consequently neither the generation count nor that experiment is
evidence for compiler Stage15/N.

Separately, `clr-meta` now closes a first profile-qualified Compiler Stage1:
exact `System.Int64` literals, dynamic `arg`, and checked binary `+`, `-`, `*`
are lowered by an AOT-seeded ClojureCLR-written compiler directly to a runnable
managed PE. This is not Stage2/self-reproduction and does not widen the
`clojure-clr` compatibility facade.

The route beyond that frozen expression family begins in a separately
versioned `clr-meta` selfhost family. Its C0/C1 gate fixes a macro-free compiler
source language and exact low-level support ABI, then recursively admits the
canonical compiler source against the same language it is required eventually
to compile. C2 now uses the explicit pinned-host B0 boundary to produce a
source-hidden executable Compiler Stage1 PE with 27 prepared methods, a
stack-verified transactional PE sink, and mandatory C1/toolchain closure.
That generated compiler executes fresh same-language targets and propagates the
three frozen mutation anchors. Its C2 manifest remains a historical Stage1
receipt with `compiler_stage2=false`.

C3 builds a distinct override-style child: the admitted Stage1 compiles the
exact same canonical kernel source into a runnable Stage2. The child contains
only `CompilerStage2.dll`, the support triplet, and its own hash-bound manifest;
it excludes the Stage1 PE, C2 manifest, compiler source, and ClojureCLR. A
separate C3 gate then hides the compiler source and parent artifact, creates a
post-Stage2 random nonce target, compiles it with Stage2, and executes it in a
second fresh target/support-only directory. Thus `clr-meta` closes
`compiler_stage2=true` and `stage2_fresh_target_replay=true` at C3. It does not
close Stage3, compiler self-reproduction, a fixed point, raw reproducibility,
Stage15/N, ClojureCLR replacement, PNIX product/compiler integration, or
cross-host canonical equivalence. In particular, the current `pnix-clr`
product artifact still uses the separately declared `host-clojureclr-aot`
backend and does not consume the Stage2 compiler.
The gate accounts for all 2,237 nodes in 37 forms / 36 definitions, binds the
33-call support ABI and twelve lowering owners, and rejects 23 adversarial
admission inputs without publishing a C1 receipt; the separate C2 gate adds 16
structured no-output execution cases and four no-replace publication cases.

The artifact build consumes the exact nine-namespace product plan and emits
nine `.clj.dll` files plus `manifest.json`. The manifest records the
`host-clojureclr-aot` backend, `net10.0` target, entry namespace, plan digest,
ordered source/output rows, and both closure digests. On every product launch,
`bin/pnix-clr` checks those identities against the live plan, source tree, and
artifact bytes, requires the exact manifest keys and artifact tree, rejects
product namespace shadows in the pinned runtime lookup roots, changes cwd to
the verified artifact, and replaces `CLOJURE_LOAD_PATH` with that directory.
It then loads only the AOT product entry and never builds an absent artifact or
falls back to product source. The pinned `Clojure.Main.dll` remains the runtime substrate.
`bin/clojure-clr` is a focused compatibility facade accepting only `-e` and a
single file through the generation-2 tool; `bin/clojure-clr-bootstrap` names
the explicit upstream compiler/runtime command. The facade still runs atop
that substrate and is not a broad replacement claim.

The flake supplies the pinned .NET SDK and source-closure runners. From this
directory they reuse the live checkout; elsewhere they materialize a writable,
content-keyed cache of the checked-in CLR source plus the three canonical
`bool-01` seed files, two focused case/expected pairs for dead imports and
hasAttr/application precedence, the checked-I64 case/expected pair, two error
case/expected pairs, and the two common basic-outcome contract files before
building. NuGet restore is centrally version-pinned but does not yet have a
lock-file-backed, fully hermetic Nix package.

```sh
# Before newly created files are tracked:
./bin/pnix-clr-gate

# From a tracked monorepo Git tree:
nix flake check --no-build .
nix run .#gate
```

Expected fixtures are JSON text records. The CLR runner removes at most one
terminal LF or CRLF record delimiter before comparing the canonical JSON;
every other leading or trailing byte remains significant.

## Honest boundary

This lane proves that code executes through ClojureCLR on .NET, that missing
imports in a dead `if` branch, unused argument, or unselected attr field are
never resolved, that application binds tighter than static attr-path `?`, and
that checked signed-I64 arithmetic reproduces with lazy overflow avoidance. It
additionally proves an operational artifact dependency: the generic `clr-meta`
builder produces the plan-bound eight-DLL artifact, and the product runner
rejects
missing, stale, malformed, or extra source/output state instead of using a
source/bootstrap build fallback.

The checked arithmetic boundary is deliberately narrower than a host numeric
API: operands originate in the admitted PNIX source lexer/evaluator path and
remain `System.Int64`. This evidence does not establish an ABI boundary for
arbitrary ClojureCLR integers. It does not yet claim:

- the complete mature JVM-host language or research surface;
- compiler Stage3--15/N, ClojureCLR compiler self-reproduction, or an IL fixed
  point (the checked-Int64 Stage1 and the separate selfhost-family C2 Stage1/C3
  Stage2 are closed, but `pnix-clr` does not consume the latter as its product
  compiler);
- byte-identical raw AOT reproduction across independent builds;
- broad ClojureCLR command, language, runtime, or ecosystem compatibility, or
  replacement of ClojureCLR;
- a standalone source-free distribution (launch validation currently binds the
  live plan and source closure, and execution retains the pinned runtime);
- PNIX common compiler/PIR integration;
- the full common conformance corpus;
- dynamic (`${...}`) attr paths;
- BigInt arithmetic or general numeric promotion;
- routing through or enforcement of the `pnix.primitive-abi.v1` manifest;
- production-evaluator or full-builtin primitive-manifest enforcement;
- production `Requested` / `Suspended` evaluator integration;
- completion of the future canonical-result/JCS wire;
- any claim of behavioural parity with the other pnix hosts.

Unsupported PNIX constructs fail closed with a structured `Failed` result.
`Done`, `Failed`, `Requested`, and `Suspended` are nominal ClojureCLR
types implementing the common host-outcome schema; only `Done` and `Failed`
are integrated into this evaluator slice. A guest attrset cannot forge one.
There is no JVM evaluator fallback.

The ordered route from the current evaluator/artifact/C3 Stage2 boundary to
Stage3--15/N and an eventual profile-bounded ClojureCLR replacement is recorded
in `clr-meta/STAGE15_N_ROADMAP.md`. That roadmap is a target, not a receipt.

## Layout

```text
clojure-clr-clojure-1.12.3-alpha8/  NuGet-restored ClojureCLR publish output
clr-meta/                            PNIX-agnostic CLR meta bootstrap
pnix-clr/                            CLR-native PNIX host + artifact plan
bin/                                 build, runners, and focused gate
```

The cloned JVM/domain trees were pruned rather than textually relabelled as a
CLR port. Only CLR-owned mechanism belongs here.


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

# builtins.trace "여기까지 실행됨" 42
>> Execution error (IndexOutOfRangeException) at pnix-clr.outcome/capture (NO_FILE:0).
>> Index was outside the bounds of the array.
>> Full report at:
>> /var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/clojure-b28e336e-af9c-4018-8bdb-7e0bcb37f5fb.edn

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

# builtins.id 42
>> Execution error (IndexOutOfRangeException) at pnix-clr.outcome/capture (NO_FILE:0).
>> Index was outside the bounds of the array.
>> Full report at:
>> /var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/clojure-9f7d6c8e-fde8-4f5c-9e7c-0098413d1ff8.edn

lib.const "foo" "bar"
>> {"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":"foo"}

# builtins.flip (a: b: a - b) 3 10
>> Execution error (IndexOutOfRangeException) at pnix-clr.outcome/capture (NO_FILE:0).
>> Index was outside the bounds of the array.
>> Full report at:
>> /var/folders/20/drddxc8x52x63llrn6kbfvw00000gn/T/clojure-d53e398f-03fe-478c-8d75-1b5301114afe.edn

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


