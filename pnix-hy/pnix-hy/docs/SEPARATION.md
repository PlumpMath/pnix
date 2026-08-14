# pnix-hy ↔ hy-meta 분리 계획 (현재 코드 기반)

> 상태: 분석 + 계획, 2026-07-01 실제 코드 전체 inventory에서 작성.
> 실행 업데이트, 역시 2026-07-01: SEP1/SEP2/SEP3-v1/SEP3-v2/SEP4/SEP5,
> IB1-IB4, IR 레이어, hy-meta SR1-SR6 구현됨. 아래 역사 inventory는 분리를
> 이끈 맵으로 여전히 유용.
> 재검증 2026-07-01 (adversarial multi-agent pass): 모든 follow-up 닫힘,
> by-design cross-lane witness-schema fallback 외 새 코드 중복 없음, 회귀
> 없음, --check 44/44. NOTE: 아래 인라인 Lxxxx 줄 번호는 POINT-IN-TIME이며
> 코드 이동에 따라 drift — **심볼 이름이 권위**이고 줄 번호가 아니다.

## 0. 이 문서를 이끄는 교정

이전에 프로젝트는 **mirror**가 있어야만 meta-circular 능력이 존재한다고
취급했다. 너무 좁다. Meta-circular 능력은 전체 집합이다:

```
reader · parser · form-as-data · AST-as-data · IR-as-data · compiler-as-data ·
eval/apply · quote/quasiquote · macro expansion · stage bootstrap ·
artifact reproduction · import hook · module loading · environment replay ·
bytecode/code-object inspection · roundtrip · drift detection · witness/proof ·
gate/capability · interop · self-hosting ladder
```

**Mirror는 하나의 관찰 표면이지 전체가 아니다.** 그리고 pnix-hy가 현재
*파편*처럼 보이는 이유는 설계 사고: pnix-side mirror가 한 번도 singleton이
되지 않았고, 많은 병렬 mirror/parity/report 표면으로 쪼개져 성능과 분석이
어려웠다. 이 문서는 (a) 각 레이어가 오늘 실제로 소유하는 것, (b) 잘못된
배치로 이동/통합해야 할 것, (c) singleton 교정을 기록한다.

목표 레이어링:

```
hy-meta   = Hy/Python meta-circular 컴파일러/평가기 + 재현성 PROOF 레인
pnix-hy   = hy-meta 위에 호스팅된 pnix 런타임 (자체 meta-circular ladder)
interop   = 명시적·양방향·loss/effect/capability 표시 경계
mirror    = 많은 trace facet을 가진 ONE pnix-side 관찰 진입점 (많은 mirror 아님)
```

---

## 1. 현재 현실 (각 파일이 오늘 실제로 소유하는 것)

### 1.1 `hy-meta/bootstrap.py` (8908 lines) — 이미 호스트 proof 레인

Upstream `hy` import (`import hy`, `hy.errors`, lazy `hy.reader`) + 로컬
`stage1.compiler`. pnix를 **import하지 않음**(오직 `stage14_host_capability_matrix`
L4541의 host-ID 키로 `"pnix-hy"`/`"pnix-clj"` 문자열, 및 venv-path 힌트).
이미 실제 entrypoint로 소유:

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
| Governance overlay (stage10–16) | session/sandbox/protocol probe, capability adapter, self-improvement quarantine, verdict replay, cross-host JSON/EDN export, external-evidence/extension review (L2395–6533) |

**Takeaway:** 호스트 artifact/code-object/pyc/marshal/AST 기계와 import hook은
이미 여기 있다. pnix-hy에서 이를 재구현한 것은 새로 쓸 것이 아니라 접어
넣을 중복이다.

### 1.2 `pnix_hy/hy_mirror.py` — projection + stage7 seam, 호스트 introspection 이전됨

1. **Interop bridge** to hy-meta: `run_bootstrap` (127), `stage7_eval`/`stage7_eval_json`
   (337/349) + persistent **stage7 worker** (`_STAGE7_WORKER_SCRIPT` 212,
   `_stage7_ensure_worker` 267, `_stage7_worker_eval` 306), **projection worker**
   (`_PROJECTION_WORKER_SCRIPT` 369, `_run_hy_script` 513, `_proj_*`), `stage_status_check`
   (1882) / `stage15_check` (1901) / `stagen_check` (1906) / `closure_probe` (1911) /
   `host_summary` (1926), 및 seam `mirror_full_introspection` (2039) /
   `introspection_parity` (2047).
