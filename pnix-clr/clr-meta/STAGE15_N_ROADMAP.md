# clr-meta Stage15/N and ClojureCLR replacement roadmap

Status: checked-Int64 expression Compiler Stage1 closed; the separate
selfhost-family C0 contract and C1 recursive source-admission checkpoint are
closed; executable selfhost Compiler Stage1 is closed at C2; same-source
executable Compiler Stage2 and its source-hidden fresh-target replay are closed
at C3; the same-source Stage3--7 recompile ladder is live-gated CLOSED (2026-08-07)
with `promotion/allowed?=false`; Stage8 (reproducible assembly artifacts) is
live-gated CLOSED (2026-08-12), also `promotion/allowed?=false`; Stage9--15/N
and replacement remain OPEN.
Truth owners: the monorepo constitution, CLR source, artifact manifests, and
future stage receipts. This page is a roadmap, not closure evidence.

## Identity

`clr-meta` is the PNIX-agnostic ClojureCLR host-language layer. It may expose a
generic CLR compiler/artifact service, but it does not own PNIX syntax or
portable PNIX meaning. `pnix-clr` supplies its product plan and uses the
resulting CLR artifact; `pnix-meta` remains the owner of portable PNIX
evaluator/compiler models.

The intended composition is:

```text
pinned ClojureCLR bootstrap trust root
  -> clr-meta host-language evaluator/compiler/artifact mechanisms
  -> pnix-clr CLR mechanism and backend seam
  -> common pnix-meta meaning
```

Sharing a distribution or command directory does not merge these owners.

## Current proven boundary

The current evaluator lane has three physical generations:

```text
generation 0 = host-seeded focused evaluator
generation 1 = generation 0 interprets evaluator-source
generation 2 = generation 1 interprets evaluator-source
```

All three agree on the focused evaluator corpus. `clr-meta -e` and file mode
run generation 2 after host reading and do not call `load-string`. These are
evaluator generations, not compiler stages. Extending this nested interpreter
through 15 self-extensions was tried live and exhausted the CLR stack. That
host resource failure is neither a language `Held` result nor Stage15 evidence.

The current artifact lane is separately closed only at this boundary:

- a generic `clr-meta` builder validates one exact namespace plan and source
  closure;
- the backend is the pinned host ClojureCLR compiler, identified as
  `host-clojureclr-aot`;
- the `pnix-clr` plan produces exactly nine AOT namespace DLLs;
- the manifest binds the plan, ordered source rows, exact output rows, entry,
  target, and closure hashes;
- the product runner validates all of them and replaces its load path with the
  artifact directory; it never compiles or loads product source as fallback.

This proves a real artifact dependency. It does not prove that rebuilding the
artifact produces byte-identical DLLs, and startup still requires the pinned
ClojureCLR runtime plus the live plan/source closure for validation. Therefore
raw AOT determinism and a standalone source-free distribution remain false.

The first compiler-stage family is also closed, narrowly:

- profile `pnix.clr-meta.checked-i64-expression.v1` admits only exact Int64
  literals, `arg`, and checked binary `+`, `-`, `*`;
- the host compiler seeds two ClojureCLR-written compiler assemblies once;
- with compiler implementation source hidden, the seed directly emits a
  managed PE and never delegates an admitted target form to host compile/eval;
- every carried file plus compiler/profile/plan/source and a private pinned
  runtime snapshot is hash-bound and checked;
- the compiler source is outside this tiny target profile, so Stage2 readiness,
  self-reproduction, and every higher stage remain false.

Pinned ClojureCLR startup source compilation is explicit TCB, not counted as
target-form fallback or hidden as a source-free process claim.

The route to Stage2 is now a separate compiler family rather than a silent
widening of the checked-Int64 expression family:

- `pnix.clr-meta.compiler-kernel.v1` freezes a macro-free canonical compiler
  kernel source, its exact source-language profile, its low-level reader/data
  and PE-sink support ABI, and the compiler/receipt ABI contract;
