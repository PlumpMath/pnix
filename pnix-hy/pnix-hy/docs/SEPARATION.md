# pnix-hy ↔ hy-meta Separation Plan (grounded in the current code)

> Status: analysis + plan, written from a full inventory of the actual code on
> 2026-07-01. Execution update, also 2026-07-01: SEP1/SEP2/SEP3-v1/SEP3-v2/SEP4/SEP5,
> IB1-IB4, the IR layer, and hy-meta SR1-SR6 are implemented. The historical inventory
> below remains useful as the map that drove the split.
> Re-verified 2026-07-01 (adversarial multi-agent pass): all follow-ups closed, no new
> code duplication beyond the by-design cross-lane witness-schema fallback, no regression,
> --check 44/44. NOTE: inline Lxxxx line numbers below are POINT-IN-TIME and have drifted as
> code moved -- the **symbol names are authoritative**, not the line numbers.

## 0. The correction that drives this document

Earlier the project treated **mirror** as if the meta-circular capability only exists
when a mirror exists. That is too narrow. Meta-circular capability is the whole set:

```
reader · parser · form-as-data · AST-as-data · IR-as-data · compiler-as-data ·
eval/apply · quote/quasiquote · macro expansion · stage bootstrap ·
artifact reproduction · import hook · module loading · environment replay ·
bytecode/code-object inspection · roundtrip · drift detection · witness/proof ·
gate/capability · interop · self-hosting ladder
```

**Mirror is one observation surface, not the whole thing.** And the reason pnix-hy
currently *looks* fragmented is a design accident: the pnix-side mirror was never made a
singleton — it was split into many parallel mirror/parity/report surfaces, which is what
made performance and analysis hard. This document records (a) what each layer actually
owns today, (b) what is wrongly placed and must move/consolidate, and (c) the singleton
correction.

Target layering:

```
hy-meta   = the Hy/Python meta-circular compiler/evaluator + reproducibility PROOF lane
pnix-hy   = the pnix runtime (its own meta-circular ladder) hosted on top of hy-meta
interop   = an explicit, bidirectional, loss/effect/capability-marked boundary
mirror    = ONE pnix-side observation entrypoint with many trace facets (not many mirrors)
```

---

## 1. Current reality (what each file actually owns today)

### 1.1 `hy-meta/bootstrap.py` (8908 lines) — already the host proof lane

It imports upstream `hy` (`import hy`, `hy.errors`, lazy `hy.reader`) + local
`stage1.compiler`. It does **not** import pnix (only the strings `"pnix-hy"`/`"pnix-clj"`
as host-ID keys in `stage14_host_capability_matrix` L4541, and a venv-path hint). It
already owns, with real entrypoints:

| Concern | Representative symbols (line) |
|---|---|
| Stage bootstrap chain | `bootstrap_stage2` (79), `bootstrap_stage2_chain` (85), `bootstrap_stage3_chain` (93), `bootstrap_stage_chain` (100), `bootstrap_kernel` (120), `bootstrap_stage7_kernel` (127) |
| Hy kernel load/eval | `load_kernel_compiled_kernel` (1357), `run_kernel_check` (363), `cmd_kernel_run`/`cmd_kernel_py` (8459/8468), `cmd_stage7_kernel_run`/`_py` (8475/8484) |
| Python import hook | `KernelHyLoader` (134), `KernelHyFinder` (183), `KernelHyImportHook` (235), `install_kernel_import_hook` (250), `run_kernel_import_check` (7088, incl. `sys.modules`/`sys.meta_path` rollback) |
| Artifact / hash / pyc / marshal | `sha256_bytes/text` (71/75), `ast_data` (496), `pyc_bytes_for_code` (502), `location_stable_ast` (509), `stable_code_const` (520), `stable_code_payload` (543, marshal-free code payload), `artifact_from_ast` (566), `artifact_summary` (607) |
| Mirror / drift | `run_mirror_check` (650), `run_chain_check` (384), `run_stage3_check` (441), `run_stage7_check` (1193), `run_self_host_check` (1390), `run_bootstrap_fixedpoint_check` (1448), `run_diverse_double_compile_check` (1547, Wheeler DDC), `run_no_fallback_check` (1668), `run_parity_ledger_check` (1788) |
| stage8 / stage9 proof | `run_stage8_check` (2018), `classify_stage8_drift` (1967), `compare_stage8_artifact_bundles` (1982); `stage9_clean_env` (2084), `stage9_manifest` (2097), `stage9_probe_result` (2170), `run_stage9_probe_subprocess` (2256), `run_stage9_check` (2313) |
| Clean env / subprocess | `stage9_clean_env` (2084), `run_stage9_probe_subprocess` (2256), `run_stage10_subprocess_client` (2514) |
| Host introspection / boundary | `run_reader_boundary_check` (829), `run_compatibility_boundary_check` (877), `run_front_end_boundary_check` (6992, cedes reader/mangling to `hy.reader`), `version_ast_coverage_matrix` (6534), `run_source_position_check` (6577, PEP 657), `run_ast_forward_compat_check` (6697), `run_macro_require_parity_check` (6832) |
| Governance overlay (stage10–16) | session/sandbox/protocol probes, capability adapters, self-improvement quarantine, verdict replay, cross-host JSON/EDN export, external-evidence/extension review (L2395–6533) |

