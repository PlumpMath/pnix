# production-readiness

This example fixes the floor needed before a shared `.px` stdlib is started:
direct PNIX evaluation, a relative-imported library, host calls into its
exported functions, a PNIX-written recursive interpreter, native host
projection, and a live PNIX -> `clj-meta` execution.
It deliberately does not implement `pnix-meta`.

```sh
cd pnix-clj
clojure -M examples/production-readiness/run.clj
```

The four `.px` files are self-contained copies.  The monorepo readiness gate
checks that every host copy has identical bytes.