- the C1 admission gate reads without evaluation and accounts recursively for
  every source node, lexical binding, global call, support call, and sink call;
- C2 implements the exact reader/data/environment and stack-verified
  transactional PE-sink ABI, then B0 emits a nine-field/27-method Compiler
  Stage1 PE;
- the public builder makes C1 admission mandatory, clears the seed environment,
  hashes the complete pinned runtime closure before and after B0, and publishes
  a no-replace compiler/support bundle;
- the generated Stage1 runs without the compiler source or ClojureCLR, compiles
  a post-Stage1 nonce target plus arithmetic/equality/truthiness targets, and
  propagates all three mutation anchors;
- C2's historical artifact and receipt keep `compiler_stage2=false`; no later
  checkpoint rewrites that boundary;
- C3 validates that exact C2 parent and makes Stage1 compile a byte-exact frozen
  copy of the same canonical kernel source into a runnable Stage2;
- the override-style C3 bundle contains only Stage2, its copied support triplet,
  and its own manifest, with the Stage1 PE, C2 manifest, canonical source, and
  ClojureCLR absent and parent lineage retained by hashes;
- a separate C3 replay puts only Stage2 and support in the compiler directory,
  compiles a post-Stage2 nonce target, and executes the target in another fresh
  directory containing only that target and support;
- At C3 itself Stage3+, self-reproduction, Stage15/N, fixed points, raw
  reproducibility, replacement, PNIX product integration, and cross-host
  equivalence remain false; Stage3--7 same-source recompile is a later
  separately gated ladder (see STAGE{3--7}_DESIGN.md), not C3 scope.

`C0` and `C1` here are admission-checkpoint names, not compiler stage numbers.
Static source admission is necessary for self-compilation, but is not itself a
compiler artifact or a self-hosting result. `C2` is the separately gated
host-seeded Compiler Stage1 artifact; it is still not Stage2 self-compilation.
`C3` is the distinct Stage1-to-Stage2 same-source transition and source-hidden
fresh-target replay. It is not Stage3 convergence or compiler
self-reproduction.

The C1 receipt accounts for 37 top-level forms, 36 definitions, and 2,237
recursive nodes with zero unknown/rejected/interpreted/opaque/payload nodes.
It binds 33 support calls, four intrinsics, twelve lowering-owner rows, and
three future semantic-mutation anchors. The focused 4-test / 288-assertion
gate rejects 23 malformed, crossed, forbidden, or mutated cases without an
output receipt. Receipt SHA-256:
`3a1635882bfcdf67c50a90cbc058100c496d59eba71b11a2271bd24492302741`.

The C2 focused contract tests pass 4 tests / 37 assertions. Its executable gate
prepares all 27 generated methods, validates a 62-file pinned bootstrap runtime
closure, executes a source-hidden post-Stage1 nonce target and a 7,900-node
near-budget target, checks 16 structured no-output failures and four
publication-preservation cases, and historically keeps
`compiler_stage2=false`. Identity
mutation changes generated metadata. Add/subtract lowering mutations alter the
compiler's own control arithmetic and yield the predicted
`bad-def-arity`/`call-arity` no-output rejections; this is propagation, not a
claim that those mutated compilers still implement swapped target arithmetic.

The C3 artifact manifest records `compiler_stage2=true`, the exact source hash
chain, parent/child/support/toolchain lineage, 27 prepared methods, and exact
Stage1/Stage2 structural-description equality. It deliberately records
`stage2_fresh_target_replay=false`, because the artifact builder is not the
replay gate. The separate C3 gate receipt changes only that gate-owned claim to
true after the isolated post-Stage2 nonce target compiles and executes. Raw PE
equality is neither required nor promoted by that structural comparison.

## Compiler stages

Compiler stages start in a different namespace and receipt family from the
evaluator generations:

