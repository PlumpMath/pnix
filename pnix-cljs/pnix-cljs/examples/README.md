# examples — pnix-cljs (ClojureScript / Node)

> **Maturity:** experimental seed. These examples exercise the **admitted**
> surface (parse/eval, Node library import, Done/Failed outcomes). They do
> **not** claim five-host parity, Stage15/N, or a full metacircular tower.

## Counts vs peers

| Host | Catalog size (order of magnitude) |
|------|-----------------------------------|
| clj / hy | dense research catalogs |
| rs | mid-size pillar catalog |
| **cljs** | **core 00–06** (this tree) — grow only with real surface |

Shared theme map: monorepo [`examples/EXAMPLES_BALANCE.md`](../../../examples/EXAMPLES_BALANCE.md).

## Pattern

Each numbered slice has:

- `README.md` — what / why / how to run
- host-native “limit” or “way” files (JS or `.px`) where useful

## Catalog

| Dir | Theme |
|-----|--------|
| [`00-foundation`](00-foundation/) | seed eval through dist module |
| [`01-pure-eval-boundary`](01-pure-eval-boundary/) | plain Node `eval` vs pnix eval boundary |
| [`02-host-library-import`](02-host-library-import/) | require local library export + `evalFile` |
| [`03-outcome-projection`](03-outcome-projection/) | Done / Failed (not silent throw-only) |
| [`04-js-embed-pnix`](04-js-embed-pnix/) | host-main: JS drives `.px` |
| [`05-experimental-honesty`](05-experimental-honesty/) | what this host does **not** claim |
| [`06-meta-pair-boundary`](06-meta-pair-boundary/) | pnix-cljs vs cljs-meta roles |

## Run (typical)

```bash
cd pnix-cljs
./bin/build-cljs   # when dist is missing/stale
node pnix-cljs/examples/00-foundation/node.js
# library import smoke (monorepo):
#   ./bin/pnix-cljs-library-smoke
```

See also monorepo `examples/host-import/cljs/`.
