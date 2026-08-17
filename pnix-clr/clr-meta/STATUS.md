# clr-meta 상태 (peer host-meta floor)

마지막 검증: 2026-08-17.

## Peer-floor 선언

**clr-meta**는 `pnix-clr` 아래의 PNIX-agnostic ClojureCLR host bootstrap이다.
practical peer floor는 **product substrate**에 대해 다른 host meta와 맞으며,
정직한 Stage3–15/N ladder는 여전히 open:

| Peer | Peer floor | clr-meta 대응 |
|---|---|---|
| JVM Clojure host | bytecode selfhost | eval gen0–2 + C0–C3 Stage1/2 |
| Hy host | stage ladder / fixed-point | C3 Stage2 + source-hidden fresh-target replay |
| Rust host | TV + stage chain | checked-I64 Stage1 + selfhost PE emit |
| ClojureScript host | fixed-point compiler | Stage2 same-source recompile (full IL fixed point 아님) |

Meta-first 순서: `pnix-clr` 전에 `clr-meta`. Artifact builder + hash-bound
load path closed. Stage3–15/N은 roadmap (`STAGE15_N_ROADMAP.md`)으로 남으며,
clj/rs/cljs에서 Stage15 replacement를 주장하지 않는 것과 같은 honesty.

## Closed claims

이 세션 live-verified (2026-08-07), `./bin/clr-meta-gate eval-only`:

```text
bootstrap-test (gen0→1→2 self-interpretation)  ready=true
  18 tests / 171 assertions, 0 fail / 0 error
  all corpus cases stage-values agree across gens
tool-gate (-e / file gen2 + strict reader)     PASS
  (+ 20 22) => 42 via evaluator-generation-2
  reader-eval / tagged / trailing / map rejected
```

문서화된 closed (heavy C1–C3 게이트; 이 세션 full chain 재실행 아님):

```text
checked-I64 Compiler Stage1 family
selfhost C1 admission  receipt 3a163588…
selfhost C2 executable Stage1 artifact
selfhost C3 Stage2 + source-hidden fresh-target replay
host-clojureclr-aot runtime artifact builder
```

## 이 wave에서 closed (2026-08-07) — Compiler Stage3–7 + path fix

```text
./scripts/clr-meta-compiler-selfhost-stage3-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage4-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage5-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage6-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage7-gate   PASS
  compiler_stage{5,6,7} = true; stageN_fresh_target_replay = true
  structural_description_equal_to_parent = true
  same_source_recompile chain Stage2→…→Stage7
  promotion/allowed? = false
  self_reproduction / stage8 / stage15_n / fixed_point = false
receipts: work/compiler-selfhost-stage{3–7}-gate.receipt.json
builders: scripts/clr-meta-build-compiler-selfhost-stage{3–7}
CLIs:     --build-compiler-selfhost-stage{3–7}
design:   STAGE{3–7}_DESIGN.md
gate chain: scripts/clr-meta-gate → stage3…stage7

pnix-clr relative FILE.px path fix (caller cwd before artifact cd)
pnix-clr common-slice: live five-host floor (URI, JSON, dynamic attrs, exact int,
  mixed float, non-finite observation, POSIX ERE classes, failed-thunk replay,
  kernel/math guest modules). See RESIDUAL_SURFACE.md for open principles.
  promotion/allowed? = false
```

## 이 wave에서 closed (2026-08-12) — Compiler Stage8 reproducible assembly artifacts

```text
./scripts/clr-meta-compiler-selfhost-stage8-gate   PASS
  Two independent Stage7 builds from the same frozen Stage6 parent are now
  byte-identical (sha256-equal, cmp-equal), not just structurally equal.
  Found and canonicalized the only two non-deterministic PE fields this
  codegen path (PeSink.cs, PersistedAssemblyBuilder-based) actually produces:
    PE COFF TimeDateStamp -> 0
    Module Mvid -> 00000000-0000-0000-0000-000000000000
  Found empirically (cmp -l byte diffing of two real builds), not assumed --
  confirmed no PDB/debug-info variance exists in this codegen path either.
  New describe-determinism verb re-derives both fields independently of the
  writer, so the gate does not just trust that the canonicalizer ran.
  Bonus (unplanned, observed live): Stage3, Stage4, Stage5, Stage6, and
  Stage7's own compiler DLLs are now ALL sha256-identical to each other too
  (not merely structurally equal), since canonicalization removes the only
  two things that varied between what were otherwise identical recompiles of
  the same frozen kernel.
  claims.stage8 = true; raw_artifact_reproducibility = true (scoped to
    compiler_stage7_persisted_assembly_builder_output); promotion/allowed? = false
receipt: work/compiler-selfhost-stage8-gate.receipt.json
contract: compiler-selfhost/stage8-contract.edn
design: STAGE8_DESIGN.md
gate chain: scripts/clr-meta-gate → …stage7 → stage8
```

