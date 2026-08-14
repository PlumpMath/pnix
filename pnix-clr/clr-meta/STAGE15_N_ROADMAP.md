# clr-meta Stage15/N 및 ClojureCLR replacement roadmap

상태: checked-Int64 expression Compiler Stage1 closed; 별도 selfhost-family
C0 contract와 C1 recursive source-admission checkpoint closed; executable
selfhost Compiler Stage1이 C2에서 closed; same-source executable Compiler
Stage2와 source-hidden fresh-target replay가 C3에서 closed; same-source
Stage3--7 recompile ladder가 live-gated CLOSED (2026-08-07),
`promotion/allowed?=false`; Stage8 (reproducible assembly artifacts)과
Stage9 (clean-process compiler/runtime replay) 모두 live-gated CLOSED
(2026-08-12); Stage10--15/N (session/sandbox, adapter, quarantine, horizon,
cross-implementation, evidence-federation, versioned-extension closure) 또한
live-gated CLOSED (2026-08-12, 같은 날), 모두 `promotion/allowed?=false`;
broad ClojureCLR compatibility/replacement만 OPEN으로 남음.
Truth owner: monorepo constitution, CLR source, artifact manifest, 미래 stage
receipt. 이 페이지는 roadmap이지 closure 증거가 아니다.

## Identity

`clr-meta`는 PNIX-agnostic ClojureCLR host-language layer다. generic CLR
compiler/artifact service를 노출할 수 있지만, PNIX syntax나 portable PNIX
meaning을 소유하지 않는다. `pnix-clr`가 product plan을 공급하고 결과 CLR
artifact를 사용한다; `pnix-meta`는 portable PNIX evaluator/compiler model의
owner로 남는다.

의도된 composition:

```text
pinned ClojureCLR bootstrap trust root
  -> clr-meta host-language evaluator/compiler/artifact mechanisms
  -> pnix-clr CLR mechanism and backend seam
  -> common pnix-meta meaning
```

distribution 또는 command directory를 공유해도 이 owner들이 병합되지 않는다.

## 현재 proven boundary

현재 evaluator lane은 세 physical generation을 가진다:

```text
generation 0 = host-seeded focused evaluator
generation 1 = generation 0 interprets evaluator-source
generation 2 = generation 1 interprets evaluator-source
```

세 모두 focused evaluator corpus에서 합의한다. `clr-meta -e`와 file mode는
host reading 후 generation 2를 실행하며 `load-string`을 호출하지 않는다.
이것들은 evaluator generation이지 compiler stage가 아니다. 이 nested
interpreter를 15 self-extension으로 확장하는 것을 live로 시도했고 CLR 스택을
소진했다. 그 host resource failure는 language `Held` 결과도 Stage15 증거도
아니다.

현재 artifact lane은 이 boundary에서만 별도로 closed:

- generic `clr-meta` builder가 하나의 exact namespace plan과 source closure를
  검증;
- backend는 pinned host ClojureCLR compiler, identity `host-clojureclr-aot`;
- `pnix-clr` plan이 정확히 아홉 AOT namespace DLL을 생산;
- manifest가 plan, ordered source row, exact output row, entry, target,
  closure hash를 바인딩;
- product runner가 모두 검증하고 load path를 artifact directory로 교체;
  fallback으로 product source를 compile하거나 load하지 않음.

이는 real artifact dependency를 증명한다. artifact rebuild가 byte-identical
DLL을 만든다는 것은 증명하지 않으며, startup은 여전히 validation을 위해
pinned ClojureCLR runtime과 live plan/source closure를 요구한다. 따라서 raw
AOT determinism과 standalone source-free distribution은 false로 남는다.

첫 compiler-stage family도 좁게 closed:

- profile `pnix.clr-meta.checked-i64-expression.v1`은 exact Int64 literal,
  `arg`, checked binary `+`, `-`, `*`만 admit;
- host compiler가 두 ClojureCLR-written compiler assembly를 한 번 seed;
- compiler implementation source가 숨겨진 상태에서 seed가 managed PE를 직접
  emit하고, admitted target form을 host compile/eval에 위임하지 않음;
