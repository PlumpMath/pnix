# 0002 — host-callable-into-pnix-eval

- Status: **ACCEPTED 2026-07-01** (human: "다음~" after 0001). Implemented same day.
- Scope: pnix-hy **interop** lane (`pnix_hy/interop.py`). INSIDE the current scope. Bundles
  candidates B1 (core) + B2 + B3 + B5 + B4 from `0000-interop-language-feature-candidates.md`.
- Placeholder/out-of-scope check: no intentional placeholder touched. This is a NEW capability
  (pnix source reaching a host callable), so it is **capability-gated on `host-call`** — the
  same effect class the gate already enforces. No pnix macros. No ABI-envelope change.
- Boundary impact: none (lane-local; InteropRecord field schema + LOSS_STATUSES unchanged).

## What was implemented

- **B1** `host_callable_to_pnix(fn, *, name=None, arity=None, granted=("host-call",))` — wraps a
  host callable as a pnix `rt.NativeFunc` so pnix SOURCE can apply it as a builtin once placed
  into `ctx["env"]` (which `eval_source_raw` merges into the eval environment). Each pnix arg is
  converted host-ward via `to_host`, the callable is invoked, and the result is converted back
  via `from_host`. Multi-arg callables are **curried** by required-positional arity. Calling a
  host function from pnix is a genuine `host-call` effect: when `host-call` is not granted the
  wrapper **denies at apply time** (`rt.pnix_error`).
- **B2** `call_host(..., kwargs=...)` — threads keyword args through both the host-adapter
  (`call_opaque(fn, args, kwargs)`) and the local (`fn(*args, **kwargs)`) paths, matching
  `call_host_method`.
- **B3** `host_callable_arity(fn)` — projects a host callable's `inspect.signature` into the
  pnix `functionArgs` shape (`{param: has_default}`), plus `*args`/`**kwargs` markers. Mirrors
  `rt.function_args_value` for pnix closures.
- **B5** `host_module_to_pnix(module, *, wrap_callables=True)` — callable public attrs become
  applicable pnix NativeFuncs (depends on B1); data attrs unchanged. Default `wrap_callables=
  False` keeps the old opaque-ref behaviour (no regression).
- **B4** every record-returning crossing (`call_host`, `call_host_method` local paths) now
  carries a `witness_id` on success.
- `host_bridge_report()` self-check registered in `--check` as `interop_host_bridge`.

## Acceptance criteria (all met)

- pnix source `pyLen [1 2 3]` → 3 and curried `pyAdd 3 4` → 7 via env-injected host callables.
- Without `host-call`, the injected callable denies at apply time.
- `call_host(f,(1,),kwargs={...})` passes kwargs; record carries a witness_id.
- `host_callable_arity` matches the `functionArgs` `{name: has_default}` shape.
- a host module's callables are applicable in pnix (`m.add 3 4` → 7); `--check` 45 → **46**,
  `--gate` PASS (sacred lanes untouched).

## Forbidden (kept)

- No pnix macros; no change to `realize_value`/`stable_data`, the InteropRecord schema, or
  `LOSS_STATUSES`. The host-call bridge stays behind the capability gate.