**Takeaway:** the host artifact/code-object/pyc/marshal/AST machinery and the import hook
already live here. Anything in pnix-hy that re-implements these is a duplicate to fold in,
not a new thing to write.

### 1.2 `pnix_hy/hy_mirror.py` — projection + stage7 seam, host introspection relocated

1. **Interop bridge** to hy-meta: `run_bootstrap` (127), `stage7_eval`/`stage7_eval_json`
   (337/349) + the persistent **stage7 worker** (`_STAGE7_WORKER_SCRIPT` 212,
   `_stage7_ensure_worker` 267, `_stage7_worker_eval` 306), the **projection worker**
   (`_PROJECTION_WORKER_SCRIPT` 369, `_run_hy_script` 513, `_proj_*`), `stage_status_check`
   (1882) / `stage15_check` (1901) / `stagen_check` (1906) / `closure_probe` (1911) /
   `host_summary` (1926), and the seam `mirror_full_introspection` (2039) /
   `introspection_parity` (2047).
2. **Hy→pnix projection**: `hy_form_projection` (603), `hy_form_and_macro_projection`
   (719), `hy_meta_circular_projection` (733), `hy_macroexpand_projection` (862),
   `hy_macro_step_trace` (1003), `hy_quasiquote_projection` (1169), `hy_defmacro_projection`
   (1344), `hy_reader_macro_projection` (1515), `hy_import_projection` (1683),
   `hy_module_projection` (1814).
3. **HOST machinery relocated to hy-meta**: the former contiguous host-introspection block now
   lives in `hy-meta/host_introspect.py`; `hy_mirror.py:_load_host_introspect` (1998) path-imports
   and re-exports it so old projection call sites keep working. The only pnix-hy-owned surface is
   the stage7 parity seam: `mirror_full_introspection` (2039) and `introspection_parity` (2047).

### 1.3 `pnix_hy/pnix_runtime.py` (15552 lines) — genuine runtime + embedded kernel/compiler

- **Genuine pnix runtime (L1–4485) — STAYS.** Reader/parser (`Token` 30, `tokenize` 451,
  `Parser` 567, `parse` 1127, `source_position_value` 1944), AST/emit/hash (`emit_source`
  1280, `stable_data` 4448, `ast_hash` 4483), evaluator/value (`eval_ast` 3530,
  `eval_source` 4414, `force_value` 1401, `apply_pnix` 3713, `apply_binary` 3342, `Thunk`
  37, `Closure` 58, `AttrSet` 117, `PnixError` 25, `_type_of` 4092), ~164 builtins
  (`native_builtins` 3789), env/scope (`initial_env` 4278, `build_let_env` 3065, `with_env`
  3493), and the one low-level mirror primitive `mirror_event` (3522) + the
  `eval_source(..., {"mirror": True})` branch (4421) emitting `MIRROR_SCHEMA` (22).
- **Embedded host-compilation lane — "soft" host (no `ast`/`dis`/`marshal`/`importlib`):**
  the stage7 **Hy kernel source as raw strings** — `HY_AST_EVALUATOR_SOURCE` (4490, the
  pnix interpreter written in the Hy subset), `HY_AST_COMPILER_SOURCE` (10903, the
  pnix→Python compiler in the Hy subset), `COMPILER_PRELUDE` (9371, the Python target
  runtime) — plus generators `hy_runtime_source_for_*` (9338–9357) /
  `hy_compiler_source_for_*` / `hy_compiler_emit_for_asts` (11290–11332); the host-direct
  pnix→Python emitter `_px_*` (`_px_emit` 11683, `_px_t` 11536, `_px_try_fold` 11470 …
  11352–12041) and its executor `compile_px_source` (12056) / `run_px_source` (12111) which
  call `compile()`/`exec()` on host CPython (12046, 12107); and the external-oracle
  subprocess harness `_run_original_px` (12268) / `original_oracle_report` (12299).
