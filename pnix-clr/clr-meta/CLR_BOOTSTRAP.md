# clr-meta bootstrap lane

이것은 `clr-meta`의 첫 CLR-native, pnix-agnostic proof lane이다. 복사된 JVM
meta source는 relabel이 아니라 prune되었다; Git history와 reference host는
의도적 port를 위해 사용 가능하다.

## Claim

ClojureCLR host compiler가 하나의 small evaluator를 seed한다. 그 evaluator가
자신의 source를 interpret하여 evaluator generation 1을 만들고, generation 1이
같은 source를 interpret하여 generation 2를 만든다. 세 generation 모두
literal, symbol, quote, `if` 양 branch, sequential `let`, closure capture,
named recursion, variadic rest binding을 다루는 focused corpus에서 합의해야
한다.

게이트는 evaluator self-interpretation만 claim한다. receipt는 명시적으로
ClojureCLR compiler self-reproduction, CLR IL fixed point, full Clojure
language surface, host-free bootstrap, PNIX semantics를 claim하지 않는다.

receipt의 historical `:stage-chain` 번호 0, 1, 2는 이 evaluator generation을
식별한다. compiler stage로 읽히면 안 된다. 특히 어떤 compiler stage도
확립하지 않는다; 아래 별도 증거의 checked-I64 Stage1 slice가 한다. nested
evaluator를 15 self-extension으로 확장하려는 live 시도는 CLR 스택을
소진한다; 그 host resource failure는 open limitation으로 기록되며, `Held`로
변환되거나 stage evidence로 세지 않는다.

일반 `clr-meta -e`와 file tool path는 이제 evaluator generation 2를 강제하고
같은 closed primitive environment와 strict one-form host reader를 사용한다.
Reader evaluation은 false, data reader는 inert tagged value로 대체, EOF
필수, documented scalar/list/vector domain 밖 map/set/regex/tagged 또는
conditional reader value는 evaluation 전 거부된다. `load-string`을 호출하지
않는다. host reader, ClojureCLR runtime, initial compiler는 explicit
substrate boundary로 남는다.

## Artifact-production slice

`pnix.clr-meta.runtime-artifact`는 두 번째 PNIX-agnostic 메커니즘이다. caller가
strict namespace plan과 exact source root를 제공한다. builder는 그 closure와
pairwise-disjoint path를 검증하고, load path에 그 source root만 있는 fresh
child에서 pinned host ClojureCLR AOT compiler를 호출하며, undeclared
namespace dependency를 거부하고, exact DLL set을 검증한 뒤, plan, source,
output, entry, target, byte hash를 바인딩하는 JSON manifest를 atomically
publish한다.

현재 `pnix-clr` plan은 아홉 namespace를 선언하고 backend identity
`host-clojureclr-aot`로 정확히 아홉 DLL을 만든다. `pnix-clr` product runner는
manifest와 live plan/source/output closure를 검증하고, load path를 artifact로
교체하며, cwd를 verified tree로 바꾸고, pinned runtime lookup root의
product namespace shadow를 거부하며, product source를 compile하거나 load하는
대신 fail closed한다. 이는 artifact dependency를 확립한다; self-compilation이
아니다. 또한 두 번째-build byte-equality claim이 admit되지 않았으므로 raw
rebuild determinism을 증명하지 않는다.

## Compiler Stage1 slice

별도 `pnix.clr-meta.compiler-stage1-*` family는 `pnix.clr-meta.checked-i64-expression.v1`
에 대해서만 Compiler Stage1을 닫는다: exact `System.Int64` literal, `arg`,
checked binary `+`, `-`, `*`. pure lowering core와 CLR backend를 담은
host-AOT seed가 current process의 absolute dotnet host와
clear-then-allowlist environment를 통해 한 번 만들어진다; compiler source가
load path에서 제거된 상태에서 그 seed가 managed console PE를 emit·JIT-verify하고
게이트가 fresh process에서 dynamic argument를 실행한다. Exact profile, plan,
target source, compiler source/AOT, 여섯 carried bundle file, complete
Clojure publish-directory snapshot이 closure hash를 가진다. CoreCLR과 BCL은
그 snapshot 멤버가 아니라 external TCB로 남는다.

이것은 host-compiler-free process가 아니다. Pinned `Clojure.Main` startup
source compilation과 strict EDN reader는 declared TCB boundary로 남는다.
닫힌 것은 admitted-form boundary다: target form traversal, semantics,
lowering, opcode selection은 ClojureCLR-written이며 host compiler/evaluator
fallback이 zero다. Self-source classification이 모든 compiler top-level form을
tiny profile 밖으로 명시적으로 보고하므로 `stage2_ready=false`다.

## Selfhost compiler C0--C3 slice

checked-I64 Stage1 identity는 frozen이다. 별도 `pnix.clr-meta.compiler-kernel.v1`
family가 두 non-executable checkpoint와 두 executable checkpoint로 시작한다:

- C0: compiler source profile, compiler/support boundary, generated
  compiler ABI, forbidden fallback 및 payload surface, lineage 요구사항,
  semantic mutation probe를 고정;
- C1: entire canonical macro-free compiler kernel source를 strict-read하고
  recursively classify (lexical binding 및 exact global, support, PE-sink
  call arity 포함);
- C2: explicit pinned-host B0 boundary를 사용해 separately frozen CLR
  support ABI를 통해 Compiler Stage1을 emit·execute;
- C3: 그 generated Stage1이 exact same canonical kernel source를 runnable
  Stage2로 compile하게 한 뒤, compiler source와 parent artifact 없이 Stage2를
  fresh target에 대해 별도 replay.

