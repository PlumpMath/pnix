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
