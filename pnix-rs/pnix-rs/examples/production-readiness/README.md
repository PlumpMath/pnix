# production-readiness

This example fixes the floor needed before a shared `.px` stdlib is started:
direct PNIX evaluation, a relative-imported library, host calls into its
exported functions, a PNIX-written recursive interpreter, and live `rs-meta`
substrate/tower composition. It deliberately
does not implement `pnix-meta`.

```sh
cd pnix-rs
./examples/production-readiness/run.sh
```

The four `.px` files are self-contained copies.  The monorepo readiness gate
checks that every host copy has identical bytes.
