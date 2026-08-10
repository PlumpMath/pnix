# 0010 — module distribution (installable) without losing any existing feature

- Status: **SHIPPED 2026-07-02** (accepted "모듈로도 배포가능하게 추가"). Additive only.
- Scope: pnix-hy packaging + path resolution. **All existing behavior is preserved byte-for-byte
  when `PNIX_HY_HOME` is unset** (in-repo / flake / editable). No feature removed.
- Placeholder/out-of-scope check: no sacred change (`pnix_runtime.py` untouched), no second
  mirror/gate, no hy-meta duplication. Purely a path-resolution + packaging-metadata add.

## Goal

`import pnix_hy` on plain Python already works (core: pnix eval / safe_eval / gate / action / ir /
witness / explain — zero deps, no tree). This proposal makes the **projection / proof tiers also
reachable from an INSTALLED (pip/nix, off-tree) package**, while keeping in-repo behavior identical.

## What was implemented (additive)

- **Layered `HY_ROOT` / hy-meta discovery** — default = the repo sibling of `pnix_hy` (UNCHANGED);
  `PNIX_HY_HOME=/path/to/checkout` overrides it so an off-tree install can find the vendored `hy`
  + `hy-meta` tree. Routed through one place each: `hy_mirror.HY_ROOT` and `interop._hy_meta_dir()`
  (used by `_host_interop` + `_host_import_hook`).
- **`pnix_hy/deploy.py` + `deployment_info()` + CLI `--deployment`** — reports install path,
  resolved `HY_ROOT`, whether hy-meta / vendored hy / proof Python are found, and which tiers
  (`core` / `projection` / `full_gate`) work here, with a hint. Diagnostic-only.
- **`pyproject.toml`** — `full = ["hy==1.3.0"]` extra (alongside `projection`), `[project.urls]`,
  and a comment documenting the tiers + `PNIX_HY_HOME`.

## Tiers

```
pip install pnix-hy                 # core: pnix eval / safe_eval / gate / action / ir / witness / explain
pip install pnix-hy[projection]    # + Hy 1.3.0 for Hy<->pnix projection / mirror-over-Hy
pip install pnix-hy[full]          # + proof ladder; also set PNIX_HY_HOME=<checkout> (hy-meta + hy)
```
Projection/proof degrade gracefully (raise `HyMirrorError` at call time / report `available:False`)
when Hy or the tree is absent — the core is never affected.

## Acceptance criteria (all verified)

- (A) in-repo, `PNIX_HY_HOME` unset → `--check` 56/56, `--gate` PASS, projection works — **identical
  to before**.
- (B) off-tree copy of `pnix_hy` + `PNIX_HY_HOME=<repo>` → projection works off-tree
  (`hy_macro_over_pnix` → `(+ 1 2)`; `deployment_info` tiers projection=True).
- (C) off-tree + no `PNIX_HY_HOME` + no Hy → core works (`check_action` accepted), projection
  degrades (`HyMirrorError`), `deployment_info` projection=False + hint.

## Forbidden (kept)

- No behavior change when `PNIX_HY_HOME` is unset. No sacred-lane / `pnix_runtime.py` change. No
  bundling of hy-meta into the core wheel (it stays the host lane; `full` reaches it via env/checkout).
