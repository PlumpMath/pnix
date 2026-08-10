# 0006 — cross-boundary error contract (D1) + interop role matrix (D4)

- Status: **ACCEPTED 2026-07-01** (human: "다음~" after 0005). Implemented same day.
- Scope: pnix-hy interop lane (`interop.py`) + a doc (`docs/INTEROP_ROLE_MATRIX.md`).
  Candidates D1 + D4 from `0000-interop-language-feature-candidates.md`.
- Placeholder/out-of-scope check: D1 is implemented **pnix-side only** — it changes the
  ambiguous EXCEPTION return of `call_host`/`call_host_method`, which nothing consumes, into an
  unambiguous envelope. It does **NOT** touch the shared hy-meta↔pnix-hy ABI envelope (§14
  witness field schema / §18-19 opaque-ref shape), so no both-lane change / drift-guard is
  needed. No pnix macros.
- Boundary impact: none on the shared envelope.

## Problem (D1)

`call_host`/`call_host_method` returned host exceptions as `{"exception": "ValueError: ..."}` —
indistinguishable from a legitimate pnix attrset `{ exception = ...; }`. And
`wrap_pnix_callable` let a raw `rt.PnixError` leak across the boundary into host code.

## What was implemented

- **D1** unambiguous error value: on a host exception, `call_host`/`call_host_method` now return
  `{"__interop_error__": {"kind","type","message"}}` (a reserved key), with helpers
  `is_interop_error(result)` / `interop_error_of(result)`. Capability DENIAL keeps its `{"denied":
  ...}` shape (distinct from an exception).
- `try_call_host(fn, args, kwargs=...)` — a `tryEval`-shaped wrapper: `{"success": True,
  "value": v}` or `{"success": False, "error": {...}}`; never collides with an attrset.
- `wrap_pnix_callable` now catches `rt.PnixError` and raises a typed `InteropError` instead of
  leaking the raw pnix internal to host callers.
- `error_contract_report()` self-check registered in `--check` as `interop_error_contract`.
- **D4** `docs/INTEROP_ROLE_MATRIX.md` — a feature × owner × status × proposal table so agents
  don't re-open intentional gaps (esp. "pnix macros = do not implement").

## Acceptance criteria (all met)

- a host exception → `is_interop_error` True (type surfaced); a pnix attrset `{ exception = 1; }`
  → `{"exception": 1}` and `is_interop_error` False (no misread).
- `try_call_host` returns `{success:True,value:3}` / `{success:False,error:{type:ValueError}}` /
  `{success:False,error:{kind:denied}}`.
- `wrap_pnix_callable` raises `InteropError` (not raw `PnixError`) on a pnix eval failure.
- `--check` 51 → **52**; `--gate` PASS (sacred lanes untouched).

## Forbidden (kept)

- No change to the shared ABI envelope (§14/§18-19), `realize_value`/`stable_data`, or
  `LOSS_STATUSES`. Capability-denial shape unchanged. No pnix macros.

## Deferred (still candidates)

- **D2** opaque-ref lifecycle on the SHARED shape and **D3** versioned correspondence ABI are
  genuinely cross-lane (both lanes + gate drift-guard) — left in `0000` for a dedicated
  both-lane proposal. A7 (opaque-ref passthrough) remains low-value.