- **Fragmented parity surface:** `self_test_report` (15498, `SELF_TEST_CASES` 14267),
  `fixture_report` (12193), `original_oracle_report` (12299), `rust_corpus_report` (14185)
  — four independent report functions, each with its own schema.

### 1.4 `pnix_hy/pnix_mirror.py` (2807) + `pnix_hy/cli.py` (688) — interop + the fragmented mirror

- **pnix self-mirror runners:** `run_once` (25), `mirror_chain` (45), `run_mirror` (77),
  `stage_tower` (95), `self_test_report` (233) — the last drives **4 parity lanes**:
  `runtime_parity` (`hy_runtime_batch` 120), `source_parity` (`hy_source_runtime_batch`
  147), `compiler_parity` (`hy_compiler_batch` 174), `compiler_source_parity`
  (`hy_compiler_source_batch` 203).
- **Interop = the projection/synthesis toolkit** (see §5).
- **Production/runtime layer:** `safe_eval` (2729), `static_purity_check` (2696),
  `_IMPURE_BUILTINS` (2649), `cached_eval` (3077), `diagnose` (3153), `eval_receipt` (3217),
  `specialize_pnix` (2220), `meta_circular_tower` (2067), `pnix_evaluation_trace` (2541).
- **31 `*_report` self-checks** registered in `cli.py:_toolkit_reports()` (471) — each a
  separate observation surface. `cmd_gate` (cli.py 541) bundles the 4 parity lanes +
  runtime self-test + rust corpus + closure + the 31 reports into one ship-gate.

---

## 2. What must MOVE / CONSOLIDATE into hy-meta

These are Hy/Python host compiler/evaluator artifacts, not pnix runtime semantics. The
move is mostly **consolidation**: hy-meta already owns the canonical versions, so pnix-hy's
copies should become thin calls across the interop boundary.

### 2.1 Host introspection relocation — DONE

Current hy-meta home: `hy-meta/host_introspect.py`, exposed through `hy_mirror.py` only for
compatibility. pnix-hy keeps only the **seam** that runs the same introspection inside the stage7
kernel and compares (`mirror_full_introspection` 2039, `introspection_parity` 2047) — that is
genuinely an interop/parity surface, not host machinery.

### 2.2 Host EXECUTION of emitted code in pnix_runtime — delegate to hy-meta

