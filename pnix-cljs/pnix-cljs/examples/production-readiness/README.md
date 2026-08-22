# production-readiness

This example fixes the floor needed before a shared `.px` stdlib is started:
direct PNIX evaluation, a relative-imported library, host calls into its
exported functions, a PNIX-written recursive interpreter, native JavaScript
projection, and live evaluation by the Stage-15
`cljs-meta` fixed compiler.  It deliberately does not implement `pnix-meta`.

```sh
cd pnix-cljs
node pnix-cljs/examples/production-readiness/run.js
```

The four `.px` files are self-contained copies.  The monorepo readiness gate
checks that every host copy has identical bytes.
