# clr-meta

`clr-meta`는 `pnix-clr` 아래의 PNIX-agnostic ClojureCLR host bootstrap이다.
네 개의 의도적으로 분리된 메커니즘을 유지한다: focused evaluator
self-interpretation witness, profile-qualified direct-IL Compiler Stage1,
admitted·executable same-language selfhost Compiler Stage1-to-Stage2
family, generic host-ClojureCLR AOT runtime-artifact builder.

## 상태 / primary 게이트

[STATUS.md](STATUS.md) 참조. Primary 게이트: `./bin/clr-meta-gate` (eval-only 또는 full C0–C3 chain).

## Evaluator generation

```text
host ClojureCLR compiler -> generation 0 evaluator
generation 0 interprets evaluator source -> generation 1
generation 1 interprets evaluator source -> generation 2
```

세 generation 모두 literal, quote, conditional, sequential binding, closure,
named recursion, variadic binding을 다루는 focused Clojure corpus에서 합의한다.
interpreted evaluator는 host `eval`을 호출할 수 없다.

이것들은 physical evaluator generation이며, Compiler Stage1, Stage2, 또는
Stage15/N이 아니다. `clr-meta -e`와 file mode는 strict host reader가
`*read-eval*` 비활성, inert data reader, admitted list/vector/scalar domain의
recursive check로 정확히 하나의 form을 읽은 뒤 generation 2를 사용한다;
trailing form과 host-created reader value는 evaluation 전에 실패한다. tool
path에는 `load-string`이 없다. 이 nested interpreter를 15
self-extension으로 확장하려는 live 시도는 CLR 스택을 소진한다. 이는 open
runtime limitation이며 Stage15/N claim을 뒷받침하지 않는다; stage receipt나
language `Held` outcome이 아니다.

## Compiler Stage1: checked Int64 expression profile

`pnix.clr-meta.checked-i64-expression.v1`은 evaluator generation 및 runtime
artifact와 분리된 첫 compiler-stage receipt family다. exact source surface는
`System.Int64` literal, dynamic parameter `arg`, binary checked `+`, `-`,
`*`만으로 이루어진 하나의 strict-EDN form이다. Metadata, BigInt/decimal/float
value, unknown symbol/operator/arity, extra form, byte/reader/node/depth
budget overrun은 structured rejection이다.

pinned host compiler는 current process의 absolute dotnet host와
clear-then-allowlist environment를 통해 두 ClojureCLR-written compiler
namespace를 AOT-seed한다. fresh child는 그다음 compiler AOT seed를 보지만
implementation source는 보지 않으며, validation/lowering/opcode selection을
소유하고, `PersistedAssemblyBuilder`와 CLR metadata API를 통해 runnable
managed PE를 직접 쓴다. target은 `System.Private.CoreLib`와
`System.Console`만 참조; ClojureCLR/evaluator resource를 포함하지 않는다.
Dynamic argument와 admitted primitive 전부에 대한 checked overflow가 fresh
process에서 행사된다.

```sh
./bin/clr-meta --build-compiler-stage1 \
  clr-meta/compiler-stage1/profile.edn \
  clr-meta/compiler-stage1/plan.edn \
  clr-meta/compiler-stage1/example.clj /tmp/clr-stage1
dotnet /tmp/clr-stage1/target/program.dll 7  # 27
```

경계는 명시적이다: `Clojure.Main`은 여전히 `clojure.main` 같은 pinned runtime
startup source를 compile하고, `clojure.edn`은 reader TCB다. builder는 완전한
Clojure publish-directory snapshot을 복사·hash하며, CoreCLR과 BCL은 external
TCB로 남는다. inherited child environment를 clear하고, earlier lookup root를
scan하며, admitted target form이 host `compile`/`eval`로 fallback하지 않음만
증명한다. 따라서 compiler self-reproduction, Stage2--15/N, IL fixed point,
raw PE reproducibility, broad ClojureCLR replacement, standalone source-free
distribution은 false로 남는다.

## Selfhost compiler family: C0/C1 admission, C2 Stage1, C3 Stage2

checked-Int64 family는 frozen으로 유지된다. `Run(Int64) -> Int64` artifact는
real narrow Compiler Stage1 target이지만, 현재 compiler implementation의
완전한 top-level form조차 표현할 수 없고 정직하게 Stage2로 rename할 수 없다.