`pnix_runtime.py` itself does `compile()`/`exec()` of generated Python (12046, 12107) and
`subprocess.run` for the external oracle (12279). The **emitter** is a pnix concern (it is
pnix's compiler), but the **host execution** should go through hy-meta APIs
(`run_python_source`/`run_code_object`, `clean subprocess`) so pnix-hy does not own raw
host exec/subprocess. The external-oracle harness (`_run_original_px`/
`original_oracle_report`) is a parity oracle against an out-of-repo Rust binary — keep it
optional and clearly outside the core runtime.

### 2.3 Already-correct (no move): the import hook

pnix-hy must NOT own raw Python `importlib`. It already does not — the hook lives in
hy-meta (`KernelHyLoader`/`KernelHyFinder`/`KernelHyImportHook`). When pnix defines `.px`
import semantics, the actual Python `sys.meta_path` integration should be a hy-meta service
(`hy_meta.install_pnix_import_hook(...)`), with pnix-hy owning only the pnix module model.

---

## 3. What STAYS in pnix-hy (pnix-runtime meta-circular)

The pnix runtime is itself meta-circular and is the reason these live in pnix-hy:

- **Reader/tokenizer/parser** (`Token`, `tokenize`, `Parser`, `parse`, position model) —
  the pnix language surface. (`pnix_runtime.py` L1–1944.)
- **AST / emit / hash** (`emit_source` = AST→source, `ast_stable`, `ast_hash`,
  `stable_data`) — pnix canonical representation. Host Python/Hy artifacts are *execution*
  artifacts; pnix IR is the canonical semantics.
- **Evaluator / apply / value model / builtins / env** (`eval_ast`, `eval_source`,
  `force_value`, `apply_pnix`, `apply_binary`, `Thunk`/`Closure`/`AttrSet`,
  `native_builtins`, `build_let_env`, `with_env`) — the actual pnix runtime.
- **The stage7 Hy-subset kernel SOURCE** (`HY_AST_EVALUATOR_SOURCE`,
  `HY_AST_COMPILER_SOURCE`, `COMPILER_PRELUDE`) and the host-direct pnix→Python emitter
  (`_px_*`). Key point: **this is pnix's OWN self-hosting ladder** — pnix written so it can
  run on the hy-meta host. It is not Hy's meta-circular. It STAYS in pnix-hy as the pnix
  self-hosting artifact; only its *loading/execution* should be a hy-meta service (§2.2).
- **Production runtime layer** (`safe_eval`, `static_purity_check`, `cached_eval`,
  `diagnose`, `eval_receipt`, `specialize_pnix`, `pnix_evaluation_trace`) — pnix runtime
  semantics + sandbox/cache/diagnostics, on top of hy-meta.
- **pnix runtime stage ladder + witnesses/gates** (target; see §6) — distinct from
  hy-meta's stage8/stage9, which prove the *host* compiler. pnix stages prove the *pnix
  runtime*.

---

## 4. Interop (Hy/Python ↔ pnix) — what exists vs what the plan wants

### 4.1 What EXISTS today (read carefully — this is the real surface)

Interop is currently realized as a **source-to-source projection/synthesis toolkit**, not
as a value-protocol. All in `pnix_mirror.py`:

| Function (line) | Direction | What it does |
|---|---|---|
| `pnix_to_hy_form` (1148) / `_pnix_to_hy` (1016) | pnix→Hy | synthesize Hy *source* from a pnix AST, honest `gaps` |
| `synthesize_pnix_from_hy` (1530) / `_python_expr_to_pnix` (1397) / `_python_stmt_to_pnix_binding` (1475) / `_python_module_to_pnix` (1509) / `_joinedstr_to_pnix` (1376) | Hy/Python→pnix | synthesize pnix *source* from a Hy fragment's Python lowering |
| `align_python_to_pnix(_tree)` (722/918), `align_hy_to_pnix(_tree)` (768/936) | labeling | tag Python/Hy AST nodes with their pnix correspondence (`differs`) — does NOT emit pnix |
| `correspondence_table` (552, `_CORRESPONDENCE` 490, 28 rows) | taxonomy | curated AST↔pnix-tag/value-type map |
| `projection_value_roundtrip` (1284), `hy_to_pnix_value_roundtrip` (1578) | semantic | eval both sides, compare canonical JSON |
| `pnix_projection_closure` (1676), `hy_projection_closure` (1736) | involution | round-trip both directions, value-preserving |

**De-facto value mapping** (no single `to_host`): `rt.stable_data` (pnix value→Python:
null→None, bool→bool, int→i64, float→float, string→str, list→list, attrset→sorted dict,
Closure/native→sentinels) + `rt.to_json_string_value`; `_python_expr_to_pnix` (Python
literal→pnix source); `_pnix_to_hy` (pnix AST→Hy source); `_value_to_hy` (pnix value→Hy).

### 4.2 What the plan wants that does NOT exist yet

Grepping the whole package: there is **no** `to_host`/`from_host`, and **no** loss /
effect / capability protocol. "Loss" is tracked ad hoc as `gaps`/`#_pnix-gap[...]`
placeholders + the `differs` flag; "effect/capability" exists only as
`static_purity_check`'s `_IMPURE_BUILTINS` purity gate. So the explicit interop protocol is
**new work**:

- a shared record: `interop/id, direction, source/target language, input/output kind,
  loss-status (lossless|lossy|opaque|effectful|unsupported|dangerous), effect-class
  (pure|host-call|import|file|subprocess|network|…), capability-required, witness-id`;
- value mapping with **opaque refs** (host objects must NOT enter pnix canonical terms
  directly — wrap them) — today there is no opaque-ref type, only the stable_data
  sentinels;
- callable + module bridges with arity/effect/exception/witness checks;
- host-side adapter in hy-meta (`hy_meta.interop`, opaque Python object control) vs
  pnix-side adapter in pnix-hy (`pnix_hy.interop`, pnix value/function/module mapping).

Interop must work even when mirror is OFF; mirror may observe interop but does not define
it.

---

## 5. Mirror: the singleton correction

### 5.1 Current fragmentation (the problem, with exact locations)

There is no single `mirror_run(source)` (grep: no `mirror_run`, no `facet`). Instead:

- pnix side: `run_once` (25), `mirror_chain` (45), `run_mirror` (77), `stage_tower` (95),
  `self_test_report` (233) — and `self_test_report` runs **4 parity lanes**
  (`runtime_parity`/`source_parity`/`compiler_parity`/`compiler_source_parity`).
- runtime side: a single low-level `mirror_event` (pnix_runtime.py 3522) primitive, but a
  fragmented parity surface — `self_test_report`, `fixture_report`, `original_oracle_report`,
  `rust_corpus_report`, each its own schema.
- Hy side: `meta_circular_tower` (2067) plus the registered `*_report` facilities.

Each duplicates parse/lower/eval and emits its own trace/schema — the "many mirrors"
problem: no canonical route, no single result hash, expensive analysis, hard convergence.

### 5.2 Target: one mirror, many trace facets

```
pnix_hy.mirror_run(source, opts)  ->  parse · lower · eval · record facets · result hash · witness
```

emitting facet events under ONE run:

```
:mirror/source :mirror/token :mirror/ast :mirror/ir :mirror/eval-step
:mirror/value :mirror/effect :mirror/interop :mirror/error :mirror/witness
```

Do NOT keep `source-mirror / ast-mirror / ir-mirror / eval-mirror / interop-mirror /
value-mirror` as independent canonical mirrors. Migrate by **merging** the existing
runners (`run_mirror`/`mirror_chain`/`stage_tower`/`self_test_report` and the per-facility
`*_report`s) into one faceted `mirror_run`, preserving every current event as a facet,
deduplicating parse/lower/eval, producing one result hash + one witness.

Note: hy-meta MAY keep several mirror *checks* (compiler/kernel/artifact/stage/clean-replay)
because those are host *artifact-comparison* surfaces (`run_mirror_check`, `run_stage7_check`,
`run_diverse_double_compile_check`, `run_stage8_check`, `run_stage9_check`) — they are
**check categories**, not competing runtime mirrors. The singleton rule is for the
**pnix runtime** mirror.

---

## 6. pnix runtime stage ladder (target, distinct from hy-meta stage8/9)

hy-meta stages prove host compiler/evaluator stability. pnix-hy needs its OWN ladder that
proves pnix runtime stability (the current `run_mirror`/`stage_tower`/parity lanes are the
raw material to reshape into this):

```
pnix-stage1 direct pnix eval
pnix-stage2 parse → normalized AST → eval        (eval_normalized_source 4443 is the seed)
pnix-stage3 AST/IR store-backed eval
pnix-stage4 AST/IR roundtrip integrity            (ast_hash / emit→reparse already exist)
pnix-stage5 singleton mirror route                (mirror_run)
pnix-stage6 deterministic replay                  (delegated to hy-meta clean-replay API)
pnix-stage7 runtime closure                       (current 4-lane convergence reshaped)
```

Plus pnix witnesses/gates (`eval/stage/roundtrip/mirror/interop` witnesses; `host-call /
import / eval / file / subprocess / module-mutation` gates) — today only `static_purity_check`'s
purity gate exists.

---

## 7. Phased migration priority

1. **Phase 1 — split host machinery out of pnix-hy.** Fold the hy_mirror.py HOST block
   (§2.1, L1941–2382) into hy-meta (consolidating with `artifact_from_ast` et al.); route
   pnix_runtime's `compile()`/`exec()`/subprocess (§2.2) through hy-meta APIs. Keep the
   genuine runtime (§3) and the pnix-in-Hy kernel source in pnix-hy.
2. **Phase 2 — define the interop protocol** (§4.2): the shared record, opaque refs,
   value/callable/module bridges, host-side (`hy_meta.interop`) + pnix-side
   (`pnix_hy.interop`) adapters. Make it work with mirror off.
3. **Phase 3 — replace the many mirrors with `mirror_run`** (§5.2): merge runners, preserve
   events as facets, one result hash + witness.
4. **Phase 4 — pnix runtime stage ladder** (§6), distinct from hy-meta stage8/9.
5. **Phase 5 — gates + witnesses** across eval/interop/replay/drift.

### Final architecture (one line each)

```
hy-meta   = Hy/Python self-compile/evaluate/reproduce proof lane (owns stage chain, kernel,
            import hook, Python AST/code/pyc/marshal artifacts, mirror/drift, stage8/9,
            clean replay, host introspection)  [bootstrap.py today; split into modules later]
pnix-hy   = pnix runtime on top of hy-meta (owns pnix reader/parser/AST/eval/value/builtins/
            env, the pnix-in-Hy self-hosting kernel source, sandbox/cache/diagnose/receipt,
            the singleton pnix mirror, pnix stage ladder + gates/witnesses)
interop   = explicit bidirectional bridge; host objects ↔ pnix values only through
            loss-marked, effect-classified, capability-checked adapters (NEW work)
mirror    = NOT the source of meta-circularity; ONE pnix-side observation entrypoint with
            many trace facets
```