## 이 wave에서 closed (2026-08-12, 같은 날) — Compiler Stage9 clean-process replay

```text
./scripts/clr-meta-compiler-selfhost-stage9-gate   PASS
  Every prior stage gate calls the compiler-selfhost-runtime support DLL
  directly, or runs bootstrap-test in-process inheriting the calling shell's
  environment -- none of them exercise bin/clr-meta itself (the thing a user
  actually runs) under a fully cleared environment (env -i, nothing
  inherited). Stage9 closes that gap and adds a property nothing before it
  checked: replay -- the same clean-process command run twice must produce
  byte-identical stdout, not just be correct once.
  4-case entrypoint matrix, each run twice independently:
    --gate (evaluator gen0-2 self-interpretation report, :ready true)
    -e "(+ 40 2)" (evaluator-generation-2 eval mode)
    single-file mode (same exact output shape as -e)
    -e '#?(:clj 1 :cljr 2)' (negative: reader conditionals stay rejected)
  All 4 cases byte-identical across both runs; correctness content also
  checked (not just self-consistency).
  claims.stage9 = true; replay_identical_across_two_runs = true;
    promotion/allowed? = false
receipt: work/compiler-selfhost-stage9-gate.receipt.json
design: STAGE9_DESIGN.md
gate chain: scripts/clr-meta-gate → …stage8 → stage9
```

## 이 wave에서 closed (2026-08-12, 같은 날) — Compiler Stage10–15/N + StageN

```text
./scripts/clr-meta-compiler-selfhost-stage10-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage11-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage12-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage13-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage14-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage15-gate   PASS
./scripts/clr-meta-compiler-selfhost-stagen-gate    PASS
./scripts/clr-meta-manifest-check                   PASS

  Every other host (hy-meta, rs-meta) had already closed this whole range;
  clr-meta had none of it -- no adapter matrix, no quarantine storage, no
  cross-host law, nothing. Built following the SAME pattern those hosts
  already established (a policy TSV per stage under proofs/, declaring an
  explicit DONE/GROW/HELD/DISABLED stance for every relevant boundary, plus a
  live replay of whatever's DONE), adapted to clr-meta's own real surfaces
  rather than a blind copy of rs-meta's wording:
    Stage10: proofs/session-sandbox.tsv -- load-context shadow rejection
      (bin/clr-meta already rejects a planted pnix.clr-meta namespace
      shadow before running anything; now proven live, twice, plus a
      shadow-removed sanity check) + a 2-command session replay through
      bin/clr-meta under env -i.
    Stage11: proofs/adapter-schema.tsv -- local-clojureclr (replays Stage9
      once) + compiler-selfhost-native (via Stage8's own latest checked
      receipt, not a re-run -- see below) + github-actions/external-nuget-
      feed/cross-implementation held.
    Stage12: proofs/quarantine-policy.tsv -- local-verification (Stage11) +
      candidate-intake (manifest-check) + remote-ci/manual-promotion/self-
      modification/external-evidence held.
    Stage13: proofs/horizon-policy.tsv -- stage-manifest + session-replay
      (Stage12) + stale-evidence/external-memory/organism-state/ambient-
      network held (all degrade-to-held by policy default).
    Stage14: proofs/cross-impl-schema.tsv -- clr-meta-local +
      independent-mini-backend (both via a fresh bootstrap-test run) +
      compiler-selfhost-native (Stage8 receipt) + remote-ci/alternate-
      clojureclr/mrustc-style-second-compiler held. Note:
      independent-mini-backend is the one row here already closed to a
      genuine Trusting-Trust bar (a real second, independently-authored
      implementation cross-validated against host eval) -- the other DONE
      rows are local self-consistency checks, not independent-implementation
      comparisons; the design doc calls this distinction out explicitly.
    Stage15: proofs/evidence-federation.tsv -- local-proof (Stage14) +
      stage-manifest (manifest-check) + remote-ci/external-web/external-
      tool/human-note held.
    StageN: proofs/extension-policy.tsv -- manifest-index + timeout-cost
      (Stage15) + stageN-seed (self-validated) + breaking-change/external-
      law/future-stage held.

  Cost-shape correction made while building this (recorded so it isn't
  silently re-broken later): the first draft had every stage re-run its
  predecessor's *entire* gate TWICE (mirroring Stage8-10's own "replay
  twice" pattern). That's wrong past Stage10 -- each predecessor already
  proves its own replay property internally, so doubling again at every
  hop compounds to quadratic cost by StageN (measured: an early stage12
  draft alone took ~90s; the fixed version's whole stage11-15/N+StageN
  chain together takes well under that). Fixed: every stage from Stage11
  onward calls its referenced predecessor exactly ONCE, and the two
  genuinely expensive artifacts (compiler-selfhost-native, referenced by
  both Stage11 and Stage14) are verified via Stage8's own latest checked
  receipt rather than by re-running Stage8's multi-minute chain-rebuild gate
  again from inside a later stage.

  Also fixed live: proofs/stage-manifest.tsv's own validator
  (scripts/clr-meta-manifest-check) initially used `declare -A`
  (bash 4+ associative arrays), which fails outright under macOS's system
  /bin/bash (3.2) -- rewritten to plain string matching, matching every
  other script in this codebase's existing bash-3.2-safe convention.

  claims.stage10 through claims.stagen = true;
    promotion/allowed? = false on every one of them
receipts: work/compiler-selfhost-stage{10-15,n}-gate.receipt.json
designs: STAGE{10-15,N}_DESIGN.md
gate chain: scripts/clr-meta-gate → …stage9 → stage10 → … → stagen
```