2. **Hy→pnix projection**: `hy_form_projection` (603), `hy_form_and_macro_projection`
   (719), `hy_meta_circular_projection` (733), `hy_macroexpand_projection` (862),
   `hy_macro_step_trace` (1003), `hy_quasiquote_projection` (1169), `hy_defmacro_projection`
   (1344), `hy_reader_macro_projection` (1515), `hy_import_projection` (1683),
   `hy_module_projection` (1814).
3. **HOST 기계 hy-meta로 이전**: 예전 연속 호스트-introspection 블록은 이제
   `hy-meta/host_introspect.py`에 있고; `hy_mirror.py:_load_host_introspect` (1998)이
   path-import + re-export해 옛 projection 호출 사이트가 동작한다. pnix-hy 소유 표면은
   stage7 parity seam뿐: `mirror_full_introspection` (2039)과 `introspection_parity` (2047).

### 1.3 `pnix_hy/pnix_runtime.py` (15552 lines) — 진짜 런타임 + embedded kernel/compiler

- **Genuine pnix runtime (L1–4485) — STAYS.** Reader/parser (`Token` 30, `tokenize` 451,
  `Parser` 567, `parse` 1127, `source_position_value` 1944), AST/emit/hash (`emit_source`
  1280, `stable_data` 4448, `ast_hash` 4483), evaluator/value (`eval_ast` 3530,
  `eval_source` 4414, `force_value` 1401, `apply_pnix` 3713, `apply_binary` 3342, `Thunk`
  37, `Closure` 58, `AttrSet` 117, `PnixError` 25, `_type_of` 4092), ~164 builtins
  (`native_builtins` 3789), env/scope (`initial_env` 4278, `build_let_env` 3065, `with_env`
  3493), 및 저수준 mirror primitive `mirror_event` (3522) +
  `eval_source(..., {"mirror": True})` 분기 (4421)가 `MIRROR_SCHEMA` (22) emit.
- **Embedded host-compilation lane — "soft" host (`ast`/`dis`/`marshal`/`importlib` 없음):**
  stage7 **Hy kernel source as raw strings** — `HY_AST_EVALUATOR_SOURCE` (4490, Hy subset으로
  쓴 pnix 인터프리터), `HY_AST_COMPILER_SOURCE` (10903, Hy subset pnix→Python 컴파일러),
  `COMPILER_PRELUDE` (9371, Python target 런타임) — plus generators `hy_runtime_source_for_*`
  (9338–9357) / `hy_compiler_source_for_*` / `hy_compiler_emit_for_asts` (11290–11332); host-direct
  pnix→Python emitter `_px_*` (`_px_emit` 11683, `_px_t` 11536, `_px_try_fold` 11470 …
  11352–12041) 및 executor `compile_px_source` (12056) / `run_px_source` (12111)이 호스트
  CPython에서 `compile()`/`exec()` 호출 (12046, 12107); external-oracle subprocess harness
  `_run_original_px` (12268) / `original_oracle_report` (12299).
- **Fragmented parity surface:** `self_test_report` (15498, `SELF_TEST_CASES` 14267),
  `fixture_report` (12193), `original_oracle_report` (12299), `rust_corpus_report` (14185)
  — 네 독립 report 함수, 각자 스키마.

### 1.4 `pnix_hy/pnix_mirror.py` (2807) + `pnix_hy/cli.py` (688) — interop + 파편 mirror

- **pnix self-mirror runners:** `run_once` (25), `mirror_chain` (45), `run_mirror` (77),
  `stage_tower` (95), `self_test_report` (233) — 마지막이 **4 parity lanes** 구동:
  `runtime_parity` (`hy_runtime_batch` 120), `source_parity` (`hy_source_runtime_batch`
  147), `compiler_parity` (`hy_compiler_batch` 174), `compiler_source_parity`
  (`hy_compiler_source_batch` 203).