- 모든 carried file + compiler/profile/plan/source와 private pinned runtime
  snapshot이 hash-bound·checked;
- compiler source가 이 tiny target profile 밖이므로 Stage2 readiness,
  self-reproduction, 모든 higher stage는 false로 남음.

Pinned ClojureCLR startup source compilation은 explicit TCB이며, target-form
fallback으로 세거나 source-free process claim으로 숨기지 않는다.

Stage2로의 경로는 이제 checked-Int64 expression family의 silent widening이
아니라 별도 compiler family다:

- `pnix.clr-meta.compiler-kernel.v1`이 macro-free canonical compiler kernel
  source, exact source-language profile, low-level reader/data 및 PE-sink
  support ABI, compiler/receipt ABI contract를 freeze;
- C1 admission 게이트가 evaluation 없이 읽고 모든 source node, lexical
  binding, global call, support call, sink call을 recursively 집계;
- C2가 exact reader/data/environment와 stack-verified transactional PE-sink
  ABI를 구현한 뒤, B0가 nine-field/27-method Compiler Stage1 PE를 emit;
- public builder가 C1 admission을 필수로 하고, seed environment를 clear하며,
  B0 전후 complete pinned runtime closure를 hash하고, no-replace
  compiler/support bundle을 publish;
- generated Stage1이 compiler source나 ClojureCLR 없이 실행되고,
  post-Stage1 nonce target과 arithmetic/equality/truthiness target을
  compile하며, 세 mutation anchor 모두 propagate;
- C2의 historical artifact와 receipt는 `compiler_stage2=false`를 유지;
  later checkpoint가 그 boundary를 rewrite하지 않음;
- C3가 exact C2 parent를 검증하고 Stage1이 같은 canonical kernel source의
  byte-exact frozen copy를 runnable Stage2로 compile하게 함;
- override-style C3 bundle은 Stage2, copied support triplet, 자체 manifest만
  포함; Stage1 PE, C2 manifest, canonical source, ClojureCLR은 없고 parent
  lineage는 hash로 유지;
- 별도 C3 replay가 compiler directory에 Stage2와 support만 두고,
  post-Stage2 nonce target을 compile하며, target과 support만 있는 다른
  fresh directory에서 target을 실행;
- C3 자체에서 Stage3+, self-reproduction, Stage15/N, fixed point, raw
  reproducibility, replacement, PNIX product integration, cross-host
  equivalence는 false로 남음; Stage3--7 same-source recompile은 later
  separately gated ladder (`STAGE{3--7}_DESIGN.md` 참조), C3 scope 아님.

여기서 `C0`와 `C1`은 admission-checkpoint 이름이지 compiler stage 번호가
아니다. Static source admission은 self-compilation에 필요하지만 그 자체가
compiler artifact나 self-hosting 결과가 아니다. `C2`는 separately gated
host-seeded Compiler Stage1 artifact이며 여전히 Stage2 self-compilation이
아니다. `C3`는 distinct Stage1-to-Stage2 same-source transition과
source-hidden fresh-target replay다. Stage3 convergence나 compiler
self-reproduction이 아니다.

C1 receipt는 37 top-level form, 36 definition, 2,237 recursive node를
unknown/rejected/interpreted/opaque/payload node zero로 집계한다. 33 support
call, 네 intrinsic, 12 lowering-owner row, 세 future semantic-mutation
anchor를 바인딩한다. focused 4-test / 288-assertion 게이트가 23
malformed, crossed, forbidden, mutated case를 output receipt 없이 거부한다.
Receipt SHA-256:
`3a1635882bfcdf67c50a90cbc058100c496d59eba71b11a2271bd24492302741`.

C2 focused contract test는 4 tests / 37 assertions를 통과한다. executable
게이트가 27 generated method를 모두 prepare하고, 62-file pinned bootstrap
runtime closure를 검증하며, source-hidden post-Stage1 nonce target과
7,900-node near-budget target을 실행하고, 16 structured no-output failure와
네 publication-preservation case를 검사하며, historically
`compiler_stage2=false`를 유지한다. Identity mutation이 generated metadata를
바꾼다. Add/subtract lowering mutation이 compiler 자체 control arithmetic을
바꿔 예측된 `bad-def-arity`/`call-arity` no-output rejection을 낸다; 이는
propagation이지, 그 mutated compiler가 여전히 swapped target arithmetic을
구현한다는 claim이 아니다.

