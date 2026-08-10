# 0003 — hy-macro-quasiquote-over-pnix

- Status: **ACCEPTED 2026-07-01** (human: "다음~" after 0002). Implemented same day.
- Scope: pnix-hy meta-circular projection lane (`pnix_hy/pnix_mirror.py` composition +
  `pnix_hy/hy_mirror.py` Hy-subprocess primitive). INSIDE the current scope. Bundles C1 + C2 +
  C3 from `0000-interop-language-feature-candidates.md` — the two SCOPE_LOCK-blessed macro/quote
  interop directions plus their staging correspondence.
- Placeholder/out-of-scope check: **does NOT add pnix macros/quasiquote** (pnix stays
  non-homoiconic — SCOPE_LOCK §3). Every macro/quote here is Hy's, acting ON pnix-projected
  forms / fed FROM pnix values. `#_pnix-gap` markers are observed, never filled. No ABI change.
- Boundary impact: none (composes existing projections; runs in the Hy proof subprocess).

## What was implemented

- **C1** `pnix_mirror.hy_macro_over_pnix(pnix_source, macro_wrap="(when True {form})")` — project
  a pnix expression to a Hy form (`pnix_to_hy_form`), splice it into a Hy macro call, and return
  the Hy macroexpansion (`hy_macroexpand_projection`), best-effort re-synthesizing the expansion
  back to pnix. Demonstrates a Hy MACRO operating over pnix-projected code.
- **C2** `pnix_mirror.hy_quasiquote_over_pnix(template, holes)` — evaluate a Hy quasiquote
  template whose unquote hole(s) are fed from pnix VALUES: each `holes[var]` pnix source is
  evaluated (host pnix), converted to Hy via `_value_to_hy`, bound, and the quasiquote is
  evaluated in the Hy proof subprocess (new `hy_mirror.hy_eval_form`), returning the constructed
  Hy form. A pnix value flows into a Hy quasiquote hole.
- **C3** `pnix_mirror.quasiquote_specialize_correspondence(hy_template, pnix_source,
  dynamic_vars)` — make `hy_mirror._QUASIQUOTE_PNIX_NOTE`'s "quasiquote = manual staging;
  specialize_pnix = automatic staging" analogy EXECUTABLE: the quasiquote hole vars must
  correspond to the pnix dynamic vars (dynamic side) and the static skeleton to the folded
  residual (static side).
- `hy_mirror.hy_eval_form(source)` — evaluate Hy source in the proof Python and project the
  resulting value as a model tree (the C2 primitive).
- `hy_macro_quasiquote_over_pnix_report()` self-check registered in `--check` as
  `interop_hy_macro_bridge`.

## Acceptance criteria (all met)

- C1: pnix `1 + 2` → `(+ 1 2)`, `(when True (+ 1 2))` is a macro expanding to an `if`.
- C2: template `` `(sum ~a ~b) `` with holes `{a:"1 + 2", b:"10"}` → Hy form `(sum 3 10)`.
- C3: `` `(+ ~x 10) `` ↔ `specialize_pnix("x + 10", ("x",))` → hole var `x` corresponds to the
  dynamic var `x`; static skeleton corresponds to the residual `(+ x 10)`.
- `--check` 46 → **47**; `--gate` PASS (sacred lanes untouched).

## Forbidden (kept)

- No pnix-side macro/quasiquote/reader-macro. No change to `realize_value`/`stable_data`, the
  InteropRecord schema, or `LOSS_STATUSES`. `#_pnix-gap` observed, not filled.