## 이 wave에서 closed (2026-08-12, 같은 날) — compiler self-reproduction / B==C fixed point

```text
./scripts/clr-meta-compiler-self-reproduction-check   PASS
  Not built from scratch -- found: Stage8's own gate output already logged,
  as an unplanned bonus observation, that Stage3-7's compiled
  CompilerStageN.dll shared one sha256. This check formalizes that finding
  with its own dedicated, named receipt: builds Stage1 through Stage7 fresh
  and confirms ALL SEVEN stages -- not just an adjacent pair, and including
  Stage1's host-seeded build itself -- share the exact same sha256
  (19872f28ed3576cbdf50001649fb9fd773023778fa0bb5c3d7aee45a61baecb7 in the
  verifying run). Holds because of Stage8's PE canonicalization: every
  stage compiles the same frozen compiler_kernel.clj source through the
  same PersistedAssemblyBuilder codegen path, and once the only two
  non-deterministic PE fields are canonicalized away nothing is left to
  differ -- including Stage1, since it goes through the same PeSink.Finish()
  path as every later stage. A live compile+execute of an unseen target
  through the shared Stage7 artifact confirms the shared bytes are not
  vacuously identical-but-broken (add_result: 42).
  claims.compiler_self_reproduction = true; claims.fixed_point = true
    (scope: compiler_stage1_through_7_persisted_assembly_builder_output);
    promotion/allowed? = false
receipt: work/compiler-self-reproduction-check.receipt.json
design: SELF_REPRODUCTION_DESIGN.md
gate chain: scripts/clr-meta-gate → …stage7 → self-reproduction-check → stage8
```

## Open claims (주장하지 말 것)

```text
clr_il_fixed_point = false
broad_clojureclr_compatibility / replacement = false
pnix_common_compiler_integration = false
cross_host_canonical_equivalence / clr_host_promotion = false
```

Stage1부터 StageN, 그리고 compiler self-reproduction은 이제 모두 closed
(위 "이 wave에서 closed" 섹션 참조) — `promotion/allowed?`는 각각에서
`false`로 유지된다. 이 중 어느 것도 general CLR IL fixed point를 닫지
않기 때문이다 (Compiler Stage1-7 `PersistedAssemblyBuilder` output에
scoped이며, 이 repo가 만들 수 있는 모든 artifact kind가 아님) 또는 broad
ClojureCLR replacement — 그것이 실제 promotion 게이트로 남는다.

`raw_aot_rebuild_determinism`은 이 블록에서 이동됨 (2026-08-12): Stage8이
Compiler Stage1-7 `PersistedAssemblyBuilder` artifact family에 대해 특별히
닫는다. 이 repo가 만들 수 있는 모든 artifact에 대한 general claim이 아니다
— debug info를 쓰는 미래 codegen path는 자체 determinism check가 필요하며,
`stage8-contract.edn`의 explicit non-claim을 따른다.

