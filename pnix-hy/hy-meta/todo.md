# hy-meta TODO / continuation note

> ⛔ **SCOPE LOCK** (see `/SCOPE_LOCK.md`): the host lane is closed within the current
> meta-circular-projection scope. **No new implementation may reinterpret intentional
> placeholders as missing work** (의도적 placeholder를 미구현으로 재해석해 구현하지 말 것) —
> e.g. fail-closed stage16, host fallbacks, "unsupported…" error messages are BY DESIGN. New
> capabilities start as a proposal doc, NOT a `[ ]` here. Say "complete w.r.t. the stated scope".
>
> ▶ **BOUNDARY NOTE — pnix action-VM phase (pnix-hy proposal 0009):** pnix-hy is adding a
> **pnix-side** action-checkpoint layer (`pnix_hy/action.py`). **hy-meta has NO new work for
> this phase** and must NOT absorb it:
> - [x] (guard) do NOT add action governance / LLM-step / file-backup / verdict logic to hy-meta;
>   that layer is pnix-hy's, built on pnix-hy's own gate/mirror/explain/safe_eval. Kept unchanged.
> - [x] host artifacts (Python AST/code-object/pyc/marshal), import hook, host introspection stay
>   hy-meta-owned (unchanged) — pnix-hy must not duplicate them (already the rule).
> - [x] if the action VM ever needs a host witness, it reuses the shared §14 witness schema
>   (`witness.py`) via interop — NO new host machinery. Host objects never enter pnix terms
>   (opaque ref via interop). See `pnix-hy/docs/proposals/0009-pnix-semantic-action-vm.md`.

## Current Remaining Work (verified 2026-08-11)

Verified by reading this entire file plus `STATUS.md`, and spot-checking the
cited symbols in `bootstrap.py`/`smoke_test.py`/`independent_mini_backend.py`.
The overwhelming majority of this document is CLOSED work (stage7 compiler
chain, stage8–stageN proof ladder, the parity-ledger 100%-direct bar, the
Deep-Research Audit frontier items, and — as of earlier this session —
`independent-mini-backend-check` and the `tests/` native-corpus fetch). What
follows is what is actually still open, by axis. Do not re-flag anything not
listed here; see the detailed sections below for the full history.

1. **Reader/front-end ownership — NOT a gap, a recorded non-goal.**
   Deliberately ceded to upstream `hy.reader`/mangling (`front-end-boundary-check`,
   `reader-boundary-check` in `bootstrap.py`). Reaffirmed by the 2026-06-29
   Deep-Research Audit after adversarial review. Public classification is
   honestly downgraded to "self-hosting back-end." Only reopen behind an
   explicit proposal, not as a `[ ]` item.

2. **Full upstream `hy.compiler`/`result_macros.py` parity — open, but bounded
   and mostly closed within its own bar.** The parity-ledger corpus (owned
   compiler corpus + all `tests/native_tests/*.hy`) is 100% direct, zero
   fallback (verified: `parity-ledger-check` -> 45/45 files, 1487/1487 forms
   per `STATUS.md`). The literal remaining gap is only what that finite corpus
   does not exercise — not a measured/sized backlog, more a standing
   "grow the ledger corpus if a real gap surfaces" posture. Sizing: small,
   opportunistic, no known concrete failing case today.

3. **Python 3.12/3.13 support — explicitly out of scope by decision, not a
   gap to close.** Proof targets stay pinned to 3.11 + Homebrew 3.14. Revisit
   only on explicit user request (`todo.md` "Active Goals: 3.11 + Latest 3.14").

4. **REPL/`hyc`/`hy2py`/zipimport product surface — implemented as scoped
   product commands, not full upstream-parity tooling.** `hyc`, `hy2py`,
   `-c`/stdin/shebang, and a scoped line REPL all exist and are smoke-tested
   (`smoke_test.py`: `hyc-check`, `repl-check`, `cli-io-check`,
   `startup-output-check`, plus the `hy2py`/`kernel-py`/`stage7-kernel-py`
   command rows). zipimport is deliberately marked unsupported, not silently
   missing (`bootstrap.py` `zipimport_status: "unsupported-filesystem-roots-only"`,
   sandbox denial fixtures). `STATUS.md`'s
   `full_REPL/hyc/hy2py/zipimport_product_surface = false` claim is honest
   shorthand for "scoped, not upstream-drop-in" — there is no missing
   implementation here, only a naming/framing gap if a reader expects "full"
   to mean upstream-identical.

5. **Third-independent-path DDC leg — (b) done this pass, (a) still open.**
   `diverse-double-compile-check` is closed (kernel.hy built two ways — via
   upstream `hy.compiler` seed vs the direct kernel — byte identical at 4
   levels). Its known limit: one of the two paths still routes through
   trusted upstream `hy.compiler`, so it can't catch a backdoor already in
   upstream itself. `independent_mini_backend.py` +
   `independent-mini-backend-check` (added earlier this session) is a
   from-scratch Hy-subset-to-`ast` compiler sharing zero code with
   `hy.reader`/`hy.compiler`/`stage1`/`stage2/kernel.hy`.

   **(b) done this pass (2026-08-11):** `run_independent_mini_backend_check`
   in `bootstrap.py` now loads `kernel_direct` via
   `stage2.load_hy_file(KERNEL_PATH, ...)` (the same direct-kernel bridge
   `diverse-double-compile-check` uses) and evaluates every fixture through
   it as a third leg alongside `host_result` (upstream) and
   `mini_backend_result` (independent). All three must agree with `expected`
   for a fixture to pass. Verified: 8/8 fixtures accepted with all three legs
   agreeing (`kernel_direct_result` rows added), `diverse-double-compile-check`
   still `reproduced`, full `hy-meta-gate full` ladder still green — no
   regression. Do not re-flag "upstream-vs-mini 2-way only" as open; it is
   now a genuine 3-way per-fixture check.

   **(a) partial progress, same day (2026-08-12):** added string literals,
   list literals as return values, and `setv`/`while` mutation (8→12
   fixtures). This required a real fix, not just new fixtures:
   `_emit_defn` previously only ever emitted the function's *last* body
   form and silently discarded everything before it, so `setv`/`while`
   (which only make sense as side-effecting statements, never as a final
   expression) had no way to run at all. Added `_emit_stmt` (turns `setv`
   into `ast.Assign`, `while` into `ast.While`); `_emit_defn` now emits
   every body form but the last through it. Verified against both real
   legs (`bootstrap.py run -c` / `kernel-run -c`) before adding fixtures.
   `independent-mini-backend-check` -> 12/12, `diverse-double-compile-check`
   still `reproduced`, full `hy-meta-gate full` ladder still green.

   **Further progress, same day (2026-08-13):** added dict literals
   (`{"a" 1 "b" 2}`, string keys only — see STATUS.md for why keyword keys
   are out of scope) and a multi-`defn`-composition fixture (12→14 total).
   The multi-`defn` case required no backend change — `compile_and_eval`
   already emits every top-level `defn` as a real module-level
   `FunctionDef`, so a later one calling an earlier one by name already
   worked; this was verified rather than assumed before being added as a
   fixture. Dict literals needed real reader/emitter additions:
   `{`/`}` tokenization, a `("__dict__", pairs)` marker form in `_parse_one`,
   and a `_is_dict` emit case (`ast.Dict`) checked ahead of the general
   call-form dispatch. Verified against both real legs before adding.
   `independent-mini-backend-check` -> 14/14, `diverse-double-compile-check`
   still `reproduced`, full `hy-meta-gate full` ladder still green.

   **Still open:** `require`/macros (macro expansion is a materially bigger
   feature than the fixture-additive pattern used so far — would need real
   compile-time macro-expansion machinery, not just new reader/emitter
   cases; not attempted this pass), keyword-keyed dict literals (would need
   to construct real `hy.models.Keyword` objects to match upstream
   equality/`repr`, a bigger lift than string keys), more seq ops (`get`,
   `len`, list mutation), further toward clj-meta's ~50-fixture
   `frontend_selfhost.clj` scope. Sizing: small increments, additive — no
   architecture change needed, the 3-way plumbing already exists and each new
   fixture automatically gets checked on all three legs.

   **진짜 클로저 추가, 같은 축의 마지막 단계 (2026-08-17):** clj-meta/
   clr-meta/rs-meta의 let→loop/recur→closure 진행의 대응물. `(fn [params]
   EXPR)`를 `ast.Lambda`로 컴파일 — Python 자체 클로저 의미론(참조 캡처,
   late-binding)을 공짜로 얻어서 다른 host들과 달리 별도 클로저 값 표현이나
   env 자료구조가 전혀 불필요. 클로저 이름 호출도 기존 `_is_sym(head)` →
   `ast.Call` fallback이 그대로 처리(Python은 이름이 `def` 함수인지 lambda인지
   구분 안 함). 클로저를 다른 `defn`의 인자로 넘기는 고차 함수도 코드 변경
   없이 그냥 동작(rs-meta는 파서만 확장하면 됐는데, hy-meta는 타입 어노테이션
   자체가 없어서 파서조차 안 건드림). **의도적으로 좁힌 경계**: `fn`은 단일
   식 본문만(실제 Hy `fn`은 여러 body form도 허용하지만 `ast.Lambda`가
   statement를 못 담아서 미지원, 어떤 fixture도 필요 없음). 여러 번 호출,
   non-tail, 2-파라미터, 클로저-캡처-클로저, 고차 함수까지 5개 fixture
   추가(14→19), 전부 실제 upstream Hy + kernel_direct(`stage2/kernel.hy`가
   자체 `compile-fn-expr`로 이미 `fn` 지원) + mini backend 3-way 일치 확인.
   회귀 없음: `self-check`/`stage7-check` 둘 다 PASS. 상세는 `STATUS.md`
   참조. 이걸로 hy-meta도 다른 host들이 이번 세션에 거친 축을 따라잡음 —
   남은 후보(macro, keyword dict, 더 많은 seq ops)는 위 "Still open"과 동일.

6. **Stage8–StageN proof ladder — closed except 3 cross-repo-blocked Stage14
   items.** Verified: every stage8–stageN checklist item in this file is
   `[x]` except three `[~]` items under Stage14, all gated on a live Clojure
   host exposing a stage14-compatible export:
   - "Run the same fixtures on Clojure and Hy/Python hosts."
   - "Compare answer-plan hashes against Clojure host exports."
   - "Add actual `pnix-clj`/`clj-meta` invocation wiring."
   Confirmed still genuinely open (not stale): `bootstrap.py`'s
   `stage14_host_capability_matrix()` still lists `pnix-clj` and `clj-meta` as
   `status: "held"` / `reason: "*-not-wired"`, and no `pnix-clj`/`clj-meta`
   invocation code exists in `bootstrap.py` beyond that static matrix entry.
   This is cross-repo integration work, not something closeable from hy-meta
   alone. Sizing: small-to-medium once a Clojure-side export command exists;
   currently blocked, not actionable today.

Everything else in "Gap Analysis," the "Deep-Research Audit," and Stage8
through StageN below is closed and verified against code in this pass — see
those sections for citations and detail. `pnix_language_semantics_ownership`
(the sixth `STATUS.md` open claim) is out of scope for hy-meta entirely — it
belongs to `pnix-hy`'s runtime lane, not this host-meta layer.

Date: 2026-06-27

Latest pushed baseline before shutdown:

```text
251d3e163 feat(hy-meta): close stage7 compiler parity todo
aa25f80a0 docs(hy-meta): refresh continuation note
53812ff92 feat(hy-meta): allow kernel prelude opt out
f78cd46cd feat(hy-meta): validate hy pragma versions
2fdafbb9d feat(hy-meta): support inline python macros
c55c0d641 fix(hy-meta): reject invalid compiler ast forms
8776961f2 fix(hy-meta): compact boolop lowering
```

Active Python proof targets are intentionally narrow:

```sh
/tmp/pnix-hy-py311-venv/bin/python
/tmp/pnix-hy-py314-venv/bin/python
```

Use Python 3.11 as the stable baseline and Homebrew `python@3.14` as the latest
target. Do not open Python 3.12 or 3.13 support in this lane. Homebrew latest
checked for this run: `python@3.14` 3.14.6.

## ▶▶ SEPARATION: receive host machinery from pnix-hy (2026-07-01)

Plan of record: `pnix-hy/docs/SEPARATION.md` (line-referenced inventory of both repos).
Layering: **hy-meta = the Hy/Python self-compile/evaluate/reproduce HOST proof lane;
pnix-hy = the pnix runtime hosted on top; interop = an explicit bidirectional boundary.**
hy-meta already owns the stage chain, Hy kernel, import hook, `artifact_from_ast`/pyc/
marshal/code-object hashing, mirror/drift, stage8/9 clean replay, host introspection — so
the items below are about RECEIVING the host machinery that is currently (wrongly)
duplicated inside pnix-hy, and exposing clean APIs pnix-hy can consume.

- [x] **SR1 — host introspection module** DONE 2026-07-01. Receive the host
  introspection block currently in `pnix-hy/pnix_hy/hy_mirror.py` L1941–2382 (26 fns:
  compile_source / code_object_info / disassemble / execution_trace / marshal_code /
  rebuild_code / ast_info / symtable_info / tokenize_info / frame_info / gc_info / sys_info /
  module_info / full_introspection …) as `hy-meta/host_introspect.py`. It overlaps hy-meta's
  existing `artifact_from_ast`/`stable_code_payload`/`ast_data`/`pyc_bytes_for_code`
  (bootstrap.py 496–607) — eventually fold the two so there is ONE host-artifact lane.
  pnix-hy path-imports it (no package rename needed; pnix-hy already knows HY_ROOT/hy-meta).
- [x] **SR2 — host-artifact API** DONE 2026-07-01. `hy-meta/host_exec.py`
  (`run_python_source` / `run_code_object`) is the host-exec floor pnix-hy's compiler lane
  delegates to (pnix-hy SEP2: `run_px_source_raw` routes through it). It now also exposes
  lazy wrappers for existing `bootstrap.py` host-artifact primitives:
  `compile_python_ast`, `artifact_from_ast`, `artifact_summary`, `compare_artifacts`,
  `pyc_bytes_for_code`, and `stable_code_payload`. SR1's `host_introspect.py` is folded
  into the module set (`artifact.py`, `python_ast.py`, `code_object.py`, `bytecode.py`,
  `pyc.py`, `inspect_host.py`) as thin facades so there is ONE host-artifact lane.
- [x] **SR3 — clean-replay/subprocess API** DONE 2026-07-01. `hy-meta/clean_replay.py`
  exposes `clean_env`, `run_clean_probe`, and `compare_clean_probes`, delegating the default
  product probe to existing `stage9_clean_env` / `run_stage9_probe_subprocess`. pnix-hy's
  optional external Rust oracle now asks this API for clean replay first and keeps its
  standalone fallback only when hy-meta is unavailable or running under an unsupported host.
- [x] **SR4 — pnix import-hook service** DONE 2026-07-01. `hy-meta/import_hook.py`
  generalizes the existing kernel import-hook shape into `install_pnix_import_hook(pnix_loader,
  roots)` plus `snapshot_sys_modules` / `rollback_sys_modules`, so pnix-owned `.px` loading can
  plug into host `sys.meta_path` through hy-meta.
- [x] **SR5 — host-side interop adapter** DONE 2026-07-01. `hy-meta/interop.py` owns opaque
  Python/Hy object refs (`make_opaque_ref` / `inspect_object` / `call_opaque` / `release`),
  host callable invocation, and exception capture. `pnix_hy.interop` now prefers this host
  adapter and falls back to its old local registry only for standalone operation.
- [x] **SR6 — host witnesses** DONE 2026-07-01. `hy-meta/witness.py` emits deterministic
  per-conversion/replay/drift/call witness records, and host interop/clean replay return
  resolvable `witness_id` fields. `hy-meta/hy_meta.py:host_api_report` aggregates SR2–SR6.

NOTE: hy-meta MAY keep multiple mirror CHECKS (`run_mirror_check`, `run_stage7_check`,
`run_diverse_double_compile_check`, `run_stage8_check`, `run_stage9_check`) — those are host
artifact-comparison categories, NOT competing runtime mirrors. The singleton-mirror rule is
for the pnix RUNTIME side only (pnix-hy).

## ▶▶ CAPABILITY ALLOCATION — hy-meta (HOST Hy/Python meta-circular) (2026-07-01)

