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

## Dual-axis + host library (do not confuse)

Canonical monorepo doc: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md).

| Axis | Entry | Role |
|------|-------|------|
| **host-main** | `pnix-rs-rs` (bare `cargo`/`rustc`) | `PNIX_RS_LIB_DIR` + link/include env |
| **pnix-main** | `pnix-rs-pnix` / `px-eval` | pnix REPL / one-shot eval |
| **library** | flake `packages.pnix-rs-library` | `libpnix_rs.*` + `include/pnix_rs.h` |
| **meta** | `rs-meta` / `bootstrap` | pnix-agnostic |

Host-language `.px` import: `pnix_rs::eval_file` / C `pnix_rs_eval`.  
Never put full `pnix-rs` + `pnix-rs-library` in one `buildEnv` (dylib clash).  
HM: `~/dot-nix/dev/rs`.