## Trusting-Trust defense roadmap (Diverse Double-Compiling)

Rust (`mrustc`)와 달리 wild에 lean할 independently-authored third-party
ClojureCLR compiler가 없다 — 두 번째 independent backend를 in-house로 만들어야
하며, reference JVM host (`frontend_selfhost.clj`/`diverse_double_compile.clj`
pair)가 자체 DDC witness에 이미 적용한 것과 같은 제약, 여기서 따른 것과
같은 패턴이다.

**이 세션에 independent mini backend 추가 (2026-08-11):**
`independent_mini_backend.clj`는 새 from-scratch Int64
tokenizer/reader + analyzer + `System.Reflection.Emit.DynamicMethod` IL
emitter로, Compiler Stage1-7 family
(`compiler_stage1.clj`, `compiler_selfhost_*.clj`)와 코드를 공유하지 않는다.
그 family는 full PE executable를 만들기 위해
`System.Reflection.Emit.PersistedAssemblyBuilder`를 쓴다. `DynamicMethod`는
method를 memory에서 JIT하고 invokable handle을 직접 반환한다 — Stage1-7
family가 공유하는 assembly/PE-writing path를 건드리지 않는다. pinned
ClojureCLR runtime과 CLR 자체는 trusted host substrate로 남으며, reference
host의 tiny frontend witness에서 JVM classfile format이 하는 것과 같은
honest role이다.

15 value-returning fixture (`+`/`-`/`*` checked arithmetic,
`<`/`>`/`<=`/`>=`/`=` comparison, nested `if` 포함 `if`, 0/1/2/3/4-arg
function)와 4 checked-overflow negative fixture (real host와 mini backend
모두 `Int64.MaxValue + 1`, `Int64.MinValue - 1`, `Int64.MaxValue * 2`,
`Int64.MaxValue + Int64.MaxValue`를 거부해야 함)를 다룬다. real host
ClojureCLR `eval`에 대해 cross-validate — 19 모두 합의. aggregate
`bootstrap-test` entry point의 일부로 `scripts/clr-meta-gate`가 호출하는
`independent-mini-backend-test` (`clr-meta/test/pnix/clr_meta/`)에 연결.
이 세션 live 검증 (2026-08-11, later widening pass): namespace 자체 test
run이 `{:test 2, :pass 38, :fail 0, :error 0}`; full
`bin/clr-meta-gate --no-build` re-run이
`{:test 20, :pass 209, :fail 0, :error 0}, :ready true`, regression 없음.

**닫는 것과 여전히 닫지 않는 것:** genuine 2-way behavioral comparison
(real host `eval` ≡ from-scratch `DynamicMethod`-based mini backend)가
이제 존재하며 success surface와 checked-overflow negative surface 모두
통과한다 — documented plan만이 아님. Nested `if`와 더 많은 function arity가
이전에 여기 적힌 "not the full Stage1 profile shape" gap을 닫고;
checked-overflow fixture가 "negative cases not exercised" gap을 닫는다 —
mini backend의 `Add_Ovf`/`Sub_Ovf`/`Mul_Ovf` IL opcode는 항상 checked였고
(Compiler Stage1 profile의 `:overflow :system-overflow-exception`과 일치),
지금껏 테스트되지 않았을 뿐이다. 여전히 bounded fixture set이지 full
conformance corpus가 아니며, (이 세션 모든 host에 대해 정착한 같은 honest
bar) byte-identical IL이 아니라 behavior equivalence다. `DynamicMethod`-JITted
method와 `PersistedAssemblyBuilder`-written PE는 construction상 다른 CLR
artifact kind이기 때문이다.

**Independent-interpreter DDC track closed, 같은 날 (2026-08-12):** 위에서
"별도, 아직 시작하지 않은 track"으로 플래그됨 — gen0-2 evaluator lane을
cross-check하기 위해 independent *interpreter* 구축 (mini backend가 이미
두 번째 *compiler*인 것과 대비).
`src/pnix/clr_meta/independent_mini_interpreter.clj`는 `bootstrap.clj` 자체
9-case `conformance-cases` corpus가 증명하는 small, environment-driven Lisp
subset용 from-scratch tokenizer/reader + tree-walking interpreter
(`quote`/`if`/`let`/`fn` including named recursion and `&` variadic rest)로,
`pnix.clr-meta.main`의 reader나 `pnix.clr-meta.bootstrap/evaluate`와 코드를
공유하지 않는다. real, textual `bin/clr-meta -e` evaluator-generation-2
tool-eval path에 대해 cross-validate (pre-parsed data 아님, independently-
authored *reader*도 확인)
`scripts/clr-meta-independent-mini-interpreter-gate` 경유. live 검증: 9/9
fixture 첫 run accept, full aggregate 게이트 여전히 green.
`INDEPENDENT_MINI_INTERPRETER_DESIGN.md` 참조. interpreter alone은 여전히
full Wheeler bar를 자체로 넘지 않는다 (mini-backend 자체 scope note와 같은
honest bar) — necessary, not sufficient piece.