- **Interop = projection/synthesis toolkit** (§5 참고).
- **Production/runtime layer:** `safe_eval` (2729), `static_purity_check` (2696),
  `_IMPURE_BUILTINS` (2649), `cached_eval` (3077), `diagnose` (3153), `eval_receipt` (3217),
  `specialize_pnix` (2220), `meta_circular_tower` (2067), `pnix_evaluation_trace` (2541).
- **31 `*_report` self-checks** registered in `cli.py:_toolkit_reports()` (471) — each a
  separate observation surface. `cmd_gate` (cli.py 541) bundles the 4 parity lanes +
  runtime self-test + rust corpus + closure + the 31 reports into one ship-gate.

---

## 2. hy-meta로 MOVE / CONSOLIDATE 해야 할 것

이들은 Hy/Python 호스트 컴파일러/평가기 artifact이지 pnix 런타임 의미가 아니다.
이동은 대부분 **통합**: hy-meta가 이미 정본 버전을 소유하므로 pnix-hy 사본은
interop 경계를 넘는 얇은 호출이 되어야 한다.

### 2.1 호스트 introspection 이전 — DONE

현재 hy-meta 홈: `hy-meta/host_introspect.py`, 호환용으로 `hy_mirror.py` 통해서만
노출. pnix-hy는 stage7 커널 안에서 같은 introspection을 돌리고 비교하는
**seam**만 유지 (`mirror_full_introspection` 2039, `introspection_parity` 2047) —
진짜 interop/parity 표면이지 호스트 기계가 아니다.

### 2.2 pnix_runtime 안 호스트 EXECUTION of emitted code — hy-meta에 위임

`pnix_runtime.py` 자체가 생성 Python에 `compile()`/`exec()` (12046, 12107)와
external oracle용 `subprocess.run` (12279)을 한다. **Emitter**는 pnix 관심사
(pnix 컴파일러)이지만, **호스트 실행**은 hy-meta API
(`run_python_source`/`run_code_object`, `clean subprocess`)를 거쳐 pnix-hy가
raw host exec/subprocess를 소유하지 않게 해야 한다. External-oracle harness
(`_run_original_px`/`original_oracle_report`)는 out-of-repo Rust binary 대비
parity oracle — optional 유지, core 런타임 밖으로 명확히.

### 2.3 이미 올바름 (이동 없음): import hook

pnix-hy는 raw Python `importlib`를 소유해서는 안 된다. 이미 안 함 — hook은
hy-meta (`KernelHyLoader`/`KernelHyFinder`/`KernelHyImportHook`). pnix가 `.px`
import 의미를 정의할 때 실제 Python `sys.meta_path` 통합은 hy-meta 서비스
(`hy_meta.install_pnix_import_hook(...)`)이고, pnix-hy는 pnix module model만
소유.

---

## 3. pnix-hy에 STAYS (pnix-runtime meta-circular)

pnix 런타임 자체가 meta-circular이며 이들이 pnix-hy에 있는 이유이다:

- **Reader/tokenizer/parser** (`Token`, `tokenize`, `Parser`, `parse`, position model) —
  pnix 언어 표면. (`pnix_runtime.py` L1–1944.)
- **AST / emit / hash** (`emit_source` = AST→source, `ast_stable`, `ast_hash`,
  `stable_data`) — pnix 정본 표현. 호스트 Python/Hy artifact는 *실행*
  artifact; pnix IR가 정본 의미.
- **Evaluator / apply / value model / builtins / env** (`eval_ast`, `eval_source`,
  `force_value`, `apply_pnix`, `apply_binary`, `Thunk`/`Closure`/`AttrSet`,
  `native_builtins`, `build_let_env`, `with_env`) — 실제 pnix 런타임.
- **stage7 Hy-subset kernel SOURCE** (`HY_AST_EVALUATOR_SOURCE`,
  `HY_AST_COMPILER_SOURCE`, `COMPILER_PRELUDE`) 및 host-direct pnix→Python emitter
  (`_px_*`). 핵심: **이것은 pnix 자체의 self-hosting ladder** — hy-meta 호스트에서
  돌릴 수 있게 쓴 pnix. Hy의 meta-circular이 아니다. pnix self-hosting artifact로
  pnix-hy에 STAYS; *loading/execution*만 hy-meta 서비스 (§2.2).
