# pnix-rs — the Rust host of pnix

You are inside **pnix-rs**, one host embedding of the pnix language. This tree is
**self-contained**: it depends on no sibling repository and shares no corpus,
gate, or `.px` core with another host. Everything it needs to build and gate is
here.

Keep these two identities separate:

- **rs-meta** owns this host language's self-host proof + native acceleration;
  it is pnix-agnostic.
- **pnix-rs** owns the pnix RUNTIME on this host: parse/evaluate pnix, wire
  acceleration to `rs-meta`, and provide the bridge (effect/capability adapters +
  canonical-result emission).

Non-negotiable here: **meta first, never cram** — do not grow this host's
product surface ahead of its `rs-meta` foundation. **Non-regression** — keep this
repo's gate green. This repo's own `SCOPE_LOCK.md` (if present) governs local
scope.
