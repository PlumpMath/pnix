# pnix-hy — the Python/Hy host of pnix

You are inside **pnix-hy**, one host embedding of the pnix language. This tree is
**self-contained**: it depends on no sibling repository and shares no corpus,
gate, or `.px` core with another host. Everything it needs to build and gate is
here.

Keep these two identities separate:

- **hy-meta** owns this host language's self-host proof + native acceleration;
  it is pnix-agnostic.
- **pnix-hy** owns the pnix RUNTIME on this host: parse/evaluate pnix, wire
  acceleration to `hy-meta`, and provide the bridge (effect/capability adapters +
  canonical-result emission).

Non-negotiable here: **meta first, never cram** — do not grow this host's
product surface ahead of its `hy-meta` foundation. **Non-regression** — keep this
repo's gate green. This repo's own `SCOPE_LOCK.md` (if present) governs local
scope.

## Dual-axis + host library (do not confuse)

Canonical monorepo doc: [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md).

| Axis | Entry | Role |
|------|-------|------|
| **host-main** | `pnix-hy-python` / `pnix-hy-hy` (bare `python`/`hy`) | `PYTHONPATH` → `pnix_hy` |
| **pnix-main** | `pnix-hy-pnix` | pnix REPL / eval of `.px` |
| **library** | installable `pnix_hy`; `PNIX_HY_HOME` / `PNIX_HY_LIBRARY` | host-bound Python package |
| **meta** | `hy-meta` | pnix-agnostic |

Host-language `.px` import: `import pnix_hy as ph; ph.eval_file("x.px")` (`run_px`).  
Do **not** globally override `pkgs.python311` in nix overlays — PATH join only.  
HM: `~/dot-nix/dev/py`.
