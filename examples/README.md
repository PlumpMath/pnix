# examples/

Host-language import demos live under **`host-import/`**, not this directory.

```bash
# single-file eval-file
cd host-import/clj && clojure -M -m smoke
# => 3

# multi-module import ./lib.px
cd host-import/clj-imports && clojure -M -m smoke
# => 3
```

Other hosts: see [`host-import/README.md`](host-import/README.md).

Do **not** run `clojure -M -m smoke` from `examples/` itself — there is no
`deps.edn` or `smoke.clj` on the classpath here.
