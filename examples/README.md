# examples/

Two layers:

1. **Per-host product catalogs** — `pnix-<host>/pnix-<host>/examples/`  
   Theme balance / maturity: **[EXAMPLES_BALANCE.md](EXAMPLES_BALANCE.md)**
2. **Monorepo host-import smokes** — **`host-import/`** (dual-axis day-1 demos)

## Host product catalogs (not this directory)

| Host | Path | Depth |
|------|------|--------|
| clj | `pnix-clj/pnix-clj/examples/` | dense (~90) |
| hy | `pnix-hy/pnix-hy/examples/` | dense (~35) |
| rs | `pnix-rs/pnix-rs/examples/` | mid (~15) |
| cljs | `pnix-cljs/pnix-cljs/examples/` | core 00–06 |
| clr | `pnix-clr/pnix-clr/examples/` | core 00–06 |

## host-import (this tree)

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