C3 artifact manifest는 `compiler_stage2=true`, exact source hash chain,
parent/child/support/toolchain lineage, 27 prepared method, exact
Stage1/Stage2 structural-description equality를 기록한다. artifact builder가
replay 게이트가 아니므로 의도적으로 `stage2_fresh_target_replay=false`를
기록한다. 별도 C3 gate receipt가 isolated post-Stage2 nonce target이
compile·execute된 뒤에만 그 gate-owned claim을 true로 바꾼다. Raw PE equality는
그 structural comparison이 require하거나 promote하지 않는다.

## Compiler stage

Compiler stage는 evaluator generation과 다른 namespace 및 receipt family에서
시작한다:

1. **Compiler Stage1** — admitted ClojureCLR-written compiler가 bootstrap
   compiler에 의해 seed되고, supported language surface를 집계하며,
   verifiable CLR artifact를 emit하고, 그 admitted surface 안에 hidden
   host-compiler fallback이 없음.
2. **Compiler Stage2** — Stage1 compiler가 compiler source를 compile하고
   runnable next-generation compiler artifact를 생산.
3. **Compiler Stage3--7** — 각 previous compiler가 같은 closed source를
   compile; semantic observation과 explicitly normalized artifact identity가
   fresh load 아래에서 converge.
4. **Stage8** — reproducible assembly artifact closure, PE metadata, MVID,
   debug information, path, timestamp에 대한 explicit policy 포함.
5. **Stage9** — clean-process compiler/runtime replay.
6. **Stage10** — isolated load-context, classpath, session, sandbox replay.
7. **Stage11** — source, IR, compiler, runtime, compatibility surface 전역
   one accepted/failed boundary.
8. **Stage12** — compiler change는 replay와 gate admission까지 quarantine.
9. **Stage13** — long-horizon stale artifact, cache, source-drift closure.
10. **Stage14** — cross-implementation law와 differential receipt.
11. **Stage15** — external evidence는 replay와 explicit admission까지
    evidence-only.
12. **StageN** — 새로 바인딩된 모든 runtime, adapter, proof, product surface가
    complete applicable closure ledger를 replay.

Stage15/N hardening이 Stage2 self-compilation을 대체할 수 없다. 각 단계는
자체 receipt를 요구한다; shared label이나 deep evaluator chain은 compiler
fixed point가 아니다.

## 실제 replacement를 향한 순서

1. `bin/clojure-clr-bootstrap`를 explicit pinned trust root로 유지.
2. checked-Int64 expression family를 narrow Stage1 receipt로 frozen 유지;
   `Run(Int64)` target ABI를 relabel하여 compiler로 만들지 말 것.
3. 별도 버전 selfhost family를 explicit checkpoint로 구축: C0 attack/ABI
   contract, C1 complete recursive source admission, real host-seeded
   Compiler Stage1 artifact, semantic mutation propagation이 C2에서 closed;
   C3가 이제 Stage1-to-Stage2 exact same-source compilation과 source-hidden
   fresh-target replay도 닫음.
