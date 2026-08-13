# ClojureCLR admitted surface inventory (P3.2 step 1)

**Date:** 2026-08-14  
**Status:** inventory only — does **not** claim full ClojureCLR replacement.  
**Related:** monorepo `HOST_ENV_P2_P3.md` § P3.2 · `clr-meta/todo.md` § Post host-env

This is the honest map of what **`bin/clojure-clr`**, **`bin/clr-meta`**, and the
upstream bootstrap admit **today**, so later work can expand by named profiles
instead of silently stretching the facade.

---

## Named profiles (do not conflate)

| Profile | Entrypoint | Role |
|---------|------------|------|
| **`tool-eval`** | `bin/clojure-clr` | Focused facade: `-e` / one **single-form** file |
| **`tool-eval-multi`** | `clojure-clr --multi-form FILE` | Opt-in: multiple top-level forms L→R, last value (named gate) |
| **`bootstrap`** | `bin/clojure-clr-bootstrap` | Upstream Clojure.Main (full CLI flags the substrate admits) |
| **`bootstrap-project`** | `examples/clojure-clr-project/` | Multi-ns sample on bootstrap + `CLOJURE_LOAD_PATH` |
| **`meta`** | `bin/clr-meta` | Selfhost builders, gates, runtime-artifact, tool-eval family |

Gate for the named profiles (also wired into `bin/pnix-clr-gate`):

```bash
./bin/clojure-clr-profiles-smoke
# tool-eval + tool-eval-multi + bootstrap-project → 42 (5 checks)
```

TFM: **net10.0** product path; Rhino **sdk_8** separate — see `TFM_POLICY.md`.

---

## `bin/clojure-clr` — admitted CLI

Source of truth: `bin/clojure-clr` (fail-closed).

| Admitted | Form | Behavior |
|----------|------|----------|
| Yes | `-e FORM` or `--eval FORM` (exactly 2 argv) | `exec bin/clr-meta "$@"` (single form) |
| Yes | single path that is an existing file (exactly 1 argv) | `exec bin/clr-meta FILE` (single form; trailing fails) |
| Yes | `--multi-form FILE` (exactly 2 argv, file exists) | `tool-eval-multi` — all top-level forms, last value |
| No | REPL, `-i`, `-M`, deps.edn, clojure CLI parity | stderr + exit 2 |

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
2. **[x] TFM policy write-up** — [`TFM_POLICY.md`](TFM_POLICY.md) (net10 product
   vs net8 Rhino / multi-target Pnix.Clr).
3. **[x] Project template + smoke (bootstrap profile)** —
   `examples/clojure-clr-project/` loads **two namespaces** via
   `clojure-clr-bootstrap` + `CLOJURE_LOAD_PATH` (not the facade).
   `./smoke` expects `42`. Still **not** `clojure-clr` multi-file, not
   deps.edn parity.
4. **[x] Named profiles + dual smoke** — `tool-eval` / `bootstrap` /
   `bootstrap-project` documented; `bin/clojure-clr-profiles-smoke` +
   `clojure-clr --help` (2026-08-14).
5. **[x] tool-eval-multi** — `--multi-form FILE` +
   `scripts/clr-meta-tool-eval-multi-gate` (wired into `clr-meta-gate`);
   default single-form trailing rejection preserved (2026-08-14).
6. **[x] profiles-smoke in product aggregate** — `bin/pnix-clr-gate` runs
   `clojure-clr-profiles-smoke` (~17s, 2026-08-14).
7. **[x] Local nupkg pack smoke** — `bin/pnix-clr-nupkg-smoke` (export layout +
   dual-TFM pack; local feed only, 2026-08-14).
8. **[ ] nuget.org / registry** — only after local pack stays green; needs
   owner secrets/signing (do not auto-publish).

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
