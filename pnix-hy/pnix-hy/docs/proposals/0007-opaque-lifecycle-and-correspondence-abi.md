# 0007 — opaque-ref lifecycle (D2 in-scope) + versioned correspondence ABI (D3 in-scope)

- Status: **ACCEPTED 2026-07-01** (human: "다음~" after 0006). Implemented same day.
- Scope: pnix-hy interop lane (`interop.py`) + projection lane (`pnix_mirror.py`). INSIDE the
  current scope. Ships ONLY the lane-local, ABI-shape-preserving slices of D2 + D3 from
  `0000-interop-language-feature-candidates.md`.
- Placeholder/out-of-scope check: the opaque-ref DICT shape (`__pnix_opaque__`/`__hy_meta_opaque__`,
  §18-19) is **unchanged**; lifecycle metadata is a lane-local sidecar. The correspondence ABI is
  a content hash over the EXISTING `correspondence_table()`; no shared-shape change. No pnix macros.
- Boundary impact: none on the shared hy-meta↔pnix-hy envelope.

## What was implemented

- **D2 (lane-local slice)** opaque-ref lifecycle for the pnix fallback registry:
  - a sidecar `_OPAQUE_META` records `{kind, created_by, released}` per local ref (the ref DICT
    shape is untouched).
  - `make_opaque_ref(obj, kind, *, prefer_host=True)` — new keyword lets a caller force the local
    lane (default behaviour unchanged: host adapter preferred when present).
  - `release_opaque` marks the sidecar `released=True` and drops the strong ref.
  - `opaque_lifecycle()` → `{live, released, total}`; a leak = created-but-never-released.
- **D3 (versioned-artifact slice)** `pnix_mirror.correspondence_abi()` — the existing
  `correspondence_table()` re-emitted as a content-hashed, versioned ABI artifact
  (`abi_version` + `abi_sha256` over normalized rows: `source_node/hy_form/pnix_tag/value_type/
  loss/supported`), so a downstream lane (pnix-hs/pnix-rs, future) can pin a version by hash and
  detect drift. Does NOT replace `correspondence_table` or its `_TAG_PROBES` drift-guard.
- Reports registered in `--check`: `opaque_lifecycle`, `correspondence_abi`.

## Acceptance criteria (all met)

- two local opaque refs: resolve works; releasing one → `opaque_lifecycle()` shows 1 live / 1
  released; a made-not-released ref is countable (leak signal). Ref dict shape unchanged.
- `correspondence_abi()` is deterministic (stable `abi_sha256` across calls), `row_count` equals
  the live table, every row has `loss ∈ {lossless,lossy}` and a `supported` flag.
- `--check` 52 → **54**; `--gate` PASS (sacred lanes untouched).

## Deferred (still genuinely cross-lane — NOT shipped here)

- **D2 refcount on the SHARED ref shape** and **D3 cross-repo vocabulary unification** need both
  hy-meta + pnix-hy + a gate drift-guard. Left in `0000` for a dedicated both-lane proposal.

## Forbidden (kept)

- No change to the opaque-ref DICT shape, the §14 witness schema, `realize_value`/`stable_data`,
  or `LOSS_STATUSES`. No pnix macros.
