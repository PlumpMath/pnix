# pnix-clr — experimental CLR host limb of ~/pnix

Keep these identities separate:

- `clr-meta` owns the ClojureCLR host-language bootstrap, the focused
  evaluator-generation lane, the generic CLR artifact builder, and future
  native CLR acceleration. It is PNIX-agnostic: the product-owned artifact
  plan is an input, not knowledge compiled into `clr-meta`.
- `pnix-clr` owns the PNIX runtime mechanism on .NET: parse/evaluate PNIX,
  provide CLR host adapters, and declare the exact namespaces in its
  runtime-artifact plan.

pnix-clr is self-contained: it depends on no sibling repository and shares no
corpus with another host.

The product dependency is now operational rather than gate ordering alone.
`clr-meta` consumes `pnix-clr/runtime-artifact.edn`, requires its declared
source set to be exact, and emits a hash-bound `host-clojureclr-aot` artifact
containing exactly eight namespace DLLs. `bin/pnix-clr` validates the plan,
source closure, output closure, exact manifest/tree shape, entrypoint, and every
recorded digest. It rejects product namespace shadows in ClojureCLR's pinned
runtime lookup roots, changes cwd to the verified artifact, replaces
`CLOJURE_LOAD_PATH` with that directory, and loads product code only from the
artifact. Missing or stale evidence fails closed; there is no product source
or build fallback. The pinned ClojureCLR runtime remains an explicit substrate.

`clr-meta -e` and file mode read exactly one form with reader evaluation and
data readers disabled, reject values outside the admitted portable form
domain, use physical evaluator generation 2, and contain no `load-string`
path. Generations 0, 1, and 2 are nested evaluator generations;
they are not compiler Stage1, Stage2, or Stage15/N. A live attempt to extend
this nested interpreter through 15 self-extensions exhausts the CLR stack. It
therefore exposes a separate evaluator resource limit; it is not compiler
Stage15/N evidence or a stage receipt. `bin/clojure-clr` is only a focused
`-e`/single-file compatibility facade over that generation-2 tool; unsupported
command profiles fail closed, and the pinned upstream compiler/runtime remains
the explicit bootstrap trust root beneath it.

The current slice is an experimental net10 bootstrap plus adoption of four
common corpus cases and the common production basic-outcome contract. Its
local gate proves nominal CLR outcomes, the common 11-case projection, and
focused dead-import/hasAttr precedence behavior. The fourth case adds checked
add/subtract/multiply/divide and unary negation only for Int64 values originating
in the admitted PNIX source path, including structured overflow failures and
lazy avoidance of dead overflow expressions. Float literals, structural
equality (lists/attrsets), and an extended builtin surface (math, bitwise,
list/attrset helpers — maturity pass 2026-08-11) work, but the checked-I64
guarantees above cover only integers; general numeric promotion, BigInt
semantics, primitive-manifest enforcement, the mature JVM surface, production
request/suspension, a compiler Stage1--15/N chain, compiler self-reproduction,
an IL fixed point, raw AOT reproducibility, broad ClojureCLR compatibility/
replacement, a standalone source-free distribution, PNIX common-compiler
integration, and membership in any cross-host gate remain unclaimed. Grow and gate
`clr-meta` first, then admit `pnix-clr` slices. The current aggregate gate
enforces that order and the artifact dependency; direct compiler acceleration
and replacement remain future work. See `clr-meta/STAGE15_N_ROADMAP.md` for the
ordered target and promotion boundaries.

Keep failures structured and never use `Held` as a language-error sink.

The host substrate is the pinned upstream `Clojure` NuGet package
(1.12.3-alpha8), published by `bin/build-clr` from `clr-bootstrap/`. That signed,
version-pinned package is the explicit bootstrap trust root; no upstream
compiler sources are vendored here. The cloned JVM/domain surfaces were
deliberately not carried into the active CLR host: port only CLR-owned
mechanism here, since a textual rename is not CLR evidence.

## Dual-axis + host library (do not confuse)

Canonical monorepo doc: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md).  
C# surface detail: [`csharp/Pnix.Clr/README.md`](csharp/Pnix.Clr/README.md).

| Axis | Entry | Role |
|------|-------|------|
| **host-main (C#)** | `pnix-clr-cs` / MSBuild + `Pnix.Clr` | process→CLI Eval API |
| **host-main (CLR)** | `pnix-clr-clr` / `clojure-clr` | focused `-e`/file facade + library env |
| **pnix-main** | `pnix-clr-pnix` / `pnix-clr` | pnix REPL / eval of `.px` |
| **library** | `bin/export-pnix-clr-library` → `pnix-clr-library` | guest AOT `*.clj.dll` + managed DLL + props |
| **meta** | `clr-meta` | pnix-agnostic |

Env: `PNIX_CLR`, `PNIX_CLR_ROOT`, `PNIX_CLR_ARTIFACT` (+ legacy
`PNIX_CLR_RUNTIME_ARTIFACT`), `PNIX_CLR_LIBRARY`.

```bash
./bin/export-pnix-clr-library
# flake: nix run .#pnix-clr-library   /   nix run .#pnix-clr-refs
```

Guest AOT DLLs are **ClojureCLR-bound**, not a portable multi-host `.px` package.
Do not claim compiler Stage15/N from evaluator generations. Rhino plugins pin
**sdk_8**; this host’s AOT/runtime is **net10** — do not silently mix TFMs.  
HM: `~/dot-nix/dev/cs`.
