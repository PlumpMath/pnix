# pnix-hy / hy-meta — Meta-Circular Implementation Audit

> Produced by a read-only multi-agent audit (23 agents, 11 capability dimensions, ~1.06M
> tokens) of the codebase vs the §1–§24 "Pure Meta-Circular Capability Checklist", on
> 2026-07-01. Each capability was audited then ADVERSARIALLY VERIFIED against the actual
> code; every claim cites `file:symbol`. Goal: find duplication, the real status, missing
> items, and exact implementation guidance (reuse, don't rebuild).

## Mistake-hunt verdict (2026-07-01)

A broad adversarial mistake-hunt (15 agents, 7 dimensions, ~1.13M tokens) swept ALL of
pnix-hy + hy-meta for real defects: idempotency/hidden-state, hollow/always-True reports,
failure-hiding exception handling, scope/sacred violations, doc↔code drift, flaky gates, and
interop edge cases. **Verdict: NO sacred-surface or SCOPE_LOCK violations; no hidden global
state beyond the one already-fixed opaque case; results correct.** 18 real defects were
demonstrated (3 high, 10 medium, 5 low); **17 fixed this pass, 1 deferred**:
- **Robustness (high):** `--check`/`--gate` now isolate each report/lane via `cli._safe_report`
  (a slow/raising report → FAIL row, never a whole-audit crash); `hy_meta_host_api_report`,
  `_one_shot_hy_script`, and `run_bootstrap` now convert `TimeoutExpired`/`OSError` into a
  FAIL/`HyMirrorError` (host-api budget raised to env-configurable 300s) — this neutralizes the
  60s-timeout crash the user hit.
- **Hollow-report vacuity (medium):** the 4 sacred-mirror batch lanes + `compiler_emit_shape_report`
  + `fixture_report` now require `len==len(cases)`/`bool(cases)` (no `zip`-truncation / `all([])`
  vacuous pass).
- **Interop correctness (medium/low):** `from_host` now aggregates nested effect/capability;
  `to_host` marks a nested-function container lossy/host-call; nullary host callables handled
  (`_required_positional_count` returns 0, `host_callable_to_pnix` special-cases it);
  `hy_macro_over_pnix` uses `.replace` not `.format` (Hy `{}` literals); `classify_drift` guards
  unparseable source; `try_call_host` guards the reserved-key collision.
- **Docs:** current-state `--check` counts corrected to 54 (dated-historical snapshots preserved);
  role-matrix witness miscite + 28→29 row count fixed.
- **Deferred (1, medium):** `hy_mirror._proj_worker_run`/`_stage7_worker_eval` `readline()` has no
  deadline — a *wedged* worker could block the gate. Trigger is synthetic (our worker script
  always responds or exits); a deadline change risks the hot path, so it is a noted candidate,
  not fixed. All fixes verified: `--check` 54/54, `--gate` PASS; `pnix_runtime.py` change is a
  single report-only guard (no eval/4-lane semantics touched).

## Stub-hunt verdict (2026-07-01)

A third, independent multi-agent sweep (15 agents, 6 areas, ~0.79M tokens) hunted the ENTIRE
codebase for GENUINELY unimplemented code — `NotImplementedError`, hollow `pass`/`...`/
`return None|{}|[]` bodies, hard-coded `ready=True` reports, non-delegating "facades",
registered-but-empty builtins, `TODO/FIXME`. **Verdict: NONE genuinely unimplemented.** Every
candidate resolved to intentional, documented placeholder or real-but-demoted logic; all ~44+
`*_report` readiness flags compute from concrete probes; both todos have ZERO open `[ ]`.
Confirmed-intentional placeholders (do NOT implement): derivation `outPath`/`drvPath` store
addressing (`pnix_runtime.py:2694`; no store hashing by design), `builtins.placeholder` (the
real Nix builtin), `trace`/`warn` value-identity (pure-mirror side-effect omitted), the §9
pnix macro/quasiquote/reader-macro absence (`_QUASIQUOTE/_DEFMACRO/_READER_MACRO/_IMPORT_PNIX_NOTE`),
the `#_pnix-gap[...]` projection markers (always paired with a `gaps.append`), the fail-closed
stage16 peer-review record, host standalone/optional fallbacks, and runtime "unsupported …"
ERROR MESSAGES. The codebase is complete w.r.t. its stated meta-circular-projection scope.

## Re-verification update (2026-07-01)

An adversarial multi-agent re-verification (after codex closed every follow-up) confirms:
**all follow-ups are genuinely implemented and functional** (`--check` 44/44; gate/interop/
host `*_report` all `ready:True`); the separation holds (pnix mirror still the singleton
`pnix_mirror.singleton_mirror_run`, 4-lane gate untouched, `pnix_runtime.py` untouched by the
closing commits). So the items below under "Duplication to reconcile" and "Missing / partial"
are now RESOLVED except:
- **By-design residual (not a bug):** `pnix_hy/gate.py:WITNESS_FIELD_SCHEMA` + `_witness_fields`
  are a byte-identical copy of `hy-meta/witness.py` — the intended pnix STANDALONE fallback
  (host emitter is preferred via `interop._host_interop`). It lacks a drift-guard; optional
  follow-up: assert equality against the host schema when present (mirror `gate.py` vocab guard).
- The "compile/artifact/import witnesses MISSING" notes below are now FALSE — implemented at
  `hy-meta/host_exec.py:compile_artifact_witness` (report `ready:True`).
- Inline Lxxxx numbers below are point-in-time and have drifted; symbol names are authoritative.

The original audit (point-in-time map that drove the work) follows unchanged.

## Summary

The §1-§24 surface is substantially complete and the host/pnix separation holds. All of
§1-§8 (stage/bootstrap/kernel, reader/AST/IR, code-artifact, mirror, eval/apply) are
CONFIRMED present and correctly owned, with the pnix mirror genuinely collapsed to a
singleton (`pnix_mirror.singleton_mirror_run`) and the 4-lane convergence gate kept separate
as intended. Real code duplication is small and mostly cosmetic or reconcilable-by-
delegation; there is no competing second runtime/mirror/introspection implementation. The
biggest *structural* risks are three: (1) a three-shape witness schema split across
`gate.py`, `witness.py`, and `interop.InteropRecord`; (2) the import-hook trio
(finder/context-manager/install) near-duplicated between `bootstrap.py` and `import_hook.py`,
with `bootstrap` hand-inlining sys.modules transactions instead of reusing the named
primitives; and (3) hand-maintained effect/purity vocab in two places (`gate.EFFECT_OF` vs
`interop.EFFECT_CLASSES`, plus `_IMPURE_BUILTINS`). The largest *capability* gaps are
method-level interop (`apply-host-method`, `opaque_call_method` — genuinely missing) and
several "named API" surfaces that exist de-facto but lack a single entrypoint (host reify-*,
host explain-*, host gate/sandbox-run, host roundtrip-*/cache/drift APIs). Many todo markers
are stale (done work still flagged ⬜, or ◑ that should be ✅). No false-missing or
false-duplication claims were found in the verified audits.

## Duplication to reconcile

| Capability | The two (or more) implementations — file:symbol | Reconcile / canonical |
|---|---|---|
| Witness emitter (§14) | `pnix-hy/pnix_hy/gate.py:make_witness` (L59) vs `hy-meta/witness.py:make_witness` (L18) — divergent canonicalization (`sort_keys`+`default=str` vs `separators`+`default=repr`) and schema id (`pnix-hy.witness.v0` vs `hy-meta.witness.v0`); same payload → different `witness_id` | Pick `hy-meta/witness.py:make_witness` as canonical host emitter; have `gate.make_witness` reach it via `interop._host_interop` (pnix-hy/interop.py:L70), keep pnix schema only for pnix-native records. Unify field schema (next row). |
| Shared witness FIELD schema (§14) | `gate.py:make_witness` (L65-66) / `witness.py:make_witness` (L22-28) / `interop.py:InteropRecord` (L46-61, has `loss_status`/`effect_class`/`capability_required`) | Three shapes, no cross-verifiable `in_hash/out_hash/env_hash/status/loss` as named fields. Define ONE shared field vocabulary; do NOT add a 4th. `InteropRecord` already has the richest vocab — promote its field names. |
| Import-hook finder + context-mgr + install (§10.1/§10.2) | `bootstrap.py:KernelHyFinder`(L183)/`KernelHyImportHook.__enter__/__exit__`(L241-247)/`install_kernel_import_hook`(L250) vs `import_hook.py:PnixModuleFinder`(L55)/`PnixImportHook`(L103-115)/`install_pnix_import_hook`(L118) | Near-identical (differ only by suffix `.hy`/`.px`, loader class, root attr). Extract a shared base in `import_hook.py`; parametrize suffix+loader. NOTE: the LOADERS (`KernelHyLoader` L134 vs `PnixModuleLoader` L15) differ substantially — do NOT merge those. |
| sys.modules transaction (§10.3/§17) | `import_hook.py:snapshot_sys_modules`(L123)/`rollback_sys_modules`(L136) vs hand-inlined pop/restore in `bootstrap.py:run_kernel_import_check`(L7122-7123) and `run_compatibility_boundary_check`(L918-919, restore L1009-1015) | `bootstrap.py` never calls the named helpers (grep = 0). Replace the manual loops with `import_hook.snapshot_sys_modules`/`rollback_sys_modules`. Canonical = the named primitives. |
| Effect-class vocab (§15/§18) | `gate.py:EFFECT_OF` (L18-27; comment claims "shared with interop.EFFECT_CLASSES" but does NOT import it) vs `interop.py:EFFECT_CLASSES` (L31-34) | Make `gate.EFFECT_OF` import/derive from `interop.EFFECT_CLASSES` (single source). |
| Impure-builtin vs effect list (§15) | `pnix_mirror.py:_IMPURE_BUILTINS` (L2649-2653, 19 names incl. `exec`/`getFlake`) vs `gate.py:EFFECT_OF` (L18-27; `exec`/`getFlake` ABSENT → fall to `unknown`⇒denied) | Two hand-maintained lists. Derive one from the other so `exec`/`getFlake` get explicit effect classes instead of `unknown`. |
| Clean-probe subprocess summary (§11) | `clean_replay.py:run_clean_probe` (L21, subprocess+sha256+elapsed_ms L42-70) re-implements `bootstrap.py:run_stage9_probe_subprocess` (L2256); only delegates when `command is None` (L35-39) | Low priority. Extract a shared `_run_probe`; canonical = `bootstrap.run_stage9_probe_subprocess`. |
| `model_to_json` (§3.1) — COSMETIC | `pnix-hy/hy_mirror.py:model_to_json` redefined 9× inside isolated worker-script string constants (L546, 648, 800, 906, 1054, 1223, 1404, 1572, 1741) | Cosmetic only — each lives in a separate subprocess string. Optional: hoist to a shared snippet constant. NOT a competing implementation. |
| Code-payload helpers (§16) — ACCEPTABLE | `bootstrap.py:stable_code_payload`(L543)/`pyc_bytes_for_code`(L502) overlap `host_introspect.py:marshal_code`(L247) | Both in hy-meta, distinct consumers (deterministic hash vs roundtrip introspection). Keep both. |
| Opaque-ref dual key (§19) — INTENTIONAL | `hy-meta/interop.py` `__hy_meta_opaque__` vs `pnix-hy/interop.py` `__pnix_opaque__` (L116-117), two registries | Intended SR5 host-prefer / local-fallback delegation. Do NOT collapse. |

**Resolved (no action — listed so they are not re-flagged):** host introspection block
formerly duplicated in `hy_mirror.py` is gone (now path-imported + re-exported from
`hy-meta/host_introspect.py`, `_load_host_introspect` L1998-2011, 0 live defs);
`pnix_runtime.py` has no `import ast`/marshal/pyc machinery; `host_exec.py` artifact helpers
are verified thin facades over `bootstrap.py`.

## Stale allocation markers

| Marker location | Current | Should be | Evidence (real file:symbol) |
|---|---|---|---|
| hy-meta/todo.md (§10.2-host SR4) | ⬜ | ✅ | `import_hook.py:install_pnix_import_hook` L118 (SR4 DONE) |
| hy-meta/todo.md (§18-host SR5) | ⬜ | ✅ | `hy-meta/interop.py:make_opaque_ref`L24/`resolve_opaque`L48/`interop_report`L122 (SR5 DONE) |
| hy-meta/todo.md (§14-host) | ⬜ | ◑ | `witness.py:replay_witness`L47/`witness_report`L55 done; compile/artifact/import witnesses still absent |
| hy-meta/todo.md (§19) | ⬜ | ◑ | core boundary done (`interop.py:make_opaque_ref`L24/`inspect_object`L54/`release`L110); only method-level missing |
| pnix-hy/todo.md (§7) | ◑ | ✅ | `pnix_mirror.py:reify_pnix` L2876 + wired (`cli.py:cmd_reify`, `__init__`) |
| pnix-hy/todo.md (§13 roundtrip vocab) | ⬜ | ✅ | `pnix_mirror.py:ROUNDTRIP_STATUS_VOCAB` L24, `roundtrip_status` L2949 |
| pnix-hy/todo.md (§15 per-cap granting) | ⬜ | ✅ | `pnix_mirror.py:safe_eval` threads `granted=` via `gate.gate_check` L2743-2758 |
| pnix-hy/todo.md (§21 explain) | ◑+⬜ | ✅ (pnix side) | `pnix_mirror.py:explain_pnix` L2999 + `cli.py:cmd_explain` |
| hy-meta/todo.md (§3.1 label) | credited to Python-AST symbols | relabel | "Hy form-as-data" lives in `pnix-hy/hy_mirror.py:model_to_json` L546; hy-meta has no `hy.models`→data converter |
| docs/SEPARATION.md (host_introspect narration) | "present/to-move" | done | relocation DONE (`hy_mirror.py:_load_host_introspect` L1998) |
| docs/SEPARATION.md (seam line nums 2407/2415) | stale | update | real seam `mirror_full_introspection` L2039 / `introspection_parity` L2047 |
| docs/SEPARATION.md (§1.4 line nums) | stale | update | real: `diagnose` L3157, `eval_receipt` L3221, `meta_circular_tower` L2067, `pnix_evaluation_trace` L2541 |
| pnix-hy/ir.py docstring header | "(SEP §3.2)" | "§3.4" | pnix AST/IR is §3.4 per allocation |
| pnix-hy/todo.md (§18 apply-host-method) | claimed/implied | unbacked | no `call_method`/`apply_host_method` exists (grep empty both repos) — genuinely missing |

## Missing / partial capabilities — implementation plan

### Owner: interop

- **apply-host-method (§18) — MISSING. Thin (closeable now).** Target:
  `hy-meta/interop.py` + `pnix-hy/pnix_hy/interop.py`. Add host
  `call_method(ref, method_name, args, kwargs) -> InteropRecord` and pnix
  `call_host_method(ref, name, *args)`. REUSE: `hy-meta/interop.py:call_opaque` (L72, for
  exception capture + `record_witness`), `resolve_opaque` (L48); pnix `interop.py:
  check_capability` (L233), `call_host` (L247) layering, `wrap_pnix_callable` pattern. Do
  NOT rebuild invocation/witness machinery.
- **opaque local-fallback witness gap (§19) — PARTIAL. Thin.** Target:
  `pnix-hy/pnix_hy/interop.py:make_opaque_ref` else-branch (L109-111, currently returns no
  `witness_id`). REUSE: `pnix-hy/gate.py:make_witness` (L59).
- **opaque_ref_id named accessor (§19) — PARTIAL. Thin.** Trivial wrapper returning
  `ref['__hy_meta_opaque__']` / `ref['__pnix_opaque__']`. No new state.
- **pnix-side inspect_opaque accessor (§19) — PARTIAL/optional. Thin.** REUSE:
  `hy-meta/interop.py:inspect_object` (L54) via `_host_interop` (L70). Thin pass-through.
- **opaque_allowed_methods + opaque_call_method (§19) — MISSING. Deeper.** Target: shared
  `call_method` in `hy-meta/interop.py` serving BOTH §18 and §19 (single impl). REUSE:
  `call_opaque` (L72); add per-ref allowed-method registry alongside `_OPAQUE` (pnix interop
  L65).
- **Witness schema unification (§14) — DEEPER (cross-package).** Promote
  `interop.InteropRecord` field names (L46-61); have `gate.make_witness` (L59) delegate to
  `witness.make_witness` (L18) via `_host_interop`.

### Owner: hy-meta

- **Host reify-* uniform surface (§7) — PARTIAL. Thin.** Target: `hy-meta/host_exec.py` —
  add `reify_host(...)`. REUSE: `bootstrap.py:artifact_from_ast` (L566)/`artifact_summary`
  (L607), `host_introspect.py:full_introspection` (L453), `witness.py:make_witness` (L18).
  Mirror the pnix `reify_pnix` (pnix_mirror.py L2876) shape. Pure composition.
- **Host compile/artifact/import witnesses (§14) — MISSING. Thin-ish.** Emit in
  `hy-meta/host_exec.py` (compile/artifact) and at `bootstrap.py:run_kernel_import_check`
  (L7088). REUSE: `witness.py:make_witness` (L18)/`record_witness` (L31),
  `host_exec.py:compile_python_ast` (L80)/`artifact_from_ast` (L93).
- **Host unified named gate/sandbox-run surface (§15) — MISSING. Deeper.** New host
  `sandbox_run(...)` + `gate_check(...)` (mirror pnix `gate.gate_check` L30 shape). REUSE:
  `clean_replay.py:run_clean_probe` (L21), `bootstrap.py:install_kernel_import_hook`
  (L250)/`stage10_sandbox_probe` (L2627)/`stage11_capability_matrix` (L2921),
  `witness.py:record_witness` (L31).
- **Host generalized classify-drift API (§12) — PARTIAL. Medium.** Extend
  `bootstrap.py:classify_stage8_drift` (L1967) or add `classify_drift` emitting distinct
  `bytecode`/`marshal`/`pyc`/`value`/`env` kinds (today only `semantic`/`raw-marshal-or-pyc`/
  `none`). REUSE: `stage8_hash_keys` (L1955), `compare_stage8_artifact_bundles` (L1982).
- **Host named roundtrip-* API + status vocab (§13) — PARTIAL. Medium.** Add
  `roundtrip_python_ast`/`roundtrip_code_result`/`roundtrip_stage`. REUSE:
  `bootstrap.py:run_stage8_check` (L2018), `host_introspect.py:marshal_code` `roundtrip_ok`
  (L255); REUSE `pnix_mirror.py:ROUNDTRIP_STATUS_VOCAB` (L24) rather than a new one.
- **Host named artifact-cache API (§20) — PARTIAL. Medium.** Add
  `cache_key`/`cache_get`/`cache_put`. REUSE: `bootstrap.py:build_stage8_artifact_bundle`
  (L1909), `stage8_hash_keys` (L1955), `artifact_from_ast` (L566), `sha256_text` (L75).
- **Host snapshot/diff/rollback for builtins + sys.path + sys.meta_path (§17) — PARTIAL.
  Medium.** Target: `hy-meta/import_hook.py` — add `snapshot_host_state`/`diff_host_state`/
  `rollback_host_state`. REUSE: `snapshot_sys_modules` (L123)/`rollback_sys_modules` (L136);
  fold in the bespoke meta_path/sys_path capture inlined in
  `bootstrap.py:run_compatibility_boundary_check` (L905-906).

### Owner: pnix-hy

- **Wire .px import hook to host SR4 service (§10.2) — PARTIAL. Thin-ish.** Target:
  `pnix-hy/pnix_hy/interop.py` — add adapter installing the host meta-path finder for `.px`.
  REUSE: `hy-meta/import_hook.py:install_pnix_import_hook` (L118) via `_host_interop` (L70);
  `pnix_runtime.py:import_value` (L4341)/`read_px_file` (L4324). `PnixModuleLoader.exec_module`
  already stores `__pnix_result__` (import_hook.py L40-41), so value-import maps with no
  loader change.
- **§9 quote / quasiquote / hygiene — DEFERRED (by design).** The pnix language has no
  quote/macro construct; intentionally absent (only Hy-observation projections exist:
  `hy_mirror.py:hy_quasiquote_projection` L1153, `hy_macroexpand_projection` L846). Not a gap
  unless the pnix language grows macros. Hygiene status stays `partial`.

## Recommended order

1. **Thin wrappers — close now (low risk, pure reuse):** interop `call_method`/
   `call_host_method` (§18); opaque local-fallback `witness_id` via `gate.make_witness` (§19);
   `opaque_ref_id` accessor (§19); pnix-side `inspect_opaque` pass-through (§19); host
   `reify_host` facade (§7); flip all stale todo markers + fix SEPARATION.md line drift.
2. **Quick reconciles (delete duplication):** `bootstrap.py` reuse `snapshot_sys_modules`/
   `rollback_sys_modules` instead of manual pop/restore (§10.3/§17); `gate.EFFECT_OF` derive
   from `interop.EFFECT_CLASSES` and unify with `_IMPURE_BUILTINS` (§15); extract shared
   `_run_probe` for clean-probe (§11).
3. **Deeper (shared design):** import-hook finder/context-mgr/install shared base (§10.1/§10.2,
   keep loaders separate); single `call_method` powering `opaque_allowed_methods`/
   `opaque_call_method` (§19); witness schema unification across the three shapes (§14); host
   compile/artifact/import witnesses (§14).
4. **Deeper named-API build-outs:** host unified gate/sandbox-run (§15); host classify-drift
   kinds (§12); host roundtrip-* + status vocab reuse (§13); host artifact-cache API (§20);
   host snapshot/diff/rollback host-state (§17); wire .px import hook to SR4 (§10.2).
5. **Deferred (external/by-design):** pnix §9 quote/quasiquote/hygiene — only if the pnix
   language adds macros; full result-macro compiler parity and cross-repo Clojure schema
   unification (tracked in memory, out of this surface).
