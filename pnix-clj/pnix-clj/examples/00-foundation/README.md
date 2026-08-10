# pnix-clj foundation

Run from `pnix-clj/pnix-clj`:

```sh
clojure -M examples/00-foundation/basic.clj
clojure -M examples/00-foundation/interop.clj
clojure -M examples/00-foundation/meta_circular.clj
```

These commands demonstrate product mechanisms.

No example performs automatic host code generation. `compile-source` means
lowering and execution through the existing clj-meta mechanism; it does not
create a new semantic owner.