admission analyzer는 kernel을 evaluate하거나 compile하지 않는다. 모든 source
node가 classify되고 unknown symbol, macro, metadata, reader escape, arbitrary
interop, undeclared ABI call이 남지 않은 뒤에만 hash-bound receipt를
publish한다. Negative mutation은 receipt를 남기지 않아야 한다.

canonical closure는 37 top-level form, 36 definition, 2,237 recursively
classified node를 포함한다. 33 support ABI call, 네 intrinsic, 12 lowering
owner, 세 semantic mutation anchor가 모두 hash-bound다. focused 게이트는
4 tests / 288 assertions와 23 no-output negative case를 통과한다; receipt
SHA-256은
`3a1635882bfcdf67c50a90cbc058100c496d59eba71b11a2271bd24492302741`이다.

C0/C1 checkpoint 이름은 compiler stage가 아니며, C1 receipt 자체는 모든
executable claim을 false로 유지한다. 별도 C2 게이트가 이제 executable
selfhost Compiler Stage1 artifact를 닫는다. public builder는 먼저 C1
admission을 실행하고, B0 child environment를 allowlist로 clear하며, seed
전후 pinned ClojureCLR runtime의 모든 regular file을 hash하고, atomic
no-replace directory move로 compiler/support bundle을 publish한다. bundle에는
canonical compiler source도 ClojureCLR도 없다.

generated compiler는 아홉 object constant와 27 public static object method를
가진다. PE sink는 method-local handle, 모든 operation과 branch join의 stack
height, label closure, return placement, finish-only publication을 검증한다.
execution 게이트는 모든 generated method를 prepare하고, source-hidden
process에서 checked arithmetic, equality, nil/false/zero truthiness,
post-Stage1 random nonce를 담은 target, 7,900-node near-budget sequence를
실행한다. 16 malformed/profile/closure case는 output을 남기지 않는다; 네
builder race/existing-output case는 winner를 보존한다.

세 C2 mutation anchor 모두 executable 증거다. identity change는 generated
target metadata에 나타난다. add 또는 subtract lowering을 바꾸면 compiler
자체의 control arithmetic도 바뀌어 exact structured
`validate/bad-def-arity` 또는 `validate/call-arity` failure를 output 없이
만든다. 그 outcome은 propagation을 증명하지만 still-functional
arithmetic-swap compiler를 claim하지 않는다. C2 manifest와 receipt는
historical `compiler_stage2=false` boundary를 유지한다.

C3는 C2 rewrite가 아니라 override-style child다. builder는 먼저 exact C2
manifest, input/bundle closure, live Stage1 description, support triplet,
canonical source hash를 검증한다. explicit byte-exact private source copy를
freeze하고 fresh allowlisted process에서 Stage1을 호출한다. 결과 Stage2는
Stage1과 같은 아홉 field, 27 prepared method, metadata, reference, resource,
callable entry shape를 가진다. Raw Stage1/Stage2 PE equality 또는 inequality는
gate condition이 아니다.

C3 child bundle은 `CompilerStage2.dll`, copied support triplet, C3 manifest만
소유한다. Stage1 PE, C2 manifest, canonical compiler source, ClojureCLR을
패키징하지 않는다; manifest는 hash로 parent lineage를 유지한다. builder
manifest는 `compiler_stage2=true`와 `same_source_recompile=true`를 닫고,
`stage2_fresh_target_replay=false`는 게이트가 소유하도록 올바르게 남긴다.

C3 게이트는 compiler replay directory에 Stage2와 support triplet만 둔다.
Stage2 존재 후 random nonce source를 만들고, 그 nonce가 Stage1, Stage2,
support에 없음을 증명하며, Stage2로 compile하고, target과 support triplet만
있는 두 번째 fresh directory에서 target을 실행한다. Delayed
identity/add/subtract mutation은 grandchild target에서 한 generation 뒤에
관찰된다. 따라서 C3 gate receipt는 `compiler_stage2=true`와
`stage2_fresh_target_replay=true`를 닫지만 Stage3 또는 self-reproduction는
닫지 않는다.

## 실행

`clr-meta/`에서:

```sh
scripts/clr-meta-gate
```

`Clojure.Main`이 이미 빌드된 경우:

```sh
scripts/clr-meta-gate --no-build
```

runner는 bundled ClojureCLR `Clojure.Main` project와 `net10.0`을 사용한다.
`bin/clojure-clr-bootstrap`가 그 trust root를 명명하고, `bin/clojure-clr`는
그 아래에 host되는 focused `-e`/single-file generation-2 facade다.

성공은 focused test와 bootstrap receipt가 `:ready true`를 포함하고, 독립
checked-I64 Compiler Stage1 receipt, selfhost C2 gate receipt, C3 Stage2
gate receipt가 통과해야 한다. C2 builder는 C1 admission을 필수 첫 단계로
실행한다; C3 builder는 C2 artifact를 silently reseed하지 않고 require·revalidate한다.

## Open target

Compiler Stage3--15/N, compiler self-reproduction, IL fixed point, raw
reproducibility, broad ClojureCLR compatibility/replacement, standalone
lineage replay, PNIX product/common-compiler integration, cross-host canonical
equivalence, CLR host promotion은 open으로 남는다. PNIX-agnostic C3 Stage2
artifact를 닫아도 그 claim이 true가 되지 않는다. 필요한 순서와 distinct
receipt는 `STAGE15_N_ROADMAP.md`에 정의된다.
