# Host-import mini examples (P2.2 easy start)

Tiny **host-main** demos: each host language loads its pnix product library and
evaluates `hello.px` (`1 + 2` → `3`).

**Prereq:** dual-axis env (HM profile) or equivalent env vars — see
[`../../HOST_IMPORT.md`](../../HOST_IMPORT.md).

**Important:** `cd` into the **host subdirectory** first. Running
`clojure -M -m smoke` from `examples/` or `examples/host-import/` fails with
`Could not locate smoke__init.class` — each demo has its own `deps.edn` /
entry file.

| Host | How to run (from monorepo root or absolute) |
|------|-----------------------------------------------|
| clj | `cd examples/host-import/clj && clojure -M -m smoke` |
| clj multi-module | `cd examples/host-import/clj-imports && clojure -M -m smoke` |
| hy | `cd examples/host-import/hy && python smoke.py` |
| cljs | `cd examples/host-import/cljs && node smoke.mjs` |
| rs | `cd examples/host-import/rs/pnix-rs-smoke && cargo run -q -- ../../hello.px` |
| clr | `cd examples/host-import/clr && ./smoke` (HelloPnix + export) |

Shared source: [`hello.px`](hello.px).

Regression:
- monorepo `../../bin/host-import-smoke` (PATH tools)
- monorepo `../../bin/host-library-smokes` (local library feeds)
