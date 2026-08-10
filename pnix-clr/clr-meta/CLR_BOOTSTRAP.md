# clr-meta bootstrap lane

This is the first CLR-native, pnix-agnostic proof lane in `clr-meta`. The
copied JVM meta sources were pruned rather than relabelled; Git history and the
reference host remain available for deliberate ports.

## Claim

The ClojureCLR host compiler seeds one small evaluator.  That evaluator
interprets its own source to produce evaluator generation 1, and generation 1
interprets the same source to produce generation 2. All three generations must
agree on a focused corpus covering literals, symbols, quote, both `if`
branches, sequential `let`, closure capture, named recursion, and variadic rest
binding.

The gate claims evaluator self-interpretation only.  Its receipt explicitly
does not claim ClojureCLR compiler self-reproduction, a CLR IL fixed point, the
full Clojure language surface, a host-free bootstrap, or PNIX semantics.

The receipt's historical `:stage-chain` numbers 0, 1, and 2 identify these
evaluator generations. They must not be read as compiler stages. In particular,
they do not establish any compiler stage; the separately evidenced checked-I64
Stage1 slice below does. A live attempt to extend the nested evaluator through
15 self-extensions exhausts the CLR stack;
that host resource failure is recorded as an open limitation, not converted to
`Held` and not counted as stage evidence.

The normal `clr-meta -e` and file tool paths now force evaluator generation 2
and use the same closed primitive environment plus a strict one-form host
reader. Reader evaluation is false, data readers are replaced with inert
tagged values, EOF is required, and maps/sets/regexes/tagged or conditional
reader values outside the documented scalar/list/vector domain are rejected
before evaluation. They do not call `load-string`. The host reader, ClojureCLR
runtime, and initial compiler remain explicit substrate boundaries.

## Artifact-production slice

`pnix.clr-meta.runtime-artifact` is a second, PNIX-agnostic mechanism. A caller
provides a strict namespace plan and exact source root. The builder validates
that closure and pairwise-disjoint paths, invokes the pinned host ClojureCLR
AOT compiler in a fresh child with only that source root on its load path,
rejects undeclared namespace dependencies, verifies the
exact DLL set, and atomically publishes a JSON manifest binding plan, source,
output, entry, target, and byte hashes.

The current `pnix-clr` plan declares nine namespaces and produces exactly nine
DLLs with backend identity `host-clojureclr-aot`. The `pnix-clr` product runner
validates the manifest and live plan/source/output closures, replaces its load
path with the artifact, changes cwd to that verified tree, rejects product
namespace shadows in the pinned runtime lookup roots, and fails closed instead
of compiling or loading product source. This establishes an artifact dependency; it is not
self-compilation. It also does not prove raw rebuild determinism because no
second-build byte-equality claim has been admitted.

## Compiler Stage1 slice

The separate `pnix.clr-meta.compiler-stage1-*` family closes Compiler Stage1
only for `pnix.clr-meta.checked-i64-expression.v1`: exact `System.Int64`
literals, `arg`, and checked binary `+`, `-`, `*`. A host-AOT seed containing
the pure lowering core and CLR backend is produced once through the current
process's absolute dotnet host and a clear-then-allowlist environment; with compiler source
removed from its load path, that seed emits and JIT-verifies a managed console
PE and the gate executes dynamic arguments in fresh processes. Exact profile,
plan, target source, compiler source/AOT, six carried bundle files, and the
complete Clojure publish-directory snapshot have closure hashes. CoreCLR and
the BCL remain external TCBs rather than members of that snapshot.

This is not a host-compiler-free process. Pinned `Clojure.Main` startup source
compilation and the strict EDN reader remain declared TCB boundaries. What is
closed is the admitted-form boundary: target form traversal, semantics,
lowering, and opcode selection are ClojureCLR-written and have zero host
compiler/evaluator fallback. Self-source classification explicitly reports all
compiler top-level forms outside the tiny profile, so `stage2_ready=false`.

## Selfhost compiler C0--C3 slice

The checked-I64 Stage1 identity is frozen. A separate
`pnix.clr-meta.compiler-kernel.v1` family begins with two non-executable
checkpoints and two executable checkpoints:

- C0 fixes the compiler source profile, compiler/support boundary, generated
  compiler ABI, forbidden fallback and payload surfaces, lineage requirements,
  and semantic mutation probes;
- C1 strict-reads and recursively classifies the entire canonical macro-free
  compiler kernel source, including lexical bindings and exact global,
  support, and PE-sink call arities;
- C2 uses the explicit pinned-host B0 boundary to emit and execute Compiler
  Stage1 through the separately frozen CLR support ABI;
- C3 makes that generated Stage1 compile the exact same canonical kernel source
  into a runnable Stage2, then separately replays Stage2 against a fresh target
  with the compiler source and parent artifact absent.

