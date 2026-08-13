# pnix-clj — the Clojure/JVM host of pnix

You are inside **pnix-clj**, one host embedding of the pnix language. This tree is
**self-contained**: it depends on no sibling repository and shares no corpus,
gate, or `.px` core with another host. Everything it needs to build and gate is
here.

Keep these two identities separate:

- **clj-meta** owns this host language's self-host proof + native acceleration;
  it is pnix-agnostic.
- **pnix-clj** owns the pnix RUNTIME on this host: parse/evaluate pnix, wire
  acceleration to `clj-meta`, and provide the bridge (effect/capability adapters +
  canonical-result emission).

Non-negotiable here: **meta first, never cram** — do not grow this host's
product surface ahead of its `clj-meta` foundation. **Non-regression** — keep this
repo's gate green. This repo's own `SCOPE_LOCK.md` (if present) governs local
scope.

## Dual-axis + host library (do not confuse)

Canonical monorepo doc: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md).

| Axis | Entry | Role |
|------|-------|------|
| **host-main** | `pnix-clj-clj` / bare `clojure` | injects `pnix-clj` via `-Sdeps` local/root |
| **pnix-main** | `pnix-clj-pnix` | pnix REPL / eval of `.px` |
| **library** | `pnix-clj` sources; `PNIX_CLJ_ROOT` | host-bound JVM library, not portable `.px` |
| **meta** | `clj-meta` | pnix-agnostic |

Host-language `.px` import: `(pnix-clj.core/eval-file "x.px")` / `eval-source`.  
Public surface: [`pnix-clj/docs/HOST_IMPORT.md`](pnix-clj/docs/HOST_IMPORT.md).  
HM: `~/dot-nix/dev/clj` + overlay.
