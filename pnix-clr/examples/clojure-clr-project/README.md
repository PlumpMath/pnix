# clojure-clr-project — multi-ns template (P3.2 step 3 start)

**Honest scope:** runs on the **upstream ClojureCLR substrate** via
`bin/clojure-clr-bootstrap` (net10.0), **not** the focused
`bin/clojure-clr` facade (that still only admits `-e` / one file).

See inventory: [`../../docs/CLOJURE_CLR_ADMITTED_SURFACE.md`](../../docs/CLOJURE_CLR_ADMITTED_SURFACE.md).

## Layout

```text
src/demo/lib.clj    ; (ns demo.lib) …
src/demo/main.clj   ; (ns demo.main (:require [demo.lib …])) (-main)
run                 ; CLOJURE_LOAD_PATH=src + require demo.main
smoke               ; expect stdout 42
```

## Run

```bash
cd pnix-clr/examples/clojure-clr-project
./run
# => 42

./smoke
# => clojure-clr-project smoke: PASS (42)

# All named profiles (facade + this template):
#   ../../bin/clojure-clr-profiles-smoke
```

First run may call `bin/build-clr` (NuGet publish of pinned Clojure package).

## What this proves / does not

| Proves | Does not claim |
|--------|----------------|
| Two namespaces on disk load via `CLOJURE_LOAD_PATH` | `clojure-clr` facade grew multi-file support |
| Bootstrap substrate works for small host-language projects | Full `deps.edn` / Maven CLI parity |
| TFM **net10.0** product path | Rhino **sdk_8** plugin path (separate) |

## Next (not started)

- Wire `./smoke` into `bin/clr-meta-gate` or a thin `pnix-clr` script only if
  product owners want it on every aggregate (cost vs value).
- Optional: `-m demo.main` once confirmed stable on this ClojureCLR pin.
- Still not: nuget.org, full replacement branding.
