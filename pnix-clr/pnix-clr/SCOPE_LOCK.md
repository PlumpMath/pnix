# pnix-clr / clr-meta scope lock

`pnix-clr` is the ClojureCLR-hosted PNIX mechanism. `clr-meta` is the separate,
PNIX-agnostic ClojureCLR meta/bootstrap lane.

## Bootstrap scope

- a PNIX-agnostic `clr-meta` artifact builder which consumes the
  product-owned `runtime-artifact.edn` plan and an exact source closure
- the separately versioned `clr-meta` selfhost compiler family: closed C0/C1
  source admission plus the owner-authorized C2 slice that implements its
  exact low-level support ABI and generates an executable Compiler Stage1 PE
  from the canonical kernel through the explicit pinned-host B0 trust root;
  C2 must prove fresh-process unseen-target compilation and semantic mutation
  propagation while keeping Stage2 and self-reproduction false
- a `host-clojureclr-aot` manifest containing exactly the nine DLLs declared by
  that plan, with plan/source/output hashes and an explicit entrypoint
- artifact-only PNIX product loading: validate the live plan, source set,
  output set, exact manifest/tree shape, and every recorded digest; reject
  pinned-runtime and cwd namespace shadows, replace cwd and the load path with
  the artifact, and fail closed rather than compile or load product source
- `clr-meta` focused tool evaluation through physical evaluator generation 2,
  with a non-evaluating, exact-one-form portable-domain reader and no
  `load-string` tool path
- CLR-native PNIX tokenization, parsing, and evaluation mechanism
- nominal CLR `Done | Failed | Requested | Suspended` carriers and observer
  implementing `pnix.machine.host-outcome.v1`; guest maps cannot forge them
- production evaluator integration for `Done` and structured `Failed`, plus
  carrier/observer shape only for `Requested` and `Suspended`
- the exact integer/string/`if`/checked-`+`/integer-`/` mechanisms needed
  by the common 11-case basic-outcome contract
- relative, read-only, lexically confined loading of canonical modules from
  `../../pnix-meta` (not yet a symlink-safe security boundary)
- deterministic seed JSON projection; `bool-01` bytes match the pinned common
  expected file
- common-corpus evidence that dead `if` branches, unused arguments, and
  unselected attr fields do not resolve or read their import expressions
- null and bool/int/string scalar equality plus static identifier attr-path
  `?`, with application binding tighter than `?`; structural equality remains
  excluded
- source-originated `System.Int64` unary negation and checked add, subtract,
  multiply, and truncating division, including structured overflow and lazy
  dead-overflow behavior required by `production-checked-i64-01`
- **README corpus language surface** (peer parity intent with clj/hy/rs/cljs):
  builtins + `lib` (core/attrs/lists/strings/predicates/math/combinators/FS/
  best-effort fetch), nested attr paths (`foo.bar = expr`), partial builtin
  application, and `root-environment` frames. Implemented inside the existing
  nine namespaces (`evaluator.clj` / `host.clj`); no new artifact namespaces
- ClojureCLR/.NET host adapters
- a focused net10 gate that cannot fall back to the JVM host

The runtime remains intentionally narrow beyond that surface. Additional
syntax or ABI claims are admitted only with oracle evidence and common-corpus
agreement. Expanding the README surface does **not** establish tri-host
promotion.

The artifact dependency does not merge layer identities. `pnix-clr` owns the
namespace plan and PNIX mechanism; `clr-meta` owns generic validation and CLR
artifact production; `pnix-meta` owns portable meaning. The pinned ClojureCLR
compiler/runtime remains the explicit bootstrap and host-AOT trust root.

Evaluator generation numbers and compiler stage numbers are disjoint. The
current evaluator generations 0, 1, and 2 prove a focused nested interpreter.
They do not prove Compiler Stage1, Stage2, or Stage15/N. Extending that nested
interpreter through 15 self-extensions currently exhausts the CLR stack and is
an open host resource limitation, not a `Held` result or stage receipt.
**clr-meta meta floor remains C3 Stage2; Stage3–15/N are still open.**

## Out of scope

- JVM classfiles, ASM, Java reflection, Maven/JAR execution, or JVM fallback
- copying portable PNIX semantics out of `../../pnix-meta`
- service admission, deployment policy, or proof receipts on basic execution
- Hangul/NL/dictionary/agent/domain content
- claims of complete mature JVM-host parity, IL fixed-point self-hosting, or
  established tri-host membership before their gates exist
- Compiler Stage2--15/N, compiler self-reproduction, byte-identical raw AOT
  rebuilds, or a CLR IL fixed point; the only newly admitted compiler growth is
  the exact C2 selfhost-family Compiler Stage1 artifact described above
- broad ClojureCLR language/command/runtime/ecosystem compatibility or
  replacement; `bin/clojure-clr` currently admits only focused `-e` and
  single-file profiles through generation 2 and remains hosted by the explicit
  bootstrap trust root
- a standalone source-free distribution; launch validation still binds the
  live plan and source closure, and AOT execution retains the pinned runtime
- PNIX common compiler/PIR integration or CLR host promotion
- BigInt arithmetic or full numeric promotion beyond Int64 + finite Double
- `pnix.primitive-abi.v1` manifest routing/enforcement, production-evaluator
  primitive-manifest enforcement, or full-builtin manifest enforcement
- production effect request/resume, finite-fuel suspension, common-machine
  replacement, or canonical-result/JCS completion
- Nix UTF-8 byte-string model, string-context propagation, pattern lambdas,
  or derivation/store purity gates
  (float literals, `with`, structural `==` for list/attrset, language
  `assert`, and `inherit` / `inherit (expr)` are admitted)

## Rule

Build and gate `clr-meta` first. The aggregate gate then builds the exact AOT
artifact, checks its negative matrix, and admits the seed `pnix-clr` runtime
only through that artifact. Missing or stale artifact state is an
infrastructure/configuration failure; it does not authorize source or bootstrap
fallback. `pnix-clr` loads common `.px`. Unsupported language input returns a
nominal structured `Failed` outcome; it is never made safe by `Held`.

The target order is compiler Stage1, self-reproducing Stage2, repeated
Stage3--7 convergence, Stage8--15/N hardening, individually admitted
ClojureCLR compatibility profiles, and only then transfer of the broader
compatibility command from the bootstrap-hosted focused facade to a generated
compiler tool. PNIX common-compiler
integration and CLR host promotion close independently afterward. See
`../clr-meta/STAGE15_N_ROADMAP.md`. Passing the current CLR artifact/adoption
gate is evidence, not automatic replacement or admission as an established
host. The shared constitution in `../../project-wiki/CONSTITUTION.md` remains
authoritative.
