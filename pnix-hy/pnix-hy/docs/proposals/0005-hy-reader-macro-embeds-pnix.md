# 0005 — Hy reader macro embeds pnix at read-time (C4)

- Status: **ACCEPTED 2026-07-01** (human: "다음~" after 0004). Implemented same day.
- Scope: pnix-hy projection lane (`pnix_mirror.py` composition + `hy_mirror.py` Hy-subprocess
  primitive). INSIDE the current scope. Candidate C4 from
  `0000-interop-language-feature-candidates.md`.
- Placeholder/out-of-scope check: this is HY's reader-macro machinery embedding pnix — it does
  NOT add a pnix reader-macro (pnix stays non-homoiconic, SCOPE_LOCK §3). No ABI change.
- Boundary impact: none (runs in the Hy proof subprocess).

## What was implemented

- `hy_mirror.hy_read_with_pnix_reader(source)` — registers a real Hy `#px` reader macro on a
  `HyReader` (`reader_macros["px"]`, handler `(reader, key)` → `reader.parse_one_form()`), reads
  the source, and returns the read model forms plus the embedded pnix strings. `#px "1 + 2"`
  reads to `(pnix-eval "1 + 2")` at READ time.
- `pnix_mirror.hy_reader_embed_pnix(hy_source)` — composes that reader with pnix semantics: for
  each embedded fragment it evaluates the pnix (`rt.eval_source`) and projects it to its Hy form
  (`pnix_to_hy_form`). Hy's reader embeds pnix; pnix-hy supplies the meaning.
- `hy_reader_embed_pnix_report()` self-check registered in `--check` as `hy_reader_embed_pnix`.

## Acceptance criteria (all met)

- `(+ 10 #px "1 + 2")` reads (via the `#px` reader macro) to `(+ 10 (pnix-eval "1 + 2"))`.
- the embedded fragment `1 + 2` evaluates (host pnix) to 3 and projects to the Hy form `(+ 1 2)`.
- `--check` 50 → **51**; `--gate` PASS (sacred lanes untouched).

## Not done (declined this round)

- **C9 `stage7_projection_report`** — declined as ceremony: `pnix_meta_circular_projection`
  already evaluates a pnix form across all four substrates (host interp/compiler + stage7
  runtime/compiler) and reports convergence, which IS the Hy-stage7 ↔ pnix projection seam. A
  separate typed report would duplicate it. Left as a candidate note in `0000`.

## Forbidden (kept)

- No pnix-side reader-macro/macro. No `realize_value`/`stable_data`/InteropRecord/LOSS_STATUSES
  change. `#_pnix-gap` observed, not filled.