따라서 `compiler-selfhost/`는 별도 `pnix.clr-meta.compiler-kernel.v1` family를
시작한다. C0 contract는 canonical compiler ABI, macro-free source profile,
exact low-level reader/data 및 PE sink support ABI, forbidden host
compiler/evaluator/process surface, receipt lineage, 세 future anti-baking
mutation site를 고정한다. C1 게이트는 reader evaluation 비활성으로
canonical source를 strict-read하고 모든 syntax node와
lexical/global/support/sink reference를 recursively 집계한다.

frozen source는 37 top-level form과 36 definition을 가진다. 2,237 recursive
node가 33 support call, 네 intrinsic, 12 explicit lowering-owner row로
닫히며; unknown, rejected, interpreted, opaque, payload node는 모두 zero다.
focused 게이트는 4 tests / 288 assertions와 23 negative case를 통과한다.
deterministic receipt SHA-256은
`3a1635882bfcdf67c50a90cbc058100c496d59eba71b11a2271bd24492302741`이다.

```sh
clr-meta/scripts/clr-meta-compiler-selfhost-admission-gate
```

그 C1 receipt는 execution이 아닌 source admission으로 남는다. C2는 별도
executable contract와 `Pnix.ClrMeta.CompilerSupport`를 추가한다: strict
bounded reader/data/environment ABI, stack- 및 control-flow-checked
transactional PE sink, strict generated-artifact host. public builder는
스스로 C1 admission을 실행하고, explicit B0 seed 전후에 complete pinned
ClojureCLR runtime closure를 snapshot하며, generated Compiler Stage1 PE와
세 support runtime file만 담은 no-replace bundle을 publish한다:

```sh
./bin/clr-meta --build-compiler-selfhost-stage1 /tmp/clr-selfhost-stage1
dotnet /tmp/clr-selfhost-stage1/runtime/Pnix.ClrMeta.CompilerSupport.dll \
  compile /tmp/clr-selfhost-stage1/compiler/CompilerStage1.dll \
  SOURCE.clj OUTPUT.dll
clr-meta/scripts/clr-meta-compiler-selfhost-stage1-gate
```

C2 게이트는 27 generated compiler method를 모두 prepare하고, execution
process에서 canonical compiler source와 ClojureCLR을 숨기며, checked
add/subtract, closed equality, Clojure truthiness target을 compile·run한 뒤,
Stage1이 존재한 후에만 random nonce source를 만들어 genuinely fresh target을
증명한다. 또한 7,900-node near-budget program을 실행하고, 16 structured
no-output failure와 네 builder publication-preservation case를 검사하며,
세 frozen mutation site를 검증한다. Identity mutation은 target metadata에
도달한다. Add/subtract mutation은 generated compiler 자체의 control
arithmetic을 바꿔 예측된 `bad-def-arity` / `call-arity` rejection을 target
output 없이 만든다; 이는 propagation 증거이지, mutated compiler가 여전히
target arithmetic을 성공적으로 바꾼다는 claim이 아니다.

C2는 executable selfhost Compiler Stage1 artifact만 닫는다. immutable
historical manifest와 gate receipt는 따라서 계속 `compiler_stage2=false`라고
말한다: later checkpoint가 earlier artifact의 boundary를 rewrite하지 않는다.

C3는 대신 새 contract와 child artifact를 추가한다. Stage2 builder는 complete
C2 parent manifest와 live closure를 검증하고, 같은 canonical kernel source의
byte-exact private copy를 freeze하며, fresh allowlisted process에서
generated Stage1을 실행해 그 source를 runnable Stage2로 compile한다. parent
manifest, parent compiler, source, support triplet, toolchain, contract,
child compiler, input/bundle closure hash를 바인딩한다. override-style
child는 `CompilerStage2.dll`, 세 support runtime file, 자체 C3 manifest만
포함한다: parent Stage1 PE와 manifest, 그리고 canonical source와 ClojureCLR은
패키징되지 않는다.

```sh
./bin/clr-meta --build-compiler-selfhost-stage2 \
  /tmp/clr-selfhost-stage1 /tmp/clr-selfhost-stage2
clr-meta/scripts/clr-meta-compiler-selfhost-stage2-gate
```

