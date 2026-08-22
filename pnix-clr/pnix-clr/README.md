# pnix-clr runtime

This directory contains the CLR-native PNIX host mechanism. It runs as
ClojureCLR on .NET 10 and parses/evaluates `.px` source directly — it does not
depend on a sibling corpus tree. Its product entry now runs from a
`clr-meta`-produced AOT artifact rather than adding `src/` to the runtime load
path.

From the outer `pnix-clr/` directory:

```sh
./bin/build-pnix-clr-artifact
./bin/pnix-clr-artifact-gate --no-build
./bin/pnix-clr -e 'true && !false'
./bin/pnix-clr -e 'if true then 40 + 2 else 0'
./bin/pnix-clr -e 'builtins.typeOf 1'
./bin/pnix-clr -e 'lib.sum [1 2 3 4]'
./bin/pnix-clr -e 'builtins.getAttrFromPath [ "foo" "bar" ] { foo.bar = 42; }'
./bin/pnix-clr --production-outcome-self-check
./bin/pnix-clr-production-outcome-gate
./bin/pnix-clr-gate
```

`runtime-artifact.edn` is the product-owned seam. It declares entry namespace
`pnix-clr.main` and an ordered, exact set of eight namespaces:

```text
outcome -> lexer -> parser -> host -> json -> evaluator
        -> production-outcome -> main
```

The PNIX-agnostic `clr-meta` builder checks that these are exactly the `.clj`
files under `src/`, AOT-compiles them with the explicitly pinned host backend,
and publishes exactly eight namespace DLLs plus `manifest.json`. The manifest
binds the plan hash, source rows and closure hash, output rows and closure hash,
entry, target, producer, and backend.

Before execution, `bin/pnix-clr` revalidates the contract and every plan,
source, and output byte. It rejects missing, stale, extra, or malformed state,
requires the exact manifest keys and artifact tree, rejects product namespace
shadows in ClojureCLR's pinned runtime lookup roots, changes cwd to the verified
artifact, and sets `CLOJURE_LOAD_PATH` to that directory rather than appending
to an inherited path. It loads only the recorded AOT product entry, never builds
a missing artifact, and never falls back to product source or the bootstrap
command. The pinned `Clojure.Main.dll` still hosts the AOT namespace, and the
live plan/source closure is still required for launch-time validation; this is
therefore not a standalone source-free distribution.

## Language surface (README corpus parity)

The seed evaluator now exposes a builtins/`lib` surface aimed at the same
"실행 테스트해봄" / README smoke corpus used by the peer clj/hy/rs/cljs hosts:

- **Core:** `typeOf`, `tryEval`, `throw`, `abort`, `trace`, `warn`, `toString`,
  `toJSON`, `toXML`, `toFile`
- **Attrs:** `attrNames`, `attrValues`, `hasAttr`, `getAttr`, `getAttrFromPath`,
  `hasAttrByPath`, `attrByPath`, `mapAttrs`, `filterAttrs`, `listToAttrs`,
  `removeAttrs`, `recursiveUpdate`, `zipAttrsWith`, `intersectAttrs`, `catAttrs`
- **Lists:** `length`, `head`, `tail`, `last`, `init`, `elem`, `elemAt`,
  `concatLists`, `flatten`, `concatMap`, `genList`, `foldl`, `foldl'`, `foldr`,
  `partition`, `unique`, `range`, `sum`, `product`, `zipLists`, `zipListsWith`,
  `intersectLists`, `subtractLists`
- **Strings:** `substring`, `stringLength`, `concatStringsSep`,
  `concatMapStringsSep`, `replaceStrings`, `removePrefix`, `removeSuffix`,
  `hasPrefix`, `hasSuffix`, `splitString`, `toLower`, `toUpper`, `boolToString`,
  `match`, `split`
- **Predicates / math / combinators / FS / fetch** as listed in the evaluator
  (best-effort network fetch via `HttpClient` or `curl`/`git`)
- **Nested attr paths:** `{ foo.bar = 42; }` desugars at parse time and works
  with `getAttrFromPath`
- **`lib`:** full builtins map plus nested `lib.attrsets` / `lib.lists` /
  `lib.strings` and common aliases (`lib.sum`, `lib.head`, `assertMsg`, …)

Builtins live inside `evaluator.clj` (with IO helpers in `host.clj`) so the
eight-namespace artifact plan is unchanged.

### Admitted language forms (peer-parity slice)

- **Floats:** finite decimal literals (`1.5`); `builtins.typeOf` → `"float"`;
  JSON projection uses invariant-culture decimal; NaN/Inf fail closed.
- **`with attrs; body`**, language **`assert cond; body`**, and
  **`inherit` / `inherit (expr)`** in attrsets and `let` (rec inherit binds
  from the enclosing scope, not the rec frame).
- **Structural `==` / `!=`** for lists and attrsets (plus scalars).

### Known gaps vs mature peers

- **String model:** CLR `System.String` / char-index ops, not Nix UTF-8 byte
  offsets or string-context tracking.
- **Fetch:** best-effort only; offline or missing `git`/`curl` can fail or
  stub a store path.
- Still **not** established: host promotion, full nixpkgs `lib`, or
  `pnix.primitive-abi.v1` enforcement. `clr-meta` Stage1–N gates are closed
  with `promotion/allowed?=false` and are not this product's compiler.
  Minimal pattern lambdas and `//`/`++` are live on this host (see corpus
  gap map).

The bootstrap surface remains fail-closed. The README at `../README.md`
records the exact non-claims. This host owns parsing/evaluation machinery,
CLR adapters, nominal host MachineOutcome carriers, and bounded observer
projections only — it does not depend on or gate against any sibling corpus
tree; `--production-outcome-self-check` verifies the host's own nominal
outcome-boundary contract in isolation (see `examples/08-production-outcome-self-check`).

**Principles this host currently owns** (see `../clr-meta/RESIDUAL_SURFACE.md`):
dynamic attribute keys, exact signed-64-bit integer ops, mixed float promote
with signed-zero and NaN-cell identity, POSIX ASCII ERE classes, failed-thunk
replay of catchable throws, and running portable kernel/math `.px` modules as
guest programs.

Still unclaimed: full self-host fixed point, derivation host ABI surface,
module-compile stack closure, BigInt, and host promotion.

This product artifact does not *promote* Compiler Stage1–15/N, compiler
self-reproduction, or a general IL fixed point. `clr-meta` has closed those
compiler-selfhost gates with `promotion/allowed?=false` (see
`../clr-meta/STATUS.md`); this host does not consume that ladder as a product
compiler. Evaluator generations 0..2 belong to the separate focused `clr-meta`
interpreter lane and are not compiler stages. Unclaimed remain: host
promotion, broad ClojureCLR replacement, PNIX common compiler/PIR.