- **Production runtime layer** (`safe_eval`, `static_purity_check`, `cached_eval`,
  `diagnose`, `eval_receipt`, `specialize_pnix`, `pnix_evaluation_trace`) — pnix 런타임
  의미 + sandbox/cache/diagnostics, hy-meta 위.
- **pnix runtime stage ladder + witnesses/gates** (목표; §6) — *호스트* 컴파일러를
  증명하는 hy-meta stage8/stage9와 구별. pnix stage는 *pnix 런타임*을 증명.

---

## 4. Interop (Hy/Python ↔ pnix) — 존재하는 것 vs 계획이 원하는 것

### 4.1 오늘 존재하는 것 (주의 깊게 — 실제 표면)

Interop은 현재 value-protocol이 아니라 **source-to-source projection/synthesis
toolkit**으로 실현. 모두 `pnix_mirror.py`:

| Function (line) | Direction | What it does |
|---|---|---|
| `pnix_to_hy_form` (1148) / `_pnix_to_hy` (1016) | pnix→Hy | pnix AST에서 Hy *source* 합성, 정직한 `gaps` |
| `synthesize_pnix_from_hy` (1530) / `_python_expr_to_pnix` (1397) / `_python_stmt_to_pnix_binding` (1475) / `_python_module_to_pnix` (1509) / `_joinedstr_to_pnix` (1376) | Hy/Python→pnix | Hy fragment의 Python lowering에서 pnix *source* 합성 |
| `align_python_to_pnix(_tree)` (722/918), `align_hy_to_pnix(_tree)` (768/936) | labeling | Python/Hy AST 노드에 pnix 대응 태그 (`differs`) — pnix를 emit하지 않음 |
| `correspondence_table` (552, `_CORRESPONDENCE` 490, 28 rows) | taxonomy | curated AST↔pnix-tag/value-type 맵 |
| `projection_value_roundtrip` (1284), `hy_to_pnix_value_roundtrip` (1578) | semantic | 양측 eval, 정본 JSON 비교 |
| `pnix_projection_closure` (1676), `hy_projection_closure` (1736) | involution | 양방향 왕복, 값 보존 |

**De-facto value mapping** (단일 `to_host` 없음): `rt.stable_data` (pnix value→Python:
null→None, bool→bool, int→i64, float→float, string→str, list→list, attrset→sorted dict,
Closure/native→sentinels) + `rt.to_json_string_value`; `_python_expr_to_pnix` (Python
literal→pnix source); `_pnix_to_hy` (pnix AST→Hy source); `_value_to_hy` (pnix value→Hy).

### 4.2 계획이 원하지만 아직 없는 것

전체 패키지 grep: **`to_host`/`from_host` 없고**, loss / effect / capability
protocol도 **없다**. "Loss"는 ad hoc `gaps`/`#_pnix-gap[...]` placeholder +
`differs` 플래그; "effect/capability"는 `static_purity_check`의 `_IMPURE_BUILTINS`
purity gate로만. 따라서 명시 interop protocol은 **새 작업**:

- 공유 레코드: `interop/id, direction, source/target language, input/output kind,
  loss-status (lossless|lossy|opaque|effectful|unsupported|dangerous), effect-class
  (pure|host-call|import|file|subprocess|network|…), capability-required, witness-id`;
- **opaque refs** 있는 값 매핑 (호스트 객체가 pnix 정본 항에 직접 들어가면 안 됨 —
  wrap) — 오늘 opaque-ref 타입 없고 stable_data sentinel만;
- arity/effect/exception/witness 검사 있는 callable + module bridge;
- hy-meta 호스트 측 adapter (`hy_meta.interop`, opaque Python object control) vs
  pnix-hy pnix 측 adapter (`pnix_hy.interop`, pnix value/function/module mapping).

Interop은 mirror OFF여도 동작해야 한다; mirror는 interop을 관찰할 수 있으나
정의하지 않는다.