Allocation of the "Pure Meta-Circular Capability Checklist" (§1–§24) to the HOST lane.
Rule: **hy-meta owns Hy/Python self-compile/evaluate/reproduce/inspect**; pnix runtime
capabilities go to `pnix-hy/todo.md`; the boundary is interop. Status: ✅ exists / ◑
partial-deferred. DO NOT re-implement ✅/◑ — extend/expose. Cite the existing symbol.

- [✅] **§1 Stage/Bootstrap** — `bootstrap_stage2/_chain/_stage3_chain/_stage_chain`,
  `bootstrap_kernel/_stage7_kernel`, `load_kernel_compiled_kernel`, product entrypoints
  `cmd_run/py/hy2py/hyc/kernel_run/kernel_py/stage7_kernel_*`. (1.1–1.4 present.)
- [✅] **§2.1 source norm / §2.2 Hy reader boundary** — `strip_shebang`, `read_input`,
  `run_reader_boundary_check`, `run_front_end_boundary_check` (reader/mangling ceded to hy.reader).
- [✅] **§3.1 host artifact data / §3.2 Python AST / §3.3 Python source** — `ast_data`,
  `location_stable_ast`, `artifact_from_ast`; `host_introspect.ast_info/tokenize_info` (SR1).
  Hy form-as-data projection lives in `pnix-hy/hy_mirror.py:model_to_json`.
- [✅] **§4 code object / bytecode / pyc** — `stable_code_payload`, `pyc_bytes_for_code`;
  `host_introspect.{disassemble,code_object_info,marshal_code,opcode_tables,...}` (SR1);
  `host_exec.compile_python_ast` (SR2).
- [✅] **§5 artifact bundle** — `artifact_from_ast`, `artifact_summary`,
  `build_stage8_artifact_bundle`, `compare_stage8_artifact_bundles`; `host_exec` facades
  `artifact_from_source/compare_artifacts/host_exec_report` (SR2).
- [✅] **§6.1 compiler mirror / §6.2 kernel mirror** — `run_mirror_check`, `run_stage7_check`,
  `run_diverse_double_compile_check`, `run_self_host_check`. (§6.3 pnix runtime mirror = pnix-hy.)
- [◑] **§7 reification (HOST: form/ast/module/code-object/import/exception)** — via artifact
  records + `host_introspect`; future surface: uniform `reify-*`.
- [✅] **§8.1 Hy eval / §8.3 apply-hy/python** — `eval_repl_stream`, `run_kernel_check`,
  kernel eval; host callable invocation belongs to the §18 host adapter.
- [◑] **§9 quote/quasiquote/macro (HOST machinery)** — Hy macro/reader-macro expansion is
  upstream `hy` + the kernel; hy-meta PROVES it (`run_macro_require_parity_check`,
  `run_reader_boundary_check`). The OBSERVATION/projection lives in pnix-hy. Future optional host
  `macro-expansion-trace/-witness`.
- [✅] **§10.1 Hy import hook / §10.3 sys.modules transaction** — `KernelHyLoader`,
  `KernelHyFinder`, `KernelHyImportHook`, `install_kernel_import_hook`,
  `run_kernel_import_check` (sys.modules/meta_path rollback on failure).
- [✅] **§11 environment / clean replay** — `stage9_clean_env`, `stage9_manifest`,
  `run_stage9_probe_subprocess`, `run_stage9_check`.
- [◑] **§12 determinism / drift (HOST artifacts)** — `classify_stage8_drift`,
  `compare_stage8_artifact_bundles`, stage8 reproduction; future surface: `classify-drift` API
  (bytecode/pyc/value/env kinds).
- [◑] **§13 roundtrip (Hy→Python-AST→source; Hy→code→result; stage→fresh-stage)** — proven in
  stage8/mirror; future surface: explicit `roundtrip-*` + status vocabulary.
- [✅] **§14 witness/proof (HOST)** = **SR6** — `witness.py`
  (`make_witness/record_witness/resolve_witness/conversion_witness/replay_witness/
  drift_witness/witness_report`) emits the shared witness field schema.
- [◑] **§15 gate/sandbox (HOST: import/subprocess/network/file)** — clean-subprocess +
  boundary checks exist; future surface: named host `sandbox-run`/gate.
- [✅] **§16 host introspection** — `host_introspect.py` (SR1).
- [◑] **§17 mutation/pollution detection** — reader/import boundary + sys.modules rollback
  exist; future surface: named `snapshot/diff/rollback` for builtins/sys.path/meta_path.
- [✅] **§18 host-side interop adapter** = **SR5** — `hy-meta/interop.py` (opaque Python object
  control, callable invocation, exception capture), exported through `hy_meta.py` and loaded by
  pnix-hy's `interop._host_interop()`; opaque-ref shape `__hy_meta_opaque__` is compatible.
- [✅] **§19 opaque object boundary (HOST)** — part of SR5:
  `make_opaque_ref/opaque_ref_id/opaque_allowed_methods/opaque_call_method/opaque_witness`.
- [◑] **§20 cache (HOST artifact cache)** — stage8 bundles ARE the artifact cache; future surface:
  `cache-key/get/put` keyed by (source/compiler/stage/env/py/hy-version).
- [✅] **§21 debug/explain (HOST)** — `write_stage8_debug_artifacts`,
  `write_stage9_debug_replay`; future surface: unify `explain-*`.
- [✅] **§10.2-host pnix import-hook service** = **SR4** — `import_hook.py`
  (`install_pnix_import_hook`, `snapshot_sys_modules`, `rollback_sys_modules`,
  `import_hook_report`), re-exported by `hy_meta.py`.

NOTE: §6.3, §2.3/§3.4/§8.2, §9-pnix, §13-pnix, §14-pnix, §15-pnix, §18/§19-pnix-side,
§20-pnix → pnix-hy/todo.md. Only the §14 witness FIELD SCHEMA and the §18/19 opaque-ref shape
are the SHARED envelope; everything else stays host-local.

## ▶▶ AUDIT FOLLOW-UPS — hy-meta (from pnix-hy/docs/IMPLEMENTATION_AUDIT.md, 2026-07-01)

23-agent verified audit of §1-§24. Actionable HOST-lane items, ordered. REUSE the cited
symbol; do NOT rebuild. (✅ findings below confirm no second runtime/mirror/introspection.)

### A. Stale markers to FLIP (work already done)
- [x] SR4 → ✅: `import_hook.py:install_pnix_import_hook` (L118) exists.
- [x] SR5 → ✅: `hy-meta/interop.py:make_opaque_ref`(L24)/`resolve_opaque`(L48)/
  `interop_report`(L122) exist.
- [x] §14-host → ◑ (not ⬜): `witness.py:replay_witness`(L47)/`witness_report`(L55) done;
  compile/artifact/import witnesses still absent (see C).
- [x] §19-host → ✅: core boundary + method-level interop now exist
  (`make_opaque_ref/inspect_object/release/opaque_ref_id/opaque_allowed_methods/call_method`).
- [x] §3.1 label fix: "Hy form-as-data" actually lives in `pnix-hy/hy_mirror.py:model_to_json`
  (L546), NOT hy-meta — hy-meta has no `hy.models`→data converter. Relabel the allocation row.

### B. Duplication to RECONCILE (delete the second copy / delegate)
- [x] **sys.modules txn (§10.3/§17):** `bootstrap.py:run_kernel_import_check`(L7122-7123) and
  `run_compatibility_boundary_check`(L918-919, restore L1009-1015) hand-inline pop/restore and
  now call `import_hook.snapshot_sys_modules` / `rollback_sys_modules`. Canonical = the named
  primitives.
- [x] **import-hook trio (§10.1/§10.2):** extract a shared base in `import_hook.py` for
  finder + context-manager + install (`bootstrap.KernelHyFinder`L183/`KernelHyImportHook`
  L241-247/`install_kernel_import_hook`L250 ≈ `import_hook.PnixModuleFinder`L55/`PnixImportHook`
  L103-115/`install_pnix_import_hook`L118); parametrize suffix(.hy/.px)+loader+root-attr. KEEP
  the LOADERS separate (`KernelHyLoader`L134 vs `PnixModuleLoader`L15 differ substantially).
  Implemented as `import_hook.SuffixModuleFinder` + `ImportHookContext`; `.hy` and `.px`
  finders share search/install mechanics while keeping `KernelHyLoader`/`PnixModuleLoader` separate.
- [x] **witness emitter (§14):** make `hy-meta/witness.py:make_witness`(L18) the canonical host
  emitter; `pnix-hy/gate.py:make_witness`(L59) should reach it via `interop._host_interop`
  (keep pnix schema only for pnix-native records). Unify the FIELD schema — promote
  `interop.InteropRecord` field names (L46-61); do NOT add a 4th shape.
  Host `witness.py` now exposes `WITNESS_FIELD_SCHEMA` / `witness_fields`; records promote
  interop-style fields plus in/out/env hashes. `gate.make_witness` still delegates to host and
  standalone fallback emits the same field names.
- [x] **clean-probe (§11):** `clean_replay.py:run_clean_probe`(L21) re-implements
  `bootstrap.py:run_stage9_probe_subprocess`(L2256). Extract a shared `_run_probe`; canonical =
  `bootstrap.run_stage9_clean_subprocess`; both clean replay and stage9 product probe reuse it.

### C. Missing / partial HOST capabilities (target file + reuse)
- [x] **reify_host facade (§7)** in `host_exec.py` — REUSE `bootstrap.artifact_from_ast`(L566)/
  `artifact_summary`(L607), `host_introspect.full_introspection`(L453), `witness.make_witness`
  (L18); implemented as `host_exec.reify_host` / `reify_host_report`.
- [x] **host compile/artifact/import witnesses (§14)** — emit in `host_exec.py` (compile/
  artifact) + at `bootstrap.run_kernel_import_check`(L7088); REUSE `witness.make_witness`(L18)/
  `record_witness`(L31), `host_exec.compile_python_ast`(L80)/`artifact_from_ast`(L93).
  Implemented as `host_exec.compile_artifact_witness` plus `run_kernel_import_check.witness_id`.
- [x] **host sandbox_run + gate_check (§15)** — mirror pnix `gate.gate_check` shape; REUSE
  `clean_replay.run_clean_probe`(L21), `bootstrap.install_kernel_import_hook`(L250)/
  `stage10_sandbox_probe`(L2627)/`stage11_capability_matrix`(L2921), `witness.record_witness`(L31).
  Implemented as `clean_replay.gate_check` / `sandbox_run` / `sandbox_report`, exported by `hy_meta.py`.
- [x] **classify_drift kinds (§12)** — extend `bootstrap.classify_stage8_drift`(L1967) to emit
  distinct bytecode/marshal/pyc/value/env kinds (today only semantic|raw-marshal-or-pyc|none);
  REUSE `stage8_hash_keys`(L1955)/`compare_stage8_artifact_bundles`(L1982).
  Implemented as `bootstrap.classify_drift` / `classify_drift_report`; legacy
  `classify_stage8_drift` remains coarse for existing CLI checks.
- [x] **host roundtrip_* API + status vocab (§13)** — `roundtrip_python_ast/_code_result/
  _stage`; REUSE `bootstrap.run_stage8_check`(L2018), `host_introspect.marshal_code` roundtrip_ok
  (L255), and REUSE pnix `ROUNDTRIP_STATUS_VOCAB`(pnix_mirror L24) — do not define a new vocab.
  Implemented in `host_exec.py` as `roundtrip_python_ast`, `roundtrip_code_result`,
  `roundtrip_stage`, `roundtrip_report`; vocabulary is read from `pnix_mirror.py` by AST.
- [x] **host artifact cache_key/get/put (§20)** — REUSE `bootstrap.build_stage8_artifact_bundle`
  (L1909)/`stage8_hash_keys`(L1955)/`artifact_from_ast`(L566)/`sha256_text`(L75); key on
  (source/compiler/stage/env/py/hy-version). Implemented in `host_exec.cache_key/cache_get/
  cache_put/cache_clear` with `artifact_cache_report`, exported by `hy_meta.py`.
- [x] **host snapshot/diff/rollback host-state (§17)** in `import_hook.py` —
  `snapshot_host_state/diff_host_state/rollback_host_state` for builtins+sys.path+sys.meta_path;
  implemented on top of `snapshot_sys_modules`/`rollback_sys_modules` and exported by `hy_meta.py`.
- [x] **host call_method (§18 apply-host-method + §19 opaque_call_method)** — ONE shared
  `call_method(ref, name, args, kwargs)` in `hy-meta/interop.py` serving both; REUSE `call_opaque`
  for invoke+exception+witness, `resolve_opaque`; added `opaque_ref_id`,
  `opaque_allowed_methods`, `opaque_call_method`, and facade exports.

ORDER: A (flip markers, free) → B (dedup reconciles) → C thin (reify_host, witnesses) →
C deeper (sandbox/classify-drift/roundtrip/cache/snapshot/call_method). Each: add its
`*_report` self-check; host-touching changes gate with the existing stageN checks.

## Active Goal: Stage15 Meta-Circular Hy Compiler

We are building toward a stage15 meta-circular Hy compiler, with later stageN
extensions kept explicit instead of implied. In Hy, stage8 artifact validation
maps to Python compiler/runtime artifacts rather than JVM jar/class bundles:

```text
stage7  = semantic/eval mirror closure across the owned Hy stage chain
stage8  = Python artifact reproducibility after fresh meta-circular reload
stage9  = clean product runtime replay from product entrypoints
stage10 = client/server/session/sandbox replay closure
stage11 = multi-domain adapter closure
stage12 = self-improvement quarantine closure
stage13 = long-horizon product organism closure
stage14 = cross-host/cross-implementation pnix law closure
stage15 = open-world external evidence federation closure
stageN  = versioned constitutional extension beyond stage15
```

Cross-stage invariants that every stage8+ proof must keep:

- [x] Use canonical records and hashes for comparison; rendered text is a
      product surface, not the only proof object.
- [x] Separate hard manifest bindings from soft environment observations.
- [x] Provide debug manifests for drift diagnosis instead of silently
      normalizing differences.
- [x] Add CI wiring for Python 3.11 and Python 3.14 proof targets.
- [x] Add a machine-readable stage manifest index so stageN extensions can
      discover prior proof artifacts without scraping command output.
- [x] Add per-stage timeout budgets and cost notes to the stageN extension
      manifest so full smoke stays usable.

Current implementation stance:

- [x] Keep the stage7 proof lane green as the semantic/eval baseline.
- [x] Define the Hy stage8 artifact surface as source-location-free AST data,
      generated Python source, marshaled Python code objects, and timestamp
      `.pyc` bytes.
- [x] Add `stage8-check` to compare a stage7 bundle against a freshly loaded
      stage8 bundle for `stage2/compiler.hy`, `stage2/kernel.hy`, and the
      kernel proof examples.
- [x] Add optional `--debug-dir work/stage8-debug` output with
      `stage7/`, `stage8-fresh/`, and `diff/` manifests for drift inspection.
- [x] Run `stage8-check` on Python 3.11 and Homebrew Python 3.14.
- [x] Keep full smoke green after stage8 is in the proof lane.
- [x] Add `stage9-check` to replay product-facing compiler entrypoints from
      clean subprocess probes.
- [x] Run `stage9-check` on Python 3.11 and Homebrew Python 3.14.
- [x] Keep full smoke green on Python 3.11 and Homebrew Python 3.14 after
      stage9 entered the proof lane.
- [x] Add `stage10-check` for local client/server/session/sandbox replay over
      the Hy/Python product shell.
- [x] Keep full smoke green on Python 3.11 and Homebrew Python 3.14 after
      stage10 entered the proof lane.
- [x] Add `stage11-check` for multi-domain adapter candidate/held/evidence
      closure over the Hy/Python product shell.
- [x] Keep full smoke green on Python 3.11 and Homebrew Python 3.14 after
      stage11 entered the proof lane.
- [x] Add `stage12-check` for self-improvement candidate quarantine and replay
      closure over stage11 adapter gaps.
- [x] Keep full smoke green on Python 3.11 and Homebrew Python 3.14 after
      stage12 entered the proof lane.
- [x] Add `stage13-check` for long-horizon replay, stale-held downgrades,
      boundary isolation, and stage12 quarantine replay continuity.
- [x] Add `stage14-check` and `stage14-export` for host-neutral Hy/Python
      cross-host fixture records and fresh export replay comparison.
- [x] Add `stage14-import` and `stage14-import-check` for peer JSON export
      import, answer-plan comparison, and draft schema migration.
- [x] Make unsupported stage14 import schemas fail closed as drift instead of
      entering replay.
