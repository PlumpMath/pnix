# clr-meta

`clr-meta` is the PNIX-agnostic ClojureCLR host bootstrap beneath
`pnix-clr`. It keeps four deliberately separate mechanisms: a focused evaluator
self-interpretation witness, a profile-qualified direct-IL Compiler Stage1,
an admitted and executable same-language selfhost Compiler Stage1-to-Stage2
family, and a generic host-ClojureCLR AOT runtime-artifact builder.

## Status / primary gate

See [STATUS.md](STATUS.md). Primary gate: `./bin/clr-meta-gate` (eval-only or full C0–C3 chain).

## Evaluator generations

```text
host ClojureCLR compiler -> generation 0 evaluator
generation 0 interprets evaluator source -> generation 1
generation 1 interprets evaluator source -> generation 2
```

All three generations agree on a focused Clojure corpus covering literals, quote,
conditionals, sequential bindings, closures, named recursion, and variadic
binding. The interpreted evaluator cannot call host `eval`.

These are physical evaluator generations, not Compiler Stage1, Stage2, or
Stage15/N. `clr-meta -e` and file mode use generation 2 after a strict host
reader has read exactly one form with `*read-eval*` disabled, inert data
readers, and a recursive check of the admitted list/vector/scalar domain;
trailing forms and host-created reader values fail before evaluation. The tool
path contains no `load-string`. A live attempt to extend this nested interpreter through 15
self-extensions exhausts the CLR stack. That is an open runtime limitation and
does not support a Stage15/N claim; it is not a stage receipt or a language
`Held` outcome.

## Compiler Stage1: checked Int64 expression profile

`pnix.clr-meta.checked-i64-expression.v1` is the first compiler-stage receipt
family, separate from evaluator generations and runtime artifacts. Its exact
source surface is one strict-EDN form made only from a `System.Int64` literal,
the dynamic parameter `arg`, and binary checked `+`, `-`, and `*`. Metadata,
BigInt/decimal/float values, unknown symbols/operators/arities, extra forms,
and byte/reader/node/depth budget overruns are structured rejections.

The pinned host compiler AOT-seeds two ClojureCLR-written compiler namespaces
through the current process's absolute dotnet host and a clear-then-allowlist
environment. A fresh child then sees the compiler AOT seed but not its implementation
source, owns validation/lowering/opcode selection, and writes a runnable
managed PE directly through `PersistedAssemblyBuilder` and CLR metadata APIs.
The target references only `System.Private.CoreLib` and `System.Console`; it
contains no ClojureCLR/evaluator resource. Dynamic arguments and checked
overflow for every admitted primitive are exercised in fresh processes.

```sh
./bin/clr-meta --build-compiler-stage1 \
  clr-meta/compiler-stage1/profile.edn \
  clr-meta/compiler-stage1/plan.edn \
  clr-meta/compiler-stage1/example.clj /tmp/clr-stage1
dotnet /tmp/clr-stage1/target/program.dll 7  # 27
```

The boundary is explicit: `Clojure.Main` still compiles pinned runtime startup
sources such as `clojure.main`, and `clojure.edn` is the reader TCB. The
builder copies and hashes the complete Clojure publish-directory snapshot,
while CoreCLR and the BCL remain external TCBs. It clears the inherited child
environment, scans earlier lookup roots, and proves only that admitted target
forms never fall back to host `compile`/`eval`. Therefore compiler
self-reproduction, Stage2--15/N, an IL fixed point, raw PE reproducibility,
broad ClojureCLR replacement, and a standalone source-free distribution remain
false.

## Selfhost compiler family: C0/C1 admission, C2 Stage1, and C3 Stage2

The checked-Int64 family stays frozen. Its `Run(Int64) -> Int64` artifact is a
real narrow Compiler Stage1 target, but it cannot express even one complete
top-level form of its current compiler implementation and cannot honestly be
renamed Stage2.

`compiler-selfhost/` therefore starts a separate
`pnix.clr-meta.compiler-kernel.v1` family. Its C0 contract fixes the canonical
compiler ABI, macro-free source profile, exact low-level reader/data and PE
sink support ABI, forbidden host compiler/evaluator/process surfaces, receipt
lineage, and three future anti-baking mutation sites. Its C1 gate strict-reads
the canonical source with reader evaluation disabled and recursively accounts
for every syntax node and lexical/global/support/sink reference.

The frozen source has 37 top-level forms and 36 definitions. Its 2,237
recursive nodes close with 33 support calls, four intrinsics, and twelve
explicit lowering-owner rows; unknown, rejected, interpreted, opaque, and
payload nodes are all zero. The focused gate passes 4 tests / 288 assertions
and 23 negative cases. Its deterministic receipt SHA-256 is
`3a1635882bfcdf67c50a90cbc058100c496d59eba71b11a2271bd24492302741`.

```sh
clr-meta/scripts/clr-meta-compiler-selfhost-admission-gate
```

That C1 receipt remains source admission rather than execution. C2 adds a
separate executable contract and `Pnix.ClrMeta.CompilerSupport`: a strict
bounded reader/data/environment ABI, stack- and control-flow-checked
transactional PE sink, and strict generated-artifact host. The public builder
runs C1 admission itself, snapshots the complete pinned ClojureCLR runtime
closure before and after the explicit B0 seed, and publishes a no-replace
bundle containing only the generated Compiler Stage1 PE and its three support
runtime files:

