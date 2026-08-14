# Examples balance (five hosts)

Each host owns `pnix-<host>/pnix-<host>/examples/`. Catalogs are **host-bound**
(not a shared multi-host corpus): same *themes* where the host has a real
surface; **host-specific** numbering where the host’s pillars differ.

## Counts (day-to-day snapshot)

| Host | Product examples root | Catalog depth (approx.) | Notes |
|------|----------------------|-------------------------|--------|
| **clj** | `pnix-clj/pnix-clj/examples/` | ~90 numbered slices | densest: spine, machine, oracle, AI gates |
| **hy** | `pnix-hy/pnix-hy/examples/` | ~35 | dense: specialize, cogen, compartment, Jones |
| **rs** | `pnix-rs/pnix-rs/examples/` | ~15 | balanced mid: gate, mirror, BTA, embed |
| **cljs** | `pnix-cljs/pnix-cljs/examples/` | core catalog (00–06) | experimental seed; Node library import |
| **clr** | `pnix-clr/pnix-clr/examples/` | core catalog (00–06) | experimental seed; C# library + in-process opt-in |

Monorepo host-import smokes (not the product catalog): `examples/host-import/`.

## Shared theme matrix (portable *intent*, host *realization*)

Themes are labels for humans — not a claim that every host implements the same
API or parity. Empty = not yet catalogued **or** host does not admit the surface.

| Theme | clj | hy | rs | cljs | clr |
|-------|-----|----|----|------|-----|
| Foundation (eval seed) | 00 | 00 | 00 | 00 | 00 |
| Pure / fail-closed eval | 01 | 01 | 01 | 01 | 01 |
| Determinism / hash / drift | 13–15, 21 | 02 | 02 | — | — |
| Host library import | host-import + 51 | 14 | 15 | **02** | **02** |
| Host↔pnix interop / embed | 04, 07–08 | 04, 07–08 | 04, 15 | **04** | **04** |
| Outcome / receipt honesty | 02, 05 | 05 | 05 | **03, 05** | **03, 05** |
| Specialize / Futamura | 03, 33 | 03, 33 | 06 | — | — |
| Self-host / meta pair | 11, 35 | 11, 35 | 11 | **06** | **06** |
| Compartment / capability | 23, 31 | 23, 31 | 08 | — | — |
| Cache / incremental | 12, 30 | 12, 22, 30 | 07 | — | — |
| Machine / abstract CEK | 61, 78–92 | 35 | — | — | — |
| In-process host eval | — | — | — | — | **05** (opt-in) |

## Balancing rules

1. **Do not clone clj/hy research slices** onto cljs/clr/rs just to match count.
2. **Do** keep a readable **00–0N core path** on every host: foundation →
   sandbox → host import → outcome → embed → honesty → meta boundary.
3. Host-specific deep catalogs grow only with **runnable** surface + README.
4. Maturity differs: cljs/clr examples must say **experimental** and fail closed
   on unadmitted claims (no Stage15/N, no five-host semantic parity).

## Where to start per host

| Host | Entry |
|------|--------|
| clj | `pnix-clj/pnix-clj/examples/START_HERE.md` |
| hy | `pnix-hy/pnix-hy/examples/README.md` + `FOUNDATION_PATH.md` |
| rs | `pnix-rs/pnix-rs/examples/README.md` + `FOUNDATION_PATH.md` |
| cljs | `pnix-cljs/pnix-cljs/examples/README.md` + `FOUNDATION_PATH.md` |
| clr | `pnix-clr/pnix-clr/examples/README.md` + `FOUNDATION_PATH.md` |

Last updated: 2026-08-14 (cljs/clr core catalogs filled; matrix documented).