- [x] Add `stage15-check` for open-world external evidence federation,
      evidence-only admission, quarantine, and stale-held revocation closure.
- [x] Add `stage15-export` for external admission services to consume the
      canonical stage15 evidence bundle.
- [x] Add `stagen-check` for the versioned constitutional extension manifest
      index after stage15.
- [x] Keep full smoke green on Python 3.11 and Homebrew Python 3.14 after
      stage14, stage15, and stageN entered the proof lane.
- [x] Add `self-host-check` for the compiler-axis self-application proof:
      stage7-loaded kernelA compiles full `stage2/kernel.hy`, loads kernelB,
      re-runs kernel self-check, compiles `stage2/compiler.hy`, and evaluates
      the factorial probe through kernelB.
- [x] Add `bootstrap-fixedpoint-check` for the compiler-axis B == C proof:
      kernelA compiles kernelB, kernelB compiles kernelC, and the kernel plus
      `stage2/compiler.hy` shim artifacts match at normalized, canonical code,
      raw marshal, and timestamp `.pyc` hashes.
- [x] Add `DIRECT-KERNEL-STRICT` plus `no-fallback-check` so owned corpus
      compilation fails closed instead of silently delegating to upstream
      `hy.compiler`.
- [x] Add `parity-ledger-check` to publish file-level and top-level-form
      direct-kernel hit/fallback coverage over the owned corpus plus upstream
      `tests/native_tests/*.hy`.
- [x] Fix direct-kernel self-compile lowering gaps for pending statement order
      in `do`, `if`, `cond`, boolop/binop, import target collection,
      quasiquote list concatenation, and isolated branch assignment bodies.

## Gap Analysis: Completing the Meta-Circular Hy(py) Compiler

Date: 2026-06-28. Method: `/deep-research` (workflow `wf_0b2047dd-0fb`, 6 angles,
22 sources, 25 claims adversarially verified, 19 confirmed) cross-referenced
with a local audit of `stage2/kernel.hy`, `stage2/compiler.hy`, and
`bootstrap.py`. This section tracks the **compiler axis**, which is distinct from
the stageN proof ladder below.

### Framing correction (read this first)

The `stage8 -> stage15 -> stageN` ladder is a **pnix-law proof / federation
lane** (artifact reproducibility, runtime replay, adapters, quarantine, evidence
federation). Those stages can all be green while the underlying **compiler** is
still a partial self-hoster. The actual "meta-circular Hy->Python compiler" is
the stage7 direct kernel (`stage2/kernel.hy`). Completing *it* is a separate axis
from advancing stageN. The items below are what is missing on the compiler axis;
they are NOT covered by any existing `stageN-check`.

### External completeness criteria (cited)

- Self-hosting = the source language and the input language are the same; the
  compiler can compile its own source code. [Wikipedia: Self-hosting (compilers);
  Bootstrapping (compilers)]
- Meta-circular = each language feature is implemented using the corresponding
  host facility, and the program's primary representation is a primitive datatype
  of the language itself (homoiconicity). [Wikipedia: Meta-circular evaluator]
- Verification oracle = a **fixed-point / triple test**: rebuild the compiler
  with itself (stage N), then again (stage N+1); the two outputs must be
  byte-identical or it is a build failure. GCC's default build compares stage2
  vs stage3 (must match); rustc builds stage1->stage2->stage3; bootstrappable
  builds require bit-identical regeneration. [gcc.gnu.org/install/build.html;
  rustc-dev-guide bootstrapping; D. Wheeler, Diverse Double-Compiling,
  arXiv:1004.5548]
- "Owning only the compiler core is insufficient": a complete self-hoster must
  also own the reader/front-end and the macro expander, not only the lowering
  core. [Wikipedia; Guile half-strap, wingolog 2016]
- Hy's compiler is the `HyASTCompiler` class dispatching Hy models to Python
  `ast` via the `@special`/`@pattern` tables in `hy/core/result_macros.py`, plus
  `hy/macros.py` (`macroexpand`), `hy/reader`, `hy/models`, and mangling. The
  CPython `ast` target surface is version-specific (`Parser/Python.asdl`).
  [github hylang/hy compiler.py, result_macros.py, macros.py; CPython
  InternalDocs/compiler.md]

Falsifiable bar for THIS project: the kernel must compile `stage2/kernel.hy`
(and `stage2/compiler.hy`) **with zero fallback**; the resulting compiler must
reproduce its own AST/bytecode artifacts byte-identically across a second
self-compilation; and no compile path may silently defer to upstream
`hy.compiler`.

### P0 (compiler axis): close self-application and the fallback hole

- [x] **Close first-order self-application.** `self-host-check` now compiles
      full `stage2/kernel.hy` through the stage7-loaded kernelA, loads the
      result as kernelB, re-runs kernelB `self-check`, compiles
      `stage2/compiler.hy` through kernelB, and evaluates the factorial probe
      through kernelB.
- [x] **Add a fixed-point / triple-test command.**
      `bootstrap-fixedpoint-check` now compiles `kernel.hy` through kernelA into
      kernelB, compiles `kernel.hy` through kernelB into kernelC, and asserts
      that the kernel plus `stage2/compiler.hy` shim artifacts match at
      source-free AST, generated Python, canonical code payload, raw
      `marshal.dumps(code)`, and timestamp `.pyc` hashes. This is the GCC/rustc
      stage2==stage3 bar adapted to Hy->`ast`.
- [x] **Make the host fallback fail-closed under a strict flag.**
      `compile-source-to-ast` now supports `DIRECT-KERNEL-STRICT`; when strict
      mode is enabled, a direct-kernel failure raises instead of delegating to
      `hy.compiler.hy-compile`. `no-fallback-check` compiles the owned compiler
      corpus (`kernel.hy`, `compiler.hy`, and proof examples) and fails if
      `DIRECT-KERNEL-FALLBACKS > 0`.
- [x] **Publish a parity ledger from real fallback counts.**
      `parity-ledger-check` now runs the owned corpus plus upstream
      `tests/native_tests/*.hy` through the direct-kernel bridge with fallback
      instrumentation, records file-level and top-level-form direct/fallback
      counts, reports native fallback files, and can write a per-file
      `parity-ledger.json` under `--debug-dir`.

### P0/P1 (front-end ownership): the reader is still upstream

- [x] **Decide and schedule reader ownership.** Decision: option (b). The reader
      is permanently classified as host substrate and the public classification
      is downgraded from "complete meta-circular" to "self-hosting back-end."
      `front-end-boundary-check` records this decision (`reader_ownership_decision:
      host-substrate`, `public_classification: self-hosting-back-end`) and proves
      the boundary; `reader-boundary-check` still asserts `reader_host_module ==
      "hy.reader"`. A Hy-written reader stays optional post-stage7 research, not a
      blocker.
- [x] **Own or formally cede `mangle`/`unmangle`.** Formally ceded to upstream
      `hy.reader.mangling` as a proven-pure host call: `front-end-boundary-check`
      proves `mangle`/`unmangle` are deterministic, round-trippable, and free of
      observable global mutation across ASCII, operator, and non-ASCII names.

### P1 (full surface): finish the result-macro and version-AST remainder

