# TFM / SDK policy (pnix-clr vs Rhino)

**Date:** 2026-08-14

| Path | TFM / SDK | Where |
|------|-----------|--------|
| **pnix-clr product** (AOT guest, bootstrap, gates) | **net10.0** / `dotnet-sdk_10` | `pnix-clr/`, HM `dev/cs` runners |
| **Pnix.Clr managed Eval API** | multi-target **net8.0 + net10.0** | `csharp/Pnix.Clr/` so Rhino-side net8 can Reference Eval |
| **Rhino / Grasshopper plugins** (Kimchi) | **net8.0** / sdk_8 cask or pin | `dot-nix` Rhino plugin paths — **not** pnix-clr AOT |

## Rules

1. Do **not** silently point Rhino plugin builds at sdk_10 / net10.
2. Do **not** build pnix-clr runtime-artifact with sdk_8.
3. Host-main C# that only needs `Pnix.Clr.Eval` may use net8 (process spawn to
   `pnix-clr` still uses the net10 host runtime under the hood).
4. Multi-ns **ClojureCLR** project template uses bootstrap **net10** only
   (`examples/clojure-clr-project`).

See also: monorepo `HOST_DEV_ENV.md`, `docs/CLOJURE_CLR_ADMITTED_SURFACE.md`.