1. **Compiler Stage1** — an admitted ClojureCLR-written compiler is seeded by
   the bootstrap compiler, accounts for its supported language surface, emits
   verifiable CLR artifacts, and has no hidden host-compiler fallback inside
   that admitted surface.
2. **Compiler Stage2** — the Stage1 compiler compiles the compiler source and
   produces a runnable next-generation compiler artifact.
3. **Compiler Stage3--7** — each previous compiler compiles the same closed
   source; semantic observations and explicitly normalized artifact identities
   converge under fresh loads.
4. **Stage8** — reproducible assembly artifact closure, including an explicit
   policy for PE metadata, MVIDs, debug information, paths, and timestamps.
5. **Stage9** — clean-process compiler/runtime replay.
6. **Stage10** — isolated load-context, classpath, session, and sandbox replay.
7. **Stage11** — one accepted/failed boundary across source, IR, compiler,
   runtime, and compatibility surfaces.
8. **Stage12** — compiler changes remain quarantined until replay and gate
   admission.
9. **Stage13** — long-horizon stale artifact, cache, and source-drift closure.
10. **Stage14** — cross-implementation law and differential receipts.
11. **Stage15** — external evidence stays evidence-only until replay and
    explicit admission.
12. **StageN** — every newly bound runtime, adapter, proof, or product surface
    replays the complete applicable closure ledger.

Stage15/N hardening cannot substitute for Stage2 self-compilation. Each step
requires its own receipt; a shared label or a deep evaluator chain is not a
compiler fixed point.

## Ordering toward an actual replacement

1. Keep `bin/clojure-clr-bootstrap` as the explicit pinned trust root.
2. Keep the checked-Int64 expression family frozen as its narrow Stage1
   receipt; do not turn its `Run(Int64)` target ABI into a compiler by relabeling
   it.
3. Build the separately versioned selfhost family in explicit checkpoints:
   C0 attack/ABI contract, C1 complete recursive source admission, a real
   host-seeded Compiler Stage1 artifact, and semantic mutation propagation are
   closed at C2; C3 now also closes Stage1-to-Stage2 exact same-source
   compilation and source-hidden fresh-target replay.
4. **Compiler Stage3–7 closed** (2026-08-07): successive same-source
   recompiles Stage2→3→4→5→6→7; structural descriptions equal parent;
   source-hidden fresh-target replay PASS (`STAGE{3,4,5,6,7}_DESIGN.md`,
   `scripts/clr-meta-compiler-selfhost-stage{3,4,5,6,7}-gate`).
   **Stage8 closed** (2026-08-12): reproducible assembly artifacts — two
   independent Stage7 builds from the same frozen Stage6 are byte-identical
   under an explicit, empirically-derived PE-field canonicalization policy
   (`STAGE8_DESIGN.md`, `scripts/clr-meta-compiler-selfhost-stage8-gate`).
   Next: Stage9 (clean-process compiler/runtime replay) and a separately
   named self-reproduction gate.
5. Close Stage9--15/N without making proof receipts control ordinary language
   execution.
6. Admit exact `-e`, file, REPL, compile/AOT, namespace/load, and tooling
   compatibility profiles individually.
7. Only after those gates may the `bin/clojure-clr` name expand beyond its
   current bootstrap-hosted `-e`/single-file facade and move broader command
   profiles to a generated `clr-meta` compiler product.
8. Expand from compiler/command replacement toward runtime and ecosystem
   compatibility only through separately named profiles.
9. Independently connect `pnix-clr` to the common PNIX compiler/machine model
   and run the all-admitted-host canonical gate before host promotion.

## Open claims

The following remain false until their named gates exist and pass:

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
compiler_stage9_through_15_n = false
compiler_self_reproduction = false
clr_il_fixed_point = false
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

.NET, the CLR/BCL, and any explicitly retained ClojureCLR runtime substrate are
not removed merely by closing a compiler or command profile. Differential
agreement and self-hosting are implementation evidence, not formal correctness
proofs.