artifact manifest는 `compiler_stage2=true`와 exact same-source recompile을
닫지만, artifact construction만으로는 replay 증명을 소유하지 않으므로
의도적으로 `stage2_fresh_target_replay=false`를 기록한다. 별도 C3 게이트는
그 artifact에서 시작해, Stage2와 support triplet만 exact source-hidden
directory에 복사하고, Stage2 존재 후 random nonce target을 만들어 Stage2로
compile한 뒤, target과 support triplet만 있는 다른 fresh directory에서
target을 실행한다. C3 gate receipt가 `stage2_fresh_target_replay=true`의
소유자다. Delayed identity/add/subtract mutation도 mutated Stage2를 통해
grandchild target으로 전달되어, 그 관찰을 general compiler correctness로
업그레이드하지 않고 one-generation propagation을 증명한다.

C3는 정확히 Compiler Stage2와 source-hidden fresh-target replay를 닫는다.
Compiler Stage3, self-reproduction, Stage15/N, fixed point, raw PE
reproducibility, host-free bootstrap, full Clojure surface, ClojureCLR
replacement, PNIX product/compiler integration, cross-host canonical
equivalence, host promotion은 false로 남는다.

## Runtime artifact builder

generic builder는 strict EDN plan, destination, exact Clojure source root를
받는다:

```sh
./bin/clr-meta --build-runtime PLAN OUTPUT SOURCE_ROOT
```

plan schema, entry, ordered namespace set, namespace/path collision,
pairwise plan/source/output path separation, declared와 actual `.clj`
source set 동등성을 검증한다. 그다음 load path에 declared source root만
있는 fresh child를 시작하고, pinned host ClojureCLR `compile` backend를
사용하며, undeclared namespace dependency를 거부하고, plan, source bytes,
exact output set, entry, target, closure hash를 바인딩하는 deterministic
JSON manifest를 emit한다. product identity는 caller의 plan에 남는다;
`clr-meta`는 PNIX namespace list를 포함하지 않는다.

`pnix-clr`의 경우 `runtime-artifact.edn`이 여덟 namespace를 선언하므로
정확히 여덟 `.clj.dll` output이 나온다. 이는 real artifact-production
seam이지만, backend는 정직하게 `host-clojureclr-aot`로 명명된다. manifest는
한 build가 만든 bytes를 pin한다; 두 raw AOT build가 byte-identical임을
증명하지 않는다. product runner는 추가로 cwd를 verified artifact로
고정하고, ClojureCLR earlier pinned runtime lookup root에서 product
namespace shadow를 거부한다.

외부 디렉터리에서:

```sh
./bin/clr-meta --gate
./bin/clr-meta -e '(+ 20 22)'
clr-meta/scripts/clr-meta-compiler-stage1-gate
clr-meta/scripts/clr-meta-compiler-selfhost-admission-gate
clr-meta/scripts/clr-meta-compiler-selfhost-stage1-gate
clr-meta/scripts/clr-meta-compiler-selfhost-stage2-gate
./bin/build-pnix-clr-artifact
clr-meta/scripts/clr-meta-gate
```

`bin/clojure-clr-bootstrap`는 pinned upstream compiler/runtime trust root를
명명한다. `bin/clojure-clr`는 generation 2를 통한 `-e`와 single-file
evaluation만 admit하고 broader command profile을 거부하는 focused
compatibility facade다. 여전히 그 trust root가 host하며, self-reproducing
`clr-meta` compiler가 뒷받침하지 않는다.

이는 evaluator self-interpretation, exact checked-Int64 expression Compiler
Stage1 profile, 별도 버전 selfhost kernel source의 complete static admission,
C2 executable selfhost Stage1 artifact, C3 same-source executable Stage2와
source-hidden fresh-target replay를 증명한다. Compiler Stage3, compiler
self-reproduction, Stage15/N, IL fixed point, raw AOT/PE rebuild determinism,
full Clojure language/command surface, ClojureCLR replacement, unbundled
lineage의 standalone replay, PNIX semantics/compiler integration, cross-host
canonical equivalence를 증명하지 않는다. exact current claim은
`CLR_BOOTSTRAP.md`와 emitted receipt를, ordered open target은
`STAGE15_N_ROADMAP.md`를 참조.
