# production-readiness

This example fixes the floor needed before a shared `.px` stdlib is started:
direct PNIX evaluation, a relative-imported library, host calls into its
exported functions, a PNIX-written recursive interpreter, native host
projection, and a live PNIX <-> `hy-meta` projection.
It deliberately does not implement `pnix-meta`.

```sh
cd pnix-hy
PYTHONPATH=. python3 examples/production-readiness/run.py
```

The four `.px` files are self-contained copies.  The monorepo readiness gate
checks that every host copy has identical bytes.