4. **Compiler Stage3–7 closed** (2026-08-07): successive same-source
   recompile Stage2→3→4→5→6→7; structural description이 parent와 equal;
   source-hidden fresh-target replay PASS (`STAGE{3,4,5,6,7}_DESIGN.md`,
   `scripts/clr-meta-compiler-selfhost-stage{3,4,5,6,7}-gate`).
   **Stage8 closed** (2026-08-12): reproducible assembly artifacts — 같은
   frozen Stage6에서 두 independent Stage7 build가 explicit,
   empirically-derived PE-field canonicalization policy 아래 byte-identical
   (`STAGE8_DESIGN.md`, `scripts/clr-meta-compiler-selfhost-stage8-gate`).
   **Stage9 closed** (2026-08-12): clean-process compiler/runtime replay —
   `bin/clr-meta` 자체 (Stage1-8이 이미 exercise하는 support DLL 아님)가
   entrypoint matrix를 두 fully clean (`env -i`) process invocation 사이에서
   byte-identical하게 replay (`STAGE9_DESIGN.md`,
   `scripts/clr-meta-compiler-selfhost-stage9-gate`).
   **Stage10–15/N closed** (2026-08-12, 같은 날): 각 stage는 `proofs/` 아래
   policy table (`session-sandbox.tsv`, `adapter-schema.tsv`,
   `quarantine-policy.tsv`, `horizon-policy.tsv`, `cross-impl-schema.tsv`,
   `evidence-federation.tsv`, `extension-policy.tsv`)로 관련 boundary마다
   explicit DONE/GROW/HELD/DISABLED stance를 선언하고, DONE인 것의 live
   replay — 각 stage가 immediate predecessor를 정확히 한 번 호출
   ("두 번" 아님, early draft가 맞은 quadratic cost blowup 회피), 전체
   chain을 Stage9로 anchor
   (`STAGE{10,11,12,13,14,15,N}_DESIGN.md`,
   `scripts/clr-meta-compiler-selfhost-stage{10,11,12,13,14,15,n}-gate`).
   **Compiler self-reproduction / B==C fixed point closed** (2026-08-12, 같은
   날): 처음부터 만든 것이 아니라 발견 — Stage8 자체 게이트 출력이 이미
   Stage3-7이 하나의 compiled-artifact sha256을 공유함을 unplanned bonus
   observation으로 로깅.
   `scripts/clr-meta-compiler-self-reproduction-check`가 이를 전용 receipt로
   형식화: Stage1부터 Stage7까지 fresh rebuild, 모두 exact same sha256
   공유 — adjacent B==C pair만이 아니라 일곱 전부 Stage1 host-seeded
   build 포함 (`SELF_REPRODUCTION_DESIGN.md`).
6. exact `-e`, file, REPL, compile/AOT, namespace/load, tooling
   compatibility profile을 개별 admit.
7. 그 게이트 이후에만 `bin/clojure-clr` 이름이 현재 bootstrap-hosted
   `-e`/single-file facade 너머로 확장되고, broader command profile이
   generated `clr-meta` compiler product로 이전될 수 있음.
8. compiler/command replacement에서 runtime·ecosystem compatibility로
   확장할 때는 separately named profile을 통해서만.
9. `pnix-clr`를 common PNIX compiler/machine model에 독립적으로 연결하고,
   host promotion 전 all-admitted-host canonical 게이트를 실행.

## Open claims

다음 named 게이트가 존재하고 통과할 때까지 false로 남는다:

```text
compiler_stage1_checked_i64_expression_profile = true
selfhost_family_contract_v1 = true
selfhost_family_recursive_source_admission = true
selfhost_family_executable_stage1_artifact = true
selfhost_family_mutation_propagation = true
selfhost_family_executable_stage2_artifact = true
selfhost_family_stage2_fresh_target_replay = true
compiler_stage3 = true
compiler_stage4 = true
compiler_stage5 = true
compiler_stage6 = true
compiler_stage7 = true
compiler_stage8 = true
compiler_stage9 = true
compiler_stage10 = true
compiler_stage11 = true
compiler_stage12 = true
compiler_stage13 = true
compiler_stage14 = true
compiler_stage15 = true
compiler_stagen = true
compiler_self_reproduction = true (compiler_stage1_through_7_persisted_assembly_builder_output only; see SELF_REPRODUCTION_DESIGN.md)
clr_il_fixed_point = false (a general IL fixed-point claim is broader than the above scoped self-reproduction result)
raw_aot_rebuild_determinism = true (compiler_stage7_persisted_assembly_builder_output only; see STAGE8_DESIGN.md)
broad_clojureclr_compatibility = false
clojureclr_replacement = false
standalone_source_free_distribution = false
standalone_lineage_replay = false
pnix_common_compiler_integration = false
pnix_product_compiler = false
cross_host_canonical_equivalence = false
clr_host_promotion = false
```

.NET, CLR/BCL, 명시적으로 유지되는 ClojureCLR runtime substrate는 compiler나
command profile을 닫는 것만으로 제거되지 않는다. Differential agreement와
self-hosting은 implementation 증거이지 formal correctness proof가 아니다.
