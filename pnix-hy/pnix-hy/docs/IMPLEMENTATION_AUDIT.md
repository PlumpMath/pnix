# pnix-hy / hy-meta — Meta-Circular Implementation Audit

> 읽기 전용 multi-agent audit (23 agents, 11 capability dimensions, ~1.06M
> tokens)로 codebase vs §1–§24 "Pure Meta-Circular Capability Checklist"를
> 2026-07-01에 산출. 각 capability를 감사한 뒤 실제 코드에 대해
> ADVERSARIAL VERIFY; 모든 주장이 `file:symbol` 인용. 목표: 중복, 실제 상태,
> 누락 항목, exact implementation guidance (reuse, don't rebuild).

## Mistake-hunt verdict (2026-07-01)

광범위 adversarial mistake-hunt (15 agents, 7 dimensions, ~1.13M tokens)가
pnix-hy + hy-meta 전체를 실제 결함 대상으로 스윕: idempotency/hidden-state,
hollow/always-True reports, failure-hiding exception handling, scope/sacred
violations, doc↔code drift, flaky gates, interop edge cases. **Verdict: sacred-
surface 또는 SCOPE_LOCK 위반 없음; 이미 수정된 opaque case 외 hidden global
state 없음; 결과 정확.** 18 real defects 입증 (3 high, 10 medium, 5 low);
**17 이 패스 수정, 1 deferred**:
- **Robustness (high):** `--check`/`--gate`가 이제 각 report/lane을
  `cli._safe_report`로 격리 (느린/raising report → FAIL 행, whole-audit crash
  아님); `hy_meta_host_api_report`, `_one_shot_hy_script`, `run_bootstrap`가
  `TimeoutExpired`/`OSError`를 FAIL/`HyMirrorError`로 변환 (host-api budget
  env-configurable 300s) — 사용자가 맞은 60s-timeout crash 무력화.
- **Hollow-report vacuity (medium):** 4 sacred-mirror batch lanes +
  `compiler_emit_shape_report` + `fixture_report`가 `len==len(cases)`/`bool(cases)`
  요구 (`zip`-truncation / `all([])` vacuous pass 없음).
- **Interop correctness (medium/low):** `from_host`가 nested effect/capability
  집계; `to_host`가 nested-function container를 lossy/host-call로 표시;
  nullary host callables 처리 (`_required_positional_count` returns 0,
  `host_callable_to_pnix` special-case); `hy_macro_over_pnix`가 `.format` 대신
  `.replace` (Hy `{}` literals); `classify_drift`가 unparseable source 가드;
  `try_call_host`가 reserved-key collision 가드.
- **Docs:** current-state `--check` counts 54로 교정 (dated-historical
  snapshots 보존); role-matrix witness miscite + 28→29 row count 수정.
- **Deferred (1, medium):** `hy_mirror._proj_worker_run`/`_stage7_worker_eval`
  `readline()`에 deadline 없음 — *wedged* worker가 gate를 블록할 수 있음.
  Trigger는 합성적(our worker script always responds or exits); deadline 변경은
  hot path 위험이 있어 메모 후보, 미수정. 모든 수정 검증: `--check` 54/54,
  `--gate` PASS; `pnix_runtime.py` 변경은 single report-only guard
  (eval/4-lane semantics 미접촉).

## Stub-hunt verdict (2026-07-01)

세 번째 독립 multi-agent sweep (15 agents, 6 areas, ~0.79M tokens)이 ENTIRE
codebase에서 GENUINELY unimplemented code 수색 — `NotImplementedError`, hollow
`pass`/`...`/`return None|{}|[]` bodies, hard-coded `ready=True` reports,
non-delegating "facades", registered-but-empty builtins, `TODO/FIXME`.
**Verdict: 진정 미구현 없음.** 모든 후보가 intentional documented placeholder
또는 real-but-demoted logic으로 해소; 모든 ~44+ `*_report` readiness flags가
concrete probes에서 계산; 양쪽 todos에 open `[ ]` ZERO.
Confirmed-intentional placeholders (do NOT implement): derivation
`outPath`/`drvPath` store addressing (`pnix_runtime.py:2694`; no store hashing
by design), `builtins.placeholder` (the real Nix builtin), `trace`/`warn`
value-identity (pure-mirror side-effect omitted), §9 pnix
macro/quasiquote/reader-macro absence
(`_QUASIQUOTE/_DEFMACRO/_READER_MACRO/_IMPORT_PNIX_NOTE`),
`#_pnix-gap[...]` projection markers (always paired with a `gaps.append`),
fail-closed stage16 peer-review record, host standalone/optional fallbacks,
runtime "unsupported …" ERROR MESSAGES. Codebase는 선언된
meta-circular-projection scope 기준으로 완전.

## Re-verification update (2026-07-01)

Adversarial multi-agent re-verification (codex가 모든 follow-up 닫은 후) 확인:
**모든 follow-up이 진정 구현·기능** (`--check` 44/44; gate/interop/host
`*_report` all `ready:True`); separation 유지 (pnix mirror 여전히 singleton
`pnix_mirror.singleton_mirror_run`, 4-lane gate 미접촉, `pnix_runtime.py`
closing commits 미접촉). 아래 "Duplication to reconcile" 및 "Missing / partial"
항목은 이제 RESOLVED 단:
- **By-design residual (not a bug):** `pnix_hy/gate.py:WITNESS_FIELD_SCHEMA` +
  `_witness_fields`는 `hy-meta/witness.py`의 byte-identical 사본 — 의도된
  pnix STANDALONE fallback (host emitter preferred via `interop._host_interop`).
  Drift-guard 없음; optional follow-up: host schema 존재 시 equality assert
  (mirror `gate.py` vocab guard).
- 아래 "compile/artifact/import witnesses MISSING" 노트는 이제 FALSE —
  `hy-meta/host_exec.py:compile_artifact_witness` 구현 (report `ready:True`).
- 아래 인라인 Lxxxx 번호는 point-in-time이며 drift; 심볼 이름이 권위.

원본 audit (작업을 이끈 point-in-time 맵)은 아래 그대로.

## Summary

§1-§24 표면은 실질적으로 완전하고 host/pnix separation이 유지된다. §1-§8
(stage/bootstrap/kernel, reader/AST/IR, code-artifact, mirror, eval/apply)
전부 CONFIRMED present·올바르게 소유, pnix mirror는 진정 singleton
(`pnix_mirror.singleton_mirror_run`)으로 접혔고 4-lane convergence gate는
의도대로 분리 유지. Real code duplication은 작고 대부분 cosmetic 또는
delegation으로 조정 가능; 경쟁 second runtime/mirror/introspection 구현
없음. 가장 큰 *구조* 리스크 셋: (1) `gate.py`, `witness.py`,
`interop.InteropRecord`에 걸친 three-shape witness schema 분할; (2)
import-hook trio (finder/context-manager/install)가 `bootstrap.py`와
`import_hook.py` 사이 near-duplicated, `bootstrap`이 named primitives 재사용
대신 sys.modules transactions hand-inline; (3) effect/purity vocab
hand-maintained 두 곳 (`gate.EFFECT_OF` vs `interop.EFFECT_CLASSES`, plus
`_IMPURE_BUILTINS`). 가장 큰 *capability* 갭은 method-level interop
(`apply-host-method`, `opaque_call_method` — 진정 누락)과 de-facto 존재하나
단일 entrypoint 없는 여러 "named API" 표면 (host reify-*, host explain-*,
host gate/sandbox-run, host roundtrip-*/cache/drift APIs). 많은 todo markers
stale (done work still ⬜, or ◑ that should be ✅). Verified audits에서
false-missing 또는 false-duplication 주장 없음.

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
