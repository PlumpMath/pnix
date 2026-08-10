# 0004 — interop diagnostics & invariants (C5 + C7 + C8)

- Status: **ACCEPTED 2026-07-01** (human: "다음~" after 0003). Implemented same day.
- Scope: pnix-hy projection lane (`pnix_mirror.py`) + interop lane (`interop.py`). INSIDE the
  current scope. Bundles the thin, non-ABI candidates C5 + C7 + C8 from
  `0000-interop-language-feature-candidates.md`.
- Placeholder/out-of-scope check: read-only diagnostics; `classify_drift` OBSERVES the honest
  `#_pnix-gap`/`gaps` markers and does NOT fill them (SCOPE_LOCK §3). No pnix macros. No ABI
  change (InteropRecord schema / LOSS_STATUSES untouched).
- Boundary impact: none.

## What was implemented

- **C5** `pnix_mirror.classify_drift(pnix_source)` — reclassify the pnix→Hy projection `gaps`
  (from `pnix_to_hy_form`) into a stable category enum (`no-hy-operator` /
  `no-projection-construct` / `construct-gap` / `other`), with per-category counts. Read-only;
  observes gaps, never fills them.
- **C7** `pnix_mirror.reify_hy(hy_source)` — uniform Hy-side reification surface symmetric to
  `reify_pnix`: packages the EXISTING Hy projections (reader form, Python AST/source lowering,
  synthesized pnix + its IR + value) into one `{reified:{source,form,python,pnix,ir,value}}`
  envelope. Reuses `hy_form_projection` + `synthesize_pnix_from_hy` — no second projector.
- **C8** `interop.no_mirror_report()` — invariant guard (SCOPE_LOCK): `to_host`/`from_host`/
  `make_opaque_ref`/`resolve_opaque`/`call_host`/`host_callable_to_pnix` all work with the
  mirror OFF (a `runtime_context` with no `events`), so interop never becomes mirror-dependent.
- Reports registered in `--check`: `classify_drift`, `reify_hy`, `interop_no_mirror`.

## Acceptance criteria (all met)

- `classify_drift("(x: x + 1)")` clean; `classify_drift("with m; b")` →
  `{no-projection-construct: 1}`.
- `reify_hy("(+ 1 2)")` → synthesized pnix `(1 + 2)`, value 3, Python `1 + 2`.
- interop ops succeed with no mirror context.
- `--check` 47 → **50**; `--gate` PASS (sacred lanes untouched).

## Forbidden (kept)

- No pnix macros; `#_pnix-gap` observed not filled; no `realize_value`/`stable_data`/
  InteropRecord/LOSS_STATUSES change.
