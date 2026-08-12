# clr-meta status (peer host-meta floor)

Last verified: 2026-08-12.

## Peer-floor statement

**clr-meta** is the PNIX-agnostic ClojureCLR host bootstrap beneath `pnix-clr`.
Practical peer floor matches other host metas for **product substrate**, with
an honest Stage3–15/N ladder still open:

| Peer | Peer floor | clr-meta counterpart |
|---|---|---|
| JVM Clojure host | bytecode selfhost | eval gen0–2 + C0–C3 Stage1/2 |
| Hy host | stage ladder / fixed-point | C3 Stage2 + source-hidden fresh-target replay |
| Rust host | TV + stage chain | checked-I64 Stage1 + selfhost PE emit |
| ClojureScript host | fixed-point compiler | Stage2 same-source recompile (not full IL fixed point) |

Meta-first order: `clr-meta` before `pnix-clr`. Artifact builder + hash-bound
load path closed. Stage3–15/N remains roadmap (`STAGE15_N_ROADMAP.md`), same
honesty as not claiming Stage15 replacement on clj/rs/cljs.

## Closed claims

Live-verified this session (2026-08-07) via `./bin/clr-meta-gate eval-only`:

```text
bootstrap-test (gen0→1→2 self-interpretation)  ready=true
  18 tests / 171 assertions, 0 fail / 0 error
  all corpus cases stage-values agree across gens
tool-gate (-e / file gen2 + strict reader)     PASS
  (+ 20 22) => 42 via evaluator-generation-2
  reader-eval / tagged / trailing / map rejected
```

Documented closed (heavy C1–C3 gates; not re-run full chain this session):

```text
checked-I64 Compiler Stage1 family
selfhost C1 admission  receipt 3a163588…
selfhost C2 executable Stage1 artifact
selfhost C3 Stage2 + source-hidden fresh-target replay
host-clojureclr-aot runtime artifact builder
```

## Closed this wave (2026-08-07) — Compiler Stage3–7 + path fix

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

## Closed this wave (2026-08-12) — Compiler Stage8 reproducible assembly artifacts

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

## Closed this wave (2026-08-12, same day) — Compiler Stage9 clean-process replay

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

## Closed this wave (2026-08-12, same day) — Compiler Stage10–15/N + StageN

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

## Closed this wave (2026-08-12, same day) — compiler self-reproduction / B==C fixed point

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

## Open claims (do not claim)

```text
clr_il_fixed_point = false
broad_clojureclr_compatibility / replacement = false
pnix_common_compiler_integration = false
cross_host_canonical_equivalence / clr_host_promotion = false
```

Stage1 through StageN, and compiler self-reproduction, are now ALL closed
(see the "Closed this wave" sections above) — `promotion/allowed?` stays
`false` on every one of them regardless, since none of this closes a general
CLR IL fixed point (this is scoped to the Compiler Stage1-7
`PersistedAssemblyBuilder` output specifically, not every artifact kind this
repo could ever produce) or broad ClojureCLR replacement, which remain the
actual promotion gates.

`raw_aot_rebuild_determinism` moved out of this block (2026-08-12): Stage8
closes it for the Compiler Stage1-7 `PersistedAssemblyBuilder` artifact family
specifically. It is not a general claim about every artifact this repo could
ever produce — a future codegen path that writes debug info would need its
own determinism check, per `stage8-contract.edn`'s explicit non-claims.

## Trusting-Trust defense roadmap (Diverse Double-Compiling)

Unlike Rust (`mrustc`), there is no independently-authored third-party
ClojureCLR compiler in the wild to lean on — a second, independent backend
has to be built in-house, the same constraint the reference JVM host
(the `frontend_selfhost.clj`/`diverse_double_compile.clj` pair) already
worked through for its own DDC witness, and the same pattern followed here.

