# examples — pnix-clr (ClojureCLR / .NET)

> **Maturity:** experimental seed. These examples exercise the **admitted**
> surface (pnix-main CLI, local NuGet/library export, C# host-main, optional
> in-process). They do **not** claim Stage15/N, nuget.org publish, or five-host
> semantic parity.

## Counts vs peers

| Host | Catalog size (order of magnitude) |
|------|-----------------------------------|
| clj / hy | dense research catalogs |
| rs | mid-size pillar catalog |
| **clr** | **core 00–06** (this tree) — grow only with real surface |

Shared theme map: monorepo [`examples/EXAMPLES_BALANCE.md`](../../../examples/EXAMPLES_BALANCE.md).

## Pattern

Each numbered slice has:

- `README.md` — what / why / how to run
- `.px` and/or pointers to C# projects under `pnix-clr/csharp/examples/`

## Catalog

| Dir | Theme |
|-----|--------|
| [`00-foundation`](00-foundation/) | pnix-main seed + meta smoke pointer |
| [`01-pure-eval-boundary`](01-pure-eval-boundary/) | plain .NET eval vs guest boundary |
| [`02-host-library-import`](02-host-library-import/) | local Pnix.Clr library export (not nuget.org) |
| [`03-outcome-projection`](03-outcome-projection/) | production outcome / fail-closed shape |
| [`04-csharp-embed-pnix`](04-csharp-embed-pnix/) | host-main HelloPnix |
| [`05-inprocess-opt-in`](05-inprocess-opt-in/) | experimental in-process (net10); process-spawn default |
| [`06-meta-pair-boundary`](06-meta-pair-boundary/) | pnix-clr vs clr-meta roles |

## Run (typical)

```bash
cd pnix-clr
./bin/build-pnix-clr-artifact   # when artifact missing
./bin/pnix-clr pnix-clr/examples/00-foundation/program.px
./bin/pnix-clr-library-smoke    # local feed only
```

See also monorepo `examples/host-import/clr/` and `examples/clojure-clr-project/`.