The admission analyzer does not evaluate or compile the kernel. It publishes a
hash-bound receipt only after every source node is classified and no unknown
symbol, macro, metadata, reader escape, arbitrary interop, or undeclared ABI
call remains. Negative mutations must leave no receipt.

The canonical closure contains 37 top-level forms, 36 definitions, and 2,237
recursively classified nodes. All 33 support ABI calls, four intrinsics,
twelve lowering owners, and three semantic mutation anchors are hash-bound.
The focused gate passes 4 tests / 288 assertions and 23 no-output negative
cases; its receipt SHA-256 is
`3a1635882bfcdf67c50a90cbc058100c496d59eba71b11a2271bd24492302741`.

The C0/C1 checkpoint names are not compiler stages, and the C1 receipt itself
continues to keep every executable claim false. The separate C2 gate now closes
the executable selfhost Compiler Stage1 artifact. Its public builder first
runs C1 admission, clears the B0 child environment to an allowlist, hashes all
regular files in the pinned ClojureCLR runtime before and after seeding, and
publishes a compiler/support bundle with an atomic no-replace directory move.
The bundle contains neither the canonical compiler source nor ClojureCLR.

The generated compiler has nine object constants and 27 public static object
methods. The PE sink verifies method-local handles, stack height at every
operation and branch join, label closure, return placement, and finish-only
publication. The execution gate prepares every generated method and, in a
source-hidden process, runs checked arithmetic, equality, nil/false/zero
truthiness, a target containing a post-Stage1 random nonce, and a 7,900-node
near-budget sequence. Sixteen malformed/profile/closure cases leave no output;
four builder races/existing-output cases preserve the winner.

All three C2 mutation anchors are executable evidence. The identity change
appears in generated target metadata. Changing add or subtract lowering also
changes the compiler's own control arithmetic, causing the exact structured
`validate/bad-def-arity` or `validate/call-arity` failure with no output. Those
outcomes prove propagation but do not claim a still-functional arithmetic-swap
compiler. The C2 manifest and receipt retain their historical
`compiler_stage2=false` boundary.

C3 is an override-style child rather than a rewrite of C2. Its builder first
validates the exact C2 manifest, input/bundle closures, live Stage1 description,
support triplet, and canonical source hash. It freezes an explicit byte-exact
private source copy and invokes Stage1 in a fresh allowlisted process. The
resulting Stage2 has the same nine fields, 27 prepared methods, metadata,
references, resources, and callable entry shape as Stage1. Raw Stage1/Stage2 PE
equality or inequality is not a gate condition.

The C3 child bundle owns only `CompilerStage2.dll`, the copied support triplet,
and its C3 manifest. It does not package the Stage1 PE, C2 manifest, canonical
compiler source, or ClojureCLR; the manifest retains parent lineage by hashes.
The builder manifest closes `compiler_stage2=true` and
`same_source_recompile=true`, while correctly leaving
`stage2_fresh_target_replay=false` for the gate to own.

The C3 gate places only Stage2 and the support triplet in the compiler replay
directory. It creates a random nonce source after Stage2 exists, proves that
nonce absent from Stage1, Stage2, and support, compiles it with Stage2, and
executes the target in a second fresh directory containing only the target and
support triplet. Delayed identity/add/subtract mutations are observed one
generation later in grandchild targets. The C3 gate receipt therefore closes
`compiler_stage2=true` and `stage2_fresh_target_replay=true`, but not Stage3 or
self-reproduction.

## Run

From `clr-meta/`:

```sh
scripts/clr-meta-gate
```

If `Clojure.Main` was already built:

```sh
scripts/clr-meta-gate --no-build
```

The runner uses the bundled ClojureCLR `Clojure.Main` project and `net10.0`.
`bin/clojure-clr-bootstrap` names that trust root, while `bin/clojure-clr` is a
focused `-e`/single-file generation-2 facade that remains hosted by it.

Success requires the focused tests and bootstrap receipt to contain `:ready
true`, plus the independent checked-I64 Compiler Stage1 receipt, selfhost C2
gate receipt, and C3 Stage2 gate receipt to pass. The C2 builder executes C1
admission as its mandatory first step; the C3 builder requires and revalidates
the closed C2 artifact rather than silently reseeding it.

## Open target

Compiler Stage3--15/N, compiler self-reproduction, an IL fixed point, raw
reproducibility, broad ClojureCLR compatibility/replacement, standalone
lineage replay, PNIX product/common-compiler integration, cross-host canonical
equivalence, and CLR host promotion remain open. Closing the PNIX-agnostic C3
Stage2 artifact does not make any of those claims true. Their required ordering
and distinct receipts are defined in `STAGE15_N_ROADMAP.md`.
