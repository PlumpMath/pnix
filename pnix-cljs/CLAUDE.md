# pnix-cljs agent boundary

`pnix-cljs` is the ClojureScript/JavaScript host projection of PNIX.

This tree is **self-contained**: it depends on no sibling repository and shares
no corpus, gate, or `.px` core with another host.

## Identity

```text
cljs-meta
  = ClojureScript host mechanism and self-host evaluation substrate

pnix-cljs
  = PNIX parser/evaluator and JavaScript interop surface implemented in CLJS
```

## Permanent rules

- Host values and nominal outcome classes are native ClojureScript values.
- Language types are structural data, never authoritative strings.
- Basic parse/evaluation errors are `Failed`, never `Held`.
- `cljs-meta` proof or repeat compilation may verify the implementation, but
  cannot gate ordinary `pnix-cljs` evaluation.
- Do not copy JVM, Java reflection, ASM, or Clojure-only implementation code
  into the active ClojureScript source closure.
- This seed does not claim full parity with the three established hosts,
  though the builtin surface was substantially widened in the 2026-08-11
  maturity pass (math, bitwise, list/attrset helpers ported from the
  reference host's `evaluator.clj`).

## Dual-axis + host library (do not confuse)

Canonical monorepo doc: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md).

| Axis | Entry | Role |
|------|-------|------|
| **host-main** | `clojurescript` → `pnix-cljs`; `node` with `NODE_PATH` | load `share/pnix-cljs` |
| **pnix-main** | `pnix-cljs-pnix` | pnix REPL / eval of `.px` |
| **library** | flake package `share/pnix-cljs` | host-bound JS module, not portable `.px` |
| **meta** | `cljs-meta` / `pnix-cljs-cljs` | fixed-point host mechanism |

Host-language `.px` import: module `evalFile` / `evalSource` from the share tree.  
`shadow-cljs` remains a **build orchestrator**; default runtime host is `pnix-cljs`.  
HM: `~/dot-nix/dev/cljs` (`pnix-cljs-host`).