**Independent mini backend added this session (2026-08-11):**
`independent_mini_backend.clj` is a new, from-scratch Int64
tokenizer/reader + analyzer + `System.Reflection.Emit.DynamicMethod` IL
emitter, sharing zero code with the Compiler Stage1-7 family
(`compiler_stage1.clj`, `compiler_selfhost_*.clj`), which uses
`System.Reflection.Emit.PersistedAssemblyBuilder` to produce full PE
executables. `DynamicMethod` JITs a method in memory and hands back an
invokable handle directly — it never touches the assembly/PE-writing path
the Stage1-7 family shares. The pinned ClojureCLR runtime and the CLR itself
remain trusted host substrate, the same honest role the JVM classfile format
plays for the reference host's tiny frontend witness.

Covers 15 value-returning fixtures (`+`/`-`/`*` checked arithmetic,
`<`/`>`/`<=`/`>=`/`=` comparisons, `if` including nested `if`, 0/1/2/3/4-arg
functions) plus 4 checked-overflow negative fixtures (both the real host and
the mini backend must reject `Int64.MaxValue + 1`, `Int64.MinValue - 1`,
`Int64.MaxValue * 2`, and `Int64.MaxValue + Int64.MaxValue`). Cross-validated
against real host ClojureCLR `eval` — both agree on all 19. Wired into
`independent-mini-backend-test` (`clr-meta/test/pnix/clr_meta/`), which now
runs as part of the aggregate `bootstrap-test` entry point invoked by
`scripts/clr-meta-gate`. Verified live this session (2026-08-11, later
widening pass): the namespace's own test run shows `{:test 2, :pass 38,
:fail 0, :error 0}`; full `bin/clr-meta-gate --no-build` re-run shows
`{:test 20, :pass 209, :fail 0, :error 0}, :ready true` with no regressions.

**What this closes and what it still doesn't:** a genuine 2-way behavioral
comparison (real host `eval` ≡ from-scratch `DynamicMethod`-based mini
backend) now exists and passes on both the success surface and the
checked-overflow negative surface, not just a documented plan. Nested `if`
and more function arities close the "not the full Stage1 profile shape" gap
noted here previously; the checked-overflow fixtures close the "negative
cases not exercised" gap — the mini backend's `Add_Ovf`/`Sub_Ovf`/`Mul_Ovf`
IL opcodes were always checked (matching the Compiler Stage1 profile's
`:overflow :system-overflow-exception`), they just weren't tested until now.
It is still a bounded fixture set, not the full conformance corpus, and (same
honest bar settled on for every host this session) behavior equivalence, not
byte-identical IL, since a `DynamicMethod`-JITted method and a
`PersistedAssemblyBuilder`-written PE are different CLR artifact kinds by
construction.

**Next concrete step:** Stage8 (reproducible assembly artifacts for the
`PersistedAssemblyBuilder`-based Compiler Stage1-7 family) is the next
concrete work item, not further mini-backend widening — see `todo.md`.
Building an independent interpreter (as opposed to a second compiler) to
cross-check the gen0-2 evaluator lane remains a separate, not-yet-started
track — an interpreter alone would not clear the full Wheeler bar even if
added.

## Primary gate

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

Gate scripts expect `rg` (ripgrep). Prefer **`/usr/local/bin/rg`**. Do **not**
put `pnix-clr/bin` first on `PATH` — that tree may ship an old `rg` shim.

## Last run (this machine, 2026-08-07)

| Gate | Result | Notes |
|---|---|---|
| `./bin/clr-meta-gate eval-only` | **PASS** | ready=true; tool-gate PASS |
| full C1–C3 chain | not re-run this session | scripts/*-gate; docs claim closed |
| `./scripts/clr-meta-compiler-selfhost-stage3-gate` | **PASS** | Stage2→Stage3 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage4-gate` | **PASS** | Stage3→Stage4 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage5-gate` | **PASS** | Stage4→Stage5 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage6-gate` | **PASS** | Stage5→Stage6 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage7-gate` | **PASS** | Stage6→Stage7 + source-hidden replay |
| env | dotnet 10.0.302, published Clojure.Main.dll | OK |
