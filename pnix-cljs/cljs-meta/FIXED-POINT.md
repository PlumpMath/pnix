# cljs-meta fixed point

`cljs-meta` builds a ClojureScript compiler fixed point above an explicit,
small JavaScript runtime kernel.

## Stage sequence

```text
JVM-built stage 0 compiler
  -> stage 1 self-compiled compiler artifact
  -> stage 2 self-recompile
  -> stage 3 self-recompile
```

The builder runs at least 15 compiler generations and then continues until two
successive artifacts are byte-identical (bounded by
`PNIX_CLJS_MAX_STAGES`, default 32). The fixed-point gate requires all of the
following:

```text
stage 2 artifact bytes == stage 3 artifact bytes
stage 2 source closure == stage 3 source closure
stage 2 compiler input hash == stage 1 artifact hash
stage 3 compiler input hash == stage 2 artifact hash
stage 0 bootstrap-only namespaces are absent from the final artifact
```

The explicit trust root is:

```text
Node.js
Google Closure runtime
cljs.core runtime
cljs.reader / cljs.tools.reader runtime
cljs.core macro bootstrap kernel
fixed-point stage harness
embedded cljs.core analysis cache
```

The analyzer, compiler, source-map implementation, and `cljs.js` are emitted
as the self-compiled payload. The stage 0 JVM compiler is not packaged in the
fixed artifact.

## Build and inspect

```sh
./bin/build-cljs
cat cljs-meta/dist/fixed-point/receipt.json
node cljs-meta/test/fixed_point_test.js
```

## Use the fixed compiler

```js
const cljs = require("./cljs-meta/dist/fixed-point/cljs-meta-fixed.js");

const evaluated = await cljs.evaluate("(let [x 20] (+ x 22))");
const compiled = await cljs.compile("(defn answer [] 42)");
```

`evaluate` and `compile` return `pnix.cljs-meta.result.v1` projections.

## Cross-platform closure checklist

Current evidence is limited to `x86_64-darwin`. A platform is not considered
supported merely because it appears in `flake.nix` or evaluates successfully.

- [x] `x86_64-darwin`
- [ ] `aarch64-darwin`
- [ ] `x86_64-linux`
- [ ] `aarch64-linux`

Each unchecked platform must independently satisfy:

- [ ] `./bin/build-cljs` succeeds from a clean `target/` and `dist/`.
- [ ] Stage 2 and stage 3 artifacts are byte-identical.
- [ ] Stage 2 and stage 3 source closures are identical.
- [ ] Stage input hashes prove that stage 1 compiled stage 2 and stage 2
      compiled stage 3.
- [ ] The final artifact contains no stage 0 bootstrap-only namespace.
- [ ] `node cljs-meta/test/fixed_point_test.js` passes.
- [ ] `node cljs-meta/examples/fixed-point.js` passes.
- [ ] `./bin/pnix-cljs-gate` passes.
- [ ] `nix flake check path:. --no-write-lock-file` passes natively.
- [ ] `compile` and `evaluate` produce the same canonical projections as the
      already-supported platforms.

Artifact hashes from different platforms must be compared and explained.
Platform-specific paths, tool versions, timestamps, or host renderings must be
normalized before claiming cross-platform byte determinism. Until every item
above is closed, documentation and receipts must say `platform-pending` rather
than claiming multi-platform completion.
