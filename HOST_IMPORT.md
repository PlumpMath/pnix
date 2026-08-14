# Host-language import cookbooks (index)

Dual-axis doctrine: **[HOST_DEV_ENV.md](HOST_DEV_ENV.md)**.

Per-host detail:

| Host | Cookbook |
|------|----------|
| clj | [pnix-clj/pnix-clj/docs/HOST_IMPORT.md](pnix-clj/pnix-clj/docs/HOST_IMPORT.md) |
| cljs | [pnix-cljs/HOST_IMPORT.md](pnix-cljs/HOST_IMPORT.md) |
| hy | this file § hy (package is the library) |
| rs | this file § rs + `pnix-rs/include/pnix_rs.h` |
| clr | [pnix-clr/csharp/Pnix.Clr/README.md](pnix-clr/csharp/Pnix.Clr/README.md) |

HM path helpers: `pnix-<host>-library` / `pnix-<host>-refs` (see `~/dot-nix/dev/PNIX-HOSTS.md`).

**P2/P3 roadmap:** [HOST_ENV_P2_P3.md](HOST_ENV_P2_P3.md)  
**Mini examples:** [examples/host-import/](examples/host-import/)  
**Regression:** `./bin/host-import-smoke`  
**Local library smokes (all hosts):** `./bin/host-library-smokes`

---

## Personal / local library export (not public registries)

Owner policy: **no** Maven Central / npm / crates.io / nuget.org publish.
Each host has a **local feed** materializer + smoke:

| Host | Export | Smoke | Consumer |
|------|--------|-------|----------|
| **clj** | `pnix-clj/pnix-clj/bin/export-pnix-clj-library` | `pnix-clj-library-smoke` | `{:local/root "…/pnix-clj"}` |
| **cljs** | `pnix-cljs/bin/export-pnix-cljs-library` | `pnix-cljs-library-smoke` | `NODE_PATH=…/lib/node_modules:…/share` |
| **hy** | `pnix-hy/pnix-hy/bin/export-pnix-hy-library` | `pnix-hy-library-smoke` | `PYTHONPATH=…/site` |
| **rs** | `pnix-rs/pnix-rs/bin/export-pnix-rs-library` | `pnix-rs-library-smoke` | path-dep or `-L lib -I include` |
| **clr** | `pnix-clr/bin/export-pnix-clr-library` (+ pack) | `pnix-clr-library-smoke` | `PNIX_CLR_LIBRARY` / local nupkg dir |

```bash
./bin/host-library-smokes   # clj hy rs cljs (+ clr if already exported)
```

---

## Library packaging tiers (do not over-claim)

| Host | Flake `*-library` / `*-refs` | Library body | HM helper |
|------|------------------------------|--------------|-----------|
| **clj** | app/printer `pnix-clj-library` | `pnix-clj` sources (`-Sdeps` local/root) | `pnix-clj-library` |
| **cljs** | app/printer `pnix-cljs-library` | share/ + `lib/node_modules/@plumpmath/pnix-cljs` | `pnix-cljs-library` |
| **hy** | app/printer `pnix-hy-library` | `packages.pnix-hy` site-packages | `pnix-hy-library` |
| **rs** | **package** `pnix-rs-library` + app `pnix-rs-refs` | rlib/a/dylib + header | `pnix-rs-refs` |
| **clr** | **export app** `pnix-clr-library` + `pnix-clr-refs` | `Pnix.Clr` + guest AOT + MSBuild props | `pnix-clr-refs` |

```text
nix run .#pnix-clj-library    # path contract (sources)
nix run .#pnix-hy-library
nix run .#pnix-cljs-library
nix run .#pnix-rs-library     # real embeddable artifacts
nix run .#pnix-rs-refs
nix run .#pnix-clr-library    # materialize export tree
nix run .#pnix-clr-refs
```

---

## hy (Python)

```python
import pnix_hy as ph

ph.eval_source("1 + 2")
ph.eval_file("prog.px")   # alias of run_px
```

Public top-level exports: see `pnix_hy.__all__` (`eval_source`, `eval_file`,
`run_px`, interop helpers, …). Proof/meta loaders: `load_proof_api()`,
`load_meta_api()`.

Optional host-only import hook (not common-meta):

```python
from pnix_hy import install_pnix_import_hook
# Install roots so Python import can load host-bound .px modules via pnix-hy.
# See pnix_hy.interop.install_pnix_import_hook docstring.
```

**Name clash:** flake app `.#pnix-hy-hy` is `pnix-hy --repl hy` (needs source
tree). HM PATH bin `pnix-hy-hy` is the **bare Hy interpreter** with
`PYTHONPATH` for `pnix_hy`. Do not equate them.

```bash
python -c 'import pnix_hy as ph; print(ph.eval_file("prog.px"))'
pnix-hy-library
pnix-hy-pnix
```

---

## rs (Rust)

Cargo patterns: [pnix-rs/docs/CARGO_HOST_IMPORT.md](pnix-rs/docs/CARGO_HOST_IMPORT.md).


```rust
// After linking with -L $PNIX_RS_LIB_DIR and including pnix_rs.h for C ABI
// Native:
let s = pnix_rs::eval("1 + 2")?;
let s = pnix_rs::eval_file("prog.px")?;
```

```c
#include "pnix_rs.h"
char *out = NULL;
if (pnix_rs_eval("1 + 2", &out) == 0) { /* use out */ pnix_rs_string_free(out); }
```

```bash
pnix-rs-refs
pnix-rs px-eval -c '1 + 2'
```

Never put full `pnix-rs` + `pnix-rs-library` in one `buildEnv` (dylib clash).

---

## clr (C# / ClojureCLR)

See `pnix-clr/csharp/Pnix.Clr/README.md`.

```bash
./bin/export-pnix-clr-library
./bin/pack-pnix-clr-nupkg          # optional local nupkg
# MSBuild: csharp/Directory.Build.props.sample
```

```bash
pnix-clr-refs          # may export library on first run
pnix-clr -e '1 + 2'
# C#: Eval.File / Eval.Source after Import of $PNIX_CLR_LIBRARY/build/Pnix.Clr.props
```

---

## Verified smoke (2026-08-14)

Also: monorepo `./bin/host-import-smoke` (uses PATH).

## Verified smoke log

| Host | Command | Result |
|------|---------|--------|
| clj | `(pnix-clj.core/eval-file …)` → `:value 3` | ok |
| hy | `pnix_hy.eval_file` → `3` | ok |
| cljs | `require('@plumpmath/pnix-cljs').evalSourceJson('1+2')` → value 3 | ok (user 2026-08-14) |
| rs | `pnix-rs px-eval -c '1 + 2'` → `3` | ok |
| clr | `pnix-clr -e '1 + 2'` → JSON value 3 | ok |
| helpers | `pnix-*-library` / `pnix-rs-refs` / `pnix-clr-refs` | print paths |
