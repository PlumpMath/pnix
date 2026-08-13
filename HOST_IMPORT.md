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

---

## Library packaging tiers (do not over-claim)

| Host | Flake package named `*-library`? | Library body | HM helper |
|------|----------------------------------|--------------|-----------|
| **clj** | No | `pnix-clj` sources (`-Sdeps` local/root) | `pnix-clj-library` / `refs` |
| **cljs** | No (body is `packages.pnix-cljs` share/) | `$out/share/pnix-cljs` + optional `lib/node_modules/@plumpmath/pnix-cljs` | `pnix-cljs-library` |
| **hy** | No (body is `packages.pnix-hy`) | `site-packages/pnix_hy` | `pnix-hy-library` |
| **rs** | **Yes** `packages.pnix-rs-library` | rlib/a/dylib + header | `pnix-rs-refs` |
| **clr** | **Yes** app/export `pnix-clr-library` | `Pnix.Clr` + guest AOT + MSBuild props | `pnix-clr-refs` |

```text
nix run .#pnix-rs-library     # rs: real package
nix run .#pnix-clr-library    # clr: export materializer
# clj / hy / cljs: use the product package + HM *-library helpers, not a separate flake app name
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
pnix-clr-refs          # may export library on first run
pnix-clr -e '1 + 2'
# C#: Eval.File / Eval.Source after Import of $PNIX_CLR_LIBRARY/build/Pnix.Clr.props
```

---

## Verified smoke (2026-08-14, HM profile + local tip)

| Host | Command | Result |
|------|---------|--------|
| clj | `(pnix-clj.core/eval-file …)` → `:value 3` | ok |
| hy | `pnix_hy.eval_file` → `3` | ok |
| cljs | `require('pnix-cljs-module.js').evalFileValueJson` → `3` | ok |
| rs | `pnix-rs px-eval -c '1 + 2'` → `3` | ok |
| clr | `pnix-clr -e '1 + 2'` → JSON value 3 | ok |
| helpers | `pnix-*-library` / `pnix-rs-refs` / `pnix-clr-refs` | print paths |