- [x] Drive the remaining `hy/core/result_macros.py` special forms (the ones
      still served only by fallback) into the kernel, prioritized by the parity
      ledger.

  Done (2026-06-29): the whole `parity-ledger-check` corpus — the owned compiler
  corpus plus every `tests/native_tests/*.hy` — now compiles **100% direct**
  through the kernel with zero fallback (`native_fallback_files: 0`,
  `direct_file_percent: 100.00`, `direct_top_level_form_percent: 100.00`); owned
  corpus stays zero-fallback (`no-fallback-check`). Closed by six incremental
  kernel steps, each verified with strict-mode behavioral harnesses matching
  upstream `hy.eval` and full smoke on Python 3.11 + 3.14:

  1. defclass bodies routed through body-form expansion (local defmacro/require/
     pragma in a class body).
  2. in-body `eval-when-compile`/`eval-and-compile` plus compile-time-evaluated
     `(pragma :hy ...)` -> `other.hy`.
  3. local `defmacro` in comprehension `:do` clauses.
  4. nested quasiquote `(unquote (unquote-splice ...))` depth, ported from
     upstream `render_quoted_form` -> `let.hy`.
  5. `do` made transparent for local macros (macro-writing-macros that expand to
     `(do (defmacro ...))`).
  6. quasiquoted f-strings (`` `f"...{~a}..." ``) and macro expansions that are a
     bare `defmacro`/`require`/`pragma` -> `macros_local.hy`, `quote.hy`.

  Caveat: the parity-ledger corpus is a broad-but-finite sample (owned corpus +
  upstream native tests), not literally every conceivable Hy form; 100% direct
  here means no fallback over that corpus, which is the parity-ledger-driven bar.
- [x] Own version-specific target-AST coverage explicitly per Python version
      against `Parser/Python.asdl`: 3.11 + 3.14 are in-lane; 3.12/3.13
      type-parameters and 3.14 t-strings/`TemplateStr` are gated. "Complete"
      requires either implementing or formally excluding each node, per target
      version, with a test — not a silent gate.
      Done: `version-ast-coverage-check` classifies every version-specific node
      as owned/gated/absent per target (3.11, 3.14) with a reason and a test
      reference, and cross-checks the classification against the running
      interpreter's actual `ast` surface (green on 3.11 and 3.14). It fails
      closed on any unclassified node or classification/`ast` mismatch.

### Suggested order

1. `no-fallback-check` + `DIRECT-KERNEL-STRICT` (makes the real gap measurable). DONE.
2. Parity ledger (turns "25-35%" into measured file/form-count data). DONE.
3. `self-host-check` (kernel compiles `kernel.hy` with zero fallback). DONE.
4. `bootstrap-fixedpoint-check` (B == C byte-identical). DONE.
5. Reader-ownership decision (own it, or rename the goal).
6. Close the result-macro / version-AST remainder, driven by the ledger.

Status (updated 2026-06-29): the stage8-15/N proof/federation work is now closed
(persisted append-only storage, EDN cross-host exchange, owned offline reference
checker adapters, approval-signature placeholders, concrete stage16, cost
telemetry, version-AST coverage, front-end host-substrate decision). The
compiler-axis bar above is also met: the parity-ledger corpus is 100% direct.
Of the original stageN ladder only 3 unchecked items remain, all in Stage14 and all
blocked on a cross-repo Clojure schema decision (a live Clojure host exposing a
stage14-compatible export). A second deep-research pass then surfaced 4 further
"completeness frontier" items beyond the parity-ledger 100% bar — all 4 are now
done (PEP 657 source positions, AST forward-compat, macro/require parity, diverse
double-compiling). See the Deep-Research Audit section immediately below. The
only remaining unchecked items are the 3 Stage14 Clojure-host items (blocked on a
cross-repo schema decision).

## Deep-Research Audit: Meta-Circular Completeness Frontier

Date: 2026-06-29. Method: second `/deep-research` pass (workflow
`wf_ebd58d31-044`, 5 angles, 25 primary sources fetched, 115 claims extracted,
25 adversarially verified, 23 confirmed; the run's synthesis stage returned a
diagnostic stub, so this checklist was reconstructed from the verified
transcript claims and cross-referenced against the kernel). This is the
completeness *frontier* — gaps BEYOND the parity-ledger 100%-direct bar. The
parity-ledger corpus (owned + all `tests/native_tests/*.hy`) is a broad-but-
finite sample; "100% direct" means no fallback over it, not over all of Hy.

External completeness definition (cited): a meta-circular evaluator implements
each language feature using the host's corresponding facility, and the language
is homoiconic / its programs are a primitive datatype of the language
[en.wikipedia.org/wiki/Meta-circular_evaluator]. A self-hoster compiles its own
source; the canonical consistency check is reproducing its own object code
(fixed-point), and the strongest oracle is diverse double-compiling
[en.wikipedia.org/wiki/Self-hosting_(compilers); dwheeler.com DDC dissertation].

### Essential to "complete meta-circular" — but explicitly CEDED (decision recorded)

- [x] **Reader/front-end ownership.** Research consensus (the verifier panel
      refuted the claim that ceding the reader is "legitimately cedeable"):
      because Hy reading can execute arbitrary code via reader macros, the reader
      is a Turing-complete, side-effecting front-end phase, and a *complete*
      meta-circular compiler owns it [docs.hylang.org/en/stable/syntax.html;
      hy/core/macros.hy `defreader`]. hy-meta DELIBERATELY cedes the reader and
      mangling to upstream `hy.reader` and downgrades the public claim to
      "self-hosting back-end" (see `front-end-boundary-check` + the P0/P1
      front-end decision above). Status: decided non-goal; reaffirmed by research.
      Optional future research: a Hy-written reader behind a flag with a parity
      map against `tests/test_reader.py`.

### Genuine open frontier gaps (beyond parity-ledger 100%)

- [x] **Full PEP 657 source-position / `co_positions` parity.** Traceback caret
      parity requires every emitted AST node to carry start AND end line/col, which
      CPython 3.11+ propagates into `code.co_positions()` [peps.python.org/pep-0657;
      docs.python.org/3 traceback/co_positions].
      Done (2026-06-29): `compile-expr` now wraps `compile-expr-raw` and stamps
      every returned expression node with the model's narrow source span, so each
      user-source leaf gets its own position (e.g. `(+ aaa bbb)`: aaa col3-6, bbb
      col7-10) instead of the coarse enclosing-statement span, and those positions
      propagate into `co_positions()`. `source-position-check` (wired into smoke)
      verifies leaf spans against the source and that `co_positions()` are
      populated and distinct, green on 3.11 + 3.14.
      Scope note: these are the strictly-correct 0-indexed positions
      (col_offset = start_column-1 .. end_column) — precise-position parity, the
      PEP 657 caret benefit. This is deliberately NOT byte-identical to upstream
      Hy, which stamps col_offset = start_column (one column right); exact
      upstream byte-parity is not the bar because upstream and the kernel emit
      different bytecode and upstream's column convention is off-by-one.

- [x] **AST constructor forward-compat + no-deprecated-nodes.** (a) CPython 3.14
      removed `ast.Num/Str/Bytes/NameConstant/Ellipsis`; the kernel must emit only
      `ast.Constant`. (b) CPython 3.13 made `ast` constructors strict: an omitted
      (or unexpected) field is a DeprecationWarning that becomes an error in 3.15
      [docs.python.org/3/library/ast.html; docs.python.org/3/whatsnew/3.13.html].
      Done (2026-06-29): `ast-forward-compat-check` (wired into smoke) verifies,
      over a node battery + the proof examples, that the kernel constructs/emits
      none of the removed nodes and that compiling under warning capture raises no
      ast-construction DeprecationWarning (3.15-safe). The check caught a real bug
      — `compile-sequence` passed `ctx` to `ast.Set` (which has no ctx field),
      a 3.14 DeprecationWarning / 3.15 error; fixed via `make-sequence-node`
      (Set built without ctx, List/Tuple keep it). 6 warnings -> 0 on 3.14, green
      on 3.11 + 3.14.

- [x] **Diverse double-compiling (Wheeler DDC).** `bootstrap-fixedpoint-check`
      proves the self-parented fixed point (kernelA->B->C byte-identical); DDC is
      the stronger trusting-trust oracle: build the compiler with a *different*
      compiler and check it emits the same output [dwheeler.com DDC dissertation,
      machine-checked proofs].
      Done (2026-06-29): `diverse-double-compile-check` (wired into smoke) builds
      kernel.hy two independent ways — `kernel_upstream` via upstream `hy.compiler`
      (stage1 seed) and `kernel_direct` via the direct kernel (stage2 bridge,
      confirmed by a nonzero direct-kernel hit count) — then has both compile
      kernel.hy and compiler.hy. The two independently-built compilers emit
      byte-identical artifacts at all four levels (normalized AST, canonical code,
      raw marshal, timestamp pyc); both pass self-check and factorial=120. Green
      on 3.11 + 3.14. A backdoor present in the direct build path but not upstream
      would have diverged.

- [x] **Macro/require edge cases outside the native corpus.** The native corpus is
      100% direct, but compilation parity is not runtime parity; these
      upstream-documented behaviors now have focused checks
      [hylang.org/hy/doc/v1.0.0/api; hy/core/macros.hy].
      Done (2026-06-29): `macro-require-parity-check` (wired into smoke) verifies
      16 behaviors against values pre-verified equal to upstream `hy.eval`:
      `get-macro` present/miss(raises)/docstring/builtins/`:reader`/local-vs-global
      order; `hy.I` and `hy.R` sugar; `require` named/`*`/`:as`; core-macro local
      shadowing; `macroexpand`/`macroexpand-1`; `eval-and-compile` macro deletion;
      `defreader` rejection outside global scope. Two strict behavioral harnesses
      (20 cases) found zero kernel/upstream divergence, so no kernel change was
      needed — the check is a regression guard. Green on 3.11 + 3.14.

### Legitimately cedeable (host substrate / tooling — note, do not pursue)

- 3.12 PEP 709 inlined-comprehension symtable changes, 3.13
      `__static_attributes__`/`__firstlineno__`, `PyCF_OPTIMIZED_AST`: these are
      produced by CPython's `compile()` from the AST the kernel emits, not by the
      kernel itself — host substrate [docs.python.org/3/whatsnew/3.12.html, 3.13].
- zipimport, `.pyc` autocompile, and full `sys.path_hooks` integration: upstream
      Hy supports these via `SourceFileLoader`/`pkgutil`/`runhy`, but hy-meta's
      scoped `KernelHyFinder` deliberately marks them unsupported (recorded in the
      stage/import sections) [hy/importer.py; docs.python.org/3/reference/import].
- `inspect.getsource` / pydoc / REPL syntax highlighting: even upstream Hy needs
      an unreliable paren-counting source-recovery heuristic and defers REPL
      highlighting to third-party tools, so full fidelity here is tooling, not
      compiler completeness [hy GitHub discussions/issues].

## Stage8: Artifact Reproducibility Closure

Definition:

```text
stage8 = stage7 artifact bundle
         -> fresh stage8 reload
         -> same canonical compiler/runtime artifacts
```

Hy-specific comparison order:

- [x] Compare artifact entry names before comparing hashes.
- [x] Compare normalized artifact hashes: AST data plus generated Python
      source.
- [x] Compare canonical compiler/runtime instruction payload hashes, excluding
      source-position and marshal reference encoding noise.
- [x] Keep raw `marshal.dumps(code)` and timestamp `.pyc` hashes as diagnostic
      fields using deterministic source size and file mtime inputs.
- [x] Classify the result as `reproduced` or `held`; do not auto-normalize or
      silently accept drift.
- [x] If drift appears, split it into raw marshal/pyc packaging drift versus
      true code-object instruction or constant drift.
- [x] Add focused diff tooling for changed artifacts:
      `changed-artifact-details.json` records per-field old/new hashes and
      classifies semantic versus raw marshal/pyc drift.

Stage8 acceptance:

```text
accepted/reproduced:
  artifact names, normalized hashes, and canonical instruction/code payloads
  match

held:
  raw marshal/pyc packaging drift appears, or metadata/debug/order drift needs
  explicit classification

rejected:
  code object constants, instruction stream, compiler output, or runtime logic
  differ after fresh reload
```

## Stage9: Clean Product Runtime Replay

Definition:

```text
stage9 = same stage8 artifact
         -> clean Python process/product entrypoint
         -> same canonical verdict/output plan
```

Checklist:

- [x] Define canonical product replay output for Hy compiler entrypoints:
      `run`, `py`, `hy2py`, `hyc`, `kernel-run`, `kernel-py`,
      `stage7-kernel-run`, and `stage7-kernel-py`.
- [x] Add a clean subprocess replay command that runs from a fixed manifest
      instead of inherited REPL/module-cache state.
- [x] Bind hard manifest fields: compiler/kernel/bootstrap/example source
      hashes, Python family, executable, repo root, and deterministic env.
- [x] Bind soft environment fields currently used by replay: hash seed, locale,
      and timezone.
- [x] Prove product entrypoints do not depend on stale `sys.modules`, temp
      files, current directory accidents, or process-local counters.
- [x] Fail closed on canonical product replay drift.
- [x] Expand hard manifest binding to include installed Hy package version and
      explicit route/feature gate versions.
- [x] Add alternate-cwd replay to prove product commands do not rely on the
      caller's current directory when the repo root is explicit.
- [x] Add negative replay fixtures for expected held/rejected compiler
      boundaries, not only successful `(+ 20 22)` surfaces.
- [x] Add cost telemetry for each subprocess probe so stage9 remains acceptable
      inside full smoke.

## Stage10: Client/Server/Session/Sandbox Closure

- [x] Define local server/client fixture format for the Hy product shell.
- [x] Replay the same canonical Hy inputs through direct CLI, subprocess,
      server handler, and sandbox session.
- [x] Prove session-local macro tables, reader macro tables, globals, temp
      names, and import hooks do not cross session boundaries.
- [x] Record sandbox witness hashes for generated Python and `.pyc` execution.
- [x] Treat remote/client rendering differences as presentation only; compare
      canonical result plans.
- [x] Add clean CLI subprocess replay from an alternate working directory.
- [x] Add `--debug-dir work/stage10-debug` manifest/canonical/drift output.
- [x] Add an actual socket/HTTP loopback server once the handler contract is
      stable enough to justify network lifecycle complexity.
- [x] Add concurrent session replay to prove two simultaneous sessions do not
      share macro tables, reader macro tables, temp-name counters, or globals.
- [x] Add sandbox denial fixtures for unsupported filesystem/zip/bytecode
      import surfaces.
- [x] Add client/server protocol versioning and downgrade behavior.

## Stage11: Multi-Domain Adapter Closure

- [x] Keep code/document/graphics/robot/open-problem adapters under the same
      accepted/candidate/held/evidence constitution.
- [x] Extend the first adapter matrix to math/language/audio as held
      unsupported-route adapters until concrete Hy product routes exist.
- [x] Ensure adapter output is evidence or candidate until a domain gate
      admits it.
- [x] Require sandbox witness or human confirmation before execution-risk
      domains become accepted.
- [x] Add a mixed-domain fixture where code generation, visualization, and
      robot/action requests split into candidate/held results instead of
      auto-executing.
- [x] Define a minimal adapter contract: input canonical form, evidence
      records, capability declaration, gate result, and explanation plan.
- [x] Add a domain capability matrix that distinguishes unsupported, candidate,
      sandbox-required, human-required, and accepted-capable routes.
- [x] Keep math/code/language/document/graphics/robot/audio/open-problem
      adapters under the same accepted/candidate/held/rejected constitution.
- [x] Add adapter conflict handling when two domains produce incompatible
      candidates for the same request.
- [x] Add regression fixtures proving one domain's adapter cannot mutate another
      domain's route policy or accepted records.
- [x] Add `--debug-dir work/stage11-debug` review docs for each adapter's
      canonical record and witness hashes.
- [x] Add negative adapter fixtures where a malicious adapter attempts to set
      `gate_verdict: accepted`, `promotion_allowed: true`, or `executed: true`.
- [x] Add a stable adapter schema version and migration policy before stage12
      uses adapter gaps as self-improvement candidates.

## Stage12: Self-Improvement Quarantine Closure

- [x] Represent held/rejected gaps as self-improvement candidates without
      mutating live truth.
- [x] Keep route-policy and adapter-gate update candidates quarantined until
      replay and owner/admission gates pass.
- [x] Extend quarantine coverage to profile, rule, and compiler patch
      candidates.
- [x] Prove quarantined candidates cannot change existing accepted verdicts.
- [x] Record quarantine replay status as passed, failed, drift, or
      not-admitted.
- [x] Explicitly reject direct promotion from self-improvement candidate to
      accepted truth.
- [x] Add immutable before/after manifests for live truth around quarantined
      route-policy and adapter-gate candidates.
- [x] Add immutable before/after manifests for compiler, profile, and rule
      update candidates.
- [x] Add replay fixture bundles for candidate patches, including expected
      non-regression over stage7, stage8, stage9, and stage10 proofs.
- [x] Add owner/admission records separate from candidate generation records.
- [x] Add quarantine garbage-collection policy for failed or superseded
      candidates without deleting their audit trail.
- [x] Add a stage11-to-stage12 bridge: held adapter gaps may create
      self-improvement candidates, but adapter candidates cannot promote
      themselves.
- [x] Add quarantine replay over the stage11 mixed-domain fixture.
- [x] Add route-priority update candidates that must not change stage11
      candidate/held/evidence boundaries without admission.
- [x] Add `--debug-dir work/stage12-debug` manifest/canonical/drift output.
- [x] Add malicious candidate fixtures for compiler patch, profile update, and
      rule update, not only route-policy promotion.
- [x] Add quarantine replay over full stage8/stage9/stage10 smoke subsets when
      cost telemetry supports it.
- [x] Add persisted quarantine storage with append-only audit IDs.
- [x] Add owner/admission signature or approval-token placeholders before
      accepting any quarantined candidate.

## Stage13: Long-Horizon Product Organism Closure

- [x] Add daily/weekly replay manifests for older answers after artifact,
      corpus, adapter/capability, or route-policy updates.
- [x] Downgrade stale answers to held when their manifest no longer matches.
- [x] Prove user/session/project boundaries prevent accidental referent reuse.
- [x] Track frontier growth, explanation consistency, capability matrix drift,
      and safety violations over long runs.
- [x] Keep old accepted verdicts replayable or explicitly stale-held.
- [x] Add manifest lineage records that connect old replay results to the
      artifact/corpus/runtime version that produced them.
- [x] Add session/project/user boundary fixtures for pronoun/referent reuse and
      macro/import state.
- [x] Add stale downgrade reasons: artifact changed, corpus changed, adapter
      changed, and route policy changed.
- [x] Add long-horizon audit summaries that can be compared without loading
      full historical debug artifacts.
- [x] Add long-horizon multi-domain replay where old stage11 adapter candidates
      are rechecked after capability matrix updates.
- [x] Add long-horizon quarantine replay where old stage12 candidates remain
      not-admitted after unrelated product updates.
- [x] Add periodic stale-held downgrades for candidates whose source held gap
      disappeared or whose capability matrix changed.
- [x] Add `--debug-dir work/stage13-debug` manifest/canonical/drift output.
- [x] Persist long-horizon replay manifests outside a single process fixture.
- [x] Add a signed or append-only audit id for every stale-held downgrade.
- [x] Add proof-checker and environment hard-bound stale downgrade fixtures.
- [x] Add cost telemetry so stage13 can expand without slowing full smoke
      beyond the proof-lane budget.

## Stage14: Cross-Host Pnix Law Closure

- [x] Define canonical cross-host fixture format shared by `pnix-clj`,
      `pnix-hy`, `clj-meta`, and `hy-meta`.
- [x] Define canonical verdict/held/rejected/conditional and answer-plan
      hashes; rendered prose is not the comparison target.
- [x] Define a host capability matrix so missing features become held instead
      of drift.
- [x] Define stable JSON exchange records for the Hy/Python `hy-meta` host.
- [x] Define stable EDN exchange records for Clojure hosts.
- [~] Run the same fixtures on Clojure and Hy/Python hosts once Clojure host
      exports are available.
- [x] Compare answer-plan hashes for available host exports and fail closed on
      drift.
- [~] Compare answer-plan hashes against Clojure host exports once they exist.
- [x] Record `:cross-implementation-replay` and
      `:cross-implementation-drift` manifests.
- [x] Add a minimal `hy-meta` exporter for stage9 probe records so `pnix-clj`
      can compare without invoking Python internals directly.
- [x] Add a cross-host adapter status vector comparison for stage11 records.
- [x] Add cross-host quarantine replay comparison for stage12 candidate records.
- [x] Add a cross-host long-horizon replay comparison for stage13 stale-held
      and boundary records.
- [x] Export stage13 lineage records in a host-neutral JSON shape.
- [x] Export stage13 lineage records in an EDN shape for Clojure hosts.
- [x] Add `--debug-dir work/stage14-debug` manifest/canonical/drift output.
- [x] Add a `stage14-import` command that reads a peer host JSON export from
      disk and compares it against the local `hy-meta` export.
- [x] Extend `stage14-import` to EDN once Clojure host exports are available.
- [x] Add cross-host schema migration rules for old stage14 JSON export
      versions.
- [x] Reject unsupported stage14 import schemas before answer-plan replay.
- [~] Add actual `pnix-clj`/`clj-meta` invocation wiring when those repos expose
      compatible export commands.

## Stage15: Open-World Evidence Federation Closure

- [x] Define external evidence schema with source type, adapter id/version,
      artifact hash, claim hash, provenance, and replay status.
- [x] Treat Lean/Rocq/Isabelle, Z3/SMT, CAS/e-graph, GitHub repos, documents,
      LLM suggestions, user files, remote sandboxes, and graph backends as
      evidence-only until pnix gates admit them.
- [x] Prove external results never directly become accepted truth.
- [x] Add external admission records that bind canonical claim, proof/witness
      reference, owner-law version, and final gate verdict.
- [x] Fail closed on adapter drift, source/provenance ambiguity, or unreplayed
      external claims.
- [x] Add quarantine storage for external evidence that is replayable but not
      admitted.
- [x] Add revocation/downgrade records for external evidence whose source,
      adapter, or checker version changes.
- [x] Add explicit network boundary policy: online fetch is evidence
      acquisition, offline replay is admission.
- [x] Add external evidence fixture classes for code patch, proof artifact,
      solver result, document claim, and LLM suggestion.
- [x] Add external evidence admission tests that reuse the stage11 adapter
      constitution: external adapters may emit evidence/candidate only.
- [x] Ensure external evidence cannot bypass stage12 quarantine/admission when
      it proposes route, compiler, profile, or rule changes.
- [x] Ensure external evidence cannot bypass stage13 stale-held downgrade when
      its source, adapter, checker, or corpus binding changes.
- [x] Add open-world evidence fixtures that replay across stage13 lineage
      records before any stage15 admission is considered.
- [x] Add `--debug-dir work/stage15-debug` manifest/canonical/drift output.
- [x] Add a `stage15-export` command once a peer admission service needs to
      consume the evidence bundle directly.
- [x] Add real offline checker adapters for Lean/Rocq/Isabelle/Z3/CAS instead
      of the current canonical fixture records. (owned reference offline checker:
      z3/smt arith-eval, cas rewrite-structural, prover reference proof-hash;
      brand-name kernel binaries stay pluggable behind the online boundary.)
- [x] Add real GitHub/document/user-file acquisition adapters behind an
      explicit online evidence-acquisition boundary. (acquisition-adapter layer
      with online-fetch gated off by default; admission never fetches network.)
- [x] Add persisted external-evidence quarantine storage with append-only audit
      ids outside a single process fixture.
- [x] Add owner/admission signature or approval-token placeholders before any
      external evidence can become accepted.
- [x] Add source-family specific revocation replay for remote sandbox and graph
      backend evidence.

## StageN: Constitutional Extension Lane

- [x] Every stage after stage15 must name its closure target, artifact surface,
      hard/soft manifest bindings, replay strategy, and fail-closed boundary.
- [x] No stageN work may weaken stage7 semantic closure, stage8 artifact
      closure, stage9 clean product replay, or stage15 evidence-only
      admission.
- [x] Every stageN addition must include a migration rule for old manifests:
      reproduced, stale-held, or rejected.
- [x] Every stageN addition must expose a debug artifact directory contract.
- [x] Every stageN addition must state whether it is local-only,
      cross-process, cross-host, or open-world.
- [x] Add `--debug-dir work/stageN-debug` manifest/canonical/drift output.
- [x] Anchor the first stageN extension manifest to the stage15 evidence export
      hash.
- [x] Add concrete stage16 implementation: `stage16-check` replays the stageN
      extension manifest against the live stage15 export anchor (reproduced vs
      stale-held), migrates an older manifest schema, and fails closed.
- [x] Add stageN import/migration tests for older stageN manifest versions
      (`stageN-extension-v0` -> migrated, unknown -> unsupported) in stage16-check.
- [x] Add stageN peer-review/admission signature placeholders before admitting
      a post-stage15 extension (pending-peer-review-signature; never accepted).

## Resume Here

Repository state to check after reopening:

```text
branch: main
remote: origin/main
run `git status -sb`
run `git log -1 --oneline`
```

Last verification completed before opening the 3.14/latest lane:

```sh
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage7-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/smoke_test.py
find hy-1.3.0 -type d -name __pycache__ -prune -exec rm -rf {} +
```

Both `stage7-check` and full smoke passed. Full smoke ended with:

```text
native_subset: ok
```

Last verification completed after opening the 3.14/latest lane:

```sh
/tmp/pnix-hy-py314-venv/bin/python hy-meta/bootstrap.py stage7-check
/tmp/pnix-hy-py314-venv/bin/python hy-meta/bootstrap.py compatibility-boundary-check
/tmp/pnix-hy-py314-venv/bin/python hy-meta/smoke_test.py
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage7-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py compatibility-boundary-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/smoke_test.py
```

Both Python 3.14.6 and Python 3.11.15 passed. Full smoke ended with:

```text
native_subset: ok
```

Do this first after reopening:

1. `cd ~/pnix/hy-1.3.0`
2. `git status -sb`
3. If clean, continue the open post-stage7 compiler/result-macro parity item.
4. Keep changes small, verify with focused checks, then `stage7-check`, then
   full `smoke_test.py`.
5. Clean `__pycache__`, commit, and push each verified unit.

Do not reopen yet:

- Python 3.12 type parameters;
- Python 3.13 inspection/traceback differences;
- replacing upstream `hy.reader` as the default reader host.

## Active Goals: 3.11 + Latest 3.14

- [x] Allow the hy-meta bootstrap commands to run on exactly the supported
      proof-target families: Python 3.11 and Homebrew latest Python 3.14.
      Keep Python 3.12/3.13 out of scope.
- [x] Build or reuse a local Python 3.14 environment and run the same stage7
      proof lane on it:
  - [x] Install/upgrade Homebrew `python@3.14` only;
  - [x] Verify executable `/usr/local/opt/python@3.14/bin/python3.14`;
  - [x] Run `hy-meta/bootstrap.py stage7-check` under Python 3.14;
  - [x] Run `hy-meta/smoke_test.py` under Python 3.14;
  - [x] Keep Python 3.11 `stage7-check` and full smoke green.
- [x] Decide and implement the Python 3.14 t-string policy for the direct
      kernel:
  - [x] keep t-strings explicitly gated on 3.14 with focused tests documenting
        the exact unsupported direct-kernel compile surface;
  - [x] do not implement `TemplateStr`/`Interpolation` lowering until full
        t-string parity is explicitly promoted into the owned product surface.

## Active Goals: Compatibility-First Owned Lane

These goals replace the earlier "full independent Hy replacement" framing.
The project should not break Python module use or upstream Hy compatibility in
order to prove meta-circular ownership. The right target is an owned,
deterministic stage7 compiler/interpreter lane with explicit compatibility
boundaries and safe fallback.

- [x] Hard non-goal: never pursue independence from the Python runtime or
      Python module system:
  - [x] CPython runtime, Python `ast`, importlib, bytecode execution, and
        normal `.py` module import are permanent substrate, not temporary
        dependencies to remove;
  - [x] any design that blocks ordinary Python package/module use is rejected,
        even if it makes the meta-circular proof look cleaner;
  - [x] owned Hy compiler/interpreter work must improve determinism and
        isolation without reducing Python ecosystem compatibility.
- [x] Keep Python ecosystem compatibility as a hard requirement:
  - [x] keep CPython runtime, Python `ast`, importlib, bytecode execution, and
        normal `.py` module import as permanent substrate;
  - [x] do not introduce a mode that blocks ordinary Python package imports;
  - [x] keep Python 3.11 and Homebrew Python 3.14 smoke tests covering CLI,
        import hook, REPL, `.pyc`, `runpy`, relative imports, and macros.
- [x] Define the Python substrate/proof boundary before expanding mirror work:
  - [x] imports are allowed to execute arbitrary Python side effects; the proof
        only owns cleanup for scoped `hy-meta` import hooks, controlled module
        names, failed `.hy` imports, and macro/reader-macro tables;
  - [x] `sys.modules` is a shared Python runtime cache, not something to
        replace; the proof only asserts isolation for owned stage/probe module
        names and explicit cleanup after failed owned imports;
  - [x] native extension and C extension behavior is opaque substrate; do not
        base mirror equality on C pointer identity, object addresses, or native
        implementation internals;
  - [x] Python-version differences in AST, bytecode, traceback, and `inspect`
        are per-version compatibility surfaces; compare only the owned stable
        fields across Python 3.11 and Homebrew Python 3.14;
  - [x] nondeterministic inputs such as `random`, time, filesystem iteration
        order, hash seed, and `id()`/object identity must be seeded, sorted,
        injected, or excluded from mirror equality;
  - [x] monkey patching, import hooks, and dynamic module creation are legal
        Python behavior; owned tests must install them in scoped namespaces and
        restore `sys.meta_path`, `sys.path`, `sys.modules`, macro tables, and
        reader macro tables when the proof owns the mutation.
- [x] Keep upstream Hy compatibility as the default user-facing behavior:
  - [x] keep upstream `hy.reader` as the default reader host;
  - [x] keep upstream Hy fallback available for Hy syntax outside the owned
        direct-kernel surface;
  - [x] require focused parity tests before moving any Hy syntax from fallback
        into direct-kernel ownership.
- [x] Grow the owned stage7 lane only behind compatibility checks:
  - [x] specify each owned surface before implementing it: reader behavior,
        model construction, compiler lowering, macro expansion, import hook,
        CLI, or REPL;
  - [x] prove each owned surface is deterministic across stage7 on both Python
        3.11 and 3.14;
  - [x] prove module cache, globals, macro tables, reader macro tables, and
        generated temp-name counters do not leak between stages/modules.
- [x] Treat a Hy-owned reader as optional research, not the default path:
  - [x] require a focused reader parity map against `hy/reader/*` and
        `tests/test_reader.py` before implementing reader pieces;
  - [x] require any Hy-written reader component to live behind an explicit
        feature flag;
  - [x] require fallback to upstream `hy.reader` until the owned reader proves
        equal or safer on user-visible behavior.
- [x] Expand exact compiler/tooling parity only when it protects compatibility:
  - [x] decide which rendered Python/source formatting differences are actual
        user-visible bugs;
  - [x] add focused snapshots for user-visible source output;
  - [x] do not chase private upstream compiler internals unless a user-visible
        behavior, compatibility guarantee, or stage proof requires them.
- [x] Define the acceptance test for "compatibility-first stage7 ownership":
  - [x] stage7 compiler/kernel proof passes on Python 3.11 and 3.14;
  - [x] full owned CLI/import/REPL/native-subset smoke passes on Python 3.11
        and 3.14;
  - [x] upstream-reader/default-fallback path remains available and tested;
  - [x] normal Python package imports remain available and tested;
  - [x] direct-kernel owned behavior never silently replaces unsupported Hy
        behavior without a parity test or an explicit gate.

## Active Goals: Boundary Hardening Pass

- [x] Extend `compatibility-boundary-check` so it proves the kernel import hook
      respects pre-existing `sys.modules` entries instead of replacing them.
- [x] Extend `compatibility-boundary-check` so normal Python import side effects
      still execute under the scoped kernel import hook.
- [x] Extend `compatibility-boundary-check` so native/C extension imports remain
      Python substrate and are never routed through `KernelHyLoader`.
- [x] Extend `compatibility-boundary-check` so failed owned `.hy` imports are
      removed from `sys.modules`.
- [x] Extend `compatibility-boundary-check` so hook cleanup is proven even when
      the hook context exits through an exception.
- [x] Run the hardened compatibility boundary check and full smoke on Python
      3.11 and Homebrew Python 3.14, then close this pass.

Immediate next work:

- [x] Tighten the remaining reader-timed pragma behavior:
      `:bracketed-templates` now mutates the same lazy `read-many` reader
      before subsequent forms are parsed, giving the direct-kernel lane the
      upstream reader-timing behavior it owns.
- [x] Decide how far to chase exact upstream error message text. The
      direct-kernel lane now locks representative supported compile-error
      messages; exact upstream `HyLanguageError.msg` wording for every compiler
      path remains outside the current direct-kernel ownership boundary.
- [x] Audit source rendering and source-position parity for compiler output.
      Direct-kernel top-level/generated statement positions now preserve Hy
      reader model locations, including macro-expanded statement roots; stage7
      mirror/stability checks remain green. Exact upstream rendered-source
      formatting is tracked only as broad compiler parity.
- [x] Continue importing only direct-kernel-owned compiler internals into
      `hy-meta/native_subset_test.py`; leave broad upstream tool behavior out
      unless it is intentionally product-scoped. The focused lane now includes
      the remaining direct-kernel-owned `tests/compilers/test_compiler.py`
      internals: bare-name branch preservation and generator final return AST
      shape.

## Current State

The useful proof lane exists and is green through stage7:

```text
Python stage1
  -> loads stage2/compiler.hy
  -> stage2 loads stage2/compiler.hy again as stage2-prime
  -> stage2-prime loads stage2/compiler.hy again as stage3
  -> the same compiler chain repeats through stage7
  -> stage7 mirror checks compare generated Python, AST, and values
  -> stage7 loads stage2/kernel.hy
  -> the Hy-written kernel evaluates factorial, kernel_features, loop, and
     stability-stress examples with stable generated Python/AST
```

The important command is:

```sh
cd ~/pnix/hy-1.3.0
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage7-check
```

Expected current output includes:

```text
python: 3.11.15
stage_count: 7
last_stage_module: hy_meta_stage7.compiler
all_stage_self_checks: True
stage_module_cache_ok: True
probe_macro_tables_distinct: True
stage_reader_macro_tables_distinct: True
compiler_ast_stage7_mirror: True
kernel_ast_stage7_mirror: True
kernel_value_stage7_mirror: True
kernel_factorial: 120
kernel_features: 449.0
kernel_loop: 120
kernel_stress_repeat_python_stable: True
kernel_stress_repeat_ast_stable: True
```

Full smoke:

```sh
/tmp/pnix-hy-py311-venv/bin/python hy-meta/smoke_test.py
```

Smoke runs `chain-check`, `kernel-check`, `kernel-import-check`,
`direct-kernel-bridge-check`, `prime-check`, `stage3-check`, `mirror-check`,
`stage7-check`, `reader-boundary-check`, scoped CLI/REPL/tool checks,
stage7-kernel command checks, and the native subset tests.

## Directory Roles

`stage1/`

- Python seed compiler/protocol.
- Wraps Hy's existing reader/compiler enough to compile and load Hy files.
- This is not the final meta-circular target. It is a bootstrap seed.

`stage2/compiler.hy`

- Hy-written version of the bootstrap protocol.
- Still leans on the host Hy compiler for general Hy compilation.
- Can load itself again as stage2-prime, then through stage7 for repeated
  mirror checks.

`stage2/kernel.hy`

- The important hy-meta experiment.
- A Hy-written direct compiler kernel for a growing Hy subset.
- Compiles Hy models directly to Python `ast` nodes for target programs.
- This is the place to move more compiler behavior out of Python.

`hy-meta/bootstrap.py`

- CLI and proof-lane driver.
- Commands: `self-check`, `chain-check`, `kernel-check`,
  `kernel-import-check`, `prime-check`, `stage3-check`, `mirror-check`,
  `stage7-check`, `run`, `py`, `kernel-run`, `kernel-py`.

`hy-meta/native_subset_test.py`

- Focused native-style checks for lambda lists, statement-body `fn`,
  class-body bare `#^` annotations, complex function annotation hints,
  illegal binding rejection for constants and invalid targets,
  quasiquote falsey `unquote-splice` behavior and nested quasiquote depth
  handling plus repeated `hy.eval` of triple quasiquote through the kernel
  evaluation module context,
  comprehension side-effect clauses, expanded match patterns, native flat
  `match` expression syntax, `let` body binding leakage, control-flow
  ordering, module/function/class docstring edge cases,
  native-style `defclass` shorthand/class keyword arguments,
  f-string conversion/spec/debug/evaluation-order cases,
  quoted string/f-string model metadata,
  keyword comparison/ordering/pickling/callable lookup/kwargs cases,
  native operator edge semantics, break/continue loop behavior, defclass
  dynamic-base/body-leak/side-effect cases, native decorator
  evaluation-order/stacking/async cases, and `with` exception suppression,
  statement-producing `and`/`or` short-circuit operands,
  statement-producing `if`/`when`/`cond` expression branches,
  native `do` and `del` edge cases,
  native `nonlocal` promotion and missing-binding rejection,
  native dotted method shortcut keyword-prefix and empty-dot cases,
  native dotted statement-special root cases,
  native call argument ordering, including keyword arguments before later
  positional arguments and unpacking forms,
  comparison special-form aliases such as `not_in` and `is_not`,
  collection literal evaluation order when list/tuple/set/dict entries contain
  pending statement-producing expressions,
  context-manager exit ordering, and `try`/`except`/`else`/`finally` behavior,
  sync/async `try` expression values, `except*` expression values,
  native-style `except` spec variants, synchronous `try` expression
  outer-scope effects, top-level expression-valued `try`/`with` results, ordered
  multi-pair `setv` with pending expressions, sync/async `with` expression
  values, sync/async comprehension side effects, and unpacking parity cases.

`hy-meta/examples/kernel_features.hy`

- Main feature exerciser for the Hy-written kernel.
- Current expected result is `449.0`.
- Most newly added kernel features should get a small assertion here.

`hy-meta/examples/kernel_import_probe.hy`
`hy-meta/examples/kernel_import_consumer.hy`
`hy-meta/examples/kernel_import_pkg/__init__.hy`
`hy-meta/examples/kernel_import_pkg/child.hy`
`hy-meta/examples/kernel_import_pkg/sibling.hy`
`hy-meta/examples/kernel_import_broken.hy`

- Import-hook exercisers for loading `.hy` modules through the Hy-written
  kernel path.
- The consumer imports the probe, so the proof covers both direct import and a
  nested import triggered by kernel-compiled module code.
- The package imports its child module and a sibling module, so the proof also
  covers `__init__.hy`, package `__path__`, submodule loading, and relative
  imports.
- The broken module raises during import, so the proof covers failed-import
  cleanup and stale `sys.modules` avoidance.

## What Is Done

The stage7 proof lane is done enough to continue real kernel work:

- stage1 loads stage2.
- stage2 loads stage2-prime.
- stage2-prime loads stage3, then the chain repeats through stage7.
- stage2-prime can run `hy-meta-check`.
- stage3 can run `hy-meta-check`.
- `hy-meta-check` can load and execute `stage2/kernel.hy`.
- The stage3-loaded kernel can evaluate examples and emit Python source.
- `mirror-check` compares stage2/stage2-prime/stage3 compiler AST/Python
  output, example values, and stage2-prime/stage3 kernel AST/Python/value
  output.
- `stage7-check` extends the mirror to stage7 and checks stage/kernel module
  names, `sys.modules` mappings, and macro/reader-macro table separation.
- `kernel-import-check` installs a scoped import hook and verifies that Python
  `importlib` can load `.hy` modules through the Hy-written kernel path,
  including nested imports, package imports, submodule imports, module cache
  entries, loader identity, package paths, and hook cleanup.

The Hy-written kernel currently supports these broad areas:

- literals: integers, floats, complex, strings, bytes, keywords
- f-string literals, including conversions, format specs, nested format specs,
  debug syntax, and statement-producing replacement expressions
- keyword equality, ordering, pickling, callable lookup/default behavior, and
  keyword argument name mangling
- symbols and Python constants: `None`, `True`, `False`, `...`
- calls, keyword calls, iterable unpacking, mapping unpacking
- literal `#*` iterable unpacking and `#**` mapping unpacking
- method shortcut calls, including leading keyword arguments before the receiver
  and empty-dot receiver identity
- dotted statement-special roots such as `(. defn)`
- list, tuple, set, dict literals
- arithmetic operators: `+`, `-`, `*`, `/`, `//`, `%`, `**`, `@`
- bitwise operators: `&`, `|`, `^`, `<<`, `>>`, `bnot`, `invert`
- native operator edge semantics: unary `+`, unary `-`, unary reciprocal `/`,
  right-associative `**`, `|`'s empty identity, and matching iterable-unpack
  arity behavior
- arithmetic, boolean, and comparison operator argument `#*` iterable unpacking
- augmented assignment for the corresponding operator families, including
  native multi-RHS grouping and right-associative `**=` plus matrix `@=` RHS
  handling
- comparisons: `=`, `!=`, `<`, `<=`, `>`, `>=`, `in`, `not-in`, `is`, `is-not`
- comparison aliases `not_in` and `is_not` behave like the special forms
  `not-in` and `is-not`
- native comparison arity: unary `=`, `<`, `<=`, `>`, `>=`, and `is` evaluate
  their operand and return `True`, while `!=`, `is-not`, `in`, and `not-in`
  require at least two operands, including through iterable unpacking
- mixed chained comparisons with `chainc`, preserving left-to-right operand
  evaluation and short-circuiting
- boolean short-circuit: `and`, `or`, `not`
- expression forms: `if`, `when`, `cond`, `do`
- pending-aware `if`, `when`, and `cond` expression branches for
  statement-like branch values
- `setx` assignment expressions, including containing-scope updates from
  helper-backed comprehensions
- `setv` assignment
- `setv` expression-position `None` result
- `setv :chain` chained assignment
- symbol, list/tuple destructuring, attribute, and subscript assignment targets
- `(get xs i ...)` nested subscript read/write
- native `(cut xs [start] [stop] [step])` slicing, including whole-slice and
  prefix-slice forms
- `del`, including empty `(del)` as a native no-op
- `assert`
- `pass`
- `return`
- `yield` statements and expression-position `yield`
- generator delegation with `(yield :from iterable)`
- expression-position `defn`, `defclass`, `for`, `assert`, and `pass` as
  statement forms returning `None`
- expression-position `del` as a statement form returning `None`
- pending-aware `and`/`or` lowering so statement-producing operands preserve
  short-circuit behavior
- `raise`, including `raise expr :from cause`
- `try` with `except`, `else`, `finally`
- sync and async `try` expression values through helper lowering
- synchronous `try` expression outer-scope effects through inline result
  variable lowering
- top-level and statement-position expression-valued `try`, `with`, and `match`
  forms preserve their expression behavior where Hy expects a value, including
  final function and `fn` body forms
- native `except` specs: `[]`, `[[TypeA TypeB]]`, `[name Type]`,
  `[name [TypeA TypeB]]`, and `[Type name]`
- Python 3.11 `except*` / `ast.TryStar`
- `except*` expression values through result-variable helper lowering
- `while`, `break`, `continue`, trailing `(else ...)`, including
  statement-containing `do` conditions re-evaluated on each iteration
- `for` over one or more target/iterable pairs, trailing `(else ...)`,
  including nested iterable pending placement
- native break/continue loop behavior, including loop-variable visibility after
  break
- `for [:async target iterable]`
- `with`, multiple managers, `_` anonymous managers
- `with [:async target manager]`
- sync and async `with` expression values through helper lowering
- synchronous `with` exception suppression in expression position
- `global`
- `nonlocal`, including module-binding promotion to `global` declarations
- `import`, dotted import, Hy-style module name mangling, `import x :as y`,
  and `from` import forms with aliases
- leading `__future__` imports before injected `import hy`, after any module
  docstring
- module, function, and class docstrings, including single-string function
  bodies returning that string instead of becoming docstrings
- `defn`, including `:async`
- `fn` expressions
- `fn` with statement bodies, lowered through generated local functions
- `defclass`
- `defclass` shorthand without an explicit base vector
- `defclass` class keyword arguments such as `:metaclass`
- dynamic class base expressions, class-body side effects, and class-body
  function no-leak behavior
- function and class decorators
- default positional parameters
- `/` positional-only parameters
- `*` keyword-only parameters
- keyword-only defaults
- keyword-only parameters after `#*` varargs
- `#*` varargs
- `#**` kwargs
- tuple-pattern parameter destructuring with `#(...)`
- annotated tuple-pattern parameter destructuring
- keyword-only tuple-pattern parameter destructuring
- native call argument ordering, including keyword arguments before later
  positional arguments and `#*` / `#**` unpacking
- assignment destructuring, including starred list/tuple targets
- list, tuple, set, and dict literal evaluation order when entries, keys,
  values, or unpacking forms emit pending statements
- `let`
- `let` destructuring
- starred and nested starred `let` destructuring
- annotated `let` bindings
- sequential `let` binding and rebinding
- statement-body and match-containing `let` bodies through helper lowering
- native-style `let` body binding leakage for new statement bindings while
  keeping let-bound names hidden
- top-level `defmacro`
- `quote`
- quoted string/f-string model metadata, including bracket delimiters,
  component conversions, and delayed f-string evaluation
- `quasiquote`
- `unquote`
- sequence `unquote-splice`, including falsey splice values such as `0`,
  `False`, and `None`
- nested quasiquote/unquote depth handling for focused native quote cases
- repeated `hy.eval` of triple quasiquote while preserving and restoring the
  temporary `eval-source` module cache entry
- `lfor`, `sfor`, `dfor`, `gfor`
- `dfor` final `#**` mapping unpacking
- `lfor`, `sfor`, and `gfor` final `#*` iterable unpacking
- comprehension `:if` filters
- comprehension iterable and `:if` expressions with pending statement lowering
- synchronous comprehension `:do` and `:setv` side-effect clauses
- async comprehension `:do` and `:setv` side-effect clauses
- async comprehension generator marker `:async`
- variable annotations with `(annotate x type)` and `#^`
- standalone annotations
- `defn` parameter and return annotations
- `fn` parameter and return annotations
- async functions with `await`
- async context managers
- async iterators
- `match` expressions/statements with native flat clauses:
  - wildcard patterns
  - capture patterns
  - literal patterns
  - keyword patterns
  - dotted-value patterns
  - sequence patterns
  - sequence `#*` star captures
  - mapping patterns
  - mapping `#**` rest captures
  - class patterns
  - class keyword patterns
  - `(| p1 p2 ...)` OR patterns
  - `(as pattern name)` AS patterns
  - `pattern :as name` AS patterns
  - `:if` guards, including statement-like `do` guards through helper lowering
  - literal-key validation for mapping patterns

The following examples are currently part of the proof surface:

- `hy-meta/examples/factorial.hy`
- `hy-meta/examples/kernel_loop.hy`
- `hy-meta/examples/kernel_features.hy`

Current feature value:

```text
kernel_features: 449.0
```

Keep this value stable unless intentionally adding a value-contributing feature
to the final sum. For coverage-only additions, prefer `assert` in
`kernel_features.hy` so the final value remains stable.

## Validation Commands

Use these before every commit:

```sh
cd ~/pnix/hy-1.3.0

/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py kernel-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py kernel-import-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py prime-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage3-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py mirror-check
/tmp/pnix-hy-py311-venv/bin/python hy-meta/bootstrap.py stage7-check
/tmp/pnix-hy-py311-venv/bin/python -m py_compile \
  hy-meta/bootstrap.py hy-meta/smoke_test.py hy-meta/native_subset_test.py \
  stage1/compiler.py
/tmp/pnix-hy-py311-venv/bin/python hy-meta/native_subset_test.py
/tmp/pnix-hy-py311-venv/bin/python hy-meta/smoke_test.py
```

Clean generated caches from the repo root:

```sh
cd ~/pnix
find hy-1.3.0 -type d -name __pycache__ -prune -exec rm -rf {} +
```

## Commit / Push Notes

Only stage hy-meta files explicitly. Do not use `git add -A`.

Typical safe add:

```sh
git add \
  hy-1.3.0/stage2/kernel.hy \
  hy-1.3.0/hy-meta/bootstrap.py \
  hy-1.3.0/hy-meta/smoke_test.py \
  hy-1.3.0/hy-meta/native_subset_test.py \
  hy-1.3.0/hy-meta/examples/kernel_features.hy \
  hy-1.3.0/hy-meta/examples/kernel_import_probe.hy \
  hy-1.3.0/hy-meta/examples/kernel_import_consumer.hy \
  hy-1.3.0/hy-meta/examples/kernel_import_pkg/__init__.hy \
  hy-1.3.0/hy-meta/examples/kernel_import_pkg/child.hy \
  hy-1.3.0/hy-meta/examples/kernel_import_pkg/sibling.hy \
  hy-1.3.0/hy-meta/examples/kernel_import_broken.hy \
  hy-1.3.0/hy-meta/README.md \
  hy-1.3.0/hy-meta/todo.md
```

There are unrelated dirty files in the larger `~/pnix` worktree at the
time this note was written:

```text
M  hangul-codec.md
M  scripts/check-math-alive.sh
M  stdlib/lib/nl/korean-nl-mirror.px
M  stdlib/lib/nl/korean-taxonomy-lift.px
?? stdlib/lib/nl/korean-copula-vowel-test.px
```

Do not revert or include those unless that is the explicit task.

Push hook can be slow. Sometimes `git push origin main` waits 1-2 minutes after
the hook output. Wait for completion.

If `check-singleton-patterns.py` fails while reading a missing temporary
`probe*.px` file, rerun:

```sh
cd ~/pnix
python3 scripts/check-singleton-patterns.py \
  --root . \
  --registry scripts/singleton-constitution-patterns.json
```

If that passes, retry the push. Do not force push.

## What Is Not Done

This is not yet a full Hy 1.3.0-compatible self-hosting compiler.

Current practical estimate (updated 2026-06-29):

- stage3 / hy-meta proof lane: green through stage16 + version-AST + front-end
  decision.
- direct-kernel parity over the parity-ledger corpus (owned compiler corpus +
  every `tests/native_tests/*.hy`): **100% direct, zero fallback** (was estimated
  25-35% before the parity ledger turned it into measured data). This is the
  parity-ledger-driven bar; it is a broad-but-finite sample, not a proof that
  every conceivable Hy form is owned.

The earlier estimate predates the parity ledger. The remaining gap to a literally
complete Hy 1.3.0 compiler is bounded by what the native test corpus does not
exercise, plus the explicitly-ceded front-end (reader/mangle) and gated
version-AST nodes (3.12 type params, 3.14 t-strings).

The gap was hard because Hy looks Lisp-like, but the target semantics are Python
AST semantics. Python has a hard statement/expression split, version-specific
AST nodes, scope rules, annotation behavior, async forms, exception groups, and
pattern matching details. This makes a meta-circular compiler much less direct
than Clojure-like code-as-data work.

Known remaining backlog:

The list below is the working backlog. Treat an item as done only after it has
focused proof in `hy-meta/native_subset_test.py`, `kernel_features.hy`, or a
bootstrap command, and after `hy-meta/smoke_test.py` passes.

### P0: Self-hosting boundary

- [x] Reduce reliance on Python `hy.compiler` in `stage2/compiler.hy`.
- [x] Decide the real stage boundary for the final system:
  - [x] current stage7 milestone path: stage2 compiler, written in Hy, loads
        the direct kernel;
  - [x] stronger path deferred: the direct kernel compiling enough Hy to load
        more of the compiler path itself belongs to the full compiler parity
        lane, not the current stage7 proof boundary.
- [x] Add an explicit proof that generated Python and AST output are stable
      after every relevant compiler/kernel transition, not just the current
      factorial/loop/features examples.
- [x] Keep temp-name generation deterministic across repeated stages, nested
      helper lowering, comprehensions, `try`, `with`, `let`, macros, and
      import-loaded modules.
- [x] Keep module names, module globals, `_hy_macros`, `_hy_reader_macros`, and
      `sys.modules` entries isolated across repeated stage loads and import
      hook loads.
- [x] Track which behavior is delegated to the upstream Hy reader/runtime and
      which behavior is truly implemented by `stage2/kernel.hy`.

### P0: Full compiler parity surface

- [x] Keep full `hy/compiler.py` parity in the post-stage7 full-parity lane,
      not as a blocker for the current direct-kernel stage7 proof.
- [x] Keep full `hy/core/result_macros.py` parity in the post-stage7
      full-parity lane; the current kernel owns the reimplemented forms proved
      in `hy-meta/native_subset_test.py`.
- [x] Port the upstream `tests/compilers/test_ast.py` compile-success and
      compile-failure cases into focused kernel checks.
  - [x] Port focused compile-only success/failure coverage for `if`, `do`,
        `raise`, `try`, `except*`, `assert`, declarations, `defclass`, `fn`,
        import mangling, `get`, `cut`, `while`, `for`, nullary
        `break`/`continue`, f-string conversions, basic AST shape, and dotted
        import/require failures.
  - [x] Record the remaining `py`/`pys`, module prelude, pragma, bad
        exception, source-position, and deeper syntax/error-message cases as
        full upstream compiler surface outside the current stage7 direct-kernel
        proof.
- [x] Port the `tests/compilers/test_compiler.py` behavior that belongs to the
      current kernel lane into focused checks; leave broad upstream compiler
      behavior in the full-parity lane.
- [x] Match Hy's user-facing exception classes and messages for supported
      kernel compile errors; full `hy.errors` rendering parity stays in the
      post-stage7 compiler/tooling lane.
- [x] Preserve source filename, line, column, and end-position metadata where
      the current kernel changes traceback/import/inspection behavior; exact
      upstream source rendering remains outside the stage7 completion boundary.
- [x] Audit Python-version-gated behavior. The original stage7 lane was pinned
      to Python 3.11; the active proof targets are now Python 3.11 and
      Homebrew Python 3.14 only. Python 3.12/3.13 remain out of scope.

Python-version audit decision for the current stage7 lane:

- Python 3.12+ `:tp` / type-parameter AST behavior, `deftype`, `TypeVar`,
  `TypeVarTuple`, and `ParamSpec` parity stay gated; 3.12/3.13 support is not
  part of the active proof lane.
- Python 3.13 source-inspection and traceback-rendering differences stay in the
  `hy_inspect` / CLI tooling lanes, not the current direct-kernel proof.
- Python 3.14 t-strings, `TemplateStr`, `Interpolation`, and template-string
  `hy.repr` behavior are now an active policy item: either implement direct
  lowering or keep them explicitly unsupported with focused tests.
- Python 3.12+ importer/REPL API differences stay outside the scoped
  filesystem-root `KernelHyFinder` proof unless CLI/tooling ownership changes.

### P0: Macro and require system

- [x] Implement native `require` semantics in the kernel path:
  - [x] module require;
  - [x] selected macro require;
  - [x] `*` require;
  - [x] `:as` aliases;
  - [x] per-name `:as` aliases;
  - [x] relative require;
  - [x] recursive require;
  - [x] exported macro filtering through `_hy_export_macros`;
  - [x] no accidental pollution of core macro tables.
- [x] Implement one-shot macro namespaces such as `hy.R...` forms where they
      matter for kernel evaluation.
- [x] Implement or deliberately delegate `hy.I...` module import shorthand
      behavior, including mangled dotted paths.
- [x] Support local macro namespaces and local macro shadowing the way native Hy
      resolves them.
- [x] Support first-class macro behavior covered by
      `tests/native_tests/macros_first_class.hy`.
  - [x] Support global/core `get-macro` lookup, docstrings, deletion through
        `eval-when-compile`, and core macro shadow/delete behavior.
  - [x] Support compile-time `_hy_macros` mutation through top-level
        `eval-and-compile`.
  - [x] Support local `defmacro`, local `get-macro`, local `require`, and
        nested local macro shadowing inside function bodies.
- [x] Expand `defmacro` parity:
  - [x] invalid macro name rejection;
  - [x] macro keyword arguments;
  - [x] macro optional/default arguments;
  - [x] macro unpacking arguments;
  - [x] macro docstring/autoboxing behavior;
  - [x] macro redefinition warning behavior;
  - [x] macro error wrapping with useful source context.
- [x] Expand `hy.macroexpand` and `hy.macroexpand-1` parity:
  - [x] explicit `:module`;
  - [x] explicit `:macros`;
  - [x] non-expression passthrough;
  - [x] result-macro passthrough behavior;
  - [x] named-import macro expansion.
- [x] Port or explicitly delegate the upstream macro test files for the
      current direct-kernel lane:
  - [x] `tests/native_tests/macros.hy`;
    - [x] Port macro call in lambda, stararg native macro,
          constant-returning macros, compile-time helper functions,
          compile/load phase checks, `eval-and-compile` initialization, macro
          gensym uniqueness, lambda-list/docstring/error wrapping, and
          redefinition warnings.
  - [x] `tests/native_tests/macros_local.hy`;
  - [x] `tests/native_tests/macros_first_class.hy`;
  - [x] `tests/macros/test_macro_processor.py`;
  - [x] macro-related cases in `tests/native_tests/hy_misc.hy`;
  - [x] macro-related import/bin/importer tests.

Macro policy decision: macro expansion, local macro shadowing, first-class
macro lookup, require-time macro loading, phase behavior, macroexpand helpers,
and failure cleanup are covered by focused direct-kernel checks. Broad upstream
macro processor internals and full importer/CLI macro tooling stay delegated to
upstream Hy or the post-stage7 full-parity lane.

### P0: Reader macro and reader behavior

- [x] Implement or explicitly delegate `defreader`.
- [x] Implement `require-reader` and reader macro import/export behavior.
- [x] Keep reader macro tables isolated across stages, modules, import-hook
      loads, `hy.eval`, and failed imports.
  - [x] Prove stage/module/fresh-eval reader macro tables do not leak in the
        current stage7/native subset checks.
  - [x] Add failed-import and import-hook reader table cleanup proof.
- [x] Add proof that reader macros introduced earlier in a stream can affect
      later forms in the same stream where native Hy expects that behavior.
- [x] Port `tests/native_tests/reader_macros.hy`.
- [x] Port reader-macro cases from `tests/native_tests/mangling.hy`.
- [x] Port relevant `tests/test_reader.py` behavior for:
  - [x] comments and `#_` discard;
  - [x] bracket strings;
  - [x] string prefixes and escapes;
  - [x] numeric literal edge cases;
  - [x] dotted identifiers;
  - [x] column/line counting;
  - [x] shebang handling;
  - [x] reader error formatting.
- [x] Decide whether `hy-meta` will keep using upstream Hy's reader long-term
      or grow its own reader for the meta-circular path.

Reader policy decision: through the current stage7 meta-circular lane,
`hy-meta` deliberately keeps `hy.reader` as a host boundary and proves reader
macro staging/isolation around that boundary. A Hy-written reader is a separate
post-stage7 project, not a blocker for the current compiler/interpreter path.

### P0: Import and module loading

- [x] Harden `kernel-import-check` beyond the current focused proof:
  - [x] reload/re-execution behavior;
  - [x] circular imports;
  - [x] shadowed basenames;
  - [x] zipimport or deliberately unsupported status;
  - [x] bytecode/autocompile behavior or deliberately unsupported status;
  - [x] import error reporting and frame filtering;
  - [x] `runpy` behavior;
  - [x] package `__main__` behavior;
  - [x] `__all__` / export-object interactions.
- [x] Port `tests/importer/test_importer.py` into kernel-import focused checks
      where applicable.
  - [x] Covered package basics, docstrings, failed-import cleanup, reload and
        re-execution, circular imports, shadowed basenames, runpy module
        execution, package `__main__`, filtered importlib frames, and
        `__all__`/star export behavior in `kernel-import-check`.
  - [x] Recorded zipimport and bytecode/autocompile as deliberately unsupported
        for the current scoped filesystem-root `KernelHyFinder`.
  - [x] Treated direct `HyLoader`, `spec_from_file_location`, and `run_path`
        tests as upstream importer surface area outside the current kernel
        import-hook proof lane.
- [x] Port import-related native tests from `tests/native_tests/import.hy`.
- [x] Port CLI/bin import and require cases from `tests/test_bin.py` where they
      are relevant to the bootstrap/import hook path.
  - [x] Covered `hy -m` style module execution with `runpy.run_module`.
  - [x] Covered package `__main__.hy` execution through `runpy`.
  - [x] Covered command/bin macro execution with module-local macros loaded
        through the kernel import hook.
  - [x] Covered circular self-`require` during import-time macro expansion.
  - [x] Covered require/import failure traceback filtering in the import hook.
  - [x] Kept bytecode/autocompile, direct filesystem script execution,
        stdin/`-c`/interactive modes, `hyc`, `hy2py`, startup files, and output
        buffering in the P2 CLI/tooling lane rather than the current scoped
        import hook.
- [x] Verify that failed import, failed require, failed reader macro load, and
      failed macro expansion never leave stale `sys.modules`, macro-table, or
      reader-table entries.

### P1: Functions, lambda lists, return, and yield

- [x] Finish upstream `tests/native_tests/functions.hy` parity for the current
      direct-kernel lane:
  - [x] `&symbol` function names and other mangling-heavy names;
  - [x] remaining optional/keyword-only error cases;
  - [x] required/default argument order failures;
  - [x] lambda-list parsing only where a lambda list is valid;
  - [x] `__name__` preservation for more mangled names;
  - [x] generator implicit final expression as `StopIteration.value`;
  - [x] yield in `try/finally`;
  - [x] midtree `yield` with final return values;
  - [x] yield inside `for`/`while` followed by final expression return.
- [x] Expand async function and async generator parity beyond current focused
      cases with named/anonymous coroutine and async-generator collection
      checks in `test_native_async_function_upstream_remaining_cases`.
- [x] Add negative syntax checks for `return`, `yield`, `yield :from`, async
      generator returns, and invalid lambda-list shapes.

### P1: Expression-result and helper-backed statement semantics

- [x] Audit every helper-backed expression form for outer-scope effects:
  - [x] `if`;
  - [x] `when`;
  - [x] `cond`;
  - [x] `do`;
  - [x] `and`;
  - [x] `or`;
  - [x] `let`;
  - [x] `for`;
  - [x] `while`;
  - [x] `try`;
  - [x] `with`;
  - [x] `match`;
  - [x] comprehensions;
  - [x] nested combinations of the above.
- [x] Verify expression-position statement forms return native values or `None`
      exactly where Hy does.
- [x] Verify non-final statement-position expression-valued forms run for
      effects and discard values exactly where Hy does.
- [x] Add deeper tests for nested pending statements inside call arguments,
      collection literals, f-strings, comparison operands, comprehension
      iterables, and pattern guards.
  - [x] Call argument pending side-effect ordering.
  - [x] Collection literal pending side-effect ordering.
  - [x] F-string pending side-effect ordering.
  - [x] Comparison operand pending side-effect ordering.
  - [x] Comprehension iterable pending side-effect ordering.
  - [x] Pattern guard pending side-effect ordering.

### P1: `let` parity

- [x] Port the rest of `tests/native_tests/let.hy` that belongs to the current
      direct-kernel lane.
- [x] Deepen proof for let scope boundaries with:
  - [x] comprehensions;
  - [x] quasiquote;
  - [x] exceptions;
  - [x] `with`;
  - [x] mutation;
  - [x] `break`;
  - [x] `continue`;
  - [x] `yield`;
  - [x] `return`;
  - [x] imports;
  - [x] nested functions/classes;
  - [x] dotted targets;
  - [x] function-argument evaluation counts;
  - [x] let-bound `nonlocal` and `global`;
  - [x] macro definitions inside `let`.
- [x] Recheck which let body bindings should leak and which should stay hidden
      against native Hy for every statement form.

### P1: `try`, `with`, loop, and control-flow parity

- [x] Port the rest of `tests/native_tests/try.hy`, especially:
  - [x] missing-parts behavior;
  - [x] multiple body expressions;
  - [x] clause ordering failures;
  - [x] `return` through `try`/`else` paths;
  - [x] exception variable scope capture;
  - [x] nonsyntactical except forms;
  - [x] nullary `raise` edge cases.
- [x] Port the rest of `tests/native_tests/with.hy`.
  - [x] Pending statements in context manager expressions.
  - [x] Later manager expressions can see earlier manager bindings.
  - [x] Statement-level async and mixed sync/async context managers.
- [x] Port all remaining `break_continue.hy`, `conditional.hy`, and
      `logic_short_circuit.hy` cases not already represented.
- [x] Add negative compile checks for invalid `while`, `for`, `with`, `break`,
      `continue`, `raise`, `assert`, `global`, and `nonlocal` shapes.

### P1: Pattern matching parity

- [x] Port the rest of `tests/native_tests/match.hy`.
- [x] Port `tests/native_tests/model_patterns.hy` or deliberately separate it
      as macro-pattern-system work.
- [x] Expand side-effect ordering tests for:
  - [x] subject evaluation;
  - [x] pattern value evaluation;
  - [x] guards;
  - [x] failed alternatives;
  - [x] nested `let`/`match` combinations.
- [x] Add more failure tests for invalid mapping keys, duplicate captures,
      invalid star/rest usage, invalid class patterns, and bad clause counts.

### P1: Operators, calls, attributes, and assignment targets

- [x] Port remaining `tests/native_tests/operators.hy` cases not yet in the
      focused subset.
- [x] Port remaining `tests/native_tests/dots.hy`, including macro/dot
      interactions and all invalid dot/unpacking cases.
- [x] Port remaining `tests/native_tests/setv.hy`, `setx.hy`, `del.hy`, and
      `unpack.hy`.
- [x] Add negative syntax checks for invalid targets, invalid unpacking shapes,
      duplicate assignment edge cases, and malformed augmented assignment.
- [x] Audit Python target evaluation order for nested attribute/subscript
      assignments, deletion, `setv :chain`, and augmented assignment.

### P1: Comprehension parity

- [x] Port the rest of `tests/native_tests/comprehensions.hy`.
- [x] Deepen proof for:
  - [x] multidimensional `for` break/continue;
  - [x] empty comprehensions;
  - [x] global/nonlocal behavior;
  - [x] async `for` with `else`;
  - [x] `:do`/`:setv` side effects mixed with pending expressions;
  - [x] generator send protocol when final `#*` unpacking is involved.

### P1: Classes, decorators, annotations, and type parameters

- [x] Port remaining `tests/native_tests/defclass.hy` and decorator edge cases.
- [x] Add negative `defclass` syntax checks from `tests/compilers/test_ast.py`.
- [x] Keep Python 3.12+ type parameter support (`:tp`) explicitly gated while
      Python 3.12/3.13 remain outside the active proof targets:
  - [x] `defn :tp`;
  - [x] `fn :tp`;
  - [x] `defclass :tp`;
  - [x] `deftype :tp`;
  - [x] `TypeVar`, `TypeVarTuple`, and `ParamSpec` behavior;
  - [x] bounds and constraints;
  - [x] invalid annotation/unpacking combinations.
- [x] Implement `deftype` or explicitly keep it gated/unsupported while
      type-parameter support remains outside the active proof targets.
- [x] Expand annotation coverage for future annotations, complex annotations,
      class annotations, function annotations, and runtime `get-type-hints`.

### P1: Strings, models, representation, and mangling

- [x] Port remaining `tests/native_tests/strings.hy`.
- [x] Keep `tests/native_tests/tstrings.hy` gated until the direct-kernel
      t-string policy is implemented or explicitly documented for Python 3.14.
- [x] Port remaining `tests/native_tests/hy_repr.hy`.
- [x] Port remaining `tests/native_tests/mangling.hy`, including Unicode and
      PEP 3131 behavior if the project accepts non-ASCII test fixtures.
- [x] Port `tests/test_models.py` behavior that matters to quoted model
      construction, `repr`, equality, metadata, and recursive model detection.
- [x] Verify `hy.repr`, `repr`/`eval` round trips, quoted model metadata, and
      delayed f-string evaluation through stage7.

### P1: `hy.eval`, introspection, and runtime helpers

- [x] Port the rest of `tests/native_tests/hy_eval.hy`:
  - [x] macros during eval;
  - [x] extra macros;
  - [x] explicit filenames;
  - [x] failure cleanup;
  - [x] globals/locals combinations not already covered.
- [x] Port `tests/native_tests/hy_misc.hy`:
  - [x] `hy.gensym` determinism and uniqueness;
  - [x] `hy.read`;
  - [x] `hy.read-many`;
  - [x] `hy.I`;
  - [x] `hy.R`;
  - [x] macroexpand helpers.
- [x] Port `tests/native_tests/hy_inspect.hy` for reader/source/doc
      inspection where applicable.
- [x] Verify `help`, `pydoc`, docstrings, and `inspect` output where the kernel
      changes function/class/module metadata.

### P2: REPL, CLI, and tooling surface

- [x] Decide whether the direct kernel is responsible for REPL behavior or only
      for file/eval/module compilation.
- [x] Do not port `tests/native_tests/repl.hy` for the current stage7 lane;
      REPL behavior remains outside direct-kernel ownership until that boundary
      changes.
- [x] Port or explicitly exclude CLI behavior from `tests/test_bin.py` for the
      current meta-circular path; `smoke_test.py` now covers the owned
      `hy-meta/bootstrap.py run`, `py`, `kernel-run`, and `kernel-py` commands:
  - [x] `hy` command execution remains outside direct-kernel ownership;
  - [x] stdin remains outside direct-kernel ownership;
  - [x] `-c` remains outside direct-kernel ownership;
  - [x] interactive command/file modes remain outside direct-kernel ownership;
  - [x] shebangs remain outside direct-kernel ownership;
  - [x] `hyc` remains outside direct-kernel ownership;
  - [x] `hy2py` remains outside direct-kernel ownership;
  - [x] traceback formatting is covered for the kernel import hook, and full
        CLI traceback rendering remains outside direct-kernel ownership;
  - [x] startup files remain outside direct-kernel ownership;
  - [x] output buffering remains outside direct-kernel ownership.
- [x] Add a clear unsupported list for CLI features intentionally outside
      hy-meta.

### P2: Proof infrastructure and documentation

- [x] Add a native-test import map that records which upstream test files and
      test functions are:
  - [x] ported;
  - [x] partially ported;
  - [x] skipped due to Python version;
  - [x] skipped because they belong to upstream Hy tooling rather than
        hy-meta;
  - [x] still failing.
- [x] Add commands for running selected focused parity groups without the full
      smoke cost.
- [x] Add a deterministic failure minimizer for kernel/native parity probes.
- [x] Add generated Python/source diff snapshots only if they stay stable and
      useful; avoid noisy churn.
- [x] Keep README and this TODO synchronized when adding stage commands,
      expected values, or new proof files.

## Completed Implementation Steps

The previous recommended order has been implemented for the local kernel proof
surface:

1. Added `fn` statement-body lowering through generated local functions.
2. Added synchronous comprehension `:do` and `:setv` lowering through helper
   generator functions.
3. Added match sequence `#*`, mapping `#**`, and class keyword patterns.
4. Added and expanded `hy-meta/native_subset_test.py`.
5. Moved the relevant for-family and match behavior into Hy forms in
   `stage2/kernel.hy`.
6. Added `mirror-check` for AST/Python/value mirror readiness.
7. Added `stage7-check` for repeated-stage and module-cache pollution checks.
8. Added `kernel-import-check` and a scoped import hook for loading `.hy`
   modules and packages through the Hy-written kernel path.
9. Added literal `#*`/`#**` unpacking, starred assignment targets, and `dfor`
   final `#**` mapping unpacking.
10. Added async comprehension `:do`/`:setv` helper lowering.
11. Added sync `try`/`with` expression value helper lowering.
12. Added async `with` expression value helper lowering.
13. Added `except*` expression value helper lowering.
14. Added async-aware `try` expression helper lowering.
15. Added annotated `let` binding lowering.
16. Added Hy-style import module name mangling, including dotted modules.
17. Added annotated tuple-pattern parameter destructuring.
18. Added native flat `match` expression syntax, `pattern :as name`, keyword
    patterns, dotted-value patterns, top-level match result values, and guard
    helper lowering for statement-like `do` guards.
19. Added literal-key validation for mapping patterns and a native-style
    rejection check for `{x 1}` mapping patterns.
20. Hardened `kernel-import-check` with a deliberately failing `.hy` module and
    a stale-cache check for failed imports.
21. Switched kernel `match` parsing to native flat clauses to remove ambiguity
    with sequence-pattern/body pairs.
22. Reworked `let` lowering so statement-body and match-containing bodies run
    inside generated helper functions instead of leaking pending AST outside
    the let scope.
23. Added sequential `let` binding/rebinding semantics so later binding values
    can see earlier let-bound names without clobbering outer bindings.
24. Added starred and nested starred `let` destructuring support, matching the
    native unpacking shape for `[head #* tail]` style bindings.
25. Added native-style `except` spec parsing, including type lists and
    name-first exception bindings, while preserving the earlier type-first
    kernel form.
26. Reworked synchronous `try` expression lowering to inline result-variable
    statements, preserving outer-scope assignments in handlers and `else`
    clauses.
27. Fixed ordered multi-pair `setv` lowering when later right-hand sides emit
    pending statements.
28. Reworked synchronous `with` expression lowering to inline result-variable
    statements, preserving outer-scope assignments in the body.
29. Reworked async `with` expression lowering to inline result-variable
    statements with `AsyncWith`, preserving outer-scope assignments inside
    async functions.
30. Reworked async `try` expression lowering to the same inline result-variable
    path as sync `try`, preserving outer-scope assignments in async `else`,
    `except`, and `except*` expression clauses.
31. Added `fn :async` anonymous async function lowering through generated
    `AsyncFunctionDef` statements, including multi-form bodies with `await`.
32. Fixed async generator body lowering for `defn :async` and `fn :async` by
    suppressing implicit value returns when the async body contains `yield`.
33. Added bare `import *` lowering, so `(import module *)` now emits
    `from module import *` instead of treating `*` as another module name.
34. Added relative `from` import lowering with `ImportFrom.level`, plus import
    hook proof coverage for package sibling imports.
35. Expanded dot-chain lowering so `(. obj attr [index] (method ...))` supports
    attribute, subscript, method-call, assignment, and deletion target shapes
    where valid.
36. Added `...` / Ellipsis constant lowering so it remains the singleton even if
    the name `Ellipsis` is rebound.
37. Added `(cut ...)` slice assignment and deletion target lowering.
38. Added statement-containing `do` expression lowering through pending
    result-variable statements, and fixed `if`/`when`/`cond` statement test
    pending placement for those expressions.
39. Added module docstring preservation by placing the leading string literal
    before the kernel's injected `import hy`, with import-hook proof coverage.
40. Added `(yield :from iterable)` lowering through `ast.YieldFrom`, including
    delegation exception propagation proof coverage, and wired all native subset
    checks into the smoke runner.
41. Lowered statement-containing `while` conditions to a `while True` shape so
    pending condition statements run every iteration, with correct `else`,
    `continue`, and condition-local `break` behavior.
42. Added expression-position `yield` lowering, including generator `send()`
    protocol coverage and `yield :from` return-value propagation.
43. Added final `#*` iterable unpacking for `lfor`, `sfor`, and `gfor`; `gfor`
    lowers to an inner loop rather than `yield from`, preserving generator
    `send()` protocol behavior.
44. Routed comprehension clauses with statement-producing iterable or `:if`
    expressions through helper loops so pending statements execute at the
    correct nesting level instead of leaking outside the comprehension.
45. Fixed multi-binding `for` iterable pending placement so inner iterable
    statements execute at the corresponding outer-loop nesting.
46. Added keyword-only tuple-pattern parameter destructuring by reusing the
    generated temporary-argument lowering already used for positional
    destructured parameters.
47. Added operator argument `#*` iterable unpacking for arithmetic/binop,
    boolean, and comparison operators, including native-style identities for
    `+`, `*`, `and`, and `or`.
48. Added native-style `setv :chain` chained assignment, including mixed
    ordinary/chain assignment groups, destructuring targets, and Python target
    evaluation ordering for subscript assignments.
49. Added expression-position `setv` lowering so assignments are emitted as
    pending statements and the expression result is `None`, including nested
    `setv` and `setv :chain` expression coverage.
50. Added expression-position lowering for `defn`, `defclass`, `for`, `assert`,
    and `pass` statement forms, emitting their statements as pending work and
    returning `None` for native `setv` result parity.
51. Added native-style `let` body binding leakage for newly introduced
    statement bindings (`setv`, `import`, `defn`, `defclass`, `for`, `with`,
    and `match`) while keeping let-bound names hidden.
52. Added native-style `defclass` shorthand without an explicit base vector
    and class keyword argument lowering, including `:metaclass` and custom
    `__init_subclass__` keyword arguments.
53. Added expression-position `del` and pending-aware `and`/`or` lowering so
    statement-producing operands such as `for`, `del`, and `setv` preserve
    native short-circuit behavior.
54. Added leading `__future__` import placement before the kernel's injected
    `import hy`, while still preserving module docstrings.
55. Added pending-aware `if`, `when`, and `cond` expression branch lowering so
    statement-like branch results such as `setv` execute only in the selected
    branch and still return the native expression value.
56. Added f-string lowering through `JoinedStr` / `FormattedValue`, including
    conversions, format specs, nested format spec expressions, debug syntax,
    statement-producing replacement expressions, and value-before-spec
    evaluation order.
57. Added quoted string/f-string model metadata preservation so bracketed
    strings keep their delimiter metadata and quoted f-strings remain delayed
    evaluable Hy models with component conversion/expression metadata intact.
58. Added native docstring edge-case proof coverage for module, function, and
    class docstrings, including the single-string function-body case that must
    return the string rather than become a docstring.
59. Expanded f-string native proof coverage with adjacent replacement fields,
    escaped braces, bracket f-strings, nested replacement fields with
    conversion/format specs, and debug `=` format-spec behavior.
60. Added quoted f-string `repr` / `eval` roundtrip proof coverage so component
    conversion metadata survives model stringification and restoration.
61. Added native keyword proof coverage for equality, ordering, pickling,
    callable lookup/default behavior, dict-literal keyword separation, and
    keyword argument name mangling.
62. Fixed native operator edge semantics: unary `+` now calls Python unary
    plus, unary `/` lowers to reciprocal division, `**` is right-associative,
    `|` has the native empty identity, and static/dynamic arity checks now
    distinguish unpacked operands from literal single-operand calls.
63. Added native proof coverage for break/continue loop behavior, dynamic class
    bases, class-body function no-leak behavior, class-body side effects, and
    synchronous `with` expression exception suppression.
64. Fixed `setx` scope behavior in helper-backed comprehensions by collecting
    named-expression bindings, using `global` at module level and
    `nonlocal` plus unexecuted outer bindings inside functions, preserving
    native updates and empty-iteration `UnboundLocalError` behavior.
65. Fixed native comparison arity semantics so unary `=`, `<`, `<=`, `>`, `>=`,
    and `is` evaluate their operand and return `True`, while `!=`, `is-not`,
    `in`, and `not-in` reject unary use, including runtime unary cases produced
    by `#*` iterable unpacking.
66. Added `chainc` mixed chained comparison lowering with native
    left-to-right operand evaluation, one-time middle operand evaluation, and
    short-circuiting for later operands.
67. Fixed native `get`/`cut` operator parity: `get` now supports nested
    multi-index reads and targets, and `cut` now follows Hy's native optional
    argument semantics for whole slices, prefix slices, expression use,
    assignment targets, and deletion targets.
68. Fixed multi-RHS augmented assignment parity for `**=` and `@=` by compiling
    their RHS values with the native exponentiation and matrix-multiply
    operators instead of generic multiplicative RHS grouping, and added broader
    augmented-assignment proof coverage.
69. Imported focused native decorator proof coverage for function/class
    decorator lists, stacked decorators, decorator/default/body evaluation
    ordering, and async decorated functions.
70. Fixed empty `(del)` native parity so it compiles as a no-op and returns
    `None` in expression position; added focused `do`/`del` native proof
    coverage.
71. Fixed native `nonlocal` promotion by collecting module and function binding
    names during kernel compilation, splitting each declaration into true
    enclosing-function `nonlocal` names and module-binding `global` names while
    preserving SyntaxError for missing bindings.
72. Fixed method shortcut calls with leading keyword arguments before the
    receiver and added native dotted shortcut/empty-dot proof coverage.
73. Fixed bare `#^` annotation statements wrapped as a one-form expression,
    including class-body variable annotations, and added focused
    `get-type-hints` proof coverage for class and function annotations.
74. Added explicit binding-target validation for `None`, `True`, and `False`
    across assignment, deletion, `setx`, parameters, `defn`, `defclass`,
    loop/with/comprehension targets, and exception bindings, replacing AST
    `ValueError` leaks or accidental successful definitions with SyntaxError.
75. Fixed dotted statement-special roots such as `(. defn)` so expression
    position calls preserve statement lowering instead of compiling as an
    ordinary undefined name call.
76. Fixed sequence `unquote-splice` falsey handling so `~@0`, `~@False`, and
    `~@None` splice as empty sequences while preserving single evaluation for
    truthy splice expressions.
77. Fixed nested quasiquote depth handling for focused quote cases so inner
    `unquote`/`unquote-splice` forms are preserved or evaluated at the correct
    depth instead of leaking as runtime `unquote` calls.
78. Fixed `eval-source` module context for default `(hy.eval model)` calls by
    temporarily registering the evaluation module in `sys.modules` and restoring
    the previous cache state, which lets repeated triple quasiquote evaluation
    work without leaving stale module cache entries.
79. Fixed list, tuple, set, and dict literal evaluation order when nested
    expressions emit pending statements, including iterable and mapping
    unpacking forms. Literal lowering now evaluates entries sequentially through
    a temporary container instead of draining later statements before earlier
    element expressions run.
80. Fixed general call argument lowering for native Hy ordering: keyword
    arguments can appear before later positional arguments, and pending
    positional, keyword, `#*`, and `#**` argument expressions are evaluated in
    source order while preserving function-position evaluation before arguments.
81. Added comparison special-form aliases `not_in` and `is_not` so unmangled
    spellings behave like `not-in` and `is-not`, including when local bindings
    shadow the hyphenated names.
82. Fixed top-level and statement-position expression-valued forms so final
    `(try ...)`, `(with ...)`, and `(match ...)` preserve Hy expression values
    instead of being forced through statement lowering; non-final `(try ...)`
    forms without handlers now execute normally with their value discarded.
83. Extended expression-valued statement handling to final `defn` and `fn` body
    forms so functions ending in `(try ...)` or `(with ...)` return the native
    expression value instead of `None`.
84. Added native `bnot` bitwise-not operator lowering as an alias of the
    existing invert operation.
85. Added compile-time `require` macro loading for module-prefix, selected
    names, `*`, `:as`, per-name aliases, exported macro filtering, no-macro
    package submodule recursion, and builtins core-macro aliases such as
    `builtins.defn`.
86. Added one-shot `hy.R...` macro expansion for external module macros,
    including slash-to-dot module names, Unicode macro names, no-scope-leak
    behavior, and `HyRequireError` failures for missing modules/macros.
87. Added focused `hy.I...` importer proof through the kernel path, covering
    attribute and callable import forms, slash-to-dot module paths, macro
    expansion use, and no leakage into local/global names.
88. Added recursive `require *` proof for macros required by required modules,
    plus no-leak checks showing compile-time `require` does not alter
    `builtins._hy_macros` or carry required macro names into later
    `eval-source` calls.
89. Added relative `require` support by passing module/package context into
    kernel compilation and import-hook module loading, covering sibling,
    parent-package, and package-submodule alias require forms.

Next useful work is broader parity, not another small proof-lane step:

- Continue importing Hy native test groups into focused subsets.
- Harden the import hook beyond the current focused module/package import proof.
- Continue reducing reliance on Python `hy.compiler` in `stage2/compiler.hy`.

## Post-stage7: Compatibility-First Hy Ownership

The stage7 proof lane is green. The project is not trying to become a
compatibility-breaking upstream Hy replacement. These backlog items grow the
owned compiler/interpreter surface while keeping upstream Hy and Python module
compatibility as hard constraints.

- [x] Expose the final stage7-loaded kernel as user-facing compiler/interpreter
      commands, not only as an internal mirror proof:
  - [x] `stage7-kernel-run`;
  - [x] `stage7-kernel-py`;
  - [x] smoke coverage for both commands.
- [x] Replace the broad `hy.compiler.hy_compile` dependency in
      `stage2/compiler.hy` with the direct Hy-written kernel for source that is
      inside the supported kernel subset.
- [x] Grow `stage2/kernel.hy` until it can compile enough Hy to load
      `stage2/compiler.hy` itself without upstream `hy.compiler`.
- [x] Add `direct-kernel-bridge-check` coverage proving `stage2/compiler.hy`
      direct-compiles supported expressions and reloads `stage2/compiler.hy`
      itself with zero fallback.
- [x] Decide and implement the next reader boundary:
  - [x] keep upstream `hy.reader` as an explicit permanent host service;
  - [x] add `reader-boundary-check` coverage proving the stage7-loaded kernel
        uses upstream `hy.reader.read_many`, keeps stage reader macro tables
        distinct, and does not leak stream-local reader macros into fresh evals.
- [x] Close the current stage7 direct-kernel-owned `hy/compiler.py` and
      `hy/core/result_macros.py` parity lane, including the owned
      `tests/compilers/test_ast.py`, `tests/compilers/test_compiler.py`, and
      `tests/test_positions.py` surface.
  - [x] Add focused post-stage7 parity coverage for direct-kernel-owned
        `tests/compilers/test_ast.py` gaps: method-shortcut `#*` receiver
        rejection, dot-chain `#*`/`#**` part rejection, placeholder
        special-form rejection, unknown pragma rejection, macro-tag `try`, and
        `__future__` import ordering.
  - [x] Implement and cover direct-kernel `py`/`pys` inline Python lowering
        for the upstream `tests/compilers/test_ast.py` expression/statement
        passthrough cases.
  - [x] Implement and cover direct-kernel `pragma :hy` minimum-version
        validation, plus malformed/unknown pragma rejection.
  - [x] Add a direct-kernel module prelude switch matching the upstream
        `import_stdlib` boundary for focused compile-to-AST use.
  - [x] Make `pragma :bracketed-templates` reader-timed for the direct-kernel
        lazy `read-many` lane by mutating the form's reader before the next
        form is parsed.
  - [x] Preserve source locations for direct-kernel top-level/generated
        statement roots, including macro-expanded forms matching the focused
        upstream `tests/test_positions.py` behavior.
  - [x] Lock representative direct-kernel compile-error messages for supported
        syntax boundaries; full upstream `HyLanguageError.msg` wording remains
        outside the direct-kernel lane.
  - [x] Port remaining direct-kernel-owned `tests/compilers/test_compiler.py`
        internals: bare-name branch preservation and generator final return
        AST shape.
  - [x] Document the broader upstream compiler/result-macro surface as outside
        the current direct-kernel lane: exact rendered-source formatting and
        private upstream compiler internals are not active stage7 TODO items
        unless they are promoted into direct-kernel ownership with focused
        tests.
- [x] Move full CLI/REPL/tooling from "outside direct-kernel ownership" to
      implemented or explicitly product-scoped commands:
  - [x] stdin streaming through the shared `read_input` path for scoped
        bootstrap commands;
  - [x] `-c` for `run`, `py`, `hy2py`, `kernel-run`, `kernel-py`,
        `stage7-kernel-run`, and `stage7-kernel-py`;
  - [x] interactive/REPL behavior as a scoped stage2 line REPL with shared
        module state and optional prompt/flush flags;
  - [x] shebang execution for file/stdin input through the scoped bootstrap CLI;
  - [x] `hyc` as an explicit product-scoped stage2 `.pyc` writer with
        bytecode import coverage;
  - [x] `hy2py` as an explicit product-scoped stage2 command;
  - [x] startup files and output flushing for scoped `run` and `repl`.
- [x] Keep Python version gates precise while adding Homebrew latest Python
      3.14 as the only non-3.11 proof target:
      Status: Python 3.12/3.13 remain deferred by user request; Python 3.14
      stage7 and full smoke now pass on Homebrew 3.14.6.
  - [x] Python 3.12 type parameters remain gated/deferred;
  - [x] Python 3.13 inspection/traceback differences remain gated/deferred;
  - [x] Python 3.14 t-strings/`TemplateStr` policy is explicitly documented
        with focused tests: direct-kernel lowering is gated for now.

## hy-meta vs clj-meta Judgment

For the stated goal:

```text
meta-circular mirror operations
-> solve AI-related reflection/generation problem
-> solve hangul-codec full understanding/generation
-> mix with each language ecosystem for maximum productivity
```

`clj-meta` is the higher-leverage primary path.

Reasoning:

- Clojure is already much closer to the mirror problem:
  - code is data
  - persistent data structures are native
  - macro expansion is a first-class workflow
  - REPL-driven development is mature
  - the host ecosystem is stable
- Mirror operations want a canonical, inspectable, transformable form. Clojure
  gives that with less impedance than Python AST.
- Hangul-codec understanding/generation will likely benefit from a stable
  symbolic core more than from Python's statement-heavy AST model.
- If `~/pnix-clj` is already prepared, it should become the main mirror engine.

`hy-meta` is still useful, but as a bridge/backend rather than the first
canonical mirror core.

Good uses for `hy-meta`:

- Python ecosystem access:
  - AI/ML libraries
  - notebooks
  - Python packaging
  - Python AST/source generation
- A Python-facing projection of the mirror IR.
- A way to make mirror-produced artifacts run in Python.
- A testbed for understanding how hostile/non-homoiconic hosts affect the
  meta-circular design.

Recommended strategy:

```text
clj-meta = canonical mirror / symbolic core / hangul-codec reasoning engine
hy-meta  = Python ecosystem bridge and generated-code backend
pnix     = substrate / constitution / proof gates / language-neutral contracts
```

Do not spend the next block trying to make hy-meta a complete Hy compiler unless
Python ecosystem integration becomes the bottleneck. The better payoff is:

1. Advance `clj-meta` to mirror operations first.
2. Define the mirror IR and hangul-codec semantics there.
3. Keep hy-meta green and growing, but aim it at Python projection and AST
   generation.
4. Later, use clj-meta's canonical mirror forms to generate/test Hy/Python
   forms through hy-meta.

In short:

```text
Build the mirror brain in clj-meta.
Use hy-meta as the Python hand.
Keep pnix as the law/proof substrate.
```
