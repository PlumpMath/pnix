# cljs-meta

`cljs-meta` is the ClojureScript host mechanism for PNIX. Its first executable
slice uses `cljs.js` to evaluate ClojureScript from JavaScript/Node without
making proof receipts part of ordinary PNIX execution.

```sh
../bin/build-cljs
node dist/cljs-meta.js -e '(let [x 20] (+ x 22))'
```

CommonJS:

```js
const cljsMeta = require("./dist/cljs-meta-module.js");
cljsMeta.evaluate("(+ 20 22)").then(console.log);
```

## Status / primary gate

See [STATUS.md](STATUS.md). Primary gate: `./bin/cljs-meta-gate` (fixed-point tests; builds if needed).

## Fixed point

`../bin/build-cljs` also constructs three isolated compiler stages. Stage 2
and stage 3 must be byte-identical before the build succeeds.

```text
dist/fixed-point/cljs-meta-fixed.js
dist/fixed-point/cljs-meta-fixed-cli.js
dist/fixed-point/receipt.json
```

The self-hosted compiler closure includes the ClojureScript analyzer,
compiler, reader, and `cljs.js`. Its explicit runtime trust root is
`cljs.core`, `cljs.tools.reader`, Google Closure runtime, and Node.js.

The evaluator is a host mechanism. It does not own PNIX language semantics,
service admission, or artifact approval.