```sh
./bin/clr-meta --build-compiler-selfhost-stage1 /tmp/clr-selfhost-stage1
dotnet /tmp/clr-selfhost-stage1/runtime/Pnix.ClrMeta.CompilerSupport.dll \
  compile /tmp/clr-selfhost-stage1/compiler/CompilerStage1.dll \
  SOURCE.clj OUTPUT.dll
clr-meta/scripts/clr-meta-compiler-selfhost-stage1-gate
```

The C2 gate prepares all 27 generated compiler methods, hides the canonical
compiler source and ClojureCLR from the execution process, compiles and runs
checked add/subtract, closed equality, and Clojure truthiness targets, and then
creates a random nonce source only after Stage1 exists to prove a genuinely
fresh target. It also executes a 7,900-node near-budget program, checks 16
structured no-output failures and four builder publication-preservation cases,
and verifies the three frozen mutation sites. Identity mutation reaches target
metadata. Add/subtract mutations alter the generated compiler's own control
arithmetic and therefore produce the predicted `bad-def-arity` / `call-arity`
rejections with no target output; this is propagation evidence, not a claim
that a mutated compiler still swaps target arithmetic successfully.

C2 closes only the executable selfhost Compiler Stage1 artifact. Its immutable
historical manifest and gate receipt therefore continue to say
`compiler_stage2=false`: a later checkpoint does not rewrite an earlier
artifact's boundary.

C3 adds a new contract and child artifact instead. The Stage2 builder validates
the complete C2 parent manifest and live closure, freezes a byte-exact private
copy of the same canonical kernel source, and runs the generated Stage1 in a
fresh allowlisted process to compile that source into a runnable Stage2. It
binds the parent manifest, parent compiler, source, support triplet, toolchain,
contract, child compiler, and input/bundle closure hashes. The override-style
child contains only `CompilerStage2.dll`, the three support runtime files, and
its own C3 manifest: neither the parent Stage1 PE and manifest nor the canonical
source and ClojureCLR are packaged.

```sh
./bin/clr-meta --build-compiler-selfhost-stage2 \
  /tmp/clr-selfhost-stage1 /tmp/clr-selfhost-stage2
clr-meta/scripts/clr-meta-compiler-selfhost-stage2-gate
```

The artifact manifest closes `compiler_stage2=true` and the exact same-source
recompile, but deliberately records `stage2_fresh_target_replay=false` because
artifact construction alone does not own the replay proof. The separate C3
gate starts from that artifact, copies only Stage2 plus the support triplet into
an exact source-hidden directory, creates a random nonce target after Stage2
exists, compiles it with Stage2, and executes the target in another fresh
directory containing only the target and support triplet. Its C3 gate receipt
is the owner of `stage2_fresh_target_replay=true`. Delayed identity/add/subtract
mutations also pass through a mutated Stage2 into a grandchild target, proving
one-generation propagation without upgrading that observation into general
compiler correctness.

C3 closes exactly Compiler Stage2 and source-hidden fresh-target replay.
Compiler Stage3, self-reproduction, Stage15/N, a fixed point, raw PE
reproducibility, host-free bootstrap, the full Clojure surface, ClojureCLR
replacement, PNIX product/compiler integration, cross-host canonical
equivalence, and host promotion remain false.

## Runtime artifact builder

The generic builder accepts a strict EDN plan, a destination, and an exact
Clojure source root:

```sh
./bin/clr-meta --build-runtime PLAN OUTPUT SOURCE_ROOT
```

It validates the plan schema, entry, ordered namespace set, namespace/path
collisions, pairwise plan/source/output path separation, and equality of the
declared and actual `.clj` source sets. It then starts a fresh child whose load
path contains only the declared source root, uses the pinned host ClojureCLR
`compile` backend, rejects undeclared namespace dependencies, and emits a deterministic JSON
manifest binding the plan, source bytes, exact output set, entry, target, and
closure hashes. Product identity remains in the caller's plan; `clr-meta`
contains no PNIX namespace list.

For `pnix-clr`, `runtime-artifact.edn` declares nine namespaces and therefore
exactly nine `.clj.dll` outputs. This is a real artifact-production seam, but
the backend is honestly named `host-clojureclr-aot`. The manifest pins the
bytes produced by one build; it does not prove that two raw AOT builds are
byte-identical. The product runner additionally fixes cwd to the verified
artifact and rejects product namespace shadows in ClojureCLR's earlier pinned
runtime lookup roots.

From the outer directory:

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

`bin/clojure-clr-bootstrap` names the pinned upstream compiler/runtime trust
root. `bin/clojure-clr` is a focused compatibility facade that admits only
`-e` and single-file evaluation through generation 2 and rejects broader
command profiles. It is still hosted by that trust root and is not backed by a
self-reproducing `clr-meta` compiler.

This proves evaluator self-interpretation, the exact checked-Int64 expression
Compiler Stage1 profile, complete static admission of the separately versioned
selfhost kernel source, the C2 executable selfhost Stage1 artifact, and the C3
same-source executable Stage2 plus source-hidden fresh-target replay. It does
not prove Compiler Stage3, compiler self-reproduction, Stage15/N, an IL fixed
point, raw AOT/PE rebuild determinism, the full Clojure language/command
surface, ClojureCLR replacement, standalone replay of the unbundled lineage,
PNIX semantics/compiler integration, or cross-host canonical equivalence.
See `CLR_BOOTSTRAP.md` and the emitted receipt for the exact current claim, and
`STAGE15_N_ROADMAP.md` for the ordered open target.
