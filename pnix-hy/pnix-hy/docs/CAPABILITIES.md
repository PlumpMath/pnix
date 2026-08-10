# CAPABILITIES — 생성물 (하드편집 금지 / GENERATED — do not hand-edit)

`pnix-hy-project --capabilities`(=`pnix_hy.capabilities.render_capabilities_md`)로
코드에서 파생됩니다. 진실의 원천 = 코드. 중복개발 전에 여기서 `grep` 하세요.
재생성: `pnix-hy-project --capabilities > docs/CAPABILITIES.md` (proposal 0011).
핵심 basic 예: [[safe_eval]] · [[explain_pnix]].
명시적 proof/service API: `pnix_hy.proof.check_action` · `pnix_hy.deploy.deployment_info` (basic top-level export 아님).

counts: symbols=47 · reports=74 · proposals=31

## Public API (`pnix_hy.__all__`)

| name | kind | lane | module | summary |
|---|---|---|---|---|
| `CapabilityHandle` | class | interop (boundary) | `pnix_hy.interop` | 0020/I1 (SES-style): a RUNTIME-revocable capability grant. Pass the handle inside |
| `InteropError` | class | interop (boundary) | `pnix_hy.interop` | A host-facing interop error. `wrap_pnix_callable` raises this instead of leaking a raw |
| `PnixCatchableError` | class | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` | Explicit throw/assert failure caught by Nix builtins.tryEval. |
| `PnixError` | class | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` | Raised for pnix parse/eval failures with non-message classification. |
| `__version__` | value | pnix-hy | `pnix_hy` | package version (0.1.0) |
| `apply_effect_request` | function | interop (boundary) | `pnix_hy.interop` | Compatibility projection of the nominal effect adapter result. |
| `apply_host_method` | function | interop (boundary) | `pnix_hy.interop` | Invoke a host object's public method across the interop boundary. |
| `call_host` | function | interop (boundary) | `pnix_hy.interop` | Invoke a HOST callable from the pnix side, capability-gated. `host_callable` may be a |
| `call_host_method` | function | interop (boundary) | `pnix_hy.interop` | Invoke a host object's public method across the interop boundary. |
| `declare_opaque_invariants` | function | interop (boundary) | `pnix_hy.interop` | 0020/I6 (Trustworthy Proxies): declare attributes whose values must NEVER change for the |
| `eval_ast` | function | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` |  |
| `eval_from_ast` | function | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` |  |
| `eval_normalized_source` | function | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` |  |
| `eval_source` | function | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` |  |
| `eval_source_raw` | function | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` |  |
| `from_host` | function | interop (boundary) | `pnix_hy.interop` | Python host value -> pnix-usable representation, with an InteropRecord. Pure data |
| `grant_capability` | function | interop (boundary) | `pnix_hy.interop` | 0020/I1: grant a revocable capability handle for the given effect classes. |
| `harden_opaque` | function | interop (boundary) | `pnix_hy.interop` | 0020/I5 (SES harden): freeze-witness the ref's public method surface. Every subsequent |
| `host_callable_arity` | function | interop (boundary) | `pnix_hy.interop` | B3: project a host callable's signature into the pnix `functionArgs` shape |
| `host_callable_to_pnix` | function | interop (boundary) | `pnix_hy.interop` | B1: wrap a HOST callable as a pnix `NativeFunc` so pnix SOURCE can apply it as a builtin. |
| `host_module_to_pnix` | function | interop (boundary) | `pnix_hy.interop` | Map a host module/namespace to a pnix attrset: public attributes become pure pnix |
| `inspect_opaque` | function | interop (boundary) | `pnix_hy.interop` | Inspect an opaque host object without exposing the object itself. |
| `install_pnix_import_hook` | function | interop (boundary) | `pnix_hy.interop` | Install hy-meta's SR4 import hook with pnix-hy's `.px` runtime semantics. |
| `interop` | module | pnix-hy | `pnix_hy.proof` | pnix_hy.interop -- the explicit Hy/Python <-> pnix interop boundary (SEP IB1-IB2). |
| `interop_context` | function | interop (boundary) | `pnix_hy.interop` | 0020/I3: a lifecycle scope for opaque refs -- everything created inside is released on |
| `interop_error_of` | function | interop (boundary) | `pnix_hy.interop` | The `{kind,type,message}` of a cross-boundary error value, or None. |
| `is_interop_error` | function | interop (boundary) | `pnix_hy.interop` | True if `result` is a cross-boundary error value from call_host / call_host_method (D1). |
| `lend_opaque` | function | interop (boundary) | `pnix_hy.interop` | 0016 (I2): a call-scoped BORROW of an opaque ref (Canonical-ABI own/borrow). While at |
| `load_meta_api` | function | pnix-hy | `pnix_hy` | Load the basic meta-circular compiler/evaluator facade. |
| `load_proof_api` | function | pnix-hy | `pnix_hy` | Explicitly load service/proof verification APIs. |
| `make_opaque_ref` | function | interop (boundary) | `pnix_hy.interop` |  |
| `numeric_fits` | function | interop (boundary) | `pnix_hy.interop` | 0015 (I7, GraalVM fitsIn*): PREDICATE a numeric boundary conversion before it happens. |
| `opaque_allowed_methods` | function | interop (boundary) | `pnix_hy.interop` | List public callable methods for explicit method-level interop. |
| `opaque_call_method` | function | interop (boundary) | `pnix_hy.interop` | Invoke a host object's public method across the interop boundary. |
| `opaque_lifecycle` | function | interop (boundary) | `pnix_hy.interop` | D2: a snapshot of the pnix fallback opaque-ref lifecycle. `live` = objects currently held |
| `opaque_ref_id` | function | interop (boundary) | `pnix_hy.interop` |  |
| `parse` | function | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` |  |
| `pnix_runtime` | module | pnix-hy | `pnix_hy.proof` | A small pnix parser/evaluator used by the pnix-hy mirror lane. |
| `release_opaque` | function | interop (boundary) | `pnix_hy.interop` |  |
| `roundtrip_host_value` | function | interop (boundary) | `pnix_hy.interop` | A1: cross a host value host->pnix->host and report fidelity in ONE place. |
| `run_px` | function | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` | Entry point A for a `.px` file: read it and run it at host speed. |
| `run_px_source` | function | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` | Entry point A: compile pnix source and execute on host CPython. |
| `run_px_source_raw` | function | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` | Compile and execute source. Imports use raw results to preserve functions. |
| `runtime_context` | function | pnix-hy (runtime, sacred) | `pnix_hy.pnix_runtime` |  |
| `to_host` | function | interop (boundary) | `pnix_hy.interop` | pnix runtime value -> Python host value, with an InteropRecord. Data is lossless; |
| `to_host_eval` | function | interop (boundary) | `pnix_hy.interop` | Evaluate a pnix source fragment, then convert the result to the host. |
| `try_call_host` | function | interop (boundary) | `pnix_hy.interop` | D1: a `tryEval`-shaped wrapper over `call_host` -- returns `{"success": True, "value": v}` |

## Self-check reports (`--check`)

| report | module | symbol |
|---|---|---|
| `action` | `pnix_hy.cli` | `_action_report` |
| `alignment` | `pnix_hy.pnix_mirror` | `alignment_report` |
| `alignment_tree` | `pnix_hy.pnix_mirror` | `alignment_tree_report` |
| `cached_eval` | `pnix_hy.pnix_mirror` | `cached_eval_report` |
| `check_cache` | `pnix_hy.cli` | `_check_cache_report` |
| `classify_drift` | `pnix_hy.pnix_mirror` | `classify_drift_report` |
| `cogen` | `pnix_hy.cli` | `_cogen_report` |
| `compartment` | `pnix_hy.cli` | `_compartment_report` |
| `compiled_differential` | `pnix_hy.cli` | `_compiled_differential_report` |
| `compiled_runtime` | `pnix_hy.cli` | `_compiled_runtime_report` |
| `compiler_emit_shape` | `pnix_hy.pnix_mirror` | `compiler_emit_shape_report` |
| `correspondence` | `pnix_hy.pnix_mirror` | `correspondence_report` |
| `correspondence_abi` | `pnix_hy.pnix_mirror` | `correspondence_abi_report` |
| `diagnose` | `pnix_hy.pnix_mirror` | `diagnose_report` |
| `docs_drift` | `pnix_hy.cli` | `_docs_drift_report` |
| `eval_receipt` | `pnix_hy.pnix_mirror` | `eval_receipt_report` |
| `evaluate` | `pnix_hy.cli` | `_evaluate_report` |
| `execution_trace` | `pnix_hy_host_introspect` | `execution_trace_report` |
| `explain_pnix` | `pnix_hy.pnix_mirror` | `explain_pnix_report` |
| `fixture_report` | `pnix_hy.cli` | `_fixture_report` |
| `gate_check` | `pnix_hy.cli` | `_gate_report` |
| `hy_defmacro_projection` | `pnix_hy.hy_mirror` | `hy_defmacro_projection_report` |
| `hy_form_projection` | `pnix_hy.hy_mirror` | `hy_form_projection_report` |
| `hy_import_projection` | `pnix_hy.hy_mirror` | `hy_import_projection_report` |
| `hy_macro_step_trace` | `pnix_hy.hy_mirror` | `hy_macro_step_trace_report` |
| `hy_macroexpand_projection` | `pnix_hy.hy_mirror` | `hy_macroexpand_projection_report` |
| `hy_meta_circular_projection` | `pnix_hy.hy_mirror` | `hy_meta_circular_projection_report` |
| `hy_meta_host_api` | `pnix_hy.hy_mirror` | `hy_meta_host_api_report` |
| `hy_module_projection` | `pnix_hy.hy_mirror` | `hy_module_projection_report` |
| `hy_pnix_projection` | `pnix_hy.pnix_mirror` | `hy_pnix_projection_report` |
| `hy_projection_closure` | `pnix_hy.pnix_mirror` | `hy_projection_closure_report` |
| `hy_quasiquote_projection` | `pnix_hy.hy_mirror` | `hy_quasiquote_projection_report` |
| `hy_reader_embed_pnix` | `pnix_hy.pnix_mirror` | `hy_reader_embed_pnix_report` |
| `hy_reader_macro_projection` | `pnix_hy.hy_mirror` | `hy_reader_macro_projection_report` |
| `hy_to_pnix_value_roundtrip` | `pnix_hy.pnix_mirror` | `hy_to_pnix_value_roundtrip_report` |
| `hygiene` | `pnix_hy.cli` | `_hygiene_report` |
| `incremental_eval` | `pnix_hy.cli` | `_incremental_eval_report` |
| `interop` | `pnix_hy.cli` | `_interop_report` |
| `interop_error_contract` | `pnix_hy.cli` | `_interop_error_contract_report` |
| `interop_hardening` | `pnix_hy.cli` | `_interop_hardening_report` |
| `interop_host_bridge` | `pnix_hy.cli` | `_interop_host_bridge_report` |
| `interop_hy_macro_bridge` | `pnix_hy.pnix_mirror` | `hy_macro_quasiquote_over_pnix_report` |
| `interop_no_mirror` | `pnix_hy.cli` | `_interop_no_mirror_report` |
| `interop_opaque_lifecycle` | `pnix_hy.cli` | `_interop_opaque_lifecycle_report` |
| `interop_roundtrip` | `pnix_hy.cli` | `_interop_roundtrip_report` |
| `ir` | `pnix_hy.cli` | `_ir_report` |
| `ir_diff` | `pnix_hy.cli` | `_ir_diff_report` |
| `jones_optimality` | `pnix_hy.cli` | `_jones_optimality_report` |
| `meta_circular_tower` | `pnix_hy.pnix_mirror` | `meta_circular_tower_report` |
| `mirror_run` | `pnix_hy.cli` | `_mirror_run_report` |
| `pe_annotations` | `pnix_hy.cli` | `_pe_annotations_report` |
| `pe_size` | `pnix_hy.cli` | `_pe_size_report` |
| `performance_report` | `pnix_hy.pnix_mirror` | `performance_report_check` |
| `phase_separation` | `pnix_hy.cli` | `_phase_separation_report` |
| `pnix_evaluation_trace` | `pnix_hy.pnix_mirror` | `pnix_evaluation_trace_report` |
| `pnix_form_projection` | `pnix_hy.pnix_mirror` | `pnix_form_projection_report` |
| `pnix_import_hook` | `pnix_hy.cli` | `_pnix_import_hook_report` |
| `pnix_meta_circular_projection` | `pnix_hy.pnix_mirror` | `pnix_meta_circular_projection_report` |
| `pnix_meta_conformance` | `pnix_hy.cli` | `_pnix_meta_conformance_report` |
| `pnix_projection_closure` | `pnix_hy.pnix_mirror` | `pnix_projection_closure_report` |
| `pnix_stage_ladder` | `pnix_hy.cli` | `_stage_ladder_report` |
| `pnix_to_hy_form` | `pnix_hy.pnix_mirror` | `pnix_to_hy_form_report` |
| `project_hy_module` | `pnix_hy.pnix_mirror` | `project_hy_module_report` |
| `projection_value_roundtrip` | `pnix_hy.pnix_mirror` | `projection_value_roundtrip_report` |
| `reify_hy` | `pnix_hy.pnix_mirror` | `reify_hy_report` |
| `reify_pnix` | `pnix_hy.pnix_mirror` | `reify_pnix_report` |
| `repl` | `pnix_hy.cli` | `_repl_report` |
| `roundtrip_status` | `pnix_hy.pnix_mirror` | `roundtrip_status_report` |
| `safe_eval` | `pnix_hy.pnix_mirror` | `safe_eval_report` |
| `specialize` | `pnix_hy.pnix_mirror` | `specialize_report` |
| `static_purity_check` | `pnix_hy.pnix_mirror` | `static_purity_check_report` |
| `synthesize_pnix_from_hy` | `pnix_hy.pnix_mirror` | `synthesize_pnix_from_hy_report` |
| `tower_ladder` | `pnix_hy.cli` | `_tower_ladder_report` |
| `typed_witness` | `pnix_hy.cli` | `_typed_witness_report` |

## Proposals (`docs/proposals/`)

| id | status | title | path |
|---|---|---|---|
| 0000 | 후보 | Proposal 후보 — Hy(Python) ↔ pnix 언어기능 interop | `docs/proposals/0000-interop-language-feature-candidates.md` |
| 0001 | ACCEPTED | 0001 — roundtrip-host-value + loss fidelity | `docs/proposals/0001-roundtrip-host-value-and-loss-fidelity.md` |
| 0002 | ACCEPTED | 0002 — host-callable-into-pnix-eval | `docs/proposals/0002-host-callable-into-pnix-eval.md` |
| 0003 | ACCEPTED | 0003 — hy-macro-quasiquote-over-pnix | `docs/proposals/0003-hy-macro-quasiquote-over-pnix.md` |
| 0004 | ACCEPTED | 0004 — interop diagnostics & invariants (C5 + C7 + C8) | `docs/proposals/0004-interop-diagnostics-and-invariants.md` |
| 0005 | ACCEPTED | 0005 — Hy reader macro embeds pnix at read-time (C4) | `docs/proposals/0005-hy-reader-macro-embeds-pnix.md` |
| 0006 | ACCEPTED | 0006 — cross-boundary error contract (D1) + interop role matrix (D4) | `docs/proposals/0006-interop-error-contract-and-role-matrix.md` |
| 0007 | ACCEPTED | 0007 — opaque-ref lifecycle (D2 in-scope) + versioned correspondence ABI (D3 in-scope) | `docs/proposals/0007-opaque-lifecycle-and-correspondence-abi.md` |
| 0008 | SHIPPED | 0008 — meta-circular REPLs (5 modes, thin front-ends) | `docs/proposals/0008-meta-circular-repls.md` |
| 0009 | ACCEPTED | 0009 — pnix-hy as a pnix semantic / action VM (thin action layer) | `docs/proposals/0009-pnix-semantic-action-vm.md` |
| 0010 | SHIPPED | 0010 — module distribution (installable) without losing any existing feature | `docs/proposals/0010-module-distribution-tiers.md` |
| 0011 | SHIPPED | 0011 — docs-as-code: 생성 capability 인덱스 + doc↔code drift 게이트 | `docs/proposals/0011-docs-as-code-capability-index.md` |
| 0012 | SHIPPED | 0012 — CI로 강제되는 관리 게이트 (proposal 0011을 자동 강제) | `docs/proposals/0012-ci-enforced-management-gate.md` |
| 0013 | 후보 | 0013 — meta-circular / interop 확장 후보 카탈로그 (deep-research 2026-07-02) | `docs/proposals/0013-meta-circular-and-interop-candidates.md` |
| 0014 | SHIPPED | 0014 — Jones-optimality 수용 게이트 (0013 T1 승격) | `docs/proposals/0014-jones-optimality-gate.md` |
| 0015 | SHIPPED | 0015 — 경계 수치 무손실 술어 fitsIn* (0013 I7 승격) | `docs/proposals/0015-interop-numeric-losslessness.md` |
| 0016 | SHIPPED | 0016 — opaque-ref own/borrow 수명 규율 (0013 I2 승격) | `docs/proposals/0016-opaque-own-borrow-lifecycle.md` |
| 0017 | SHIPPED | 0017 — hygiene/symbol-capture self-check 리포트 (0013 P1+G4 승격) | `docs/proposals/0017-hygiene-self-check.md` |
| 0018 | SHIPPED | 0018 — pnix IR 구조 diff + 패스 파이프라인 물화 (0013 G1+P3 승격) | `docs/proposals/0018-ir-diff-and-pass-reification.md` |
| 0019 | SHIPPED | 0019 — 해시-키 검사 캐시 (0013 R2 승격) | `docs/proposals/0019-hash-keyed-check-cache.md` |
| 0020 | SHIPPED | 0020 — interop 하드닝 웨이브 (0013 I1+I3+I4+I5+I6 승격) | `docs/proposals/0020-interop-hardening.md` |
| 0021 | SHIPPED | 0021 — compartment식 게스트 격리 (0013 I8 승격) | `docs/proposals/0021-compartment-isolation.md` |
| 0022 | SHIPPED | 0022 — phase 산술 + 컴파일-실행 분리 게이트 (0013 P2+P4 승격) | `docs/proposals/0022-phase-separation-gates.md` |
| 0023 | SHIPPED | 0023 — 증분 평가: 정의-단위 내용주소 + realisation 조기중단 (0013 R1+R3+G3 승격) | `docs/proposals/0023-incremental-evaluation.md` |
| 0024 | SHIPPED | 0024 — predicate-typed witness 증명 (0013 R4 승격; envelope 무변경 설계) | `docs/proposals/0024-typed-witness-attestation.md` |
| 0025 | SHIPPED | 0025 — PE 어노테이션 + 의미변경 재특화 (0013 T4+T7 승격) | `docs/proposals/0025-pe-annotations-respecialization.md` |
| 0026 | SHIPPED | 0026 — 타워 사다리 마일스톤-1 (0013 T3+T2+T5+T6+T8 승격; 명시적 단계화) | `docs/proposals/0026-tower-ladder-milestones.md` |
| 0027 | SHIPPED | 0027 — host artifact 잔여 gap: form_sha256 + env diff (감사 G2+G5 승격) | `docs/proposals/0027-host-artifact-gaps.md` |
| 0028 | SHIPPED | 0028 — pnix compiled runtime (cogen 실행 성능벽의 진짜 해결 경로) | `docs/proposals/0028-compiled-runtime.md` |
| 0029 | SHIPPED | 0029 — efficient cogen (3rd Futamura projection done RIGHT, the "cogen approach") | `docs/proposals/0029-efficient-cogen.md` |
| 0030 | SHIPPED | 0030 — context propagation (Bondorf CPS-specializer EFFECT, without a CPS rewrite) | `docs/proposals/0030-context-propagation.md` |

