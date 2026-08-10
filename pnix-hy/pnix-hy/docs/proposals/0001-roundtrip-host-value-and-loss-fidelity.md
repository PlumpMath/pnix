# 0001 — roundtrip-host-value + loss fidelity

- Status: **ACCEPTED 2026-07-01** (human: "후보로만 두지 말고 구현시작"). Implemented same day.
- Scope: pnix-hy **interop** lane (`pnix_hy/interop.py`). INSIDE the current meta-circular-
  projection scope. Bundles candidates A1–A6 from `0000-interop-language-feature-candidates.md`.
- Placeholder/out-of-scope check: touches NO intentional placeholder. Does **NOT** modify
  `pnix_runtime.realize_value` (the canonical str-normalization of PnixPath/PnixString stays);
  only the interop-side MARKING changes. No pnix-side macros. No ABI-envelope change
  (InteropRecord field schema unchanged; loss values stay within `LOSS_STATUSES`).
- Boundary impact: none (lane-local; the shared §14 witness field schema is untouched).

## Problem

`from_host`/`to_host` silently claimed `lossless` for several host→pnix crossings that are in
fact lossy, so a fidelity audit could not trust the loss labels:
- tuple → pnix list: tuple-ness lost, but marked `lossless`.
- dict with non-`str` keys: keys `str()`'d (and can collide, `{1:'a','1':'b'}`), marked `lossless`.
- `bytes`/`bytearray`, `set`/`frozenset`: fell through to an opaque ref (no data crossing).
- `PnixPath`/`PnixString`: `to_host` collapses them to a plain `str` via `stable_data`, losing
  the path / string-context provenance, but marked `lossless`.

## What was implemented

- **A1** `roundtrip_host_value(v)` — crosses a host value host→pnix→host and reports
  `from_host_loss`, `to_host_loss`, a combined `loss_status`, and `equal`. Opaque values
  round-trip by reference (`resolve_opaque`). One helper surfaces every loss below at once.
- **A2** `from_host`: tuple → list marked `lossy` (list stays lossless).
- **A3** `from_host`: dict with a non-`str` key, or a key collision after `str()`, marked `lossy`.
- **A4** `from_host`: `bytes`/`bytearray` → reversible int-octet list, `lossy`.
- **A5** `from_host`: `set`/`frozenset` → pnix list (sorted when orderable), `lossy`.
- **A6** `to_host`: force the raw BEFORE `stable_data`; `PnixPath` → `output_kind='path'`,
  `PnixString` w/ context → `output_kind='string-context'`, both `lossy`. `realize_value`
  untouched.
- `roundtrip_report()` self-check registered in `--check` as `interop_roundtrip`.

## Acceptance criteria (all met)

- lossless data (`None/bool/int/float/str/nested list/str-key dict`) round-trips `equal` and
  `lossless` (no regression: existing `interop_report` still passes).
- tuple/set/bytes crossings report `lossy`; non-str-key & collision dicts report `lossy`;
  `to_host` of a path reports `output_kind='path'` + `lossy`.
- `--check` stays green (44 → **45** reports, all_ready True).

## Forbidden (kept)

- Do not touch `realize_value` / `stable_data` canonicalization. Do not add pnix macros. Do
  not change the InteropRecord field schema or introduce a loss value outside `LOSS_STATUSES`.
