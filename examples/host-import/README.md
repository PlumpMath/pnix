# Host-import mini examples (P2.2 easy start)

Tiny **host-main** demos: each host language loads its pnix product library and
evaluates `../hello.px` (`1 + 2` → `3`).

**Prereq:** dual-axis env (HM profile) or equivalent env vars — see
[`../../HOST_IMPORT.md`](../../HOST_IMPORT.md).

| Host | How to run |
|------|------------|
| clj | `cd clj && clojure -M -m smoke` |
| clj multi-module | `cd clj-imports && clojure -M -m smoke` (`import ./lib.px` → 3) |
| hy | `cd hy && python smoke.py` |
| cljs | `cd cljs && node smoke.mjs` |
| rs | `cd rs/pnix-rs-smoke && cargo run -q -- ../../hello.px` |
| clr | see `clr/README.md` → existing HelloPnix |

Shared source: [`hello.px`](hello.px).

Regression: monorepo `../../bin/host-import-smoke` (PATH tools, not these dirs).
