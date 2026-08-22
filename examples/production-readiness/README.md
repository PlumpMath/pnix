# Production-readiness portable fixture

These files define a small, byte-identical `.px` floor copied into all five
host-owned example trees.  They are not `pnix-meta` and are not a stdlib:

- `direct.px` proves the ordinary runtime path;
- `library.px` plus `consumer.px` prove a once-written relative-import library;
- `self_interpreter.px` is a recursive interpreter written in PNIX and proves
  the PNIX-in-PNIX path independently of each host's meta implementation.

Run all five host drivers and the public host-library imports with:

```sh
./bin/production-readiness-gate
```

`--full` additionally replays each host's expensive aggregate gates.
