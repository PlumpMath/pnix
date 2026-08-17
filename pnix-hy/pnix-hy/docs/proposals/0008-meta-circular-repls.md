# 0008 — meta-circular REPLs (5 modes, thin front-ends)

- Status: **SHIPPED 2026-07-01** (accepted "구현시작"). Structure: thin front-end (as chosen).
  `pnix_hy/repl.py` + `pnix-hy-project --repl {python|hy|pnix}` + 5 flake apps
  (`pnix-hy-{pnix,hy,python}`, `hy-meta-{hy,python}` — the final shipped names, without the
  `repl-` prefix this proposal originally drafted below); `repl` self-check → `--check`
  now 55/55; `--gate` PASS. Verified: pnix REPL retains context (`a=20; b=a+22; b`→42; attrsets;
  `:env`/`:reset`; survives a bad line), python REPL (`ph` preloaded), hy REPL (hands off to the
  proof Python's Hy 1.3.0). No core/sacred/ABI change.
- Scope: pnix-hy (a NEW `pnix_hy/repl.py` front-end + a CLI `--repl` mode) + flake apps for the
  hy-meta launchers. INSIDE the current scope as an ADDITIVE front-end layer.
- Placeholder/out-of-scope check: the REPLs are thin front-ends OVER the existing libraries;
  they do **not** enter the eval hot path, add pnix macros (pnix stays non-homoiconic), or touch
  the sacred lanes / shared ABI / `realize_value`. No new deps.
- Boundary impact: none.

## Motivation

Interactive, **context-retaining** exploration of the three hosted languages. A REPL is a
long-lived **warm** process (interpreter + `import hy` + pnix env stay hot), so it is *faster*
than repeated CLI calls — the "REPL is slow" concern only applies to a REPL that re-spawns /
cold-imports per line, which this design avoids by keeping state in one process and reusing the
existing persistent Hy worker. The two cores (`hy-meta`, `pnix_hy`) stay pure library/CLI; the
REPLs are a separate exposure layer.

## The 5 modes (only ONE needs new interpreter code)

| mode | what it is | new code? |
|---|---|---|
| hy-meta · python | stdlib CPython REPL, repo root on `sys.path` (hy-meta importable) | launcher only |
| hy-meta · hy | the standard `hy` REPL, run from the repo root | launcher only |
| pnix-hy · python | stdlib CPython REPL with `import pnix_hy` preloaded | launcher only |
| pnix-hy · hy | the standard `hy` REPL | launcher only |
| **pnix-hy · pnix** | **new pnix REPL loop with an accumulating pnix env** | `pnix_hy/repl.py` |

## Design

- **`pnix_hy/repl.py`** (the only new logic): a pnix REPL that keeps context across inputs by
  threading a growing environment through the existing runtime — `rt.eval_source_raw(src, ctx,
  realize=True)` with a persistent `ctx["env"]` (the same env-merge the `host_callable_to_pnix`
  work already uses). Semantics:
  - `name = expr` (or `:let name = expr`) — evaluate `expr` in the current env, bind `name`
    (lazy thunk; pnix is pure/lazy, so a binding is just a stored value/thunk).
  - a bare `expr` — evaluate in the current env, print the value, bind it to `_` (last result).
  - errors are caught and shown via `diagnose(src)` — the session survives a bad line.
  - `:reset`, `:env`, `:help`, EOF/`:quit` meta-commands.
  - Pure + warm: the pnix runtime stays in-process; no per-line subprocess.
- **CLI**: `pnix-hy-project --repl {python|hy|pnix}` (a new facility flag) and/or a
  `pnix-hy-repl` console-script. `python` → `code.interact` with `pnix_hy` imported; `hy` → hand
  off to the proof Python's `hy` REPL; `pnix` → `pnix_hy.repl.run()`.
- **flake apps** (launchers, `PNIX_HY_PYTHON` + repo-root aware; drafted here with a `repl-`
  prefix, shipped without it — see the Status line above): `nix run .#pnix-hy-pnix`,
  `.#pnix-hy-hy`, `.#pnix-hy-python`, `.#hy-meta-hy`, `.#hy-meta-python`. The devShell already
  puts `python`/`hy` on PATH, so these are conveniences.

## Acceptance criteria

- all 5 modes launch and **retain context** across inputs (e.g. pnix: `a = 1` then `a + 1` → 2;
  `_` holds the last value; a syntax error does not end the session).
- the pnix REPL adds no per-line process spawn (one warm process); `import pnix_hy` /
  `pnix_hy.repl` does not change the eval hot path.
- core unchanged: `--gate` still PASS; `--check` unaffected (optionally +1 if a `repl` self-check
  report is added — a headless "feed 3 lines, assert accumulated env" check).
- no new runtime deps; python/hy modes reuse stdlib / the flake's Hy 1.3.0.

## Forbidden (kept)

- No pnix macros/quasiquote (REPL is eval-only). No change to `realize_value`/`stable_data`, the
  sacred lanes, the InteropRecord/witness ABI, or the two cores' library/CLI shape. REPL code is
  a front-end module, never imported by the runtime.

## Notes

- `pnix-clj` (a separate Clojure-hosted pnix) is OUT of this repo's scope (`SCOPE_LOCK.md` §5);
  this proposal covers only the `hy-meta` + `pnix-hy` (python/hy/pnix) modes. A pnix-clj REPL
  would belong to that project's own scope/flake.
