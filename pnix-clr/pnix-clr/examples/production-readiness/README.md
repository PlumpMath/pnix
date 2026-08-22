# production-readiness

This example fixes the floor needed before a shared `.px` stdlib is started:
direct PNIX evaluation, a relative-imported library, C#/CLI calls into its
exported functions, a PNIX-written recursive interpreter, the CLR host
evaluator, and persisted Stage-15/fixed-point
evidence.  It deliberately does not implement `pnix-meta`.

```sh
cd pnix-clr
./pnix-clr/examples/production-readiness/run.sh
```

The four `.px` files are self-contained copies.  The monorepo readiness gate
checks that every host copy has identical bytes.
