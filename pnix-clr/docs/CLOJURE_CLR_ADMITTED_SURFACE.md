# ClojureCLR admitted surface inventory (P3.2 step 1)

**Date:** 2026-08-14  
**Status:** inventory only — does **not** claim full ClojureCLR replacement.  
**Related:** monorepo `HOST_ENV_P2_P3.md` § P3.2 · `clr-meta/todo.md` § Post host-env

This is the honest map of what **`bin/clojure-clr`**, **`bin/clr-meta`**, and the
upstream bootstrap admit **today**, so later work can expand by named profiles
instead of silently stretching the facade.

---

## Three distinct entrypoints (do not conflate)

| Name | Path | Role |
|------|------|------|
| **`clojure-clr`** | `bin/clojure-clr` | **Focused compatibility facade** over clr-meta tool-eval |
| **`clr-meta`** | `bin/clr-meta` | Full meta bootstrap + compiler selfhost + gates + builders |
| **`clojure-clr-bootstrap`** | `bin/clojure-clr-bootstrap` (+ `bin/build-clr`) | Upstream **Clojure NuGet** pin publish (trust root) |

TFM today: product/runtime paths target **net10.0**. Rhino/plugin work in
dot-nix may use **sdk_8** separately — never mix TFMs silently.

---

## `bin/clojure-clr` — admitted CLI

Source of truth: `bin/clojure-clr` (fail-closed).

| Admitted | Form | Behavior |
|----------|------|----------|
| Yes | `-e FORM` or `--eval FORM` (exactly 2 argv) | `exec bin/clr-meta "$@"` |
| Yes | single path that is an existing file (exactly 1 argv) | `exec bin/clr-meta FILE` |
| No | REPL, multi-file projects, `-M`, `deps.edn`, `clojure` CLI parity | stderr + exit 2 |

Error text (verbatim intent):

```text
clojure-clr compatibility: admitted surface is -e FORM or one FORM file;
use clojure-clr-bootstrap for the upstream trust root
```

### What “FORM / file” means under clr-meta tool-eval

Delegated to `pnix.clr-meta.main` (tool profile), **not** full Clojure:

- Exactly **one** form (reader evaluation disabled; tagged/conditional readers
  rejected).
- Value domain restricted to the **admitted portable form domain** (outside →
  fail closed before eval).
- Evaluation via **physical evaluator generation 2** (nested interpreter lane;
  **not** Compiler Stage1–15/N).
- No `load-string` path on this tool surface.

So `clojure-clr` is a **name-compatible sliver**, not “Clojure on CLR for
arbitrary projects.”

---

## `bin/clr-meta` — broader but still profiled

| Profile | Examples | Notes |
|---------|----------|--------|
| Tool-eval | `-e`, single file, `--gate` (eval-family) | Same reader/domain rules as above for form eval |
| Runtime artifact | `--build-runtime PLAN OUT SRC` | Hash-bound AOT for **pnix-clr** product namespaces |
| Compiler selfhost | `--build-compiler-selfhost-stageN …` | Stage ladder; see `STATUS.md` / design docs |
| Aggregate | `bin/clr-meta-gate` | Full family; do not claim promotion |

Closed compiler/selfhost claims are listed in `clr-meta/STATUS.md` and
`STAGE15_N_ROADMAP.md` Open claims (honest remaining: general IL fixed point,
broad ClojureCLR compatibility, host promotion, …).

---

## Upstream substrate (trust root)

| Piece | Location |
|-------|----------|
| NuGet pin | `Clojure` 1.12.3-alpha8 via `clr-bootstrap/` |
| Publish | `bin/build-clr` → `clojure-clr-clojure-…/…/publish/` |
| Main assembly | `Clojure.Main.dll` (net10.0) |

Broader upstream compiler/runtime operations: **`clojure-clr-bootstrap`**, not
the `clojure-clr` facade.

---

## Expansion roadmap (do not skip steps)

From `clr-meta/todo.md` Post host-env / P3.2:

1. **[x] Inventory** (this document).
2. **[ ] TFM policy write-up** — net10 product vs net8 Rhino; document in
   `HOST_DEV_ENV` / dot-nix only where packaging touches both.
3. **[ ] Named profiles** — admit one extra surface at a time (e.g. multi-form
   file, then multi-file load, then project template), each with a gate.
4. **[ ] Project template + smoke** — two namespaces from disk without
   `pnix-clr` guest CLI; still **not** full `clojure` CLI replacement.
5. **[ ] nuget.org / registry** — only after template + local pack are stable.

Forbidden shortcuts:

- Renaming `clojure-clr` to imply full ClojureCLR.
- Claiming Stage15/N or Trusting-Trust from facade `-e` alone.
- Mixing Rhino sdk_8 and pnix-clr net10 in one unstated profile.

---

## Quick smoke (facade only)

```bash
cd pnix-clr
./bin/build-clr                 # if substrate missing
./bin/clojure-clr -e '(+ 20 22)'   # => 42 via clr-meta gen2
echo '(+ 1 2)' > /tmp/t.clj
./bin/clojure-clr /tmp/t.clj
./bin/clojure-clr -M -e 1         # must fail closed (exit 2)
```