**Mini-backend에 `let` 추가, 2026-08-15:** JVM host(`pnix-clj/clj-meta`)
쪽 U6 witness를 이번에 크게 확장한 세션에서, 균형을 맞추기 위해 나머지
4개 host 중 사용자가 지목한 clr-meta부터 착수. `independent_mini_backend.clj`
(compiler-backend witness, `DynamicMethod`+`ILGenerator` 직접 emit)는
지금까지 `fn`(단일 arity, closure/캡처 없음)/`if`/이항 산술·비교뿐이었고
`let`이 없었음 — `independent_mini_interpreter.clj`(별도 트리 워킹
인터프리터 witness)는 이미 `let`을 갖고 있었지만 그건 완전히 다른 코드
경로(둘이 서로 코드를 공유하지 않는 게 이 witness들의 핵심 요구사항).

analyzer를 재설계: 기존엔 `env`가 심볼을 바로 arg 인덱스(정수)로
매핑했는데, `let` 바인딩은 arg가 아니므로 이 스킴이 안 맞음 — JVM host의
`frontend_selfhost.clj`가 쓰는 것과 같은 패턴으로, analyze 시점엔 `env`가
이름의 **존재 여부만** 추적(param과 `let` 바인딩 둘 다 그냥 `:op
:local` 노드로 귀결)하고, 실제 저장 방식(arg 슬롯 vs 선언된 local)은
EMIT 시점에 결정하도록 분리 — `.NET`의 `ILGenerator/DeclareLocal`가
살아있는 `ILGenerator`가 있어야 호출 가능하기 때문에(param의 고정
`Ldarg` 인덱스와 달리) 자연스러운 경계. `let` 바인딩은 `DeclareLocal
Int64`(boxing 전혀 없음 — 이 backend의 checked-Int64 프로파일 전체가
unboxed로 유지)로 진짜 `Int64` local을 선언 후 `Stloc`/`Ldloc`으로
읽고 쓰는, JVM local variable slot의 직접적인 .NET 대응.