---

## 5. Mirror: singleton 교정

### 5.1 현재 파편화 (문제, exact location)

단일 `mirror_run(source)` 없음 (grep: `mirror_run` 없음, `facet` 없음). 대신:

- pnix side: `run_once` (25), `mirror_chain` (45), `run_mirror` (77), `stage_tower` (95),
  `self_test_report` (233) — 그리고 `self_test_report`가 **4 parity lanes** 실행
  (`runtime_parity`/`source_parity`/`compiler_parity`/`compiler_source_parity`).
- runtime side: 단일 저수준 `mirror_event` (pnix_runtime.py 3522) primitive, 그러나
  파편 parity 표면 — `self_test_report`, `fixture_report`, `original_oracle_report`,
  `rust_corpus_report`, 각자 스키마.
- Hy side: `meta_circular_tower` (2067) plus registered `*_report` facilities.

각각 parse/lower/eval를 중복하고 자체 trace/schema emit — "많은 mirror" 문제:
정본 경로 없음, 단일 result hash 없음, 비싼 분석, 어려운 수렴.

### 5.2 목표: one mirror, many trace facets

```
pnix_hy.mirror_run(source, opts)  ->  parse · lower · eval · record facets · result hash · witness
```

ONE run 아래 facet 이벤트 emit:

```
:mirror/source :mirror/token :mirror/ast :mirror/ir :mirror/eval-step
:mirror/value :mirror/effect :mirror/interop :mirror/error :mirror/witness
```

`source-mirror / ast-mirror / ir-mirror / eval-mirror / interop-mirror /
value-mirror`를 독립 정본 mirror로 유지하지 말 것. 기존 runners
(`run_mirror`/`mirror_chain`/`stage_tower`/`self_test_report` 및 per-facility
`*_report`s)를 **병합**해 하나의 faceted `mirror_run`으로 이전, 모든 현재
이벤트를 facet으로 보존, parse/lower/eval dedup, 하나의 result hash + 하나의
witness 생성.

Note: hy-meta는 여러 mirror *checks*(compiler/kernel/artifact/stage/clean-replay)
를 유지해도 된다 — 호스트 *artifact-comparison* 표면 (`run_mirror_check`,
`run_stage7_check`, `run_diverse_double_compile_check`, `run_stage8_check`,
`run_stage9_check`)이며 **check 카테고리**이지 경쟁 runtime mirror가 아니다.
Singleton 규칙은 **pnix runtime** mirror용.

---

## 6. pnix runtime stage ladder (목표, hy-meta stage8/9와 구별)

hy-meta stage는 호스트 컴파일러/평가기 안정성을 증명. pnix-hy는 *pnix 런타임*
안정성을 증명하는 자체 ladder 필요 (현재 `run_mirror`/`stage_tower`/parity lanes가
재구성 원재료):

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
import / eval / file / subprocess / module-mutation` gates) — 오늘 `static_purity_check`의
purity gate만 존재.

---

## 7. 단계별 마이그레이션 우선순위

1. **Phase 1 — pnix-hy에서 호스트 기계 분리.** hy_mirror.py HOST 블록
   (§2.1, L1941–2382)을 hy-meta로 접기 (`artifact_from_ast` 등과 통합); pnix_runtime
   `compile()`/`exec()`/subprocess (§2.2)를 hy-meta API로 라우트. 진짜 런타임
   (§3)과 pnix-in-Hy kernel source는 pnix-hy에 유지.
2. **Phase 2 — interop protocol 정의** (§4.2): 공유 레코드, opaque refs,
   value/callable/module bridge, 호스트 측 (`hy_meta.interop`) + pnix 측
   (`pnix_hy.interop`) adapter. Mirror off로 동작.
3. **Phase 3 — 많은 mirror를 `mirror_run`으로 교체** (§5.2): runners 병합, 이벤트를
   facet으로 보존, 하나의 result hash + witness.
4. **Phase 4 — pnix runtime stage ladder** (§6), hy-meta stage8/9와 구별.
5. **Phase 5 — gates + witnesses** across eval/interop/replay/drift.

### 최종 아키텍처 (한 줄씩)

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