sequential binding(뒤 바인딩이 앞 바인딩 참조), 바인딩 안에 `if` 포함,
outer 이름을 가리는 shadowing(파라미터 `x`를 `let`으로 재바인딩), nested
`let` 4가지 조합 전부 실제 host ClojureCLR `eval` 대비 검증(전부 일치).
`independent-mini-backend-test`에 5개 fixture 추가(15→20 value fixture,
overflow negative 4개는 그대로). 회귀 없음: `bootstrap-test` 재실행
20 tests/219 assertions(기존 209에서 +10, fixture당 host/mini 두
assertion씩) 0 fail/0 error, `./bin/clr-meta-gate eval-only` 여전히
PASS. Stage1-7 compiler-selfhost family(`compiler_stage1.clj`/
`compiler_selfhost_*.clj`)는 이 변경과 코드 공유가 전혀 없어 전체
stage1-N 체인은 이번엔 재실행 안 함(이 STATUS.md 자체가 이미 "heavy
C1-C3 게이트는 매 세션 재실행 아님" 관행을 명시).

**Mini-backend에 `loop`/`recur` 추가, 같은 날 (2026-08-15):** 원래 다음
후보로 적어뒀던 "nested `fn`/closure 캡처"를 다시 들여다보니, 이
`DynamicMethod` 기반 backend에는 클래스가 없어서(메서드 하나뿐) 클로저를
표현하려면 (a) `Int64`가 아닌 새 값 종류(클로저 값)를 도입하고 (b) 임의
값을 함수처럼 호출하는 일반 apply 메커니즘을 새로 추가해야 함을 확인 —
`let` 추가보다 몇 단계 더 큰 아키텍처 변경(지금은 `+`/`-`/`if`/`let`처럼
고정된 연산자만 있고 일반 적용 형태 자체가 없음). 그래서 지금
아키텍처(전체 `Int64`, 새 값 종류 없음) 안에서 자연스럽게 이어지는
`loop`/`recur`로 대신 진행 — clj-meta 쪽에서도 실제로 `let` 다음에 왔던
항목.

`analyze-loop`는 `analyze-let`과 구조가 같지만 body를 `recur-arity-key`
env 항목(바인딩 개수)과 함께 분석. `analyze-fn`도 자기 params 개수로
같은 키를 심어서, `loop`가 전혀 없는 **bare `recur`가 fn 자신의 params를
타깃**으로 삼는 걸 자동으로 지원(JVM host의 "가장 가까운 loop/fn" 규칙과
동일 — named self-recursion을 별도 메커니즘 없이 공짜로 얻음). Emit
쪽은 `emit-let`처럼 각 바인딩을 `DeclareLocal`+`Stloc`한 뒤, 바인딩이
끝나는 바로 그 지점에 `MarkLabel`(recur의 `Br` 타깃). `emit-recur`가
핵심: **모든 새 값을 OLD 값들로부터 먼저 임시 local에 계산해 넣은
뒤에야** 실제 타깃(local이면 `Stloc`, fn arg면 .NET에서 재대입 가능한
`Starg`)에 저장 — Clojure의 "recur는 동시 재바인딩이지 순차 아님" 의미를
지키기 위함. 이게 실제로 필요한지 `(loop [i n a a b b] (if (= i 0) a
(recur (- i 1) b a)))` 같은 swap 케이스로 실제 host 대비 검증(양쪽 다
2 — 순차 구현이었다면 다른 값이 나왔을 것). 합산 loop, factorial loop,
bare recur, swap 4가지 조합 전부 실제 host `eval`과 일치. fixture 4개
추가(20→24 value fixture). 회귀 없음: `bootstrap-test` 20 tests/227
assertions(기존 219에서 +8) 0 fail/0 error, `./bin/clr-meta-gate
eval-only` 여전히 PASS. Stage1-N family와 코드 공유 없어 전체 체인
재실행 안 함.

**Mini-backend에 nested `fn` 지원 추가(closure, 진짜 런타임 값 없이),
같은 날 (2026-08-15):** 사용자가 "jvm clojure 만큼의 수준만큼 clr-meta도
높여달라"로 명시적으로 재확인 — nested closure 설계를 실제로 정리해서
착수. 핵심 통찰: 이 backend가 다루는 fixture들의 "closure"는 전부
**즉시 적용되는(immediately-applied)** 중첩 `fn` 형태였음
(`(((fn [x] (fn [y] (+ x y))) 20) 22)`처럼 — `bootstrap.clj` 자체
conformance corpus와 `independent_mini_interpreter.clj`의 closure
fixture 둘 다 정확히 이 모양). 이런 형태는 표준 **beta-reduction**
`((fn [p...] a...) args...)` ≡ `(let [p... args...] a...)` 그 자체라서,
진짜 런타임 클로저 값이나 일반 apply 메커니즘 전혀 없이, 이미 만들어져
검증된 `let` 메커니즘 그대로 재사용해서 처리 가능함을 확인.

`desugar`라는 새 raw-form 재작성 pass 추가(reader 출력과 analyzer 사이):
(1) `((fn [p...] body) a...)` → `(let [p... a...] body)` beta-reduction,
(2) `((let [b...] TAIL) a...)` → `(let [b...] (TAIL a...))` let-floating
(함수 호출의 인자는 항상 callee의 내부 스코프와 독립적으로 평가되므로
안전) — 두 규칙을 자식부터 먼저 처리(bottom-up)하며 고정점까지 재귀
적용해서 임의 깊이 중첩과 transitive capture를 공짜로 처리(자식이 먼저
reduce되므로, 바깥 레벨에서 op를 검사할 때 이미 최대한 줄어든 상태).
(3) 자연스러운 확장으로 "named local fn을 let으로 묶고 그 자리에서
한 번 호출"하는 흔한 패턴(`(let [square (fn [x] (* x x))] (square n))`)도
같은 방식으로 인식해서 처리 — 마지막 바인딩이 `fn` 리터럴이고 tail이
정확히 그 이름 호출이면 beta-reduction으로 환원.

**의도적으로 좁힌 경계, 명시적으로 정직하게 문서화**: (a) 진짜 first-class
클로저(변수에 저장 후 나중에 별도 경로로 호출, `compile-source` 경계를
넘어 반환 등)는 여전히 미지원 — 새 런타임 값 표현 + 일반 apply가
필요한 훨씬 큰 작업이라 착수 안 함(시도했다가 미지원 형태에 대해서는
"tiny analyzer: unsupported op fn"라는 명확한 구조적 에러로 실패 —
조용히 틀린 값 아님, 확인됨). (b) capture-avoiding substitution이
아니라 순수 substitution이라, 중첩 fn의 파라미터 이름이 자기 호출의
형제 인자에 등장하는 경우(`((fn [x y] (+ x y)) 1 x)`에서 두 번째
인자 `x`가 바깥 `x`를 의미해야 하는 경우)는 정확하지 않음 — 이 repo의
실제 fixture 중 이런 모양은 없음, alpha-renaming(고정 gensym) pass가
필요하면 나중에 잘 정의된 후속 작업으로 남김.

단순 중첩, transitive/4단계 중첩, `if` 안에 중첩 적용, named-local-fn
(단독 바인딩/앞선 바인딩과 함께) 6가지 조합 전부 실제 host ClojureCLR
`eval` 대비 검증(전부 일치) + 미지원 형태(let에 저장만 하고 호출 안 함,
named-local-fn을 let으로 만들었지만 나중에 부른 경우)가 조용히 틀리지
않고 명확히 에러내는 것도 확인. fixture 6개 추가(24→30). 회귀 없음:
`bootstrap-test` 20 tests/239 assertions(기존 227에서 +12) 0 fail/0
error, `./bin/clr-meta-gate eval-only` 여전히 PASS. Stage1-N family와
코드 공유 없어 전체 체인 재실행 안 함.

**Mini-backend에 진짜 first-class 클로저 추가, 같은 날 (2026-08-17):**
바로 위 슬라이스가 명시적으로 미지원으로 남겨둔 경계 — 여러 번 호출되거나
tail이 아닌 위치에서 쓰이는 클로저(desugar의 beta-reduction으로 지울 수
없는 형태) — 를 사용자의 반복된 "clr-meta pnix-clr 수준을 clojure 만큼
높여봐" 재확인을 받아 실제로 채움. `.NET Reflection.Emit` API 조사 결과
핵심 발견: **`System.Reflection.Emit.DynamicMethod`의 IL generator는 이
.NET 10.0.10 환경에서 `TypeBuilder`가 만든 생성자/메서드를 참조할 수
없다**("MethodInfo/ConstructorInfo must be a runtime ... object" 에러,
제네릭 delegate 기반과 비제네릭 abstract base class 기반 둘 다 동일하게
실패) — 반면 **호출하는 쪽 메서드 자체가 TypeBuilder-hosted면 완전히
동작**함을 end-to-end로 확인(결과 43, 정확). 이 플랫폼 특성이 아키텍처를
결정: `compile-source`가 이제 AST에 클로저가 있는지 먼저 분석하고 —
없으면 기존 `DynamicMethod` 경로를 **완전히 그대로**(회귀 위험 0) 타고,
있으면 전체 top-level fn을 새 Run-only dynamic assembly/module 위
TypeBuilder-hosted `public static` 메서드로 컴파일해서 그 안에서 자유롭게
클로저 클래스들의 생성자/`Invoke`를 참조하게 함. 두 경로 모두
`emit-top-level-body!`를 공유(기존 코드에서 인라인이던 걸 뽑아냄).

클로저 하나당 새 `TypeBuilder` 클래스 하나(캡처마다 `public Int64`
필드, 캡처를 필드에 저장하는 생성자, 파라미터 하나짜리 non-virtual
`Invoke(long):long`)를 `emit-closure!`가 즉석에서 정의하고 그 자리에서
바로 `.CreateType`까지 마침(다른 무엇이 참조하기 전에 항상 완성된
상태). `let` 바인딩 시점에 캡처 값들을 바깥 env에서 읽어 생성자를 호출한
결과를 로컬에 저장(`{:kind :closure-local}`); 호출부는 그 로컬을 읽고
인자를 밀어넣은 뒤 `Call`(비virtual이라 vtable 필요 없음). 분석기 쪽은
`env`가 이제 이름당 `true`(평범한 Int64) 또는 `{:closure-arity 1}`(클로저
값, 호출만 가능) 두 종류를 구분해서 담고, `analyze-closure-fn`이 클로저
본문을 분석한 뒤 `ast-referenced-names`(참조 JVM host의 동명 함수와 같은
발상)로 자유변수를 역산해서 캡처 목록을 만듦.

**의도적으로 좁힌 경계, 명시적으로 문서화**: (a) 단일 파라미터 클로저만
지원(multi-arity/variadic 클로저 없음); (b) 캡처값은 평범한 Int64만
지원 — 클로저를 캡처하는 클로저(transitive capture)는 명확한 에러로
거부; (c) 클로저 본문 안에 또 다른 클로저 리터럴이 중첩되는 경우도 거부
— `ast-referenced-names`가 AST 모양만으로는 "안쪽 클로저 자신의
파라미터"와 "바깥에서 자유로운 이름"을 구분 못 해서, 허용하면 안쪽
파라미터를 바깥 캡처로 잘못 묶을 위험이 있음(어떤 fixture도 이 모양이
필요하지 않아 지금은 그냥 막아둠). 세 경계 모두 조용히 틀린 값이 아니라
명확한 구조적 에러로 실패하는 것으로 확인.

여러 번 호출(non-tail, 즉 `(+ (f n) (f (+ n 1)))` 모양)과 `let`으로 묶인
평범한 Int64 값을 캡처하는 경우까지 포함해 fixture 3개 추가(30→33 value
fixture) — 전부 실제 host ClojureCLR `eval` 대비 검증. 회귀 없음:
`bootstrap-test` 20 tests/245 assertions(기존 239에서 +6, fixture 3개 ×
2 assertion) 0 fail/0 error, `./bin/clr-meta-gate eval-only` PASS. Stage1-N
family와 코드 공유 없어 이번에도 전체 체인 재실행은 하지 않음(이전 슬라이스와
같은 근거) — 다만 이번엔 추가로 `./scripts/clr-meta-gate --no-build` 전체를
background에서 별도로 돌려 Stage1-N 체인 자체의 회귀 여부도 확인 중(결과 나오는
대로 이 섹션에 추가 기록).

**다음 구체적 단계:** 이걸로 clj-meta U6가 이번 세션 초반에 거쳤던 것과
같은 순서(`let` → `loop`/`recur` → nested fn → 진짜 클로저)를 clr-meta의
mini-backend가 대부분 따라잡음. 남은 자연스러운 다음 단계는 (a)
`independent_mini_interpreter.clj` witness 쪽 fixture도 같이 넓히기(9개
그대로 — 이번 세션엔 mini-backend만 확장), (b) 사용자가 처음 지시한
"나머지 host들도 clojure만큼" 균형 지시대로, clr-meta가 어느 정도
따라잡았으니 hy-meta/rs-meta/cljs-meta 중 하나로 넘어가는 것, (c)
`todo.md` item 6/7 (broad ClojureCLR compatibility/replacement, roadmap
자체 ordering이 명시적으로 연기).

## Primary 게이트

```sh
# From pnix-clr/clr-meta/  (prefer real rg on PATH)
export PATH="/usr/local/bin:/usr/bin:/bin:$PATH"
./bin/clr-meta-gate              # full family, --no-build default
./bin/clr-meta-gate --build      # rebuild bootstrap first
./bin/clr-meta-gate eval-only    # gen0–2 + tool only (lighter peer floor)
```

Full script chain: bootstrap-test → tool-gate → compiler-stage1-gate →
selfhost-stage1-gate → selfhost-stage2-gate.

## Tooling note

게이트 스크립트는 `rg` (ripgrep)를 기대한다. **`/usr/local/bin/rg`**를
선호. `PATH` 앞에 `pnix-clr/bin`을 두지 **말 것** — 그 tree가 old `rg`
shim을 실을 수 있다.

## 마지막 run (이 머신, 2026-08-17)

| 게이트 | 결과 | 비고 |
|---|---|---|
| `./bin/clr-meta-gate eval-only` | **PASS** | ready=true; 20 tests/245 assertions; tool-gate PASS |
| full C1–C3/Stage1-N chain | 이 세션 재실행 안 함 | `independent_mini_backend.clj`와 코드 공유 없음; docs claim closed |
| `./scripts/clr-meta-compiler-selfhost-stage3-gate` | **PASS** | Stage2→Stage3 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage4-gate` | **PASS** | Stage3→Stage4 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage5-gate` | **PASS** | Stage4→Stage5 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage6-gate` | **PASS** | Stage5→Stage6 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage7-gate` | **PASS** | Stage6→Stage7 + source-hidden replay |
| env | dotnet 10.0.302, published Clojure.Main.dll | OK |
