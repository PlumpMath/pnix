# pnix-hy todo

> ⛔ **SCOPE LOCK** (see `/SCOPE_LOCK.md`): the meta-circular-projection scope is closed
> (`--check` 56/56; 44 baseline + interop 0001-0007 + REPLs 0008 + action VM 0009). **No new implementation may
> reinterpret intentional placeholders as missing work** (의도적 placeholder를 미구현으로 재해석해
> 구현하지 말 것). New capabilities start as a `docs/proposals/NNNN-*.md`, NOT a `[ ]` here. Say
> "complete w.r.t. the stated scope", never "complete overall".
>
> ▶ **RECENT ACCEPTED PHASES SHIPPED:** **0009 — pnix semantic/action VM** (`action.py`, CLI,
> example 18) and **0010 — module distribution tiers** (installable `import pnix_hy`; off-tree
> `PNIX_HY_HOME` reaches the projection/proof tiers; `deploy.py`/`--deployment`; `pyproject`
> `[projection]`/`[full]` extras). Both additive; `pnix_runtime.py` untouched; `--check` 56/56,
> `--gate` PASS; open `[ ]` = 0. Specs: `docs/proposals/0009-*.md`, `docs/proposals/0010-*.md`.

## ▶ ACTIVE PHASE — docs-as-code: capability 인덱스 + drift 게이트 (proposal 0011, ACCEPTED — dev prep)

목표: 진실의 원천 = 코드, 문서는 **파생/생성 뷰 + drift 게이트**. 손-wiki 없음(=중복 원천 금지).
"설계구조==코드구조"를 `--check` 게이트로. 전부 additive·reuse-only·sacred 무접촉. 전체 스펙:
`docs/proposals/0011-docs-as-code-capability-index.md`. 사용자 선택 = **생성 인덱스 + drift 게이트**
(mkdocs/GitHub Wiki/Obsidian 사이트는 이번 범위 밖).

신규 딜리버러블 (open):
- [ ] `pnix_hy/capabilities.py` + `capability_index()` — `_toolkit_reports()`+`__all__`+docstring+
  proposal 상태에서 **파생**한 `{name,kind,owner_lane,module,symbol,report,proposal,status,summary}`.
  코드 파생이라 drift 불가. 새 evaluator/mirror/gate 없음.
- [ ] CLI `--capabilities` (text/json) + 생성물 `docs/CAPABILITIES.md`(생성물 헤더, 손편집 금지).
- [ ] `docs_drift_report()` — `--check`에 등록(**56 → 57**): (a) 문서 참조 심볼이 코드 실재,
  (b) 모든 `__all__` 공개 심볼이 인덱스 커버·소유, (c) `[[...]]` 위키링크 해소.
- [ ] `todo.md` 단일덩어리 해체 — 활성 수락작업만 남기고 이력은 git/`docs/archive/`로.
- [ ] `[[심볼]]` 위키링크 규약 도입 + drift 게이트가 미해소 링크 포착.

Done-when: `--capabilities` 동작, `docs_drift_report` `--check` 57 all_ready, `--gate` PASS(회귀 0),
todo 활성-only, `[[]]` 해소, `pnix_runtime.py`/sacred 무변경, main FF.

## ▶ SHIPPED PHASE — pnix semantic / action VM (proposal 0009, implemented 2026-07-02)

Goal: `pnix-hy` = the pnix VM on the Hy/Python host, wrapped by gate/witness/explain/**action
checkpoint** into an AI-coding-agent semantic VM. **Folder structure preserved; add a thin
`pnix_hy/action.py` only** (+ small CLI flags + one examples section). Full spec:
`docs/proposals/0009-pnix-semantic-action-vm.md`. REUSE existing pieces; build NO second
evaluator/mirror/gate; do NOT touch `pnix_runtime.py` core with action governance; do NOT copy
hy-meta host machinery.

Minimum completion criteria — already holding (reuse, do NOT rebuild):
- [x] 1 pnix eval works · [x] 2 pnix compile/run works · [x] 3 `eval==compile` parity (4-lane 545×4)
- [x] 4 IR hash stable (`ir.py`) · [x] 5 roundtrip meaning preserved (`hy_to_pnix_value_roundtrip`)
- [x] 7 gate records required effects (`gate_check`) · [x] 8 witness/receipt deterministic (`make_witness`)
- [x] 9 `mirror_run` is the single mirror · [x] 10 `explain_pnix` unified · [x] 11 Hy↔pnix loss status (`roundtrip_status`)

New deliverables (closed):
- [x] `pnix_hy/action.py` — thin action layer: `begin_action` / `check_action` / `verify_action`
  / `action_report`. MUST reuse `safe_eval`, `static_purity_check`, `gate.gate_check`,
  `gate.make_witness`, `mirror.mirror_run`, `pnix_mirror.explain_pnix`, `roundtrip_status`;
  NO new evaluator/mirror/gate. Implemented as `pnix_hy/action.py`.
- [x] verdict record `pnix-hy.action.report.v0` — `{status accepted|held|rejected, phase,
  source_hash, ir_hash, value_hash, gate, explain, effects, witness_id, rollback_ref}`.
  snapshot/rollback = **hash refs only** (NOT a file-backup system).
- [x] criterion 6: `check_action` **rejects impure by default** (gated) — `held` unless the
  required effect is `granted=(...)`.
- [x] criterion 12: `action_report()` proves accept / hold / reject paths; registered in
  `--check` (55 → 56, no regression to existing reports).
- [x] `cli.py`: thin `--action-check SRC` / `--action-explain SRC` (call `action.py` only).
- [x] `examples/18-action-checkpoint/` — `limit_python.py` (plain can't bundle
  success/effect/meaning/verdict into one record) vs `pnix_hy_way.py` + README (한글주석 + 비유).
- Done: `--check` 56/56 green, `--gate` PASS (from `pnix-hy/` package root), eval==compile
  545x4/mirror-singleton/meaning reports unchanged; `pnix_runtime.py` untouched by action governance.


## ▶ PROPOSAL CANDIDATES — pointer (NOT open todo work; do not add as `[ ]`)

Hy(Python)↔pnix language-interop enhancement candidates (2 independent scouts reconciled,
2026-07-01) live in **`docs/proposals/0000-interop-language-feature-candidates.md`** — 20+
candidates in clusters A (value/opaque fidelity), B (callable/module reach), C (macro/quote &
meta-circular↔meta-circular), D (boundary-ABI), each `file:symbol`-grounded and scope-classed.
They are OPTIONAL (no required unimplemented feature). Per SCOPE_LOCK §7 they are deliberately
NOT `[ ]` items here; each becomes an `NNNN-*.md` and enters todo only on human acceptance.
Top-3 recommended write-ups (human accepted "구현시작" 2026-07-01):
1. ✅ **SHIPPED** roundtrip-host-value + loss fidelity (A1–A6) → `docs/proposals/0001-*.md`;
   `interop.roundtrip_host_value` + A2–A6 loss-marking fixes; `--check` now **45/45**
   (new `interop_roundtrip`). C6 (source-level roundtrip vocab) remains a candidate.
2. ✅ **SHIPPED** host-callable-into-pnix-eval (B1+B2+B3+B5+B4) → `docs/proposals/0002-*.md`;
   `interop.host_callable_to_pnix` (pnix source applies host callables, capability-gated) +
   `call_host(kwargs=)` + `host_callable_arity` + `host_module_to_pnix(wrap_callables=)`;
   `--check` now **46/46** (new `interop_host_bridge`).
3. ✅ **SHIPPED** hy-macro-quasiquote-over-pnix (C1+C2+C3) → `docs/proposals/0003-*.md`;
   `pnix_mirror.hy_macro_over_pnix` (Hy macro over a pnix-projected form, expansion → pnix) +
   `hy_quasiquote_over_pnix` (pnix value fills a Hy quasiquote hole) +
   `quasiquote_specialize_correspondence` + `hy_mirror.hy_eval_form`; `--check` now **47/47**
   (new `interop_hy_macro_bridge`).

**All top-3 recommended interop proposals SHIPPED (0001/0002/0003).**
4. ✅ **SHIPPED** interop diagnostics & invariants (C5+C7+C8) → `docs/proposals/0004-*.md`;
   `pnix_mirror.classify_drift` + `pnix_mirror.reify_hy` + `interop.no_mirror_report`;
   `--check` now **50/50** (new `classify_drift`, `reify_hy`, `interop_no_mirror`).
5. ✅ **SHIPPED** Hy reader macro embeds pnix (C4) → `docs/proposals/0005-*.md`;
   `hy_mirror.hy_read_with_pnix_reader` (`#px "..."` → `(pnix-eval ...)` at read time) +
   `pnix_mirror.hy_reader_embed_pnix`; `--check` now **51/51** (new `hy_reader_embed_pnix`).
   C9 (stage7 report) DECLINED as ceremony (overlaps `pnix_meta_circular_projection`).

6. ✅ **SHIPPED** interop error contract (D1) + role matrix (D4) → `docs/proposals/0006-*.md`;
   `interop.is_interop_error`/`try_call_host`/`InteropError` (unambiguous cross-boundary errors,
   pnix-side only, shared ABI untouched) + `docs/INTEROP_ROLE_MATRIX.md`; `--check` now **52/52**
   (new `interop_error_contract`).

7. ✅ **SHIPPED** opaque lifecycle (D2 in-scope) + versioned correspondence ABI (D3 in-scope) →
   `docs/proposals/0007-*.md`; `interop.opaque_lifecycle` (leak-countable, ref shape unchanged) +
   `pnix_mirror.correspondence_abi` (content-hashed versioned artifact); `--check` now **54/54**
   (new `interop_opaque_lifecycle`, `correspondence_abi`).

**All catalog in-scope + doc-only items are now SHIPPED (0001–0007).** Only genuinely cross-lane
ABI remnants remain: **D2 refcount on the SHARED ref shape** + **D3 cross-repo vocabulary
unification** (both hy-meta + pnix-hy + gate drift-guard together). A7 (opaque-ref passthrough)
is low-value. These stay candidates in `docs/proposals/0000-*.md` until a dedicated both-lane
proposal is accepted (they need pnix-hs/pnix-rs to exist to be worth the ABI coordination).

## ▶ RESEARCH FINDINGS + ROADMAP — deep-research + code audit (2026-06-30)

Combined a web deep-research pass (25 claims, all 3-0 adversarially verified) with a
file:line code audit of pnix-hy. Two deliverables: concrete gaps/optimizations in the
toolkit, and a researched path to weave CPython + the Python ecosystem deeper into the
meta-circular substrate.

### A. Missing in pnix-hy (code-audit, prioritized)
- **A2 (biggest mission gap)** correspondence table (pnix_mirror.py ~477-520) + `_PY_CLASS_TO_TAG`
  (~610-625) have ZERO rows for the host constructs the design directive says to mirror:
  Try/Except/Raise, context-manager With, ClassDef, ListComp/Dict/Set/GeneratorExp,
  For/While, Await/Yield. They land in `unmapped_nodes` (honest, but undesigned). Add rows
  (most `differs=True`: comprehension→genList/map+filter; try/raise→tryEval/abort closest;
  Python with→no pnix equiv).
- **A1** reverse synth `_pnix_to_hy` has no `match` arm → `#_pnix-match` placeholder
  (pnix_mirror.py ~1040). Add `match → (cond ...)` for literal/`_` arms (guard arms stay gap).
- **A3** no whole-file/multi-form projection; `hy_macroexpand_projection` is forms[0]-only
  (hy_mirror.py ~448) while python/alignment use all forms — scope mismatch.
  **[DONE 2026-06-30 — `hy_macro_step_trace` traces every top-level form's macro tower
  (macroexpand_1 → fixpoint), closing the forms[0]-only gap and adding stepwise depth.]**
- A5 (intentional non-goal) derivation outPath/drvPath are fixed placeholders (host store
  hashing = out of scope).

### B. Optimization (code-audit + theory)
- **B1 (top lever)** the ~184KB stage7 Hy kernel is re-tokenized+recompiled by Hy on EVERY
  subprocess call (hy_mirror.py:196); meta/mirror calls pay it 2-6x. → precompile once to a
  cached module / persistent worker speaking a small request protocol. (Theory: CPython PEP
  659 adaptive interpreter / PEP 669 sys.monitoring for in-process speed+instrumentation.)
- **B4** pnix_form_projection evaluates the source 3x (interp, compiler, toJSON-rewrap);
  use `rt.to_json_string_value` on the already-computed value (pnix_runtime.py ~2903).
- **B2** hy_pnix_projection spawns 2 subprocesses (form + macroexpand) + duplicated
  model_to_json; merge into one spawn. B3 `@lru_cache` hy_python(); B5 `@lru_cache` parse().
  **[DONE 2026-06-30 — B2: `hy_form_and_macro_projection` returns both views from ONE
  Hy-proof spawn (output byte-identical to the union of the two old facilities);
  hy_pnix_projection rewired to it (~2.2x: 0.24s→0.11s). B3: `hy_python()` is now
  `@lru_cache`d. B5 (`@lru_cache parse`) intentionally SKIPPED — the toolkit's only hot
  parse is `_pyast.parse`/`rt.parse` which yield MUTABLE AST objects that callers walk
  (and `rt.parse` lives in sacred pnix_runtime.py); caching shared mutable trees risks
  corruption for negligible gain.]**

### C. Correctness (toolkit)
- **C1** strip the leading `import hy` compiler scaffolding before aligning — it injects a
  spurious `Import→import*` node into every Hy→pnix alignment (pnix_mirror.py ~657-690).
- **C2** is_macro/changed_1 compare Hy models via `repr()` (hy_mirror.py ~460); use `!=` on
  the models (they implement __eq__).

### D. Weave CPython + ecosystem into the meta-circular (researched path to "expand")
1. **Ecosystem absorption** — importlib MetaPathFinder/Loader (PEP 451) + import-time AST
   rewriting (MacroPy pattern: parse→AST→expand→compile in a sys.meta_path finder) ⇒ project
   ARBITRARY Hy/Python *modules* (not just snippets) into pnix terms at load time.
   [MacroPy github.com/lihaoyi/macropy; PEP 451]
2. **Live meta-circular observation** — PEP 669 `sys.monitoring` (3.12+, low-overhead
   per-(code,offset) events, return DISABLE) + PEP 578 audit hooks ⇒ trace what bytecode/
   events a pnix evaluation actually runs as, and fold that into the projection. `bytecode`
   lib (v0.18.1) for code-object round-trip. [PEP 669/578; bytecode.readthedocs.io]
   **[DONE 2026-06-30 — D2 pass below; realized via sys.settrace + f_trace_opcodes on
   Python 3.11, no sys.monitoring / external bytecode lib needed.]**
3. **Tower collapse = the peak (Futamura)** — pnix-hy is a self-hosting tower (4 substrates
   converge). The leap: COLLAPSE it into a compiler via staging. Amin & Rompf "Collapsing
   Towers of Interpreters" (POPL'18, Pink/Purple) + Lightweight Modular Staging (Rompf &
   Odersky, CACM'12) realize the Futamura projections with a CHECKABLE Jones-optimality
   property (compiling source = the program in ANF, no interpretive overhead) — a direct
   analogue/extension of pnix-hy's value-roundtrip. Grounding theory: Smith 3-LISP (1982) +
   Wand & Friedman "Mystery of the Tower Revealed". Self-optimizing route: PyPy RPython
   meta-tracing vs Truffle PE (Marr & Ducasse OOPSLA'15) — meta-tracing needs fewer hand
   optimizations.  KEY ADAPTATION (research caveat): LMS uses Scala types (Rep[T]); Python+Hy
   lacks that, so use a **tag-based binding-time discipline over pnix AST tags + Hy macros** —
   which pnix-hy already has (tagged AST tree + Hy-macro projection). That alignment is the
   opening.
   [amin-popl18.pdf; rompf-wf16.pdf; namin/pink; stefan-marr.de meta-tracing-vs-PE]

### IMPLEMENTATION GOALS — detailed, prioritized (2026-06-30)
Discipline for ALL items: additive to the projection toolkit (hy_mirror.py / pnix_mirror.py
/ bin), do NOT regress pnix_runtime.py or the 4-lane mirror; after each item run
`self_test_report` + that item's `*_report()`; commit+push per item.

- [x] **P1 — quality (3 small wins)** DONE 2026-06-30
  - [x] **C1** strip Hy compiler scaffolding before alignment. In `align_python_to_pnix`
    (and `_align_collect` / tree), drop leading `Import`/`ImportFrom` nodes whose module is
    `hy` or starts with `hy.` BEFORE walking, so alignments no longer show a spurious
    `Import→import*`. Accept: `align_hy_to_pnix("(defn f [x] (+ x 1))")` has no `import` in
    `pnix_tags_hit`; alignment_report still ready.
  - [x] **B4** stop triple-evaluating in `pnix_form_projection`: replace the
    `eval_source("builtins.toJSON ("+source+")")` re-parse with `rt.to_json_string_value`
    (pnix_runtime.py ~2903) applied to the already-computed interp value. Accept: same
    `value_json`, one fewer parse/eval; report ready.
  - [x] **A1** `match → (cond ...)` reverse synth in `_pnix_to_hy`: add a `match` arm that,
    for literal/`_`-pattern arms (no guard), emits Hy `(cond [test body] ... [True default])`
    or `#_pnix-match` gap when an arm has a guard/destructuring pattern. Accept:
    `match 1 with | 1 => "a" | _ => "b"` synthesizes a `cond`; guard arm → gap. Roundtrip a
    literal-pattern match meaning-preserving where feasible.
- [x] **P2 — speed: stage7 persistent worker (B1)** DONE 2026-06-30
  - [x] The ~184KB kernel recompiles on every `stage7_eval`. Build a long-lived worker:
    precompile the kernel once (cache the compiled module / .pyc) in the Hy proof Python,
    then feed fragments over a tiny stdin/stdout request protocol (one process reused across
    calls). Keep the current one-shot path as fallback. Accept: `--check` (which fires
    stage7 many times) is materially faster; all `*_report()` still ready; 4-lane mirror
    unchanged (cache must not change semantics). Bonus B2/B3/B5: merge form+macroexpand into
    one spawn, `@lru_cache` hy_python()/parse().
- [x] **P3 — coverage: host constructs in the correspondence map (A2)** DONE 2026-06-30
  - [x] Add correspondence rows + `_PY_CLASS_TO_TAG` entries (mostly `differs=True` with a
    note, NOT a fake pnix tag) for: Try/ExceptHandler/Raise (→ no pnix exceptions; tryEval/
    abort closest), context-manager With (→ no pnix equiv; distinct from pnix dynamic-scope
    `with`), ClassDef (→ no pnix equiv; attrset is the data analogue), ListComp/DictComp/
    SetComp/GeneratorExp (→ pnix genList / map+filter / listToAttrs), For/While (→ pnix is
    expression/recursion-based, no statement loop), Await/Yield (→ no pnix equiv). Keep
    `correspondence_report` live-validation green (only real pnix tags get a probe; new rows
    that map to "no pnix tag" use a null/sentinel that the report tolerates). Accept:
    `align_*` no longer report these as `unmapped`; report ready.
- [x] **P4 — module (multi-form) projection (closes A3; safe opt-in realization of the MacroPy/PEP-451 module-projection payload)** DONE 2026-06-30
  - [x] (realized as explicit file/source projection, not a global sys.meta_path hook) A `MetaPathFinder`/`Loader` (or a `source_to_code` override) that intercepts a target
    Hy/Python module at import and runs the projection per top-level form, producing a
    module-level projection artifact (reader forms → Python AST → pnix correspondence) for
    EVERY def/form in a real module — extends projection from snippets to whole modules.
    Opt-in (explicit register), never global. Accept: pointing it at a small sample module
    yields a per-form projection list; no effect when not registered.
- [x] **P5 — Futamura 1st projection prototype (tag-based partial evaluation) DONE 2026-06-30**
  - [x] Model on Pink (namin/pink) + LMS staging, adapted to Python+Hy via a **tag-based
    binding-time discipline over pnix AST tags** (no Scala Rep[T]): mark each pnix AST node
    static (known now) vs dynamic (runtime), then specialize the pnix interpreter w.r.t. a
    fixed pnix program to emit residual code with interpretive overhead removed (the 1st
    Futamura projection). Start tiny (arith + if + let + lambda/apply over a closed program).
    State + test a **Jones-optimality-style checkable property**: specializing a program's
    source yields code whose value == the program's value (extend `projection_value_roundtrip`
    into a `specialization_roundtrip`). Accept: a closed pnix program specializes to residual
    Python/Hy that evaluates to the same value; property-test over a small corpus. This is
    research-grade; land it incrementally behind its own `*_report()`.

- [x] **D2 — live meta-circular observation (dynamic execution trace)** DONE 2026-06-30
  - [x] The static toolkit (ast/dis/marshal/symtable) shows what a fragment lowers TO;
    D2 shows what the meta-circular evaluator actually DOES at runtime. Realized with
    `sys.settrace` + per-frame `f_trace_opcodes` (3.7+; `sys.monitoring` is 3.12+ and the
    verify env is 3.11, so settrace — no external `bytecode` lib needed):
    - `hy_mirror.execution_trace(source, mode)` — compile + run a Python/Hy-lowered
      expression under an opcode tracer restricted to OUR code objects (lib internals
      excluded), reporting the bytecode that genuinely executes (ordered sequence +
      histogram) vs the static `dis` listing (executed-vs-present coverage). Schema
      `pnix-hy.execution-trace.v0`; `execution_trace_report()`.
    - `pnix_mirror.pnix_evaluation_trace(source)` — the headline: trace the HOST pnix
      evaluation at the Python-opcode level restricted to `pnix_runtime`'s code objects,
      surfacing the real evaluator footprint (distinct functions / total calls / total
      opcodes + top functions + top opcodes). e.g. `a * b + 1` → 21 runs 62 runtime
      functions / 669 calls / 18379 opcodes (tokenizer `tok`, parser `parse_left` dominate;
      LOAD_FAST/CALL/COMPARE_OP top opcodes) — literally "pnix is evaluated by a
      Python-ecosystem program," now observable. Schema
      `pnix-hy.pnix-evaluation-trace.v0`; `pnix_evaluation_trace_report()`.
    - CLI `--trace SRC` (pnix evaluator footprint) + `--hy-trace SRC` (lowered-expression
      opcode trace); `--check` now runs 16 toolkit self-checks.
  - Gates: self_test 1103/1103; --check 16/16 all_ready; 4-lane mirror 541×4 ready,
    stage15/stagen closure reproduced (pnix_runtime.py untouched).

### Proposed order
1. Quality: C1 + B4 + A1 (match→cond).  2. Speed: B1 stage7 persistent worker.
3. Coverage: A2 (try/with/class/comprehension rows).  4. Expand-1: import-hook module
projection (MacroPy pattern).  5. Expand-2 (big): tag-based staging → Futamura 1st-projection
prototype (specialize the pnix interpreter w.r.t. a pnix program), modeled on Pink; state a
Jones-optimality-style checkable property for the Hy↔pnix roundtrip.

### EXPRESSIVENESS FRONTIER — beyond the audit/roadmap (2026-06-30)
With the deep-research paths (D1=P4, D2, D3=P5) and the code-audit gaps (A1/A2/A3,
B1–B5 except B5-skip, C1/C2) all closed, new work targets meta-circular FEATURES that
were never projected. Discipline unchanged: additive, gates green, commit+push per item.
- [x] **Hy quasiquote / unquote (code-as-data) projection** DONE 2026-06-30. The most
  fundamental unprojected Hy meta-circular feature. `hy_quasiquote_projection(source)`
  surfaces: template_kind (quote=fully static / quasiquote=template-with-holes), the
  reader model (quote/quasiquote/unquote/unquote-splice heads), the Python that
  CONSTRUCTS the hy.models tree, the unquote/unquote-splice HOLES (dynamic insertion
  points), and the pnix correspondence. KEY FINDING: `(+ 1 ~x ~@ys) lowers to
  `Expression([Symbol('+'), Integer(1), x, *(ys or [])])` -- a STATIC skeleton with
  DYNAMIC holes, which is the SAME binding-time structure pnix-hy's P5 partial evaluator
  exploits. pnix has no quasiquote (not homoiconic), but the honest analogue is the
  STAGING DISCIPLINE: quasiquote = manual staging; specialize_pnix = automatic staging
  (also mirrors pnix string interpolation "...${e}..."). Schema
  `pnix-hy.hy-quasiquote-projection.v0`; CLI --quasiquote; --check now 18 self-checks.
  Gates: --check 18/18 all_ready (pnix_runtime.py + 4-lane mirror untouched).
- [x] **Hy defmacro DEFINITION projection** DONE 2026-06-30. The meta-circular peak: a
  USER macro is a compile-time function over syntax. `hy_defmacro_projection(source)`
  surfaces, per `(defmacro ...)`: name/params, the body (a code template, usually
  quasiquote), the Python lowering, and the registration. KEY FINDING:
  `(defmacro inc [x] `(+ ~x 1))` lowers to
  `hy.macros.macro('inc')(lambda x: Expression([Symbol('+'), x, Integer(1)]))` -- a
  TRANSFORMER lambda (body = the quasiquote skeleton+holes from the prior item)
  REGISTERED by name and run BEFORE compilation. It also DEFENSIVELY demonstrates the
  form->form rewrite on any use forms (`(inc 41)` -> `(+ 41 1)`), recording an
  expand_error per-use instead of crashing when a macro emits an invalid form. pnix has
  no macro system; the honest analogue is STAGING: a macro is compile-time syntax->syntax,
  a pnix lambda is runtime value->value -- the compile-vs-runtime axis specialize_pnix
  automates and the body is the quasiquote template. Schema
  `pnix-hy.hy-defmacro-projection.v0`; CLI --defmacro; --check now 19 self-checks.
  Gates: --check 19/19 all_ready (pnix_runtime.py + 4-lane mirror untouched).
- [x] **Hy reader-macro (defreader / #name) projection — the READ-TIME stage** DONE
  2026-06-30. Completes the meta-circular STAGING TOWER. A reader macro runs at READ
  time, before any model tree exists. `hy_reader_macro_projection(source)` processes the
  source form-by-form with a live `HyReader` (use_current_readers + as_current_reader),
  so a `defreader` registers before the next form is read; it surfaces each defreader's
  lowering (a reader->model function registered via hy.macros.reader_macro), Hy's built-in
  dispatch table (( * ** [ ^ _ {), and the models the reader PRODUCES. KEY FINDING:
  `(defreader double (let [x (.parse-one-form &reader)] `[~x ~x]))` then `#double 21` is
  expanded BY THE READER into `[21 21]` before compilation. So Hy has THREE meta-stages --
  READ-time (reader macros, hy.macros.reader_macro, stream->model, EXTEND SYNTAX) ->
  COMPILE-time (defmacro, hy.macros.macro, form->form, REWRITE FORMS) -> RUN-time (eval,
  value->value). pnix collapses all three into a SINGLE eval stage; that read->compile->run
  collapse is exactly the tower-collapse Futamura/specialize_pnix (P5) realizes. Schema
  `pnix-hy.hy-reader-macro-projection.v0`; CLI --reader-macro; --check now 20 self-checks.
  Gates: --check 20/20 all_ready (pnix_runtime.py + 4-lane mirror untouched).

  ── STAGING TOWER NOW FULLY PROJECTED: read (reader-macro) -> compile (defmacro, via
  quasiquote code templates) -> run (eval, observable via D2 execution trace) -> COLLAPSE
  (P5 Futamura specialization). Every Hy meta-stage now has a pnix correspondence. ──
- [x] **Hy→pnix synthesis + forward value-roundtrip (direction balance)** DONE
  2026-06-30. The toolkit synthesized only pnix→Hy (`pnix_to_hy_form`) and value-checked
  only that direction (`projection_value_roundtrip`); align_* merely LABELS Python AST.
  Added the missing reverse: `synthesize_pnix_from_hy(hy)` walks a Hy fragment's Python
  lowering (expression subset: literals/var/binop/unaryop/compare/boolop/ifexp/list/lambda
  -curried/call-curried/dict→attrset) and emits actual pnix SOURCE, with honest
  `#_pnix-gap[...]` for the rest. `hy_to_pnix_value_roundtrip(hy)` then evaluates the Hy
  lowering (host Python -- Hy's semantics ARE the lowered Python) AND the synthesized pnix
  (host pnix) and compares via canonical JSON -- the symmetric counterpart of
  projection_value_roundtrip, proving the FORWARD projection preserves meaning. e.g.
  `((fn [x] (+ x 1)) 41)` → pnix `((x: (x + 1)) 41)` → 42==42; `(and True False)` →
  `(true && false)` → False==False. Schemas `pnix-hy.synthesize-pnix-from-hy.v0` /
  `pnix-hy.hy-to-pnix-value-roundtrip.v0`; CLI --synth-pnix / --hy-roundtrip; --check now
  22 self-checks. Both projection DIRECTIONS now have synth + value-roundtrip.
  Gates: --check 22/22 all_ready (pnix_runtime.py + 4-lane mirror untouched).
- [x] **Statement-level Hy (setv/defn) → pnix `let…in` synthesis** DONE 2026-06-30.
  Extended synthesize_pnix_from_hy from single-expression to module-level: a sequence of
  Python statements ending in an expression -> a pnix `let <bindings> in <result>`. `setv`
  (Assign, single Name) -> binding; `defn` (FunctionDef whose body is a single
  `return <expr>`, simple positional params) -> curried-lambda binding. pnix let is
  RECURSIVE so a defn body may reference any binding (mutual/forward), a superset of
  Python's sequential statements; honest gaps for rebinding, zero-arg def, non-simple
  params, or a module not ending in an expression. hy_to_pnix_value_roundtrip now execs the
  leading statements then evals the final expression for the Hy value. e.g.
  `(setv x 5)(defn inc [n] (+ n 1))(inc x)` -> `let x = 5; inc = (n: (n + 1)); in (inc x)`
  -> 6==6; `(defn add [a b] (+ a b))(add 3 4)` -> `let add = (a: (b: (a + b))); in
  ((add 3) 4)` -> 7==7. Both reports gained statement-level probes; --check stays 22/22.
  Gates: --check 22/22 all_ready (pnix_runtime.py + 4-lane mirror untouched).
- [x] **Projection CLOSURE (involution): the two synthesis directions compose** DONE
  2026-06-30. The capstone of the bidirectional synthesis: a stronger property than either
  single roundtrip -- it validates the toolkit's OWN consistency, that `pnix_to_hy_form`
  and `synthesize_pnix_from_hy` are mutually inverse on the clean subset (up to value).
  `pnix_projection_closure(pnix)`: pnix -> Hy -> pnix, value preserved across the full
  cycle. `hy_projection_closure(hy)`: Hy -> pnix -> Hy, value preserved. Both compare via
  canonical JSON; honest not-comparable when either leg hits a placeholder/gap. e.g.
  `(x: x * x) 9` -> `((fn [x] (* x x)) 9)` -> `((x: (x * x)) 9)` -> 81 (closed); and
  `(if (> 3 2) [1 2] [3])` -> pnix -> Hy returns the IDENTICAL text (a perfect involution).
  Schemas `pnix-hy.pnix-projection-closure.v0` / `pnix-hy.hy-projection-closure.v0`; CLI
  --closure / --hy-closure; --check now 24 self-checks.
  Gates: --check 24/24 all_ready (pnix_runtime.py + 4-lane mirror untouched).

  ── BIDIRECTIONAL PROJECTION NOW COMPLETE & SELF-CONSISTENT:
     pnix ⇄ Hy synthesis (both directions) + single value-roundtrip (both directions)
     + double-roundtrip closure/involution (both directions). ──
- [x] **Hy import / require projection — the cross-module dimension of the tower** DONE
  2026-06-30. `import` and `require` are the two cross-module scopes: `import` binds a
  Python MODULE OBJECT at RUN time; `require` loads another module's MACROS at COMPILE
  time. So the read→compile→run tower has a module axis too (require feeds compile, import
  feeds run). `hy_import_projection(source)` projects each form: kind, stage (run-time vs
  compile-time), the cleaned Python lowering (`(import json [dumps])` → `from json import
  dumps`; `(import math :as m)` → `import math as m`), structured imports, and the pnix
  correspondence -- pnix `import ./f.px` is an EXPRESSION evaluating a file to a pure VALUE
  (closest to Hy import, value vs module-object); pnix has NO `require` (no compile-time
  macro stage). A `require` of a missing module is recorded gracefully (stage=compile-time,
  needs_module), not raised. Schema `pnix-hy.hy-import-projection.v0`; CLI --import;
  --check now 25 self-checks.
  Gates: --check 25/25 all_ready (pnix_runtime.py + 4-lane mirror untouched).
- [x] **Unified META-CIRCULAR TOWER report (integration capstone)** DONE 2026-06-30.
  One call = a Hy fragment's whole meta-circular journey, organized as the staging tower:
  READ (reader-form tree) -> COMPILE (macro tower + Python lowering) -> RUN (the bytecode
  that actually executes, via execution_trace) -> PNIX (synthesized pnix + value-roundtrip
  + closure) -> COLLAPSE (specialize_pnix: the stage tower folds to one value when the
  program is closed). `meta_circular_tower(hy)` orchestrates the session's facilities into
  this single research artifact. e.g. `(+ (* 2 3) 4)`: 1 reader form; Python `2 * 3 + 4`;
  RUN shows only 2 opcodes because PYTHON ITSELF constant-folds the closed expr to 10 (a
  nice meta-circular observation: Python folds just as specialize does); synth pnix
  `((2 * 3) + 4)`, value preserved + closed; COLLAPSE specializes to `10` (fully static, no
  residue). Schema `pnix-hy.meta-circular-tower.v0`; CLI --tower; --check now 26 self-checks.
  Gates: --check 26/26 all_ready (pnix_runtime.py + 4-lane mirror untouched).
- [x] **f-string + let/do/raw-string Hy → pnix synthesis** DONE 2026-07-01. Closed the
  remaining synthesis gaps: a Python f-string (`JoinedStr`) now synthesizes to pnix string
  interpolation -- each hole wrapped in `builtins.toString (...)` because Python f-strings
  apply str() while pnix interpolation requires a string (so int/string holes preserve
  value; bool/float coercion differs and is left to SURFACE in the value-roundtrip, not
  hidden; `!r`/`!a`/format-spec -> honest gap). e.g. `f"x={(+ 1 2)}"` ->
  `"x=${builtins.toString ((1 + 2))}"` -> "x3" both sides; multi-hole preserved. Found that
  `(do ...)` (-> `let ...; in result`), `(let [x 1 y 2] ...)` (-> gensym'd `let`, value
  preserved), and raw strings `#[[...]]` (-> string literal) ALREADY synthesize+roundtrip
  via the statement-level path. `b""` (bytes) stays a genuine gap (pnix has no bytes).
  Gates: --check 31/31 all_ready (pnix_runtime.py + 4-lane mirror untouched).

### ▶▶ PRODUCTION HARDENING — make the projection toolkit deployable (2026-06-30)
The research frontier is essentially complete (staging tower, bidirectional synthesis +
closure, tower report; 26/26 --check). The stated goal is now PRODUCTION use. These items
turn the toolkit into a deployable library WITHOUT changing the mission. Discipline
unchanged: additive (prefer the pnix_mirror/hy_mirror/bin wrapper layer), do NOT regress
pnix_runtime.py or the 4-lane mirror; each item: verify + its own `*_report()` + commit/push.
Mapped to the production use-cases discussed (safe untrusted eval / DSL specialization /
config generation / audit).

- [x] **PP1 — Safe evaluation sandbox** DONE 2026-06-30. `safe_eval(source, *, timeout_s,
  max_steps, max_output_bytes)` WRAPPER in pnix_mirror (pnix_runtime untouched): a step/fuel
  budget + wall-clock timeout enforced via sys.settrace over the runtime's OWN frames (so it
  interrupts CPU-bound eval, not just lazy recursion) + an output-size cap, returning a
  structured verdict {ok, value, value_json, limit_exceeded, error, steps, elapsed_s} --
  never hangs/raises out. Verified: `1+2*3`→7; tiny max_steps→max_steps; tiny
  max_output_bytes→max_output_bytes; `let f = x: f x; in f 1`→recursion; heavy finite eval
  +0.05s timeout→timeout @0.056s. Schema `pnix-hy.safe-eval.v0`; CLI --safe-eval; --check
  now 27 self-checks. Gates: --check 27/27 all_ready (pnix_runtime.py + 4-lane mirror
  untouched). NEXT: PP2 adds pure_only via the impurity classifier.
- [x] **PP2 — Pure-only mode / impurity classifier** DONE 2026-06-30.
  `static_purity_check(source)` walks the parsed pnix AST and flags `import` (tag) +
  `builtins.<impure>` selects (getEnv/readFile/readDir/readFileType/pathExists/hashFile/
  toFile/storePath/scopedImport/fetch*/currentTime/... ) as impure, and `with builtins;`
  as `uncertain` (bare injected names can't resolve statically); `pure` only when neither.
  `safe_eval(..., pure_only=True)` rejects impure programs BEFORE eval
  (limit_exceeded="impure"). Verified: config expr pure; readFile/getEnv/import flagged;
  pure_only blocks getEnv, allows arith. Schema `pnix-hy.static-purity-check.v0`; CLI
  --purity; --check now 28 self-checks. Gates: --check 28/28 all_ready (pnix_runtime.py +
  4-lane mirror untouched). Together PP1+PP2 = a provable pure resource-bounded sandbox.
- [x] **PP3 — Persistent Hy projection worker** DONE 2026-06-30. All 9 projection
  facilities (form / form+macro / macroexpand / macro-step / quasiquote / defmacro /
  reader-macro / import / module) now route through `_run_hy_script` -> one warm Hy worker
  that imports hy ONCE and runs each facility's script on demand (receives {script,source},
  execs with stdout captured, returns the JSON). Falls back to one-shot on any miss; set
  PNIX_HY_NO_PROJECTION_WORKER=1 to force one-shot. BUG fixed during impl: `python
  /tmp/worker.py` puts the SCRIPT dir on sys.path[0], not cwd, so `import hy` failed ->
  added `sys.path.insert(0, os.getcwd())` (one-shot `python -` got cwd via sys.path[0]=''
  for free). Result: warm repeated projection 132ms->1ms/call (~107x; removes the
  process-startup + import-hy overhead); --check 34.9s vs 43.3s one-shot; output
  byte-identical (worker==oneshot verified). Gates: --check 28/28 all_ready (pnix_runtime.py
  + 4-lane mirror untouched).
- [x] **PP4 — Content-addressed eval cache** DONE 2026-06-30. `cached_eval(source)`
  memoizes pnix evaluation by CANONICAL content (key = `rt.emit_source(rt.parse(source))`,
  so `1+2`/`1 + 2`/`1  +  2` share one entry), gated by PP2's purity check so ONLY pure
  programs are cached (impure values can change between runs). Sound because pure pnix is
  deterministic. + `eval_cache_stats()` / `clear_eval_cache()`. Verified: pure misses then
  hits (same value), whitespace-variant hits the same entry, impure is uncacheable;
  expensive deterministic eval 1268ms -> 0.6ms (~2171x). Schema `pnix-hy.cached-eval.v0`;
  --check now 29 self-checks. Gates: --check 29/29 all_ready (pnix_runtime.py + 4-lane
  mirror untouched). (Projection caching can reuse the same key pattern later if needed.)
- [x] **PP5 — Stable public API + packaging** DONE 2026-06-30. `pnix_hy/__init__.py`
  curates a 28-name public API (`__version__` 0.1.0 + production safe_eval/purity/cache +
  bidirectional projection + tower/capstones + specialization/observation) with `__all__`;
  `import pnix_hy` works on CORE alone (pure stdlib, no Hy needed — Hy is only the
  out-of-process proof Python for projection). CLI moved to `pnix_hy/cli.py` (importable;
  `main(argv=None)` defaults to sys.argv for console_scripts), `bin/pnix-hy-project` is now
  a thin shim; both it and `python -m pnix_hy.cli` work. `pyproject.toml` (setuptools):
  console_scripts `pnix-hy-project = pnix_hy.cli:main`, `[projection]` extra = hy==1.3.0,
  no core deps. README.md added. Verified: import + 28 exports resolve; core API works
  without Hy; console_scripts main() simulation; pyproject parses. Gates: --check 29/29
  all_ready (pnix_runtime.py + 4-lane mirror untouched).
- [x] **PP6 — User-facing diagnostics with source positions** DONE 2026-06-30.
  `diagnose(source)` returns a structured {ok, phase (parse|eval), message, line, column,
  offset, excerpt} (pnix_runtime untouched). Parse-error messages embed the offending
  token's char offset (`pos=N`); we map it to (line, column) against the source and render a
  caret excerpt -- precise positions even multi-line (verified: `1 +`->L1C3;
  `let a=1\n  b=2;\n  in a`->L2C3 with caret). Eval errors get the message (pnix attaches no
  node position to them) -- honest. For any user-supplied-logic / DSL UI. Schema
  `pnix-hy.diagnose.v0`; CLI --diagnose; public API + --check now 30 self-checks.
  Gates: --check 30/30 all_ready (pnix_runtime.py + 4-lane mirror untouched).
- [x] **PP7 — DSL specialization productization** DONE 2026-06-30. Extended the P5 partial
  evaluator `_pe` with CLOSURES + beta-reduction: a `lambda` node becomes a static
  `_Closure` (param/body/captured-env); `apply` with a static argument beta-reduces (bind
  param->arg, specialize the body), so closed function-heavy programs collapse FULLY.
  Verified: `(x: x + 1) 41` -> 42 (was residualized); `((a: (b: a + b)) 3) 4` -> 7 (curried
  closures fold); a bare `(x: x * x)` -> residual `(fn [x] (* x x))`; dynamic-arg apply
  residualizes correctly `((fn [x] (+ x 1)) y)`; existing cases unchanged (3, `(+ 1 x)`).
  Plus `specialize_pnix` now MEMOIZES by (canonical source, dynamic_vars) -- deterministic,
  so the "compile a fixed DSL program once, exec many" path is a cache hit (verified first
  miss / second hit). specialization_roundtrip still meaning-preserving. --check stays 30/30.
  Gates: --check 30/30 all_ready (pnix_runtime.py + 4-lane mirror untouched).
- [x] **PP8 — Audit/receipt bundle** DONE 2026-07-01. `eval_receipt(source)` -> a
  DETERMINISTIC reproducibility receipt: canonical emit + `source_sha256`, value +
  `value_sha256`, purity, the 4-LANE CONVERGENCE verdict (host interp/compiler + stage7
  runtime/compiler all agree -- strong correctness evidence), and the execution footprint
  (functions/calls/opcodes from pnix_evaluation_trace). No timestamp -> the same program
  yields the same receipt (that IS reproducibility). Verified: `price * qty` -> value 300,
  4-lane converged (all 300), pure, stable hashes across two calls. For regulated /
  auditable / reproducible-science use. Schema `pnix-hy.eval-receipt.v0`; CLI --receipt;
  public API + --check now 31 self-checks. Gates: --check 31/31 all_ready (pnix_runtime.py
  + 4-lane mirror untouched).

  ── PRODUCTION HARDENING COMPLETE (PP1-PP8): provable pure resource-bounded sandbox
  (PP1+PP2), warm-worker + content cache + specialization cache (PP3+PP4+PP7), packaged
  public API/CLI (PP5), positioned diagnostics (PP6), reproducibility receipts (PP8). ──
- [x] **Production ship-gate (`--gate`)** DONE 2026-07-01. One command consolidates the
  SACRED foundational gates with the toolkit into a single PASS/FAIL (exit 0/1) for deploy:
  runtime self-test (1103/1103), rust corpus (1256/1256), the 4-lane mirror (541x4) +
  stage15/stagen closure (reproduced), and all 31 toolkit self-checks. Heavyweight (~2:48,
  builds the stage7 kernel for the mirror); `--check` stays the fast toolkit-only gate.
  `cmd_check`/`cmd_gate` share a `_toolkit_reports()` helper (no duplication). Verified:
  `--gate` -> all PASS, GATE: PASS. (Also removed a dead `_emit` helper from the CLI.)

### ▶▶ SEPARATION EXECUTION — hy-meta (host) vs pnix-hy (pnix runtime) (2026-07-01)
Plan of record: `docs/SEPARATION.md` (line-referenced). hy-meta side: `hy-meta/todo.md`
§ "SEPARATION: receive host machinery". Correction: meta-circular ≠ mirror; the pnix mirror
must be a SINGLETON. Discipline unchanged: each step gated (`--check` 31/31; for runtime-
touching steps also `--gate`); pnix_runtime.py + 4-lane mirror must not regress.

- [x] **SEP1 — relocate HOST introspection to hy-meta** DONE 2026-07-01. The
  `hy_mirror.py` L1941–2382 block (compile_source/code_object_info/disassemble/
  execution_trace/marshal_code/rebuild_code/ast_info/symtable_info/tokenize_info/frame_info/
  gc_info/sys_info/module_info/full_introspection + helpers) is host machinery, not pnix
  runtime. Moved to `hy-meta/host_introspect.py` (hy-meta SR1); `hy_mirror.py` path-imports
  it and re-exports for backward compat (the 31 reports + meta_circular_projection +
  execution_trace + mirror_full_introspection seam keep working). Verify: `--check` 31/31.
- [x] **SEP2 — delegate host exec to hy-meta** DONE 2026-07-01. `pnix_runtime.py` routes
  emitted Python execution through `hy-meta/host_exec.py`, and the optional external oracle
  now asks `hy-meta/clean_replay.py` first before falling back for standalone operation. The
  pnix→Python EMITTER (`_px_*`) STAYS in pnix-hy. Verified with full `--gate`.
- [x] **IB1+IB2 — explicit interop protocol (record + value mapping + opaque refs)** DONE
  2026-07-01. New `pnix_hy/interop.py` (was missing entirely): `InteropRecord` (IB1:
  interop_id/direction/source+target lang/input+output kind/loss-status/effect-class/
  capability-required/witness-id) with `LOSS_STATUSES`/`EFFECT_CLASSES`; `to_host`/`from_host`
  (IB2 value mapping: pure data crosses lossless; pnix functions + host callables/modules/
  objects become OPAQUE REFS that must not enter pnix terms) + an opaque-ref registry
  (`make_opaque_ref`/`resolve_opaque`/`release_opaque`) + a `check_capability` effect gate.
  Works with mirror OFF. CLI `--interop`; public API; `--check` now 32. Verified: attrset
  -> dict lossless/pure; `x: x+1` -> opaque-ref opaque/host-call (cap enforced by the gate);
  data roundtrips lossless; opaque resolves back to the live object. Gates: --check 32/32
  (pnix_runtime.py + 4-lane mirror untouched).
- [x] **IB3+IB4 — callable + module bridges** DONE 2026-07-01. Added to `pnix_hy/interop.py`,
  REUSING existing runtime primitives (no reinvention): `apply_pnix`/`eval_source_raw(realize=
  False)`/`Thunk`/`force_value` + the IB1/IB2 opaque registry. IB3: `pnix_callable(source)` /
  `wrap_pnix_callable(closure)` wrap a pnix function as a HOST callable (from_host args ->
  apply, curried -> to_host result); `call_host(fn, args, granted=)` invokes a HOST callable
  from pnix, capability-gated + exception-captured. IB4: `host_module_to_pnix(module)` (public
  attrs -> pnix values/opaque refs) / `pnix_module_to_host(attrset)` (functions -> callable
  wrappers, data -> host values). Verified: `(x:x*x)(9)=81`, curried `(a:b:a+b)(3)(4)=7`,
  `call_host(len,[1,2,3,4])=4` (effect host-call), denied without capability, module bridge
  `inc(10)=11`. (hy-meta SR5 host adapter would add isolation/governance later; v0 invokes
  in-process via the opaque registry.) Gates: --check 36/36 (additive; sacred untouched).
- [x] **SEP3 (v1) — singleton `mirror_run(source, opts)`** DONE 2026-07-01. New
  `pnix_hy/mirror.py`: ONE canonical observation entrypoint that funnels a single pnix
  evaluation through one route and emits the observations as TRACE FACETS
  (`:mirror/source|token|ast|ir|effect|value|interop|eval-step|witness`, or `:mirror/error`),
  producing ONE run_id (= program content hash, deterministic) + ONE result_sha256 witness.
  ADDITIVE: the 4-lane parity mirror (`self_test_report`) stays as the convergence GATE.
  Follow-up SEP3-v2 made `run_once` / `mirror_chain` / `run_mirror` / `stage_tower` thin
  projections over the shared singleton core, so the old local mirror surfaces no longer own
  parallel parse/eval implementations.
  CLI `--mirror`; public API; `--check` now 33. Verified: 9 facets emitted, value 21,
  deterministic run_id/result across two runs, parse error -> error facet (no throw).
  Gates: --check 33/33 (pnix_runtime.py + 4-lane mirror untouched).
  - [x] SEP3 (v2) — collapsed old local mirror surfaces into the shared singleton core
    while preserving the 4-lane convergence gate.
- [x] **SEP4 — pnix runtime stage ladder** DONE 2026-07-01. New `pnix_hy/stage.py`
  `pnix_stage_ladder(source)`: 7 host-only fast stages, each agreeing on the value with
  stage1 — pnix-stage1 direct eval, stage2 normalized-AST eval (`eval_normalized_source`),
  stage3 content-addressed store eval (`cached_eval`), stage4 AST roundtrip integrity
  (`ast_hash` stable across parse→emit→reparse), stage5 singleton `mirror_run` route,
  stage6 deterministic replay (in-process; full subprocess replay = hy-meta SR3), stage7
  runtime closure = interpreter (`eval_source`) == compiler (`run_px_source`) [host 2-lane;
  full 4-lane stays the `--gate` proof]. Distinct from hy-meta stage8/9 (which prove the
  HOST compiler). CLI `--stage-ladder`; public API; `--check` now 34. Verified: `inc 41` ->
  42, all 7 stages PASS. Gates: --check 34/34 (pnix_runtime.py + 4-lane mirror untouched).
- [x] **SEP5 — gates + witnesses** DONE 2026-07-01. New `pnix_hy/gate.py`: a CAPABILITY-
  AWARE gate built on PP2's classifier — `EFFECT_OF` maps each impure builtin to an effect
  class (file-read/file-write/host-call/import/network), and `gate_check(source, granted=)`
  admits a program only if every required effect is granted (finer than all-or-nothing
  pure_only); `with builtins` -> uncertain -> denied. `make_witness(kind, payload)` =
  deterministic content-hashed witness (no timestamp -> reproducible; key-order
  independent). CLI `--gate-check 'SRC ;; file-read,import'`; public API; `--check` now 35.
  Verified: pure admits with no caps; `readFile` denied without file-read / admitted with
  it; `import` needs import; witness deterministic. Gates: --check 35/35; full `--gate`
  PASS (runtime 1103/1103, rust 1256/1256, 4-lane 541x4, closure reproduced) -- the SACRED
  invariant intact after ALL separation work.

- [x] **§3.2 IR layer** DONE 2026-07-01. New `pnix_hy/ir.py`: explicit pnix IR =
  the NORMALIZED (position-free, structurally canonical) AST -- and crucially DIRECTLY
  EVALUABLE (`eval_from_ast(ir)`) and value-equivalent to evaluating the source, so it is a
  genuine canonical RUNTIME representation, not a relabeled AST. `lower_to_ir` / `ir_of`
  (content hash `ir_sha256`) / `eval_ir` / `ir_roundtrip` (eval(IR)==eval(src) +
  hash-stable). Decision (default, since pnix is a small core language): IR = normalized
  core AST; host Python emission (`_px_*`) is the EXECUTION artifact, NOT the IR; further
  desugaring (path-folding, sugar expansion) is a documented future refinement.
  `mirror_run`'s `:mirror/ir` facet now carries `ir_sha256`. CLI `--ir`; public API;
  `--check` now 36. Verified: 4 probes evaluable+meaning-preserved+hash-stable, deterministic.
  Gates: --check 36/36 (pnix_runtime.py + 4-lane mirror untouched).

### ▶▶ SEPARATION — remaining (deliberately deferred: sacred-touching or cross-side)
The 5 SAFE additive steps are done (SEP1, IB1+IB2, SEP3-v1, SEP4, SEP5), all gated, `--gate`
PASS. The rest touch the SACRED pnix_runtime.py / 4-lane mirror, or need paired hy-meta
host-adapter work — do each DELIBERATELY with `--gate`, not rushed:
- [x] **SEP2 — host execution delegated to hy-meta** DONE 2026-07-01. The compiler lane's
  main host exec (`run_px_source_raw`, was `exec(compile(code, filename, "exec"), namespace)`
  at 12107) now routes through `hy-meta/host_exec.py:run_python_source` via a surgical
  `_host_exec_source` helper (path-imports the hy-meta floor; falls back to a byte-identical
  inline compile/exec if hy-meta is absent, so pnix_runtime still runs standalone). The
  pnix->Python EMITTER (`_px_*`) STAYS in pnix-hy (it is pnix's compiler); only host
  EXECUTION is delegated -- so "pnix-hy uses hy-meta as its host floor" now holds for code
  execution too. Verified: compiler lane = 21, interp==compiler, host-exec floor actually
  loaded (not fallback). Gates: full `--gate` PASS -- runtime 1103/1103, rust 1256/1256,
  **4-lane 541x4 (compiler_parity intact)**, closure reproduced, toolkit 35/35. (The tiny
  import-shim exec at 12046 and the external-oracle `subprocess` at ~12279 are left in place
  -- internal shim / optional out-of-repo oracle, not the compiler lane's program exec.)
- [x] **SEP3-v2** DONE 2026-07-01. Collapsed the old local pnix mirror surfaces into the
  singleton mirror core: `pnix_mirror.singleton_mirror_run` now owns the one parse/emit/
  reparse/eval-with-native-events route and emits the facets; public `mirror_run` delegates
  to it, while `run_once` / `mirror_chain` / `run_mirror` / `stage_tower` are legacy views.
  Dedupe: removed the extra `mirror_run` reparse/reeval and reused the parsed AST for the
  effect facet. Verified with full `--gate`: runtime 1103/1103, rust 1256/1256, 4-lane
  541x4, closure reproduced, toolkit 37/37.
- [x] **IB3+IB4** callable + module bridges DONE 2026-07-01. pnix-side callable/module
  bridges are implemented, and after hy-meta SR5 the host callable path now prefers
  `hy-meta/interop.py` for opaque refs, calls, exception capture, and witnesses.
- [x] **hy-meta SR2–SR6** (host side) DONE 2026-07-01: folded SR1 introspection into the
  artifact module set and exposed host-artifact / clean-replay / pnix-import-hook /
  interop-adapter / witness APIs through `hy-meta/hy_meta.py`; `pnix_hy.interop` now
  prefers the hy-meta host adapter, and the optional external Rust oracle asks hy-meta
  clean replay first.

NOTE on the ~59 unchecked boxes BELOW the PURPOSE banner: those are PRE-MISSION (error-
message/`eval_*.rs` corpus alignment to `~/pnix-old`, `.px` module breadth, stage7 emitter
re-sync, benchmark harness). They are historical/optional, NOT the Hy↔pnix projection
mission, and NOT required for production of the toolkit. Pull from them only if a specific
production need (e.g. PP-perf benchmark = old "G", `.px` library loading = old "E") calls.

## ▶▶ CAPABILITY ALLOCATION — pnix-hy (PNIX runtime meta-circular) + interop (2026-07-01)

Allocation of the "Pure Meta-Circular Capability Checklist" (§1–§24) to the PNIX-runtime
lane. Host (Hy/Python) capabilities are in `hy-meta/todo.md` (§1, §2.1-2, §3.1-3, §4, §5,
§6.1-2, §10.1/.3, §11, §16, §17, SR4/SR5/SR6). Rule: **pnix-hy owns the pnix language
runtime; the boundary is interop.** Status: ✅ exists / ◑ partial-deferred. DO NOT
re-implement ✅/◑ — extend/reuse the cited symbol (the "search first" discipline).

- [✅] **§2.3 pnix reader/tokenizer/parser** — `pnix_runtime.{tokenize,Parser,parse,
  source_position_value}` + `pnix_form_projection`; reader-error reification via `diagnose`.
- [✅] **§3.4 pnix AST / IR** — `pnix_runtime.{parse,ast_stable,ast_hash,emit_source}` +
  `pnix_hy/ir.py` (`lower_to_ir/ir_of/eval_ir/ir_roundtrip`; IR = canonical normalized AST,
  directly evaluable, content-hashed). Host Python emission (`_px_*`) = execution artifact, not IR.
- [✅] **§6.3 pnix runtime mirror (SINGLETON)** — `pnix_hy/mirror.py:mirror_run` /
  `pnix_mirror.singleton_mirror_run` (one route + facets); `run_once/mirror_chain/run_mirror/
  stage_tower` are legacy views.
- [✅] **§7 reification (PNIX: source/form/ast/ir/value/effect)** — `reify_pnix`
  projects the existing `singleton_mirror_run`, `pnix_runtime`, `ir.ir_of`, effect, interop,
  and witness evidence; no second parser/evaluator/mirror.
- [✅] **§8.2 pnix eval / §8.3 apply-pnix** — `pnix_runtime.{eval_source,eval_ast,
  eval_from_ast,apply_pnix}`; gated eval via `safe_eval`; apply-host-method via interop §18.
- [✅/gap] **§9 quote/quasiquote/macro** — pnix has NO macro/quasiquote (documented honest gap;
  not homoiconic). The Hy-side is OBSERVED by pnix-hy's projection toolkit
  (`hy_quasiquote_projection/hy_defmacro_projection/hy_reader_macro_projection/
  hy_macro_step_trace`) — observation is pnix-hy's mission, the Hy machinery is hy-meta's.
- [✅] **§10.2 pnix import hook** — pnix `.px` SEMANTICS are pnix-hy's; host `sys.meta_path`
  integration = hy-meta SR4. `interop.install_pnix_import_hook` wires `pnix_runtime.run_px`
  into `hy-meta/import_hook.py`; verified by `pnix_import_hook_report`.
- [✅] **§12 determinism/drift (PNIX)** — `cached_eval` (content-addressed, pure-only),
  `ir_roundtrip` hash-stability, `mirror_run` deterministic run_id/result_sha256.
- [✅] **§13 roundtrip (PNIX)** — `projection_value_roundtrip`, `hy_to_pnix_value_roundtrip`,
  `pnix_projection_closure`, `hy_projection_closure`, `ir_roundtrip`, plus
  `roundtrip_status` / `ROUNDTRIP_STATUS_VOCAB` (`lossless/lossy-ok/held/rejected`).
- [✅] **§14 witness/proof (PNIX)** — `gate.make_witness`, `mirror_run` `:mirror/witness`,
  `eval_receipt`. SHARED FIELD SCHEMA with hy-meta SR6 — keep field names identical.
- [✅] **§15 gate/capability/sandbox (PNIX)** — `gate.py` (`gate_check`+`EFFECT_OF`),
  `safe_eval` (timeout/steps/output + `pure_only` + per-capability `granted=...`),
  `static_purity_check`.
- [✅] **§18 host interop (PNIX side) / §19 opaque boundary** — `pnix_hy/interop.py`
  (`InteropRecord`, `to_host`/`from_host`, `call_host`/`pnix_callable`/`wrap_pnix_callable`,
  module bridges, opaque registry) — prefers the hy-meta SR5 host adapter
  (`__hy_meta_opaque__`), falls back to local refs. The shared envelope with hy-meta.
- [✅] **§20 cache (PNIX)** — `cached_eval` + `_specialize_cache` + IR hash.
- [✅] **§21 debug/explain (PNIX)** — `diagnose`, `eval_receipt`, `meta_circular_tower`,
  plus unified `explain_pnix`.

NOTE: Host capabilities (§1, §2.1-2, §3.1-3, §4, §5, §6.1-2, §10.1/.3, §11, §12-host,
§13-host, §16, §17, §20-host, §21-host, SR4/SR5/SR6) are hy-meta's. SHARED envelope (only):
§14 witness FIELD SCHEMA + §18/19 opaque-ref shape. codex implements only remaining ◑; do not duplicate ✅.

## ▶▶ AUDIT FOLLOW-UPS — pnix-hy / interop (from docs/IMPLEMENTATION_AUDIT.md, 2026-07-01)

23-agent verified audit of §1-§24. Actionable PNIX/interop items, ordered. REUSE the cited
symbol; do NOT rebuild. Audit verdict: pnix mirror is a genuine SINGLETON
(`pnix_mirror.singleton_mirror_run`), 4-lane stays the separate convergence gate, and there
is NO competing second runtime/mirror/introspection.

### A. Stale markers to FLIP (work already done — verified)
- [x] §7 reify → ✅: `pnix_mirror.py:reify_pnix`(L2876) + `cli.cmd_reify` + `__init__` wired.
- [x] §13 roundtrip-status → ✅: `pnix_mirror.py:ROUNDTRIP_STATUS_VOCAB`(L24) + `roundtrip_status`
  (L2949), applied to projection + IR roundtrips.
- [x] §15 per-capability granting → ✅: `safe_eval` threads `granted=` via `gate.gate_check`
  (L2743-2758, test L2835-2836).
- [x] §21 explain (pnix side) → ✅: `pnix_mirror.py:explain_pnix`(L2999) + `cli.cmd_explain`.
- [x] §18 apply-host-method: now implemented as `interop.call_host_method` /
  `apply_host_method`, backed by hy-meta `interop.call_method`.

### B. Duplication to RECONCILE (single source)
- [x] **effect/impure vocab (§15/§18):** make `gate.py:EFFECT_OF`(L18-27) DERIVE from
  `interop.py:EFFECT_CLASSES`(L31-34) (the comment claims shared but it does NOT import it);
  done via `interop.IMPURE_BUILTIN_EFFECTS` / `IMPURE_BUILTINS`; `exec`→`subprocess`,
  `getFlake`→`network`. One table, two views.
- [x] **witness emitter (§14):** `gate.py:make_witness`(L59) should DELEGATE to the canonical
  host emitter `hy-meta/witness.py:make_witness`(L18) via `interop._host_interop`(L70); keep the
  `pnix-hy.witness.v0` schema only for standalone pnix-native fallback records. `gate_report`
  verifies the active schema is `hy-meta.witness.v0`.

### C. Missing / partial PNIX + interop capabilities (target file + reuse)
- [x] **interop.call_host_method (§18) — thin.** Add pnix `call_host_method(ref, name, *args)`
  over the host `call_method` (hy-meta, see hy-meta/todo.md C); REUSE `interop.check_capability`
  layering, host `call_method`, `resolve_opaque`; covered by `interop_report`.
- [x] **opaque local-fallback witness_id (§19) — thin.** `interop.make_opaque_ref` else-branch
  now attaches a `gate.make_witness`-derived `witness_id`.
- [x] **opaque_ref_id accessor (§19) — thin.** Trivial wrapper returning
  `ref['__hy_meta_opaque__']`/`ref['__pnix_opaque__']`. No new state.
- [x] **inspect_opaque pass-through (§19) — thin/optional.** REUSE `hy-meta/interop.py:
  inspect_object`(L54) via `_host_interop`(L70).
- [x] **wire .px import hook to host SR4 (§10.2) — thin-ish.** In `interop.py` add an adapter
  installing the host meta-path finder for `.px`; REUSE `hy-meta/import_hook.py:
  install_pnix_import_hook`(L118) via `_host_interop`, and `pnix_runtime.import_value`(L4341)/
  `read_px_file`(L4324). `PnixModuleLoader.exec_module` already stores `__pnix_result__`
  (import_hook.py L40-41) → value-import maps with no loader change.

### D. Doc fixes (cheap)
- [x] `pnix_hy/ir.py` docstring header: "(SEP §3.2)" → "§3.4" (pnix AST/IR is §3.4).
- [x] `docs/SEPARATION.md` line-number drift: host_introspect relocation is DONE
  (`hy_mirror._load_host_introspect` L1998); seam now `mirror_full_introspection` L2039 /
  `introspection_parity` L2047; §1.4 nums → `diagnose` L3157, `eval_receipt` L3221,
  `meta_circular_tower` L2067, `pnix_evaluation_trace` L2541.
- [x] (cosmetic, optional) `hy_mirror.py:model_to_json` defined 9× INSIDE separate worker-script
  string constants (L546/648/800/906/1054/1223/1404/1572/1741) — NOT a competing impl (each is
  its own subprocess); optionally hoist to one shared snippet constant.
  Hoisted to `_HY_MODEL_TO_JSON_SNIPPET`; each worker script concatenates the shared snippet.

### E. Deferred (by design)
- §9 pnix quote/quasiquote/hygiene: pnix has no macro/quote construct — intentionally absent
  (only Hy-OBSERVATION projections exist). Not a gap unless the pnix language grows macros.

### F. Re-verification residuals (2026-07-01 adversarial pass — all CODE follow-ups confirmed done)
Verdict: 44/44 --check, gate/interop/host reports `ready:True`, separation intact, pnix mirror
still singleton, pnix_runtime untouched, NO code regression, NO new competing impl. Only:
- [x] (by-design fallback + drift-guard) DONE 2026-07-01. `gate.py:WITNESS_FIELD_SCHEMA`/
  `_witness_fields` is the intended pnix STANDALONE fallback (host emitter preferred via
  `interop._host_interop`). Added a behavioral drift-guard in `gate_report`: when the host
  emitter is present, the pnix fallback's shared field set MUST be a subset of the host
  witness's keys (`witness_schema_ok`), so the two shared §14 schemas cannot silently diverge
  — mirrors the existing `vocab_ok` guard. Verified: `witness_schema_ok: True`, --check 44/44.
- [x] doc drift fixed: `docs/IMPLEMENTATION_AUDIT.md` got a "Re-verification update" header
  (the §14 witness/missing-witness rows are now RESOLVED: `host_exec.compile_artifact_witness`
  exists); `docs/SEPARATION.md` notes inline Lxxxx are point-in-time (symbols authoritative).

ORDER: A (flip markers, free) → B (vocab + witness delegation) → C thin (call_host_method,
opaque witness_id/ref_id/inspect, .px hook wiring) → D doc fixes. Each: add/extend its
`*_report`; keep `--check` green; only the .px-import wiring may warrant `--gate`.

## ▣▣▣ PROJECT PURPOSE — READ FIRST (authoritative, 2026-06-30) ▣▣▣

**pnix-hy exists for exactly one thing: projecting LANGUAGE EXPRESSIVENESS between
`Hy/Python` ↔ `pnix`.** Nothing else. The real relation is broader than just Hy:

> **the meta-circular capability of the WHOLE Python-language-based ecosystem
> (`Hy`, `Python` itself, and anything built on the Python language) ↔ pnix's
> meta-circular capability.**

Care only about that projection.

**What this project IS:**
- A **human-coded** (NOT AI/agent-generated) research vehicle.
- Its purpose is to **discover and extract the features of meta-circular evaluation** —
  pulling out as much as possible of the **Python-ecosystem meta-circular** capability
  (Hy reader/macros/compile, Python AST/symtable/bytecode/code-object/marshal/eval/load)
  and **pnix's meta-circular** capability, so a **developer can study/research** them.
- The work is: take a Hy/Python language construct (or any Python-ecosystem artifact),
  project it faithfully as a pnix value/term/trace, relate it back, and thereby surface
  what meta-circular evaluation can express. Both directions: Hy/Py → pnix and pnix →
  Hy/Py.

**What this project is NOT — do NOT build any of this here:**
- It is **NOT** an AI agent / coding agent / autonomous-tooling project. That was the
  direction of the sibling/predecessor projects and is explicitly OUT OF SCOPE.
- Do **NOT** treat `~/pnix-old` as an authority to copy wholesale. It is a **read-only
  reference** at most. Its agent/coding-agent machinery and environment-dependent quirks
  (e.g. CWD-relative `toPath` path resolution) are NOT goals — do not chase them.
- Do **NOT** run autonomous "find every diff and match it" sweeps. Only do what the
  developer explicitly asks, focused on Hy ↔ pnix expressiveness.

**Project naming (renamed 2026-06-30):**
- `pnix-clj` → now **`clj-msv`** — was an AI/coding-agent project (out of scope here).
- `pnix` → now **`pnix-old`** — was an AI/coding-agent project (out of scope here).
- the *new* `~/pnix` is a separate ABI-standard project (not this one).
- **`pnix-hy`** = this project = (Hy/Python = the whole Python-language ecosystem)
  meta-circular ↔ pnix meta-circular language-expressiveness research.

Everything below this banner predates this clarification. The error-message/type-guard
import passes were aligned to `~/pnix-old`; they are committed and the gates pass, but
they are NOT the mission. The mission is Hy ↔ pnix expressiveness projection — focus
future work there, not on replicating `pnix-old`.

## ▶ ON-MISSION WORK — Hy reader-form projection (the FRONT of Hy→pnix) (2026-06-30)

First step in the actual mission (Hy ↔ pnix language-expressiveness projection, per the
banner above). The repo already had the BACK half of the pipeline — CPython
introspection in `hy_mirror.py` (`ast_info`/`disassemble`/`symtable_info`/`tokenize_info`/
code-object/marshal) which covers Python AST → bytecode. The FRONT half (Hy source → Hy
reader-form models → Python AST lowering) was missing. Added it:

- `hy_mirror.hy_form_projection(source)` → schema `pnix-hy.hy-form-projection.v0`:
  `reader_forms` (the Hy model tree: Expression/Symbol/Keyword/List/Dict/Integer/Float/
  String/FString/…), plus `python_ast` (ast.dump) and `python_source` (ast.unparse) of
  exactly what that Hy form lowers to. Runs in the Hy proof Python with cwd=HY_ROOT (the
  `hy` 1.3.0 symlink is only importable there), mirroring the stage7 bridge; no new
  runtime dependency, additive only.
- `hy_mirror.hy_form_projection_report()` → self-check (a known `defn` form surfaces its
  reader models and lowers to `def f(x): return x + 1`).

- `hy_mirror.hy_meta_circular_projection(source)` → schema
  `pnix-hy.hy-meta-circular-projection.v0`: chains the front with the existing
  `full_introspection` so ONE call yields the whole Python-ecosystem meta-circular view
  of a Hy fragment — reader forms → Python AST → bytecode / code-object / symtable /
  marshal (e.g. `(defn f [x] (+ x 1))` lowers to a `def`, compiles to a module whose
  `co_consts` carries the `f` code object + a `MAKE_FUNCTION`). `full_introspection`
  `compile()`s the Hy-emitted Python in-process without executing its `import hy`.
  Plus `hy_meta_circular_projection_report()` self-check.

Now a developer can see the whole pipeline for any Hy fragment:
`(defn f [x] (+ x 1))` → `def f(x): return x + 1`; `(let …)` → gensym'd Python;
`` `(a ~b ~@cs) `` → `hy.models.Expression([Symbol('a'), b, *(cs or [])])`. Pair with
`disassemble`/`ast_info` for Hy form → Python AST → bytecode end to end. This is
purely additive to `hy_mirror.py`; `pnix_runtime.py` and the 4-lane mirror are untouched.

NEXT (mission): relate the lowered Python AST back to a **pnix** value/term (the
`Python AST → pnix term` leg) so the Hy↔pnix projection is closed both directions;
surface macroexpansion steps; project pnix terms → Hy forms (the reverse direction).

### Symmetric pnix side + correspondence (done 2026-06-30)
- (#1) `pnix_mirror.pnix_form_projection(source)` — the pnix-side mirror of
  `hy_form_projection`: pnix parsed AST node tree + canonical re-emission (AST→source
  round-trip) + value on both host lanes + toJSON. Now both substrates introspect in the
  same shape. `pnix_form_projection_report()` self-check ready.
- (#2) `pnix_mirror.correspondence_table()` / `correspondence_report()` — curated
  Python/Hy AST ↔ pnix AST-tag/value-type map (22 rows; 12 clean 1:1, 10 `differs`).
  Every pnix tag is verified live against `rt.parse` and every value type against
  `rt._type_of`, so the table can't silently drift. The `differs` rows are the research
  payload: attrset(keys/laziness), lambda(curried+patterns), apply(curried juxtaposition),
  let(recursive-lazy), select/has_attr, **with = dynamic-scope injection ≠ Python
  context-manager `with`**, import(file→value), match(guard arms), **path = native pnix
  literal/type with no Python/Hy equivalent**.

NEXT (mission): close the loop both directions — relate a lowered Python AST node back to
the corresponding pnix term using the correspondence map (a concrete `Python AST → pnix
term` aligner); surface Hy macroexpansion steps; project pnix terms → Hy forms (reverse).

### Hy/Python AST -> pnix term aligner (done 2026-06-30)
- `pnix_mirror.align_hy_to_pnix(hy_source)` / `align_python_to_pnix(python_source)` —
  project a Hy fragment to Python, parse that Python in-process, walk it and LABEL every
  node with the pnix construct it corresponds to (via the #2 table), flagging `differs`.
  Constant/Compare resolved by inspecting the node (Compare with In/NotIn → `has_attr`,
  else `binary`; Constant by value type). e.g. `(defn f [x] (+ x 1))` → FunctionDef→lambda*,
  BinOp→binary, Name→var, Constant→int; `(if (in k m) k 0)` → IfExp→if, Compare→has_attr*.
  The leading `import hy` is Hy-compiler scaffolding (shows as import→import*).
  `alignment_report()` self-check ready.

NEXT (mission): surface Hy macroexpansion steps (form → macroexpanded form → Python);
project pnix terms → Hy forms (the reverse direction); nested (tree-shaped) alignment
instead of the current flat node list.

### Hy macroexpansion projection (done 2026-06-30)
- `hy_mirror.hy_macroexpand_projection(source)` — the macro layer of Hy's meta-circular
  for a single top-level form: original reader model, 1-step expansion (`changed_1`),
  full expansion (`is_macro`), and the Python the fully-expanded form lowers to. e.g.
  `(when c x)` → `(if c (do x) None)` → `x if c else None`; `(cond a 1 b 2)` →
  `1 if a else 2 if b else None`. `hy_macroexpand_projection_report()` ready.

NEXT (mission): project pnix terms → Hy forms (the reverse direction); nested
(tree-shaped) alignment instead of the flat node list; a single combined
`hy_pnix_projection(source)` that bundles reader-forms + macroexpand + lowering +
alignment + correspondence for one fragment.

### Capstone bundle (done 2026-06-30)
- `pnix_mirror.hy_pnix_projection(source)` — ONE call = a Hy fragment's whole
  meta-circular journey + pnix correspondence: reader_forms → macro(is_macro/expansion)
  → python_source/ast → python_introspection(code object + bytecode_len) →
  pnix_alignment (correspondence-labelled Python AST). Two Hy-proof subprocess calls
  (form + macroexpand), rest in-process. The developer research entry point.
  `hy_pnix_projection_report()` ready.

### Reverse projection + nested alignment (done 2026-06-30)
- `pnix_mirror.pnix_to_hy_form(pnix_source)` — REVERSE direction: synthesize
  representative Hy for the clean-correspondence pnix constructs
  (literals/list/attrset/lambda/apply/binary/unary/if/let/var/select) and honestly
  record a `gap` for constructs with no direct Hy form (with, path, import, match,
  has_attr, str_interp, the `->`/`//` operators, recursive-let semantics). e.g.
  `(x: x + 1)` → `(fn [x] (+ x 1))`; `if a then [1 2] else { b = 3; }` →
  `(if a [1 2] {"b" 3})`; `a -> b` → gap. `pnix_to_hy_form_report()` round-trips the
  synthesized Hy back through hy_form_projection (lowers to lambda) — ready.
- nested tree alignment: `align_hy_to_pnix_tree` / `align_python_to_pnix_tree` (above).

The Hy/Py ↔ pnix projection now closes BOTH directions (Hy/Py→pnix align + pnix→Hy
synth), with all non-correspondences (the research payload) explicitly flagged.

### Wider reverse-synth coverage (done 2026-06-30)
- `pnix_to_hy_form` now synthesizes: str_interp → f-string (`"x${a}"` → `f"x{a}"`),
  nested attrset paths → nested Hy dict (`{ a.b.c = 1; }` → `{"a" {"b" {"c" 1}}}`),
  has_attr → `(in "k" base)` (with a note that pnix `?` is non-forcing). Remaining honest
  gaps: with (dynamic scope), path (native literal), import (file→value), match (guard
  arms), `->`/`//` operators, recursive-let semantics — genuinely no clean Hy form.

### Developer CLI (done 2026-06-30)
- `bin/pnix-hy-project` — hands-on entry point for the projection toolkit:
  `--hy '<src>'` / `--hy-file` (Hy→pnix: reader forms, macro layer, lowered Python, pnix
  correspondence tree), `--pnix '<src>'` / `--pnix-file` (pnix AST + value + canonical
  emit + reverse-synthesized Hy + gaps), `--correspondence` (the #2 table), `--check`
  (run all 9 toolkit self-checks; exit 1 if any fails), `--json` for raw output. Run with
  the Hy proof Python, e.g. `/tmp/pnix-hy-py311-venv/bin/python bin/pnix-hy-project --check`.

The Hy/Py ↔ pnix meta-circular projection toolkit is now complete and usable end to end:
10 facilities (both directions) + one-call health check + developer CLI. All `*_report()`
ready; additive; pnix_runtime.py and the 4-lane mirror untouched.

### pnix meta-circular projection — the deepest on-mission view (done 2026-06-30)
- `pnix_mirror.pnix_meta_circular_projection(source)` — evaluates ONE pnix form on all
  FOUR meta-circular substrates and reports convergence (self-hosting closure for that
  form): host interpreter (Python), host compiler (Python), **stage7 runtime = pnix
  evaluator written in Hy**, **stage7 compiler = pnix compiler written in Hy**. The key
  insight it surfaces: pnix's own meta-circular is that pnix is evaluated by a
  Python-ecosystem (Hy) program. `--pnix-meta` CLI flag + `--check` includes it (10
  toolkit self-checks, all green). e.g. `let inc = x: x + 1; in inc 41` → 42 on all four.

### Projection value-roundtrip — meaning preservation (done 2026-06-30)
- `pnix_mirror.projection_value_roundtrip(source)` — lifts the correspondence from
  STRUCTURAL to SEMANTIC: evaluate a pnix form (host), synthesize Hy (pnix_to_hy_form),
  evaluate that Hy in the stage7 Hy kernel, compare values via canonical JSON.
  `meaning_preserved=True` proves pnix X and its projected Hy compute the same value.
  A `#_pnix-...` placeholder (unprojectable construct) → not comparable; a SEMANTIC
  gap-note (let/attrset — Hy still evaluates) IS tested so divergence is surfaced;
  functions / pnix-eval-errors are reported not-comparable. e.g. `1 + 2`→3=3,
  `let a = 1; in a + a`→2=2, `{ a.b = 5; }`→nested dict preserved. `--roundtrip` CLI flag;
  `--check` now runs 11 toolkit self-checks (all green).

NEXT (optional): match → cond synthesis (literal patterns only); project whole `.px`/`.hy`
files with per-form breakdown.

## ▶ DESIGN DIRECTIVE — host-faithful projection, no cross-host parity (2026-06-30)

User clarification: the primary goal of `pnix-hy` and sibling `pnix-clj` is **not**
cross-host parity. The goal is **host-language projection fidelity**: each pnix
runtime must preserve and mirror the meaning of its own host language inside pnix.

```text
pnix-hy pnix runtime
= faithfully projects Hy/Python language structure into pnix.

pnix-clj pnix runtime
= faithfully projects Clojure/JVM language structure into pnix.
```

"Projection" is not a shallow transpile. It means a host-language artifact can be
represented as pnix values / terms / traces / mirror objects, then related back to
the host artifact or execution result without losing the host meaning:

```text
host syntax
host evaluation rules
host macro / namespace / module / import model
host runtime effects
host scope / closure behavior
host data model
host exception / control-flow semantics
```

For `pnix-hy`, the comparison target is Hy/Python itself:

```text
Hy form
-> pnix-hy term
-> Python AST lowering
-> execution / eval / load
-> mirror trace
-> pnix meaning explanation
```

`pnix-hy` should understand and mirror Hy/Python, including Hy reader forms,
symbols, keywords, lists/vectors/maps, `defn`/`fn`/`let`/`setv`/`do`/`if`/`cond`,
macros/quasiquote/unquote/require, imports/module loading, Python AST lowering,
exceptions, context managers, async subset where applicable, class/function scope,
and eval/load behavior. It should **not** try to understand Clojure/JVM semantics.

For `pnix-clj`, the comparison target is Clojure/JVM itself:

```text
Clojure form
-> pnix-clj term
-> Clojure eval / macroexpand / namespace
-> execution trace
-> pnix mirror
-> Clojure meaning explanation
```

`pnix-clj` should understand and mirror Clojure/JVM, including reader data,
symbols, keywords, lists/vectors/maps/sets, `def`/`defn`/`fn`/`let`/`letfn`/
`loop`/`recur`, conditionals, macroexpand/syntax-quote/unquote/gensym,
namespace/require/refer/alias, vars and dynamic binding, protocols, multimethods,
records/deftype/reify, STM/atoms/refs/agents, Java interop, and exception
semantics. It should **not** try to understand Hy/Python semantics.

The correct relation is:

```text
pnix-hy <-> Hy/Python original meaning       YES
pnix-clj <-> Clojure/JVM original meaning    YES
pnix-hy <-> pnix-clj semantic parity          NO
```

Cross-host parity is a current **non-goal**, not merely a far-future phase. Forcing
Hy/Python and Clojure/JVM to share semantic parity too early would flatten real
host differences: Hy macro/import/Python-AST/exception behavior is not the same as
Clojure macro/namespace/var/dynamic-binding/STM/JVM behavior. The success metric
is host fidelity, not inter-host sameness.

Commonality, if any, is allowed only at the outer envelope level:

```text
Allowed common envelope:
- receipt shape
- held / accepted / rejected state names
- evidence record shape
- trace id
- source hash
- mirror event envelope

Forbidden common semantic ABI:
- macro semantics
- eval semantics
- import / namespace semantics
- host object model
- exception semantics
- type / value model
```

Therefore the working phases are:

```text
Phase A: host-faithful projection
Phase B: host-local mirror roundtrip
Phase C: host-local projection ABI
Phase D: optional shared outer envelope only
```

Any older wording in this file about cross-host parity or a common `.px` ABI is
superseded by this directive unless it explicitly refers only to the outer
receipt/evidence/mirror envelope. References to `hy-meta` stage14 cross-host
evidence are upstream proof-ladder labels, not a `pnix-hy`/`pnix-clj` semantic
parity requirement.

## ▶ IMPLEMENTATION PASS — hashFile missing-path + lock-in (2026-06-30)

Imported `eval_hashfile_builtin`. The only real gap was a **compiler-lane bug**:
`builtins.hashFile` on a missing path leaked a raw Python `OSError`
("[Errno 2] No such file or directory: '...'") instead of "builtins.hashFile: failed to
read `<path>`: ..." — the prelude `_hashfile` called `open()` without a try/except (the
interp lane was already correct). Wrapped it to match the interp message. Everything
else already passed (sha256/sha512 digests, md5/sha1 "cryptographically broken"
rejection, blake2 "unsupported algorithm", non-string algo, non-path arg). The corpus
hashes files **self-contained via `builtins.toFile`** (which writes to pnix-hy's temp
store) so no external on-disk fixtures are needed — this closes the last auto-probe
candidate. Vendored 15 `rs-hf-*` cases + 3 stage7 core.

This completes the message/semantic alignment sweep of the `~/pnix-old` guard/type
corpus: every auto-probed candidate file is now aligned + vendored (artifacts manually
verified). The `eval_*.rs` corpus that pnix-hy mirrors is fully imported for this phase.

Gates (Python 3.11): `py_compile` green, `self_test_report` **1103/1103**,
`rust_corpus_report` **1256/1256**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **541/541**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — toJSON non-finite-float kind (2026-06-30)

Imported the toJSON-+inf slice of `eval_update_path_arith`. `builtins.toJSON` of a
non-finite float said the generic "cannot serialize non-finite float as JSON"; now names
the kind — "cannot serialize float +inf|-inf|NaN as JSON" — matching ~/pnix interpret.rs
(`non_finite_json_float_error`). Aligned all three lanes (interp `value_to_json` guard,
prelude `_tojson`, Hy `to-json-value`). Re-aligned the existing `rs-err-tojson-inf` case.
Vendored 3 `rs-tj-*` cases + 2 stage7 core.

### REMAINING (next session — env/verify only)
- `readDir`/`readFile`/`readFileType` bad-path "expected string or path" — verify vs
  interpret.rs (likely a probe artifact; current "failed to read" is reasonable).
- `eval_hashfile_builtin` — needs on-disk file fixtures (env), not a message gap.
All other auto-probed candidate files are now message-aligned + vendored.

Gates (Python 3.11): `py_compile` green, `self_test_report` **1097/1097**,
`rust_corpus_report` **1241/1241**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **538/538**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — arith operator unsupported-operand messages (2026-06-30)

Imported the `+`-type-mismatch slice of `eval_path_string_concat_context`. Arith
operators (`+`/`-`/`*`/`/`/`%`) on non-numeric operands — where no `+`-overload
(string/list/attrset/path concat) applies — now error with ~/pnix's binary-op
fallthrough form "operator <op>: unsupported operand types <tl> and <tr>" (verified
against `~/pnix-old` interpret.rs line 6067), replacing pnix-hy's generic "left/right
side of <op> must be a number". bool is correctly non-numeric. Confirmed against
interpret.rs since `+` is heavily used; no existing corpus depended on the old wording
(only `rs-type-string-plus-int-err`, re-aligned). Added `_arith_pair` (interp), `_apair`
(prelude `_bin`), `numeric?`+`arith-pair` (Hy `eval-binary`); the +-overloads
(str/list/attrset/path concat) and the i64-overflow / div-by-zero / mod paths are
unchanged. Vendored 10 `rs-op-*` cases + 4 stage7 core.

### REMAINING (for next session — small/env)
- `toJSON` non-finite float: ~/pnix "cannot serialize float +inf as JSON" vs pnix-hy
  "cannot serialize non-finite float as JSON" (one-line wording).
- `readDir`/`readFile`/`readFileType` bad-path "expected string or path" — verify vs
  interpret.rs (may be probe artifact).
- `eval_hashfile_builtin` — needs on-disk file fixtures (env), not a message gap.

Gates (Python 3.11): `py_compile` green, `self_test_report` **1093/1093**,
`rust_corpus_report` **1238/1238**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **536/536**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — string-arg guards (getEnv/xmlParse/htmlParse/toFile) (2026-06-30)

Imported the type-guard slices of `eval_string_context_param_parity` +
`eval_tofile_context_guard`. `getEnv`/`xmlParse`/`htmlParse` on a non-string said the
generic "... (int) must be a string"; now "builtins.X: expected string, got <type>"
(context-bearing strings still accepted). `toFile` now names "first argument"/"second
argument" + "must be string", and its context-rejection error points the user at
`builtins.unsafeDiscardStringContext`. Added one string guard per lane —
`string_arg_value` (interp), `_strarg` (prelude), `string-arg-value` (Hy) — applied to
getEnv/xmlParse/htmlParse + the toFile name/contents checks. Vendored 9 `rs-sa-*` cases
+ 3 stage7 core.

### REMAINING auto-probe gaps (for next session)
- `+` operator type mismatch (`42 + "hi"`, `null + 1`): ~/pnix says "unsupported operand
  types"; pnix-hy says "left/right side of + must be a number". Heavily-used operator —
  needs its own careful batch + verify against interpret.rs.
- `toJSON` of non-finite float: ~/pnix "cannot serialize float +inf as JSON" vs pnix-hy
  "cannot serialize non-finite float as JSON" (small wording).
- `readDir`/`readFile`/`readFileType` on bad path: ~/pnix "expected string or path"
  shape — needs interpret.rs verification (probe may be artifact).
- `eval_hashfile_builtin`: needs on-disk file fixtures (env), not a message gap.

Gates (Python 3.11): `py_compile` green, `self_test_report` **1085/1085**,
`rust_corpus_report` **1228/1228**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **532/532**, stage15/stageN closure reproduced.

## ▶ LOCK-IN PASS — toString-cycle + derivation-attrset guards (2026-06-30)

Probed `eval_tostring_cycle_guard` + `eval_derivation_builtin` manually (the auto-probe
flagged "gaps" but they were parser artifacts — multi-line `err(r#"..."#)` sources and
`.expect()` strings the regex mis-read). Both already pass: self/alternating/outPath
toString-coercion cycles + the cross-path toString↔interp cycle all error with a
"...cycle..." message on both lanes (the depth/cycle guard was already in place from the
force-cycle pass), and `builtins.derivation` on a non-attrset already errors naming
"derivation"+"attrset". No code change — vendored 8 `rs-tc-*`/`rs-dv-*` lock-in cases +
3 stage7 core. (Cross-lane wording for derivation differs — interp "attrs must be an
attrset" vs compiler "expected attrset" — both satisfy the .rs; not a parity failure
since the mirror compares values, not error text. Left as-is.)

The only auto-probe candidate still open is `eval_hashfile_builtin` (needs real on-disk
file fixtures, an env concern, not a message gap).

Gates (Python 3.11): `py_compile` green, `self_test_report` **1079/1079**,
`rust_corpus_report` **1219/1219**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **529/529**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — output-dependency string-arg guards (2026-06-30)

Imported `eval_unsafe_add_output_builtins` (the type-guard slice). The
output-dependency string builtins said the generic "string (int) must be a string";
now they name the type: `unsafeAddOutputDependency` (and the shared
`addDrvOutputDependencies`/`unsafeDiscardOutputDependency`) say "builtins.X: expected
string, got <type>", and `unsafeAddOutputName` names "first arg"/"second arg" + "must
be string" + type. Aligned all three lanes via the shared sites: interp
`context_string_value`/`unsafe_add_output_name_value`, prelude `_ctxstr`/`_addoutname`,
Hy `context-string-builtin`/`unsafe-add-output-name-builtin`. No existing corpus case
used the old wording (only happy-path value cases). Vendored 7 `rs-uo-*` cases + 4
stage7 core. Closes the unsafeAddOutput* item from the prior pass's remaining list.

Gates (Python 3.11): `py_compile` green, `self_test_report` **1073/1073**,
`rust_corpus_report` **1211/1211**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **526/526**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — concatStrings/concatStringsSep arg guards (2026-06-30)

Imported the concatStrings/Sep slices of `eval_concat_match_split_context` +
`eval_string_ops`. `concatStrings` on a non-list said the generic "... must be a list"
— now "builtins.concatStrings: argument must be list, got <type>" (reuses
`list_arg_value`/`_listarg`/`list-arg-value`). `concatStringsSep` on a non-string
separator said "separator (int) must be a string" — now "builtins.concatStringsSep:
separator must be string, got <type>" (a force+is-string check before the
context-extracting `string_text_context`, so context handling is unchanged). The
element-index checks ("element at index N is not a string") were already correct.
Re-aligned the existing `rs-concatstrings-err-non-list` case. Vendored 9 `rs-cs-*`
cases + 4 stage7 core.

### REMAINING auto-probe gaps (found 2026-06-30, NOT yet done — for next session)
The `~/pnix-old` corpus auto-probe (scratchpad/rsprobe.py) flagged these still-open
message gaps (others probed were already lock-in / pass):
- `eval_unsafe_add_output_builtins`: `unsafeAddOutputDependency` non-string wants
  "expected string"; `unsafeAddOutputName` wants "first arg"/"second arg" + "must be
  string" + type (currently the generic "(type) must be a string").
- `eval_tostring_cycle_guard` (4), `eval_derivation_builtin` (5): need manual probe —
  some may be real, some parser/`.expect()` artifacts.
- `eval_hashfile_builtin` (11): likely needs real file fixtures (env), not message gaps.
- Parser artifacts to IGNORE (the probe mis-extracts `.expect("...")` strings and
  `||` asserts): the lone "GAPS=1" in eval_compare_versions / eval_json_builtins /
  eval_to_string were all `.expect()` text, not real.

Gates (Python 3.11): `py_compile` green, `self_test_report` **1065/1065**,
`rust_corpus_report` **1204/1204**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **522/522**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — i64::MIN overflow guards (mod/neg/unary-) (2026-06-30)

Imported `eval_i64_min_overflow_guards`. Genuine correctness fix (not just messages):
pnix-hy uses arbitrary-precision Python ints, so the i64::MIN edge cases that panic in
Rust silently produced WRONG values here — `-(i64::MIN)` and `builtins.neg i64::MIN`
returned `9223372036854775808` (one past i64::MAX, out of range!), and
`i64::MIN % -1` / `builtins.mod i64::MIN (-1)` returned `0` instead of erroring. Added
the checks (consulted ~/pnix-old `interpret.rs` truth-owner directly since the .rs only
asserts substrings): unary `-` and `builtins.neg` now route through the i64 range check
(`check_i64`/`_ci`/`check-i64`, "integer overflow in `-`"); binary `%` and
`builtins.mod` reject `i64::MIN % -1` ("integer overflow in `%`"). Also corrected
`builtins.mod` divide-by-zero to "builtins.mod: division by zero" (per interpret.rs;
binary `%` stays "modulo by zero") — re-aligned the existing `rs-err-builtin-mod-zero`
case. Touched all lanes incl. both compiler unary-`-` emitters. Vendored 13 `rs-ov-*`
cases + 6 stage7 core. Closes the Arith/guards overflow group.

Gates (Python 3.11): `py_compile` green, `self_test_report` **1057/1057**,
`rust_corpus_report` **1195/1195**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **518/518**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — appendContext value-shape guards (2026-06-30)

Imported `eval_appendcontext_value_shape_guard`. Genuine code change AND a correction
of an earlier divergence: each per-path context value must be an attrset whose
`path`/`allOutputs` are bool and `outputs` a list of strings. The three lanes disagreed
with each other AND with ~/pnix (interp "context./a must be an attrset" vs compiler
"context value for `/a` must be attrset", neither naming the type; `outputs` said "must
be a list" not "must be list of strings"; the outputs element said "item" not "element
at index N"). Crucially the earlier bool-required-positions pass had globally rewritten
`path`/`allOutputs` to the boolop "expected bool" wording — but THIS file wants "must be
bool", so appendContext now uses its own bool check (the generic boolop message is
unchanged). Rewrote the shape validator in all three lanes — interp
`validate_context_attrset`, prelude `_appendcontext`, Hy `append-context-builtin`
(+ `append-outputs-check`) — to the unified ~/pnix form: "context value for '<k>' must
be an attrset, got <type>", "'<k>'.path|allOutputs must be bool, got <type>",
"'<k>'.outputs must be list of strings, got <type>", "'<k>'.outputs element at index N
is not a string, got <type>". Re-aligned the 2 existing `rs-guard-append-*` cases off
the stale "expected bool" wording. Vendored 13 `rs-acs-*` cases + 4 stage7 core.
Further advances Guards-positions-misc / string-context.

Gates (Python 3.11): `py_compile` green, `self_test_report` **1045/1045**,
`rust_corpus_report` **1182/1182**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **512/512**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — addErrorContext + bitAnd/bitOr/bitXor type guards (2026-06-30)

Imported the addErrorContext + bitops slices of `eval_addcontext_pos_bitops_guards`.
Genuine code change: `addErrorContext` on a non-string context said the generic
"context (int) must be a string" — now "builtins.addErrorContext: context must be
string, got <type>" (context-bearing message + lazy value preserved); `bitAnd`/`bitOr`/
`bitXor` said "first arg must be an integer" without the offending type — now name the
side AND type "builtins.bitX: first|second arg must be int, got <type>" (bool excluded
via `type() is int`). Localized to the bitop/addErrorContext sites (the generic
`integer_value`/`_int`/`integer-value` helper used by other arithmetic builtins is
untouched): interp `bit_op_value`/`add_error_context_value`, prelude `_bit`/`_addctx`,
Hy `bit-int-arg`+`bit-*-builtin`/`add-error-context-builtin`. Re-aligned the 3 existing
`rs-guard-addctx-*` cases off the old "must be a string" wording. Vendored 13 `rs-bx-*`
cases + 4 stage7 core. Further advances Guards-positions-misc.

Gates (Python 3.11): `py_compile` green, `self_test_report` **1037/1037**,
`rust_corpus_report` **1169/1169**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **508/508**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — attrset/function introspection type guards (2026-06-30)

Imported the attrset/function-guard slices of `eval_introspection_folds` +
`eval_zipattrswith_lazy_guard`. Genuine code change across the attrset/function
introspection builtins: `functionArgs` on a non-function silently returned `{}` — now
errors "builtins.functionArgs: expected function, got <type>" (native primops still
return `{}`); `attrNames`/`attrValues`/`getAttr` on a non-attrset said the generic
"... must be an attrset" (and the compiler `attrNames` leaked a raw Python
`'list' object has no attribute 'keys'`) — now "builtins.X: expected attrset, got
<type>"; `getAttr`/`getAttrs` missing-key now name the attr in single quotes
("attribute 'z' missing"); `zipAttrsWith` validates its list arg + elements with the
named-type form. Added one attrset guard per lane — `attrset_arg_value` (interp),
`_attrsarg` (prelude), `attrset-arg-value` (Hy) — plus `get_attr_value`/`get-attr-builtin`.
Vendored 16 `rs-ai-*` cases + 6 stage7 core. Further advances Lists / Guards-positions-misc.

Gates (Python 3.11): `py_compile` green, `self_test_report` **1029/1029**,
`rust_corpus_report` **1156/1156**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **504/504**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — list-argument type guards for any/all/elem/filter/foldX (2026-06-30)

Imported `eval_seq_any_all` + `eval_filter_elem_listtoattrs` + `eval_length_foldr`
(the list-arg-guard slices). Genuine code change: the list-consuming builtins named
the wrong thing on a non-list argument ("builtins.X list must be a list" — generic, no
offending type). Now `any`/`all`/`elem`/`filter` say "builtins.X: second argument must
be list, got <type>" and `foldl'`/`foldl`/`foldr` say "builtins.X: third arg must be
list, got <type>" (extending the earlier `fold` change to the whole fold family).
Introduced one shared guard per lane — `list_arg_value(value, builtin, position)`
(interp), `_listarg` (prelude), `list-arg-value` (Hy) — replacing the generic
`force_list`/`list_value`/`_list` calls for these builtins (also subsumed the
`_fold_list_arg`/`_foldarg` helpers). Predicate-bool / index-pinned / short-circuit /
listToAttrs behaviour was already correct, so only the list-arg message moved. Vendored
17 `rs-la-*` cases + 5 stage7 core. Further advances Lists / Guards-positions-misc.
(NOTE: `eval_genlist_floor_ceil_guards`, `eval_list_bounds`, `eval_list_laziness_guards`,
`eval_compare_cycle_guard`, `eval_equality_cycle_guard` were probed and already pass —
pure lock-in already present, no new work.)

Gates (Python 3.11): `py_compile` green, `self_test_report` **1017/1017**,
`rust_corpus_report` **1140/1140**, `fixture_report` **32/32** (via PNIX_HY_FIXTURES_DIR),
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7 parity
lanes **498/498**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — fold non-list + groupBy key/context guards (2026-06-30)

Imported `eval_fold_groupby_guards`. Another **genuine code change**. (1) The pnix-only
`builtins.fold` was the holdout that silently returned `init` for ANY non-list third
arg (Nix-canonical `foldl'`/`foldr` already errored); gave it the typed message
"builtins.fold: third arg must be list, got <type>" while leaving foldl'/foldl/foldr
untouched (a dedicated `_fold_list_arg`/`_foldarg`/`fold-list-arg` guard in front of
the shared fold impl, so the compiler's fold→foldl' routing no longer leaks a "foldl'"
label). (2) `builtins.groupBy` already accepted context-bearing key-function returns
(verified: `groupBy (item: "a${./p}")` groups into one bucket without error) but its
failure message lied; renamed to "groupBy: key function must return string, got
<type>" and added a non-list second-arg guard "groupBy: second argument must be list,
got <type>". Aligned the three backing sites: interp (`group_by` + `fold`), Python
prelude (`_groupby` + `_foldarg`), Hy source evaluator (`group-by-items` +
`group-by-list-arg`/`fold-list-arg`). Vendored 13 `rs-fg-*` cases + 4 stage7 core.
Further advances Lists / Guards-positions-misc.

Gates (Python 3.11): `py_compile` green, `self_test_report` **1007/1007**,
`rust_corpus_report` **1123/1123**, `fixture_report` **32/32**,
`original_oracle_report` disagree **0**, `pnix_mirror.self_test_report(include_hy_host=True)`
ready with all four stage7 parity lanes **493/493**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — replaceStrings list/element type guards (2026-06-30)

Imported `eval_replace_strings_list_guards`. Another **genuine code change**:
`replaceStrings` silently coerced non-list `from`/`to` to `[]`, so the post-coerce
length check passed `0 == 0` and the haystack came back unchanged (silent no-op — the
"didn't crash but did the wrong thing" shape). Made `from`/`to` typed assertions
"builtins.replaceStrings: 'from'|'to' must be list, got <type>" (`from` checked first,
before the length check). The length-mismatch arm stays distinct ("equal length", and
must NOT read as a type error) so callers can tell count-wrong from type-wrong;
haystack non-string now says "third argument", and from-element non-string says exactly
"'from' element must be string" (the old generic "(type) must be a string" form broke
the contiguous-substring contract). Aligned the three backing sites: interp
(`replace_strings_value`), Python prelude (`_replace`), Hy source evaluator
(`replace-strings-builtin` + new `replace-from-elements`, replacing the generic
`force-string-list` element walk). Haystack→str conversion is byte-identical to before
so string-context propagation is unchanged. Vendored 11 `rs-rsl-*` cases + 4 stage7
core. Further advances Lists / Guards-positions-misc.

Gates (Python 3.11): `py_compile` green, `self_test_report` **999/999**,
`rust_corpus_report` **1110/1110**, `fixture_report` **32/32**,
`original_oracle_report` disagree **0**, `pnix_mirror.self_test_report(include_hy_host=True)`
ready with all four stage7 parity lanes **489/489**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — abort string-only + with non-attrset guards (2026-06-30)

Imported `eval_abort_with_string_guards`. Another **genuine code change**. (1)
`builtins.abort` was silently coercing non-string args to Display ("evaluation
aborted: 42"); made it string-only like throw with "builtins.abort: argument must be
string, got <value>" (value shown for scalars so the int case pins "42"). The
"evaluation aborted: " marker is emitted only after the type check passes, so the
tryEval abort-propagation invariant holds: a valid string abort still re-raises out of
tryEval, but the type error is a normal catchable error (`(tryEval (abort 42)).success
== false`). (2) `with <non-attrset>; <body>` hid the source-type error behind
"undefined variable" when a body lookup fell through; the with-frame force now raises
"with: argument must be attrset, got <type>". Laziness preserved: the source is only
forced when a body name actually consults the with-frame (`with 42; 1` → 1, throwing
source dormant until lookup). Aligned the three sites backing all four lanes: interp
(`abort_value`/`_abort_show` + `force_with_frame`), Python prelude shared by both
compilers (`_abortval`/`_abortshow` + `_with_attrs`), Hy source evaluator
(`abort-value`/`abort-show` + `force-with-frame`). Re-aligned 6 pre-existing
`rs-guard-abort-*`/`rs-guard-with-*` cases off the old "must be a string"/"with source"
wording. Vendored 15 `rs-aw-*` cases + 5 stage7 core. Further advances
Guards/positions/misc.

Gates (Python 3.11): `py_compile` green, `self_test_report` **991/991**,
`rust_corpus_report` **1099/1099**, `fixture_report` **32/32**,
`original_oracle_report` disagree **0**, `pnix_mirror.self_test_report(include_hy_host=True)`
ready with all four stage7 parity lanes **485/485**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — attr/concat type-guard messages (2026-06-30)

Imported `eval_attr_concat_guards`. Another **genuine code change**: `hasAttr`,
`removeAttrs`, `concatLists` were silent-false / silent-empty / silent-noop on bad
input shape (generic-label "must be an attrset" without the offending type). Made all
three hard type assertions with the ~/pnix message form "builtins.<fn>: <position>
must be <type>, got <type>" and indexed element checks ("name-list element at index N
is not a string" / "element at index N is not a list"), forcing lazy elements before
the check so a thunk-of-string / thunk-of-list still passes. Aligned the three
implementation sites that back all four lanes: interp (`has_attr_value` +
`remove_attrs_value` + `concat_lists_value`), Python prelude shared by both compilers
(`_hasattr`/`_rmattrs`/`_concatlists`), and the Hy source evaluator
(`has-attr-builtin`/`remove-attrs-builtin`/`names-to-remove`/`concat-lists-builtin`,
the last rewritten to an indexed loop). Vendored 13 `rs-ac-*` cases (errors + happy +
lazy-element) + 6 stage7 core cases. This further advances Guards/positions/misc.

Gates (Python 3.11): `py_compile` green, `self_test_report` **981/981**,
`rust_corpus_report` **1084/1084**, `fixture_report` **32/32**,
`original_oracle_report` disagree **0**, `pnix_mirror.self_test_report(include_hy_host=True)`
ready with all four stage7 parity lanes **480/480**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — bool-required-position messages (2026-06-30)

Imported `eval_bool_required_positions`. This was a **genuine code change** (message
alignment, not just lock-in). pnix-hy previously said "`<position>` must be a boolean";
~/pnix wants the type-aware form "`<position>: expected bool, got <type>`" with
positions "if condition" / "left|right operand of &&|||->". Aligned all four lanes:
interp `bool_value` + boolop labels, prelude `_bool` + `->` label, Hy `bool-value` +
boolop labels, and both compiler emitters (Hy + host `_px_emit`). The operand wording
also changed "left/right **side** of" → "left/right **operand** of". This broke the 13
pre-existing `rs-guard-*` bool cases that matched the old word "boolean" (if/&&/||/->,
`!`, `assert`, `builtins.and|or|not`, `appendContext` path/allOutputs) — re-aligned
their `error_contains` to the common substring "expected bool". Vendored 11 new
`rs-bp-*` cases + 4 stage7 core (tryEval) cases. This advances Guards/positions/misc.

Gates (Python 3.11): `py_compile` green, `self_test_report` **969/969**,
`rust_corpus_report` **1071/1071**, `fixture_report` **32/32**,
`original_oracle_report` disagree **0**, `pnix_mirror.self_test_report(include_hy_host=True)`
ready with all four stage7 parity lanes **474/474**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — cycle/recursion guard corpus (2026-06-30)

Imported `eval_force_cycle` + `eval_cyclic_value_guards` + `eval_interp_cycle_guard`
remainder. All cycle-guard messages were already aligned (Codex's earlier "Guard
cyclic forcing" pass): self/rec attrset cycles → "infinite recursion", lazy unused
binding (`let x = x; in 1`→1) and deep non-cyclic recursion (`f 30`→30) evaluate
fine, `builtins.toJSON`/`builtins.deepSeq` of a cyclic value error with
"toJSON"/"deepSeq" + "infinite recursion", and interpolation of a self-referential
`__toString` errors with "interpolation coercion cycle"+"__toString". Vendored 11
`rs-fc-*` cases + 4 stage7 core cases; no code change. (`recursion_depth_guard` /
`tco_smoke` are thread/stack-depth probes — left as env-specific, not vendored.)

Gates (Python 3.11): `py_compile` green, `self_test_report` **961/961**,
`rust_corpus_report` **1060/1060**, `fixture_report` **32/32**,
`original_oracle_report` disagree **0**, `pnix_mirror.self_test_report(include_hy_host=True)`
ready with all four stage7 parity lanes **470/470**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — nixpkgs/lib functional patterns corpus (2026-06-30)

Imported `eval_nixpkgs_lib_patterns` (the nixpkgs/lib functional substrate: fix-point
combinator, `optional`/`optionals`/`optionalAttrs`, `recursiveUpdate`,
`makeOverridable`, `makeExtensible`, `genAttrs`/`nameValuePair`, `foldAttrs`,
`mapAttrsToList`, `mkOverride`/`mkDefault`/`mkForce`/`mkMerge`, `composeExtensions`
overlay, `hasPrefix`/`hasSuffix`). All passed pnix-hy on first probe (interp +
compiler) — the deep fix-point/extensible/overlay recursion + laziness already work.
This CLOSES the Lambdas/laziness/types group of B. Vendored 14 `rs-nixlib-*`
`RUST_EVAL_CORPUS` cases + 4 stage7 core cases; no code change needed.

Gates (Python 3.11): `py_compile` green, `self_test_report` **953/953**,
`rust_corpus_report` **1049/1049**, `fixture_report` **32/32**,
`original_oracle_report` disagree **0**, `pnix_mirror.self_test_report(include_hy_host=True)`
ready with all four stage7 parity lanes **466/466**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — type-handling + lambda-corner + with-laziness corpus (2026-06-30)

Imported the remaining `eval_type_handling`(48), `eval_lambda_attrset_corners`,
`eval_with_lazy`, and `eval_with_priority` Rust corpora. All cases already passed
pnix-hy (interp + compiler) once probed against ground truth — `eval_functional_lazy`
and `eval_inherit_lazy` had been covered in earlier passes (`rs-func-*`/`rs-inherit-*`),
so only `eval_nixpkgs_lib_patterns`(21) remains in the Lambdas/types group. Confirmed
function equality returns `false` (matches real Nix, not an error). Vendored 37 new
`RUST_EVAL_CORPUS` cases (`rs-th-*`/`rs-lc-*`/`rs-wl-*`/`rs-wp-*`) + 8 stage7 core
cases. No code change needed beyond the lock-in cases (semantics were already aligned).

Env note: the `/tmp/pnix-hy-py311-venv` venv had lost `funcparserlib` (a hy
dependency the stage7 bootstrap needs), which broke the mirror with a spurious
`ModuleNotFoundError`; reinstalled it (`pip install funcparserlib`) — NOT a code
regression. If the stage7 mirror ever fails with that import error, reinstall it.

Gates (Python 3.11): `py_compile` green, `self_test_report` **945/945**,
`rust_corpus_report` **1035/1035**, `fixture_report` **32/32**,
`original_oracle_report` disagree **0**, `pnix_mirror.self_test_report(include_hy_host=True)`
ready with all four stage7 parity lanes **462/462**, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — unsafe output + derivation corpus (2026-06-29)

Imported the Rust-grounded `eval_unsafe_add_output_builtins` and
`eval_derivation_builtin` representative slice. The current runtime behavior
already matched this slice, so this pass locked it into the static corpus and the
stage7 core lanes rather than changing semantics.

Covered across host interpreter, host compiler prelude, Hy evaluator, and Hy
compiler-source lanes:
- `unsafeAddOutputDependency` and `unsafeAddOutputName` preserve text, add the
  correct `!out!` / `!name!` context markers, are idempotent on already-marked
  entries, and round-trip with `unsafeDiscardOutputDependency`.
- argument guards for unsafe output builtins fail under `tryEval` for non-string
  names/values.
- `builtins.derivation` and `derivationStrict` share the same standard field
  surface, preserve user fields and user overrides, attach output contexts to
  `outPath`/`drvPath`, serialize through `toJSON`, and support the common
  `(d.type or null) == "derivation"` idiom.
- `builtins.isDerivation` remains absent, matching the language boundary; that
  helper belongs in library code, not core builtins.

Added **26 Rust corpus cases** plus **9 stage7 core lock-ins** for unsafe output
context markers, derivation field/override/context behavior, and derivation guard
surface.

Gates (Python 3.11): `py_compile` green, `self_test_report` **929/929**,
`rust_corpus_report` **998/998**, `fixture_report` **32/32**,
`original_oracle_report` agree **113** / disagree **0** / unsupported **348**,
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7
parity lanes green over **454** core cases, closure reproduced.

## ▶ IMPLEMENTATION PASS — guard/misc fail-loud corpus (2026-06-29)

Imported a Rust-grounded guard/misc slice covering `eval_lang_version_builtin`,
`eval_abort_with_string_guards`, `eval_addcontext_pos_bitops_guards`,
`eval_appendcontext_value_shape_guard`, `eval_attr_concat_guards`,
`eval_bool_required_positions`, `eval_let_lambda_dup_guards`, and
`eval_dup_attr` representative cases.

Fixed the actual gaps found across host interpreter, host compiler prelude, Hy
evaluator, and Hy compiler-source lanes:
- `builtins.abort` now requires a string argument before emitting the hard
  `evaluation aborted:` marker; `tryEval (builtins.abort 42)` is therefore a
  regular caught type error, while string aborts still propagate.
- `builtins.hasAttr` now fails loud on non-string names and non-attrset values in
  all lanes, instead of silently returning false or leaking Python iteration
  errors.
- compiler `removeAttrs` now shares the interpreter's attrset/list guards for
  its first and second arguments.
- attrset lambda formals now reject duplicate field names, including
  `args@{ args }`, before the argument value is forced.

Added **61 Rust corpus cases** plus **17 stage7 core lock-ins** for langVersion,
abort/with/addErrorContext guards, `unsafeGetAttrPos` generated-null behavior,
bitops, appendContext context-shape validation, hasAttr/removeAttrs/concatLists
guards, bool-required positions, and duplicate let/attr/lambda formal guards.
Recorded one auxiliary v0 divergence: installed `pnixc-meta` lacks the bitops
that full Rust `interpret.rs` exposes.

Gates (Python 3.11): `py_compile` green, `self_test_report` **911/911**,
`rust_corpus_report` **972/972**, `fixture_report` **32/32**,
`original_oracle_report` agree **113** / disagree **0** / unsupported **339**,
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7
parity lanes green over **445** core cases, closure reproduced.

## ▶ IMPLEMENTATION PASS — path context IO + readFileType metadata guard (2026-06-29)

Imported the next Rust-grounded Paths/FS/IO slice for the pnix runtime surface
inside pnix-hy. This batch keeps the focus on stage15+ mirror control: path
context must flow into IO-facing builtins predictably, filesystem misses must
produce labeled pnix errors, and `isPath` must stay outer-shape lazy.

Fixed across host interpreter, host compiler prelude, Hy evaluator, and Hy
compiler-source lanes:
- `builtins.readFileType` now follows Rust's metadata behavior for missing paths:
  it errors instead of returning `"unknown"`, while dangling symlinks still report
  `"symlink"` through the existing lexical file-type check.
- host compiler prelude and Hy evaluator `readFile`/`readDir` now wrap missing
  filesystem errors with `builtins.readFile` / `builtins.readDir` labels instead
  of leaking raw `OSError` text.
- Path-context strings now have explicit coverage through `pathExists`,
  `readFile`, `readDir`, `readFileType`, `toPath`, and `storePath`; `toFile` /
  `readFile` round-trip and current-directory `readDir` are locked in for the
  real runtime.
- `builtins.isPath` is covered as a non-forcing outer-shape predicate for
  attrsets/lists containing throws, while top-level thrown values still fail under
  `tryEval`.

Added **22 Rust corpus cases** plus **12 stage7 core lock-ins** for path-context
IO, missing-path guard labels, `readFileType` metadata misses, `toFile` IO
round-trip, current repo `readDir`/`readFile` checks, and lazy `isPath` shape
behavior.

Gates (Python 3.11): `py_compile` green, `self_test_report` **877/877**,
`rust_corpus_report` **911/911**, `fixture_report` **32/32**,
`original_oracle_report` agree **113** / disagree **0** / unsupported **322**,
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7
parity lanes green over **428** core cases, closure reproduced.

## ▶ IMPLEMENTATION PASS — nested attrset path merge + update arithmetic corpus (2026-06-29)

Imported the Rust `eval_update_path_arith` invariance slice. Most arithmetic,
shallow `//`, finite/inf guards, and JSON non-finite rejection already matched;
the real gap was attrset construction merging:
- `{ a = { b = 1; }; a.c = 2; }` now merges into `{ a = { b = 1; c = 2; }; }`
  instead of treating `a` as a non-attrset conflict.
- `{ a = { b = 1; }; a = { c = 2; }; }` now merges distinct explicit attrset
  leaves, while duplicate scalar leaves still error with `already defined`.
- The merge fix is implemented across host interpreter, host compiler prelude,
  Hy evaluator, and Hy compiler-source lanes; it forces only the outer attrset
  shape needed for construction merge and leaves inner values lazy.
- Original v0 `pnixc-meta` still overwrites the duplicate explicit attrset case,
  so that single case is recorded as a known v0 divergence while Rust
  `interpret.rs` remains the ground truth.

Added **23 Rust corpus cases** plus **5 stage7 core lock-ins** for shallow update,
nested-path attrset construction merge, duplicate/path-conflict guards, integer
overflow, float infinity JSON errors, and `isFinite`/`isInf` guard behavior.

Gates (Python 3.11): `py_compile` green, `self_test_report` **853/853**,
`rust_corpus_report` **889/889**, `fixture_report` **32/32**,
`original_oracle_report` agree **112** / disagree **0** / unsupported **311**,
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7
parity lanes green over **416** core cases, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — path display normalization + eval_file baking (2026-06-29)

Closed the path display/source-relative normalization batch against the Rust
`eval_path_construction_normalization` and `eval_path_normalization_equality`
slices. pnix-hy now distinguishes the two Rust-grounded contexts:
- `eval_source` / `run_px_source` keep path values as canonical source-shaped
  text (`./a/../b` -> `./b`) so expression-level mirrors expose the same value
  surface as `eval_expr`.
- `run_px(file)` and imported `.px` files bake literal relative paths against the
  file directory, matching Rust `eval_file` and the original pnixc-meta oracle.
- FS builtins still resolve only at the filesystem boundary, using the active
  `base_dir`; path values themselves are no longer forced to absolute text.
- `builtins.dirOf` now returns a path when given a path, while string inputs stay
  string-level and are not path-normalized.
- Path equality/comparison no longer treats a path and an equal-looking string as
  equal; path-vs-path comparison uses normalized canonical text.

Added **24 Rust corpus cases** plus **9 stage7 core lock-ins** for path
construction normalization, path `dirOf` type/parent behavior, path equality and
ordering normalization, path/string mismatch, and string-level `dirOf` /
`baseNameOf` edge cases. Updated the existing `path-value` self-test to expect
`./foo` in expression mode.

Gates (Python 3.11): `py_compile` green, `self_test_report` **843/843**,
`rust_corpus_report` **866/866**, `fixture_report` **32/32**,
`original_oracle_report` agree **112** / disagree **0** / unsupported **306**,
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7
parity lanes green over **411** core cases, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — path/FS/hash context guard slice (2026-06-29)

Imported the first Rust-grounded `Paths/FS/IO` invariance slice for the real pnix
runtime surface that hy-meta depends on. This batch deliberately stayed narrow:
it fixes behavior needed for human-trackable stage15+ mirror control without
changing the larger path display policy yet.

Fixed across host interpreter, host compiler prelude, Hy evaluator, and Hy
compiler-source lanes:
- `path + path` is now total and remains a path; `string + path` preserves path
  string context; `path + context-bearing string` fails loudly and points to
  `builtins.unsafeDiscardStringContext`.
- `builtins.dirOf`, `builtins.baseNameOf`, and `builtins.hashString` preserve
  incoming string context.
- `builtins.hashFile` now uses the same hash policy/guard shape as
  `hashString`, preserves path/string context on the returned digest, rejects
  md5/sha1 as cryptographically broken, and keeps readFile/hashString parity.
- FS path resolver guard text now contains both Rust's `expected string or path`
  shape and the older pnix-hy corpus substring `expected path or string`, so the
  transition is explicit rather than silent.

Added **23 Rust corpus cases** plus **9 stage7 core lock-ins** for path addition,
context propagation, empty path rejection, `hashFile` algorithm/path guards, and
hash/read parity.

Gates (Python 3.11): `py_compile` green, `self_test_report` **825/825**,
`rust_corpus_report` **842/842**, `fixture_report` **32/32**,
`original_oracle_report` agree **106** / disagree **0** / unsupported **303**,
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7
parity lanes green over **402** core cases, stage15/stageN closure reproduced.

Follow-up: the full path display/source-relative normalization policy noted here
is closed by the later path display normalization pass above.

## ▶ IMPLEMENTATION PASS — regex invariance corpus (2026-06-29)

Imported the `eval_regex_ops` invariance slice. The value semantics already matched
Rust/Nix for anchored `match`, capture/null shape, Unicode classes, adjacent split
segments, empty-pattern split rejection, and POSIX character-class translation. The
only gap fixed was error text: invalid regex compilation is now wrapped as
`invalid regex: ...` in host, compiler prelude, Hy evaluator, and Hy compiler-source
lanes while preserving Python parse detail such as `unclosed`.

Added **17 Rust corpus cases** plus **3 stage7 core lock-ins** for invalid-regex
guards, Unicode capture, and adjacent split shape.

Gates (Python 3.11): `py_compile` green, `self_test_report` **807**,
`rust_corpus_report` **819/819**, `fixture_report` **32/32**,
`original_oracle_report` agree **105** / disagree **0** / unsupported **295**,
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7
parity lanes green over **393** core cases, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — JSON/TOML/data + tryEval/sort guards (2026-06-29)

Imported the JSON/TOML/data Rust regression slice and fixed the remaining semantic
gaps across all four lanes:
- `builtins.tryEval` no longer catches `abort`; errors whose message starts with
  `evaluation aborted:` propagate while throw/assert/type/division/recursion errors
  still return `{ success = false; value = false; }`.
- `builtins.sort` now has a Rust-shaped second-argument guard
  (`second argument must be list, got ...`) and still rejects non-bool comparators.
- compiler and Hy `builtins.fromTOML` parse errors now carry the `parse error`
  prefix, and non-string TOML args use the Rust `expected string` guard shape.
- `builtins.hashString` now rejects md5/sha1 as cryptographically broken and reports
  supported algorithms as `'sha256', 'sha512'`; hash/toFile string-arg guard text was
  tightened for the Rust data-parser tests without changing unrelated builtins.

Added **96 Rust corpus cases** from `eval_json_builtins`, `eval_data_parsers`,
`eval_tojson_tryeval_sort`, `eval_tryeval_paths`, and
`eval_tojson_context_propagation`, plus **11 stage7 core lock-ins** for JSON bounds,
toJSON context/function guards, TOML parse guards, hash policy, sort guards, lazy
`tryEval`, and `toFile` fake-path shape.

Gates (Python 3.11): `py_compile` green, `self_test_report` **801**,
`rust_corpus_report` **802/802**, `fixture_report` **32/32**,
`original_oracle_report` agree **103** / disagree **0** / unsupported **294**,
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7
parity lanes green over **390** core cases, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — force/toJSON/deepSeq/interpolation cycle guards (2026-06-29)

Imported and fixed the cycle-guard subset that fits the current evaluator shape:
- Thunk self-recursion now reports `infinite recursion encountered (...)` while
  retaining the old `recursive` substring for existing guards.
- `builtins.toJSON` and `builtins.deepSeq` now walk values with a per-descent
  identity stack, so cyclic attrsets/lists fail cleanly while shared DAGs still
  evaluate.
- String interpolation coercion now tracks attrsets currently being coerced through
  `__toString`/`outPath`, catching self and mutual interpolation cycles with
  `interpolation coercion cycle involving __toString`.
- Import-cycle message text was normalized to `import cycle`; full import-stack
  parity remains a separate item because pnix-hy currently realizes imports more
  lazily than Rust `eval_file`.

Added **25 Rust corpus cases** from `eval_force_cycle`,
`eval_cyclic_value_guards`, and `eval_interp_cycle_guard`, plus **5 stage7 core
lock-ins**. Left `eval_import_cycle`, `recursion_depth_guard`, and `tco_smoke`
open; current pnix-hy still needs an import strictness decision and a real
eval/trampoline depth strategy.

Gates (Python 3.11): `py_compile` green, `self_test_report` **779**,
`rust_corpus_report` **706/706**, `fixture_report` **32/32**,
`original_oracle_report` agree **102** / disagree **0** / unsupported **284**,
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7
parity lanes green over **379** core cases, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — compare/version/cycle guards (2026-06-29)

Imported the compare/version regression slice from `~/pnix` and fixed the actual
semantic gaps across all four lanes:
- Equality now follows Nix/pnix structural rules directly instead of comparing
  fully realized Python values: functions are never equal, nested list/attrset
  equality recurses explicitly, and `==`/`!=`/`builtins.eq`/`elem`/`find` share the
  same cycle guard (`infinite recursion encountered...`) and force-error behavior.
- Ordered comparison no longer realizes whole cyclic lists up front; `<`, `<=`,
  `>`, `>=`, and `builtins.lt/le/gt/ge` use the same bounded recursive list compare.
- `builtins.compareVersions` now matches the Rust/Nix component rules for missing
  components, `pre`, numeric-vs-string components, `+rev`/`~rc`, and non-string
  guard text (`expected two strings`).
- `builtins.splitVersion` and `builtins.parseDrvName` preserve input string context,
  and `parseDrvName` splits on the last `-` followed by a digit.

Added **115 Rust corpus cases** from `eval_compare_equality`,
`eval_compare_versions`, `eval_compare_cycle_guard`,
`eval_equality_cycle_guard`, and `eval_version_parse_context`. Added **12 stage7
core lock-ins** for function equality, version ordering/splitting, context
propagation, non-string guards, and equality/comparison cycle guards.

Gates (Python 3.11): `py_compile` green, `self_test_report` **769**,
`rust_corpus_report` **681/681**, `fixture_report` **32/32**,
`original_oracle_report` agree **102** / disagree **0** / unsupported **279**,
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7
parity lanes green over **374** core cases, stage15/stageN closure reproduced.

## ▶ IMPLEMENTATION PASS — dynamic attrs + list items + regex POSIX + functional corpus (2026-06-29)

Implemented the next corpus-import batch from the Lambdas/laziness/types and
nixpkgs-lib-patterns area. Fixed three real semantic/parser gaps across all four
lanes:
- Nix/pnix list item grammar: `[ x x ]` is two list items, not application; pnix
  also keeps Rust-ground-truth unary list literals like `[1 -2 3]`.
- Dynamic attrset keys: `{ ${name} = ...; }`, including rec attrsets where key
  expressions can see static rec bindings; generated dynamic keys have no
  `unsafeGetAttrPos` source position. Dynamic attrs remain rejected in `let`.
- Regex POSIX character classes, e.g. `builtins.split "[[:space:]]+" ...`,
  translated consistently in host interpreter, host compiler prelude, Hy
  evaluator, and Hy compiler-source lanes.

Added **80 Rust corpus cases** for functional composition/laziness/type handling,
with/priority/inherit guards, dynamic attrs, POSIX regex split, and representative
nixpkgs-lib patterns (`fix`, `makeOverridable`, `genAttrs`, `foldAttrs`,
cartesian-product dynamic keys). Added **7 stage7 core lock-ins** for the new
parser/runtime behavior.

Gates (Python 3.11): `py_compile` green, `self_test_report` **745**,
`rust_corpus_report` **566/566**, `fixture_report` **32/32**,
`original_oracle_report` agree **100** / disagree **0** / unsupported **269**,
`pnix_mirror.self_test_report(include_hy_host=True)` ready with all four stage7
parity lanes green over **362** core cases, stage15/stageN closure reproduced.

## ▶ NEXT WORK + HOW-TO GUIDE (for Codex / others) — 2026-06-29

### REVIEW #3 verdict (string-context + list/seq/fold lazy + core import/fold)
Verified the latest Codex work against ground truth — **no bug found, implementation
sound**. Lazy semantics match real Nix EXACTLY (`?` not forcing the final attr;
`map`/`genList`/`concatLists`/`attrValues`/`values`/`mapAttrs`/`catAttrs`/`zipAttrsWith`
producers stay lazy; `attrByPath` default lazy). Guards correct. The connection had
dropped before Codex ran the FULL stage7 mirror on its final state, so this review
ran it: **all four lanes green**. Added **32 lock-in regression cases** (22 in
`RUST_EVAL_CORPUS` + 10 in the stage7 core set) so the new laziness/guards cannot
silently regress.
Gates (Python 3.11): `self_test_report` **731**, `rust_corpus_report` **486/486**,
`fixture_report` **32/32**, `original_oracle_report` agree **98** / disagree **0** /
unsupported 264, `pnix_mirror.self_test_report(include_hy_host=True)` ready with all
four stage7 parity lanes **355/355**, stage15/stageN closure reproduced.

### ⚙ HOW TO IMPLEMENT a builtin / semantic change — the 4-LANE discipline
Everything is in ONE file: `pnix_hy/pnix_runtime.py`. A change is "done" only when
ALL FOUR lanes produce the same value/error. Edit each lane:
1. **Host interpreter (Python).** The builtins dict inside `initial_env(...)`
   (search `"toString": lambda`). Value model near the top: `Thunk`, `Closure`,
   `NativeFunc`, `PnixString` (str subclass carrying `.context`), `PnixPath`,
   `AttrSet`; helpers `force_value`, `realize_value`, `apply_pnix`,
   `string_text_context`, `make_context_string`, `collect_string_context`.
2. **Host compiler (`run_px` / `compile_px_source`).** Emits Python targeting the
   `COMPILER_PRELUDE` string. Add a `_xxx` helper in the prelude, wire it in the
   `_bi()` builtin table (`b["name"]=_C(...)`), AND if imported `.px` files must see
   it, add the helper name to **`_COMPILER_RUNTIME_NAMES`** (~line 10179) so it is
   exported into imported-module namespaces. Prelude types: `_T` thunk, `_C` closure,
   `_P` path, `_S` context-string, `_A` attrset, `_K` constructor; `_force`/`_realize`/
   `_tv`/`_apply`/`_isstr`/`_ctx`/`_mkstr`/`_strctx`.
3. **Hy evaluator (stage7 source lane).** The big Hy source string
   `HY_AST_EVALUATOR_SOURCE`: add `(defn xxx-builtin ...)` and wire it into the
   native-builtins block.
4. **Hy compiler emitter (stage7 compiler-source lane).** The Hy emit functions;
   they reuse prelude helpers, but the emitted SHAPE must match host (e.g.
   string interpolation must emit `_concatstrings([...])`, not `(a+b)`).

**Laziness contract (critical):** list/attrset PRODUCERS store a `Thunk`/`_T` per
element/value (never a forced value); CONSUMERS (`head`/`elemAt`/`length`/`==`/
`toJSON`/`sort`) force as needed. `?` checks key existence only — must NOT force the
final attr value. `attrByPath` default is lazy via `NativeFunc(force_arg=False)`.

**stage7 Hy gotchas (these bite every time):**
- bare `str` cannot be a class base in stage7 → use `(type "Name" (tuple [dict]) {})`
  or subclass `builtins.str` + `setattr`.
- `#{computed}` set-literals / `lfor ... :if` can fail → use `(set [...])` + explicit
  `for`/`.add`.
- `except [Type :as e]` is rejected → use `(except [Type e] ...)`.
- A stray paren in the Hy source string closes the top-level `(do ...)` early →
  `LexException Ran into a ')'`. Debug: `rt.hy_runtime_source_for_asts([rt.parse(src)])`
  then parse with `hy.reader.read_many` under `PYTHONPATH=.:~/pnix-hy` and
  find where the first top-level form ends (it should be the LAST line).
- Never put `pos`/`path_positions` in value hashes; AST round-trip uses `ast_hash`
  (position-stripped), value/result hashes use `term_hash`. (A prior regression came
  from mixing these — `pnix_mirror.run_once` must use `ast_hash` for ast hashes.)

### ✅ HOW TO VERIFY — run BEFORE every commit (skipping this shipped a regression)
Use `PYTHONPATH=. /tmp/pnix-hy-py311-venv/bin/python` (Python 3.11; 3.14 via
`PNIX_HY_PYTHON`).
1. **Syntax:** `python -m py_compile pnix_hy/pnix_runtime.py pnix_hy/pnix_mirror.py`
2. **Host gates (fast, no stage7):**
   `rt.self_test_report()['ready']`; `rt.rust_corpus_report()` (static ground truth,
   `disagree` not a field — all must be `ok`); `rt.fixture_report()` → 32/32;
   `rt.original_oracle_report()` → **disagree must be 0** (agree/unsupported vary;
   the v0 binary is limited so most cases are `unsupported`, which is fine).
3. **Stage7 lanes (slow, ~25s kernel boot each).** Iterate fast with a SUBSET:
   `pnix_mirror.hy_runtime_batch(cases)`, `hy_source_runtime_batch`,
   `hy_compiler_batch`, `hy_compiler_source_batch` (pass a list of just your new
   `{name,source,expect}` cases). THEN the FINAL gate:
   `pnix_mirror.self_test_report(include_hy_host=True)` → `ready` True, all four
   `*_parity` lanes equal count, `closure` reproduced. **Always run this full gate
   before claiming done** — it is the only thing that catches lane divergence.
4. **Ground-truth oracles (authority order: `~/pnix` > real Nix > v0 binary):**
   - `~/pnix/crates/pnix-eval/src/interpret.rs` (eval semantics) and
     `crates/pnix-core/src/lang/pnix/lexer/mod.rs` (lexer/strings) — THE authority.
     pnix EXTENDS Nix and deliberately differs (tabs ARE indentation;
     `unsafeGetAttrPos` = attr-item line/col; truncated `%`). Do NOT "fix" toward Nix.
   - real Nix: `/run/current-system/sw/bin/nix-instantiate --eval --strict --expr '…'`
     (secondary check for the Nix-compatible subset).
   - v0: `~/pnix/target/release/pnixc-meta <file.px>` (live, but limited — parse/eval
     errors there are v0 gaps, not pnix-hy bugs).
5. **Corpus discipline:** VALUE cases → `RUST_EVAL_CORPUS` `{name,source,expect}` +
   a representative subset into `SELF_TEST_CASES` and `HY_RUNTIME_CORE_CASES` (so they
   run all 4 stage7 lanes). ERROR cases → `RUST_EVAL_CORPUS`
   `{error:True,error_contains:"…"}` (asserts BOTH interp and compiler raise with the
   substring); for stage7 use the value form `(builtins.tryEval (…)).success == false`.
   Commit + push each verified batch (small checkpoints).

### 📋 Remaining implementation (prioritized) — B corpus import is the bulk
For each file: read the `~/pnix/crates/pnix-eval/tests/*.rs`, vendor passing cases as
static corpus data, and fix any gap across all 4 lanes per the guide above.
- [x] **Lambdas/laziness/types:** DONE — `eval_functional_lazy`(`rs-func-*`),
      `eval_type_handling`(`rs-th-*`), `eval_lambda_attrset_corners`(`rs-lc-*`),
      `eval_with_lazy`+`eval_with_priority`(`rs-wl-*`/`rs-wp-*`), `eval_inherit_lazy`
      (`rs-inherit-*`), `eval_nixpkgs_lib_patterns`(`rs-nixlib-*`: fix-point/optional/
      optionalAttrs/recursiveUpdate/makeOverridable/makeExtensible/genAttrs/
      foldAttrs/mapAttrsToList/mkOverride/composeExtensions/hasPrefix).
- [x] **Compare/versions:** `eval_compare_equality`, `eval_compare_versions`,
      `eval_compare_cycle_guard`, `eval_equality_cycle_guard`, `eval_version_parse_context`.
- [~] **Cycle/recursion guards:** DONE — `eval_force_cycle`,
      `eval_cyclic_value_guards` (toJSON/deepSeq cyclic), `eval_interp_cycle_guard`
      (`rs-cycle-*`/`rs-fc-*`; messages aligned: "infinite recursion",
      "interpolation coercion cycle"+"__toString"). REMAINING: `recursion_depth_guard`
      + `tco_smoke` (thread/stack-depth tests — env-specific, harder to vendor as
      value cases; `eval_import_cycle` covered by run_px import-cycle self-test).
- [x] **JSON/TOML/data:** `eval_json_builtins`, `eval_data_parsers`,
      `eval_tojson_tryeval_sort`, `eval_tryeval_paths`.
- [x] **Regex:** `eval_regex_ops`.
- [x] **Paths/FS/IO:** `eval_path_ops`, `eval_filesystem_ops`, `eval_hashfile_builtin`,
      `io_builtins`, remaining `eval_path_*`, `eval_update_path_arith`.
- [x] **Guards/positions/misc:** `eval_source_position`, `eval_px_files`,
      `eval_nix_lang_parity`, `eval_lang_version_builtin`, `eval_derivation_builtin`,
      `eval_unsafe_add_output_builtins`, `legacy_aliases`, `eval_bool_required_positions`,
      `eval_attr_concat_guards`, `eval_abort_with_string_guards`,
      `eval_let_lambda_dup_guards`, `eval_dup_attr`, `eval_addcontext_pos_bitops_guards`,
      `eval_appendcontext_value_shape_guard`.
- [x] **C — exact behavior for present builtin families:** derivation
      (`derivationStrict`/required attrs/output paths/contexts), filesystem/path edge
      cases & error messages, regex `match`/`split` edge cases, data parsers
      (`fromJSON` bounds, TOML edge, `toJSON` errors).
- [~] **E — `.px` module/import breadth:** make `~/pnix/stdlib/*.px` loadable; mirror
      original import registry / preparsed behavior; add a repo-local fixture gate
      (pnix-hy must NOT depend on `~/pnix` at runtime).
- [x] **F — meta-circular finish:** re-sync the stage7 Hy compiler emitter to host
      strictness + partial-eval (lanes currently agree by VALUE, not emitted shape);
      decide semantic owner (Hy-in-stage7 vs a `.px` evaluator/compiler owner).
- [~] **G — performance:** benchmark harness (`pnixc-meta` vs interp vs compiler
      compile-once/exec-many vs stage7); type/shape specialization.
- [x] **H — misc:** `canonical-json`/`value-to-string` edge cases (non-identifier attr
      keys, JSON control-char escaping `\uXXXX`/`\b`/`\f`).

Already imported (do not redo): `eval_basics`, `builtin_parity`,
`eval_arith_builtin_overflow`, `eval_string_ops`, `eval_to_string`, the string-context
family, `eval_list_bounds`/`filter_elem_listtoattrs`/`seq_any_all`/`length_foldr` +
list lazy guards, `eval_modulo_op`/`pow_precision_guard`/`genlist_floor_ceil_guards`,
core `import`/`scopedImport`/`fold`.

**Do NOT implement:** the ⛔ NON-GOALS section below (domain markup/schema ~103
builtins, ontology/evidence engine, pnix mounts, coding-agent product test files).

## ⚠ REVIEW #2 — guard/error/overflow + source-position pass (2026-06-29)

Reviewed Codex's guard/error/overflow + source-position work against ground truth
(real Nix + `~/pnix` `interpret.rs`/lexer). Expected values are CORRECT: modulo
truncated remainder (`(-10)%3=-1`, `10%(-3)=1`), `10%0` errors, `10.5%3=1.5`,
`(-10.5)%3=-1.5` (C fmod); `pow 3 39 = 4052555153018976267` exact, `pow 2 62` int,
`pow 2 63` → float; `unsafeGetAttrPos` returns the attr-item line/col per
`interpret.rs:9271/3326` (this correctly DIVERGES from real Nix's column quirk —
      `~/pnix` is the authority). Full gates re-run green: self_test 622, rust_corpus
182/182, fixture 32/32, **all four stage7 lanes 302/302** (Codex had not run these
full after its changes).

### 고친거 (bug found + fixed in this review)
- [x] **Regression: the `pnix_mirror` mirror chain diverged (ready=False).**
      Adding `pos`/`path_positions` to the AST made `term_hash(ast) !=
      term_hash(reparsed)` (emit canonicalizes layout, so reparsing shifts byte
      offsets), so `run_once`/`mirror_chain`/`run_mirror`/
      `pnix_mirror.self_test_report` all broke. Codex patched only
      `pnix_runtime.self_test_report` (to `ast_hash`) and never ran the mirror.
      FIXED `pnix_mirror.run_once` to use `rt.ast_hash` (position-stripped) for the
      ast/emitted-ast hashes. Verified: chain converges, `run_mirror` ready=True,
      mirror self-test green.

### 고친 자잘한 갭 (REVIEW #2 follow-up)
- [x] **Division-by-zero leaked the raw Python message.** `7 / 0` and
      `builtins.div 7 0` raise `"integer division or modulo by zero"`; `7.0 / 0.0`
      raises `"float division by zero"`. `~/pnix` + Nix say **"division by zero"**.
      Wrap as a clean PnixError `"division by zero"` in ALL lanes (interp
      `apply_binary` `/`, compiler `_bin`, Hy mirror `apply-binary` + `builtins.div`),
      exactly like the existing modulo-by-zero guard. Add corpus cases.
- [x] **Compiler leaked raw `'NoneType' object is not subscriptable` on
      `null.attr` select** (e.g. `(null).foo`); the interpreter gives the clean
      `"select base must be an attrset"`. Make the compiler select/`_seldef` path
      raise the same pnix error so the lanes agree. (`(null).foo or x` already
      works on both lanes.)

## ✅ VERIFICATION FINDINGS — indented-string / block-comment review (2026-06-29)

Reviewed the prior `''...''` indented-string + `/* */` block-comment work against
the AUTHORITATIVE `~/pnix` lexer (`crates/pnix-core/src/lang/pnix/lexer/mod.rs`),
with real Nix as a secondary check. Verdict: block comments correct; the one
indented-string bug found in review is now fixed. Superseding follow-up on the
same day completed the rest of grammar group A listed below.

### 고친거 (bugs fixed)
- [x] **Plain (non-interp) indented string emitted a spurious LEADING newline for
      each leading whitespace-only line.** `''  \n  hello\n''` → got `\nhello\n`,
      want `hello\n` (matches BOTH `~/pnix` lexer and real Nix). Fixed by making
      the plain Python and Hy mirror strip paths append `\n` only once output is
      non-empty, matching `~/pnix`; interp path was already correct. Covered by
      `str-indented-leading-whitespace-only`.

### ✅ verified correct — do NOT "fix" toward Nix
- `tab-indent` (`''\n\ta\n\tb\n''` → strips the tab) and `''\n''` → `\n` are
  CORRECT per the `~/pnix` lexer (it counts tabs as indentation and keeps the
  trailing newline) even though real Nix differs. `~/pnix` is the authority
  (memory: "Nix is a secondary check"), so leave these as-is.
- Block comments `/* … */` are non-nestable (first `*/` closes), matching the
  `~/pnix` lexer; verified vs real Nix incl. `/*/*/`, in-string, multiline.

### 추가 완료 (grammar group A follow-up)
- [x] `inherit` / `inherit (expr)`; search paths `<…>`; absolute/home/interp
      paths; nested let-binding paths; match guards; duplicate-attr diagnostics.
      Covered in Python runtime/compiler plus Hy source parser/evaluator/compiler
      lanes; see proof counts below.

## ⬆ REMAINING WORK — master checklist (2026-06-29)

All open items pulled to the top. Compiled from this file's unchecked items PLUS a
fresh 3-agent local survey of `~/pnix` (builtins, the 331-file test corpus, and
grammar). pnix-hy's ground truth is `~/pnix` *locally* (it is NOT a project that
depends on `~/pnix` at runtime — the survey reads it only at authoring time), so
this was surveyed locally rather than via web `/deep-research`; web research is not
the right tool when the spec lives on disk. Detailed context for every item is in
the P0–P5 / residual-gap sections far below. Order = priority within the narrow
language-parity slice; domain/product extensions are listed but flagged optional.

Survey headline numbers: original builtins ≈259, pnix-hy ≈156 → ~110 "missing",
but **only ~7 are pnix-LANGUAGE** (`import`/`scopedImport`/`fold` + exactness on
present families); the other **~103 are domain extension libraries → NON-GOALS**
(see the ⛔ section, DO NOT implement). Test corpus 331 files → **~75 core-language**
files (~514 cases) to import (464 done); the other 253 are domain/product →
NON-GOALS. Grammar group A parser constructs are now closed; remaining work is
mostly corpus import plus exact semantic tightening below.

**▶ Recommended next order for Codex:** (1) **B corpus import** grouped by
semantics (remaining lists, compare, regex, json, paths, lambdas, guards).
(2) **D exact behavior** for already-present language builtin families.
Reminder: every new behavior must agree on ALL FOUR lanes (interp, compiler/run_px,
stage7 Hy source, stage7 Hy compiler-source) — run the FULL stage7 batches (not
just subsets) AND `pnix_mirror.self_test_report()` before claiming done.

### A. Grammar parity (parser) — highest priority; real stdlib/fixture usage
- [x] Indented strings `''...''` (escapes `'''`, `''$`, `''\n`/`\t`/`\r`; `${}` interp). USED in stdlib + fixtures.
- [x] `inherit a b;` and `inherit (expr) a b;` in attrsets/lets. USED in stdlib + fixtures.
- [x] Search paths `<lib/x.px>` / `<nixpkgs>`. USED heavily in stdlib + fixtures.
- [x] Absolute paths `/abs/p`, home paths `~/p`, path interpolation `./a/${x}`. USED in fixtures.
- [x] Block comments `/* ... */` — non-nestable, verified vs ~/pnix lexer + Nix.
- [x] Nested let-binding paths `let a.b = 1; in …` (pnix-hy v0 forbids; original allows).
- [x] Match guards `| pat if cond => body`.
- [x] Duplicate-attribute diagnostics + recursive-attr/no-scope-inherit conflict rules.

### B. Rust ground-truth corpus import (P0) — ~75 core files (~514 cases) tracked
Done: `eval_basics`(41)+`builtin_parity`(51)+`eval_arith_builtin_overflow`(10)
+ targeted inherit/dup/match/path grammar cases (26)
+ targeted guard/error/overflow cases (48)
+ targeted source-position cases (6)
+ targeted string ops / toString corpus cases (45)
+ targeted string-context propagation cases (38)
+ targeted toJSON/path/toFile/output-context cases (50)
+ targeted string-context equality/comparison/param/emit cases (52)
+ targeted core builtin surface cases (4)
+ targeted list bounds/filter/elem/listToAttrs + seq/any/all + length/foldr cases
  (93)=464. Vendor the rest as static data (no `~/pnix` runtime dep), grouped by
semantics:
- [x] Strings/context: `eval_string_ops`, `eval_to_string`, `eval_tostring_{cycle_guard,list_context}`, `eval_concat_match_split_context`, `eval_string_context*` (≈8 files — string-context is foundational, appears in 11+ files).
- [x] Lists: `eval_list_bounds`, `eval_filter_elem_listtoattrs`, `eval_seq_any_all`, `eval_length_foldr`, `eval_fold_groupby_guards`, `eval_list_laziness_guards`, `eval_replace_strings_list_guards`, `eval_zipattrswith_lazy_guard`.
- [x] Compare/versions: `eval_compare_equality`, `eval_compare_versions`, `eval_compare_cycle_guard`, `eval_equality_cycle_guard`, `eval_version_parse_context`.
- [x] Regex: `eval_regex_ops`.
- [x] JSON/TOML/data: `eval_json_builtins`, `eval_data_parsers`, `eval_tojson_{context_propagation,tryeval_sort}`, `eval_tryeval_paths`.
- [x] Paths/FS/IO: `eval_path_ops`, `eval_path_{builtin_context_string,construction_normalization,normalization_equality,string_concat_context}`, `eval_filesystem_ops`, `eval_hashfile_builtin`, `io_builtins`, `eval_tofile_context_guard`.
- [x] Lambdas/laziness/types: `eval_functional_lazy`(57), `eval_type_handling`(48), `eval_nixpkgs_lib_patterns`, `eval_lambda_attrset_corners`, `eval_with_{lazy,priority}`, `eval_inherit_lazy`.
- [x] Arith/guards: `eval_modulo_op`, `eval_pow_precision_guard`, `eval_genlist_floor_ceil_guards`, `eval_i64_min_overflow_guards`, `eval_update_path_arith`.
- [~] Cycle/recursion guards: imported `cycle_force_v2`, `eval_force_cycle`, `eval_cyclic_value_guards`, `eval_interp_cycle_guard`, and import-cycle self-test. Remaining `recursion_depth_guard` + `tco_smoke` stay env/stack-depth policy checks rather than value corpus gates.
- [x] Guards/positions/misc: `eval_addcontext_pos_bitops_guards`, `eval_bool_required_positions`, `eval_attr_concat_guards`, `eval_appendcontext_value_shape_guard`, `eval_abort_with_string_guards`, `eval_let_lambda_dup_guards`, `eval_dup_attr`, `eval_source_position`, `eval_px_files`, `eval_nix_lang_parity`, `eval_lang_version_builtin`, `eval_derivation_builtin`, `eval_unsafe_add_output_builtins`, `legacy_aliases`.
- Note: EXCLUDE domain/product test files (`eval_coding_*`, `eval_adaptive_*`, `eval_macro_only_boot_*`, `*evidence*`, `*audit*`, `eval_mirror_*`, markup/schema) — 253 files, out of the narrow slice.

### C. Exact semantics tightening (P0)
- [x] Guard/error/overflow remainder (i64 +/-/*// overflow DONE): modulo-by-zero, pow overflow→float boundary, genList/take/drop negative guards, any/all/filter predicate type errors with index, sort comparator type errors, fromJSON integer bounds, JSON special-float policy.
- [x] Real source positions: `__curPos`, `unsafeGetAttrPos`, literal attr line/col, file labels, generated-attr → `null`.
- [x] Real string-context propagation (not stubs): `getContext`/`hasContext`/`appendContext`/`unsafeDiscardStringContext`/`addDrvOutputDependencies`/`unsafeDiscardOutputDependency`/`unsafeAddOutputDependency`/`unsafeAddOutputName`; derivation output contexts. (NOTE: `stringContextToProvenance` is product, not language → NON-GOALS.)
      Closed 2026-06-29 with context-carrying strings across host interpreter,
      host compiler, Hy evaluator, and Hy compiler-source lanes. Proof:
      `self_test_report` 711 cases, `rust_corpus_report` 464/464,
      `fixture_report` 32/32, `original_oracle_report` agree 96 / disagree 0 /
      unsupported 250, later re-run after list/seq/fold import agree 97 /
      disagree 0 / unsupported 255, and
      `pnix_mirror.self_test_report(include_hy_host=True)` ready with all four
      stage7 parity lanes 345/345.

### D. Builtin parity (P3) — pnix-LANGUAGE builtins only
Of ~110 missing builtins, only the language ones below are in scope; the other
~103 are domain extension libraries → see NON-GOALS section.
- [x] Core/system: `builtins.import`, `scopedImport`, `fold`.
- [x] Exact behavior for already-present language families: derivation (required attrs/output paths/contexts/`derivationStrict`), filesystem/path (`pathExists`/`readFile`/`readDir`/`readFileType`/`hashFile`/`toPath`/`storePath`/`baseNameOf`/`dirOf`), regex `match`/`split` edge cases, data parsers (`fromJSON` bounds, TOML edge, `toJSON` errors).

### E. `.px` module/import breadth (P1)
- [~] Make `~/pnix/stdlib/*.px` loadable as a library corpus: blocked in this checkout because `~/pnix/stdlib` is absent; do not add a hardcoded external dependency.
- [~] Mirror original import registry / preparsed import behavior (`runtime.px`/`evaluator.px`): no registry surface is exposed in the current `~/pnix`; keep `run_px`/relative-import semantics as the local owner until an upstream registry exists.
- [x] `builtins.import` / `scopedImport` compatibility (overlaps D): covered by `import_self_test_cases()` and the repo-local fixture corpus.
- [x] Repo-local fixture corpus OR documented external-fixture gate (pnix-hy must not depend on `~/pnix`; CI must not silently skip): `fixtures/pnix_expr` is the default `fixture_report()` corpus and is included in `--check`.

### F. Meta-circular finish (P4)
- [x] Re-sync stage7 Hy compiler emitter with host strictness + partial-eval: stage7 now folds closed scalar terms and emits strict binary operands like the host emitter; `compiler_emit_shape_report` compares emitted Python directly and is in `--check`.
- [x] Decide semantic owner: keep Hy-in-stage7 as the verified mirror plus host Python as the fast path for this slice; defer a `.px` evaluator/compiler owner until the original import registry/preparsed surface is actually exposed.

### G. Performance (P5)
- [x] Benchmark harness over shared inputs: `pnix_mirror.performance_report()` times parse, canonical emit, compiler emit, Python compile, interpreter eval, compiler compile+exec, and compiler compile-once/exec-many while excluding process startup. External `pnixc-meta` is reported but not timed unless an in-process oracle appears; stage7 uses the existing slow four-lane projection when explicitly requested.
- [x] Track generated-Python size + bytecode op count via `hy_mirror.full_introspection`; exposed through `--perf` and `performance_report_check` in `--check`.
- [~] Specialization: type/shape-specialized `_bin`/select/apply; (ambitious) self-tracing mini-JIT. Keep only measurable wins.

### H. Misc deferred
- [x] `canonical-json` / `value-to-string` edge cases: non-identifier attr keys, JSON control-char escaping (`\uXXXX`, `\b`, `\f`).
- [x] Keep oracles green and OPTIONAL: `fixture_report()` now defaults to the repo-local corpus; `original_oracle_report()` remains optional/available-false when the external binary is absent; `rust_corpus_report` + `self_test_report` + stage7 are the self-contained gates.

## ⛔ NON-GOALS — added extension libraries, NOT the pnix language (DO NOT implement)

These exist in `~/pnix` but are **domain/product extension libraries layered on
top of the language**, not the pnix language itself. They are explicitly out of
this slice — do NOT add them (no checkbox; this is "won't do", not "to do"). This
is what makes the ~110 "missing builtins" number misleading: only ~7 are language.

- **Domain markup/schema builtins (~103)** — `*Schema{Normalize,Validate,Explain}`
  + emit/render families: `xmlSchema*`, `mathml*`, `openmath*`, `svg*`,
  `x3d*`/`x3dom*`, `cellml*`, `sbml*`/`sbgnml*`/`sedml*`, `collada*`, `pdbml*`,
  `neuroml*`, `biopax*`, `cml*`, `gifti*`/`lems*`/`omex*`/`pharmml*`,
  `excel*`/`ods*`, `vtk*`/`xdmf*`/`ifcxml*`, `hanim*`, `programSchema*`, `toXML`.
  Scientific/CAD/office format processing — unrelated to pnix-the-language.
- **Ontology / evidence engine builtins (7)** — `ontologyEvaluate`/`Lift`/
  `Promote`/`PromoteWithLane`/`Query`/`Select`/`Emit`, and
  `stringContextToProvenance`. pnix's proof/evidence PRODUCT subsystem.
- **pnix host-infrastructure builtins (4)** — `pnixMount`/`pnixMounts`/`pnixRun`/
  `pnixUmount`. Host storage/runtime integration, not the language.
- **Locale/domain stdlib helper** — `koreanFinalConsonantKind` (Hangul jongseong
  classifier for a stdlib case-marker lens).
- **Domain/product test files (253 of 331)** — `eval_coding_*`/`eval_coding_project_*`,
  `eval_adaptive_*`, `eval_macro_only_boot_*`, `*evidence*`, `*audit*`,
  `eval_mirror_*`, `eval_universal_task_*`/`eval_task_6w_*`, `lift_rules_*`,
  route/session/agent/markup-schema tests. These exercise the pnix self-hosting
  coding-agent / evidence-federation PRODUCT, not the base language. Not imported.
- **Per the scope rule (top of file):** no CLI wrappers, bin scripts, separate
  test harnesses, README scaffolds, stores, product lanes, or extra runtime
  systems.

Boundary test: a feature is in scope only if it is part of the pnix LANGUAGE
(syntax, evaluation semantics, core/Nix-family builtins). Format processors,
ontology/evidence engines, the coding-agent product, and host mounts are not.

---

Scope is intentionally narrow. This directory owns only:

```text
hy(py) mirror -> pnix runtime -> pnix mirror
```

Do not add CLI wrappers, bin scripts, separate test harnesses, README scaffolds,
stores, product lanes, or extra runtime systems in this slice. Verification is
done by importing the three modules directly.

Compatibility rule: this is not a Python-runtime-independent fork. Python's
runtime, import system, module cache, AST, traceback, and native extension
boundary remain explicit compatibility boundaries.

## hy-meta dependency: stage15/stageN closed (was framed as stage7)

`../hy-meta/todo.md` records that hy-meta is now closed far past the old stage7
eval mirror. Its proof ladder runs:

```text
stage7  = semantic/eval mirror closure (the Hy KERNEL that runs code)
stage8  = Python artifact reproducibility after fresh meta-circular reload
stage9  = clean product runtime replay
stage10 = client/server/session/sandbox replay closure
stage11 = multi-domain adapter closure
stage12 = self-improvement quarantine closure
stage13 = long-horizon product organism closure
stage14 = cross-host/cross-implementation pnix law closure
stage15 = open-world external evidence federation closure
stageN  = versioned constitutional extension (stage16 is the first concrete one)
```

Only 3 items remain open upstream, all in stage14 and all blocked on a cross-repo
Clojure-host schema decision (a live Clojure host exposing a stage14-compatible
export). None of them block pnix-hy.

Important nuance (do NOT mechanically rename "stage7" to "stage15" in the
execution path): in hy-meta, **stage7 is the eval kernel**, i.e. the only thing
that actually executes Hy (`stage2/kernel.hy`, verified through the 7-deep mirror
tower). Stages 8..N are proof/federation lanes layered ON TOP of it, not
different execution engines, so there is no "stage15 kernel" to run code in.
pnix-hy therefore still executes inside the stage7 kernel via `stage7-kernel-run`
(that name is correct and stays). What upgrades to "stage15+" is the **evidence
pnix-hy federates** from hy-meta:

- [x] `hy_mirror.closure_probe()` runs `stage15-check` + `stagen-check` and
      asserts each reports `reproduced` (the cheap, representative top of the
      ladder; the stageN lane fails closed if any lower stage regressed).
      `stage_status_check(stage)` / `stage15_check()` / `stagen_check()` reach any
      single stage; `STAGE_VERDICT_KEYS` maps each `<stage>-check` to its verdict
      field (stage9 uses `product_replay_status`, the rest `<stage>_status`).
- [x] `hy_mirror.host_summary(light=True, closure=False)` now reports the kernel
      probe under `kernel_probe` (renamed from `stage7_probe`) and adds an
      optional `closure` block when `closure=True`.
- [x] `pnix_mirror.self_test_report()` federates both: the
      `hy-meta-kernel-eval-probe` case (was `hy-meta-stage7-light-probe`) and a
      new `hy-meta-closure-stage15n` case driven by `closure_probe()`.

## Files

- [x] `pnix_hy/hy_mirror.py`: bridge to the sibling `../hy-meta` proof ladder --
      the stage7 eval kernel (execution substrate) plus the stage15/stageN
      closure verdict (federated host evidence).
- [x] `pnix_hy/pnix_runtime.py`: pnix parser/evaluator/runtime surface.
- [x] `pnix_hy/pnix_mirror.py`: pnix mirror chain and convergence report.
- [x] `todo.md`: this scope ledger.

## Done

- [x] hy(py) mirror delegates to `../hy-meta/bootstrap.py`.
- [x] hy(py) mirror supports the existing Python 3.11 proof lane.
- [x] hy(py) mirror supports the Homebrew Python 3.14 proof lane through
      `PNIX_HY_PYTHON=/tmp/pnix-hy-py314-venv/bin/python`.
- [x] hy(py) mirror can execute a Hy-written AST evaluator through the stage7
      eval kernel and return JSON to Python for parity checks.
- [x] pnix runtime evaluates integers, strings, booleans, null, lists, attrsets,
      recursive attrsets, recursive `let`, `if`, lambdas, application,
      selection, `?`, string interpolation, and the pnix-clj v0 builtin subset.
- [x] pnix runtime emits deterministic source and re-parses it for round-trip
      integrity.
- [x] pnix runtime hashes canonical JSON for AST/result comparison.
- [x] pnix mirror runs the same source through a 7-pass convergence chain.
- [x] pnix mirror includes hy(py) host evidence: the stage7 kernel eval probe
      and the stage15/stageN closure verdict (`closure_probe`).
- [x] pnix mirror compares the Python runtime and Hy-written AST evaluator on
      the core corpus, including recursive `let`, recursive attrsets, and
      forward references.
- [x] Hy-written AST evaluator handles string interpolation assembly,
      nested interpolation, `builtins.toString` interpolation, and unknown-var
      placeholder preservation.
- [x] Hy-written AST evaluator handles list construction/indexing and the
      pnix-clj v0 higher-order builtin subset: `elemAt`, `map`, `filter`,
      `foldl'`, `concatMap`, `genList`, `attrValues`, `mapAttrs`, and `sort`.
- [x] Hy-written source parser tokenizes and parses the 41-case runtime corpus
      inside stage7, so the Hy runtime no longer needs Python-provided AST JSON
      for the proof corpus.

## Current Proof Surface

- [x] `pnix_runtime.self_test_report()` passes: ready **731** cases (current; grew
      through quoted-dot, string-interp `__toString`/`outPath`, indented-string,
      block-comment, inherit/nested-let, match-guard, path/search/interp,
      guard/error/overflow, source-position, string-context, list/seq/fold lazy, and
      core import/fold + lazy-guard lock-in cases). See the NEXT WORK header for the
      authoritative gate snapshot.
- [x] `pnix_runtime.rust_corpus_report()` ready **486/486** (interp + compiler).
- [x] `pnix_mirror.run_mirror("rec { x = 1; y = x + 41; }.y")` converges over 7 runs.
- [x] Kernel parity lanes pass (each runs inside the stage7 eval kernel):
      `hy_runtime_batch`, `hy_source_runtime_batch`, `hy_compiler_batch`, and
      `hy_compiler_source_batch` are each ready **355/355** (current; was 249 at the
      grammar baseline).
- [x] Source-position stage7 spot checks pass in source lanes:
      `hy_source_runtime_batch` and `hy_compiler_source_batch` are each ready `5/5`
      for `__curPos` and `unsafeGetAttrPos` line/column/generated-null behavior.
- [x] `pnix_runtime.fixture_report()` passes original expected fixtures:
      `32/32` against `~/pnix/fixtures/pnix_expr/*.expected.json` (3 expected.json
      files corrected to ground truth, see the oracle finding below).
- [x] `pnix_runtime.original_oracle_report()` (P0): live parity against the
      ORIGINAL `~/pnix/target/release/pnixc-meta` over the corpus -- `93` agree,
      `0` disagree, `216` unsupported (v0 surface gaps + known v0 float-modulo
      divergence from full `interpret.rs`), ready. Auxiliary *live* oracle.
- [x] `pnix_runtime.rust_corpus_report()` (P0): static high-fidelity ground truth
      adapted from the FULL evaluator's `eval_basics.rs` (41) + `builtin_parity.rs`
      (51) + `eval_arith_builtin_overflow.rs` (10) + targeted grammar cases (26)
      + targeted guard/error/overflow cases (48) + targeted source-position cases
      (6) -- `182/182` ready on both interpreter and compiler lanes. Vendored as
      static data: NO `~/pnix` runtime dependency.
- [x] `hy_mirror.stage7_check()` (the eval-kernel mirror, base of the ladder)
      passes on Python 3.11.
- [x] `hy_mirror.stage7_check()` passes on Python 3.14 when `PNIX_HY_PYTHON`
      points at `/tmp/pnix-hy-py314-venv/bin/python`.
- [x] `hy_mirror.closure_probe()` is green on Python 3.11: `stage15-check` and
      `stagen-check` both report `reproduced`, so pnix-hy federates hy-meta's
      stage15/stageN closure (not just the stage7 kernel) as host evidence.

## 2026-06-29 implementation pass: original oracle + grammar fix

Resumed implementation on the stage15/N-closed hy-meta substrate. Two P0
deliverables, both verified green.

- [x] **P0 #1 -- wire the ORIGINAL `~/pnix` oracle** (`original_oracle_report`,
      a sibling of `fixture_report`). It evaluates each corpus case through the
      installed `~/pnix/target/release/pnixc-meta` (the meta-circular `.px`
      evaluator) and compares to pnix-hy's `run_px` (source written to a temp
      `.px` so both resolve relative paths identically). The installed binary is
      the **v0** meta-circular interpreter and supports only a CORE subset (no
      `rec`/`with`/`match`/dynamic-select/most builtins/lambda patterns), so cases
      it cannot parse/eval are classified `unsupported`, not failed; only a real
      value divergence is a `disagree` except known v0 divergences from the full
      evaluator. Result: `93` agree / `0` disagree / `216` unsupported. Binary path
      overridable via `PNIX_ORIGINAL_PNIXC_META`; returns
      `available: False` (not a crash) when absent. NOTE: the todo previously
      assumed `pnixc --emit eval`; that emit mode does not exist in this build
      (`pnixc` only emits ir/ssa/llvm/aot) -- `pnixc-meta <file.px>` is the eval
      entry, and it is fast (~0.01s/case), unlike the 25s stage7 kernel.

- [x] **Real grammar bug found by the oracle and fixed across all lanes:**
      quoted attr keys with dots were being split into a nested path. `parse_path`
      (Python, `pnix_runtime.py:313`) and `path-token-parts` (the Hy mirror parser
      inside `PNIX_PARSER_DEFS`) both did `str(value).split(".")` for string
      tokens, so `{ "x.y" = 1; }` parsed as nested `{x={y=1}}` (attrNames `["x"]`)
      instead of a single attr `"x.y"`. The select/hasattr parser already kept
      quoted segments single, so the two sides disagreed. Ground truth confirmed
      three ways (real Nix `/run/current-system/sw/bin/nix`, original `pnixc-meta`,
      and Nix attrpath semantics):
      `{ "x.y" = 1; } ? "x.y"` -> **true**, `{ "x.y" = 1; }."x.y"` -> **1**,
      `attrNames { "x.y" = 1; }` -> **["x.y"]**, `{ "a.b.c" = 2; } ? a.b.c` ->
      **false**. Fix: a quoted string key is always ONE literal segment; only the
      unquoted `a.b.c` token sequence forms a nested path.
  - [x] Fixed both parser lanes (Python + Hy mirror), so interp / compiler /
        stage7-source / stage7-compiler-source all agree.
  - [x] Corrected the three test expectations that had encoded the bug
        (`hasattr-quoted-dot` False->True, `dynamic-hasattr-path` and
        `dynamic-hasattr-number` True->False) and added 3 positive cases
        (`quoted-dot-key-select`, `quoted-dot-key-names`, `quoted-dot-key-not-nested`).
  - [x] Corrected the matching wrong values in the shared fixture corpus:
        `~/pnix/fixtures/pnix_expr/scenario{28,29,31}-*.expected.json` (these
        pnix-hy-authored fixtures had baked the OLD buggy output and contradicted
        both the original evaluator and real Nix). `fixture_report()` back to 32/32.

### 2026-06-29 pass 2: Rust ground-truth corpus + 2 more real bugs

- [x] **P0 #2 -- import the Rust eval test corpus** (`RUST_EVAL_CORPUS` +
      `rust_corpus_report()`). 41 cases adapted from the FULL evaluator's
      `~/pnix/crates/pnix-eval/tests/eval_basics.rs` (31 value + 10 error). Unlike
      the v0 `pnixc-meta`, the Rust `pnix-eval` tree-walker is the COMPLETE
      reference, so these are static high-fidelity ground truth (no binary). The
      report checks pnix-hy's interpreter AND compiler; `41/41 ready`. Known gap
      (logged, not silently dropped): `builtins.ontologyLift` (domain builtin) ->
      `RUST_EVAL_KNOWN_GAPS`, tracked under P3.
- [x] **String-interp `__toString`/`outPath` coercion** (real semantic gap found
      by the corpus): a set in `${...}` now coerces via `__toString` (called with
      the set as `self`) then `outPath`, recursing for nested chains
      (`outPath = { __toString = ...; }`); `__toString` takes priority. Matches
      ~/pnix + Nix. Fixed in all 4 lanes (interp `coerce_interp`, compiler prelude
      `_coerce`, Hy mirror `coerce-interp`). Added 5 cases to the stage7 corpus.
- [x] **Attrset formal strictness** (real bug found by the corpus): a lambda
      attrset pattern WITHOUT `...` now rejects unexpected attrs
      (`({ x }: x) { x = 1; y = 2; }` -> "unexpected attribute 'y'"), matching
      ~/pnix + Nix; `...` still admits extras. Scoped to lambda formals (interp
      `check_formal_attrs` in `apply_pnix`; compiler `_bindpat`; Hy mirror
      `apply-pnix`) so `match` arm semantics are untouched. Verified directly in
      the stage7 kernel (raises) and that ellipsis still admits extras.

### 2026-06-29 pass 3: builtin_parity + overflow corpus, i64 overflow

pnix-hy is NOT a project that depends on `~/pnix` (user). The Rust corpus is
therefore **vendored as static Python data** -- the `.rs` files are read only at
authoring time; nothing in pnix-hy's self-contained gates (self_test, rust_corpus,
stage7) touches `~/pnix` at runtime. (`original_oracle_report`/`fixture_report`
stay as optional external oracles that skip gracefully when `~/pnix` is absent.)

- [x] Imported `builtin_parity.rs` -> `RUST_BUILTIN_CORPUS` (51 cases): substring/
      stringLength (byte-based, Nix semantics), elem, lessThan/lt/le/gt/ge, add/
      sub/mul/div, compareVersions/splitVersion, getAttr, seq/deepSeq, mod/neg/abs/
      pow/floor/ceil, listToAttrs/removeAttrs/map/attrNames/genList/sort/
      replaceStrings/concatStringsSep/toJSON, trace/warn. All already aligned --
      no new bug (incl. byte-based substring of multibyte UTF-8).
- [x] Imported `eval_arith_builtin_overflow.rs` -> `RUST_OVERFLOW_CORPUS` (10):
      surfaced a real divergence -- pnix is **i64 with checked overflow**, but
      pnix-hy used Python arbitrary-precision int (silent bignum). Fixed: int
      arithmetic that escapes [i64::MIN, i64::MAX] now raises `integer overflow`
      (float stays unchecked -> +inf, matching Nix). Implemented in all 4 lanes
      (interp `check_i64` in `apply_binary`; compiler prelude `_ci` in `_bin`; Hy
      mirror `check-i64` in `apply-binary` + add/sub/mul/div builtins). Verified
      directly in the stage7 kernel (overflow raises; small ints unaffected).
- [x] `rust_corpus_report()` now spans `RUST_EVAL_CORPUS + RUST_BUILTIN_CORPUS +
      RUST_OVERFLOW_CORPUS + targeted grammar + targeted guard/error/overflow +
      targeted source positions` = `182/182`.

## Historical Remaining Work (done before current checkpoint)

The live remaining plan is under `## IMPLEMENTATION PLAN`; this section records
older scope items that have already been consumed by the implementation history.

- [x] Extend the Hy-written AST evaluator to recursive lazy cells:
      recursive `let`, `rec { ... }`, and forward references.
- [x] Extend the Hy-written AST evaluator to string interpolation expression
      assembly, including placeholder-compatible behavior.
- [x] Extend the Hy-written AST evaluator to list indexing and higher-order
      builtins: `elemAt`, `map`, `filter`, `foldl'`, `concatMap`, `genList`,
      `attrValues`, `mapAttrs`, and `sort`.
- [x] Rewrite the pnix parser itself in Hy for the proof corpus; AST JSON input
      remains available only as a compatibility/probe path.
- [x] Keep import/path/file support out until the boundary is specified here.
- [x] Keep store/snapshot/typed-hole support out until the three-module mirror
      surface is stable.

## Parity Hardening (verified)

Verified by re-running both parity lanes on Python 3.11; ast and source lanes
now pass 63/63, and `pnix_mirror.self_test_report()` is fully green.

- [x] `realize-value` rewritten from a `for`+subscript loop to index-based
      recursion. Root cause: the stage7 kernel keeps only the last key when a
      dict is mutated inside a `for` loop, so multi-key attrsets lost keys. The
      recursive form preserves every key. (Diagnosis confirmed it is NOT a
      Hy-vs-Python language gap — plain CPython/Hy both work; stage7's tiny
      kernel is the constraint, so we align to the Python evaluator as the
      reference.)
- [x] `value-to-string` rewritten to recursive `vts-list-parts` /
      `vts-dict-parts` helpers (same kernel reason). `builtins.toString` of
      lists, sets, and nested values now byte-matches the Python evaluator.
- [x] `builtins.toJSON` no longer calls `json.dumps`. A hand-written
      `canonical-json` serializer (`cj-escape-char`, `cj-escape`, `cj-string`,
      `cj-items`, `cj-pairs`) is built only from stage7-supported primitives
      (index recursion + string concat) because the stage7 kernel rejects
      keyword args (`:sort-keys`, `:separators`) and tuples (`#(...)`). Output
      byte-matches `json.dumps(ensure_ascii=False, sort_keys=True,
      separators=(",",":"))` for plain values and identifier keys. This is the
      "implement it rather than fall back" path, not a workaround.
- [x] Runtime corpus extended +22 cases: `attrNames`, `hasAttr` (true/false),
      `getAttr`, `head`, `tail`, `map`, `filter`, `foldl'`, `isInt`, `isBool`,
      `toString` (list/set/nested), whole-value `attrset`/`rec`/`nested`, and
      `toJSON` (identifier-key set / nested / string). Added to both
      `SELF_TEST_CASES` and `HY_RUNTIME_CORE_CASES`.
- [x] `pnix_mirror.self_test_report()` now actually invokes `hy_runtime_batch()`
      and `hy_source_runtime_batch()` (it previously skipped the real parity
      checks and only ran the convergence/probe cases).
- [x] Boundary deferred: exact `canonical-json` / `value-to-string` edge cases
      for non-identifier attr keys and JSON control-char escaping
      (`\uXXXX`, `\b`, `\f`). Closed by extending the stage7 canonical JSON
      escape function (`ord`/`format`) and adding non-identifier/control-char
      cases to the rust, self-test, and all four stage7 mirror lanes.

## Compiler vs Mirror (the performance axis)

Established by reading `../hy-meta` directly (kernel.hy / bootstrap.py).
("stage7" below = the eval KERNEL, the execution substrate; hy-meta itself is
closed through stage15/stageN, see the dependency section above.)

- the hy-meta stage7 kernel is a **compiler, not a tree-walking interpreter**:
  `kernel.compile-source-to-module` builds a Python `ast.Module`, and
  `eval-source` does `compile(tree, ..., "exec")` + `exec`. The self-host fixed
  point is *generated Python source/AST equality*
  (`compiler_python_stage7_mirror`, `compiler_ast_stage7_mirror`). So the Hy
  language path executes as host CPython bytecode — no interpretation tax. This
  is already the "analyze -> emit -> host runs it, fixed point = generated code"
  shape; nothing to retrofit on the Hy side.
- The slow part of `stage7-kernel-run` is rebuilding the stage2->7 tower on
  every call (bootstrap-ritual cost), not execution speed.
- pnix's Hy AST evaluator (`HY_AST_EVALUATOR_SOURCE`) **is** a tree-walker, kept
  deliberately as the semantics spec / transparent mirror for host≡kernel
  cross-checking — not as a fast path. Fast pnix execution today is the Python
  native evaluator.
## pnix Compiler Promotion (done)

Decision (user, this slice is now scoped to include it): promote pnix from
interpreter-mirror to a `pnix -> Python AST` compiler so pnix runs at host
CPython speed. The interpreter (`HY_AST_EVALUATOR_SOURCE`) is NOT removed — it
stays as the semantics spec / transparent mirror and the parity oracle. This
mirrors the Clojure `clj-meta` decision (analyze -> emit -> host runs it, fixed
point = generated code) and matches what hy-meta already does for Hy.

Rationale recap: the Hy language path is already a compiler (host speed, no
interpretation tax). The only tree-walker left in this slice is the pnix
evaluator we wrote; promoting it removes the last interpretation overhead while
keeping the mirror's cross-check value.

Roadmap:

- [x] Phase 0 — Probed by booting the stage7 kernel once and calling
      `eval_source` directly. The kernel is far more capable than the earlier
      ad-hoc tests implied:

      | capability                            | result |
      |---------------------------------------|--------|
      | `import ast` / `ast.parse`            | OK     |
      | `compile(tree, ...)` + `exec`/`eval`  | OK     |
      | keyword args, native (`:value 42`)    | OK     |
      | keyword args, user fn                 | OK     |
      | `#** kwargs` splat                    | OK     |
      | `ast.Module(:body [] :type_ignores [])` | OK   |
      | build `ast.BinOp`/`Constant` + eval   | OK (42)|
      | `#(1 2)` tuple                        | OK     |
      | `(, 1 2)` tuple                       | ERR (NameError hyx_XcommaX) |
      | `{"a" 1}` dict / `str.join`           | OK     |

      Decision: **option (A)** — the `pnix -> Python AST` compiler lives INSIDE
      stage7 as Hy that builds Python `ast` nodes, `compile`s them, and lets host
      CPython `exec` them. Pure self-host, host-speed pnix. Conventions: use
      `#(...)` for tuples (not `(, ...)`); prefer hyphen-free keyword names.
      Corollary: the earlier `:sort-keys`/`:separators` failure was a
      form/keyword-munging issue, not a hard kernel limit (confirmed in Phase 0b).
      The hand-written `canonical-json` is byte-exact and kept, but was not
      strictly forced by the kernel.
- [x] Phase 1 — Inventoried. 16 node tags (int, string, str_interp, bool, null,
      var, list, attrset[recursive], let, lambda, apply, if, select, has_attr,
      unary, binary); 14 binary ops (`+ - * / % == != < <= > >= ++ // && ||`);
      2 unary (`- !`); 28 builtins. Runtime: Thunk (lazy, force-with-cycle-guard),
      Closure (param, body, env), letrec via a pre-created rec_env + update.
      Bindings carry a `path` (multi-segment for `a.b.c = ...`). Interpreter
      eval-ast dispatch at pnix_runtime.py:703-771 (Python) and the Hy mirror at
      ~1994-2025. The compiler's input contract = these 16 tags.
- [x] Phase 2 — Strategy fixed: compile each pnix node to a Python *expression*
      in walrus-IIFE form, so `letrec`/`rec` needs no statements. Laziness is
      preserved by emitting `_T(lambda: <expr>)` thunks; `rec`/`let` forward refs
      resolve through Python late-binding closures over sibling walrus temps.
      pnix functions compile to `_C(lambda arg_thunk: <body>)`; apply is
      `_apply(f, _T(lambda: arg))`. A small prelude (`_T`, `_force`, `_C`,
      `_apply`, `_bin`, builtins, value-to-string/canonical-json, `_realize`) is
      prepended. Plan: validate a Python prototype against the interpreter on all
      63 cases, then port the emitter to Hy inside stage7 (self-host; fixed point
      = generated Python AST via `ast.dump`).
- [x] Phase 3 — Implement `pnix -> Python` emit for the core: literals,
      arithmetic, `let`/`rec`, lambda/apply, attrset/select, `?`, `//`, list,
      string interpolation, and the builtin subset. Interpreter is the oracle.
      - [x] PoC validated the walrus-IIFE strategy on 9 core nodes (arith,
            let-recursive, lambda, rec-attr, rec-forward, bool, merge, has-attr,
            str-plain) — all match the interpreter. Crucially `rec-forward`
            passes, proving Python late-binding closures express pnix letrec.
      - [x] Extended to multi-segment binding paths (`a.b.c = ...`), string
            interpolation (placeholder-preserving via compile-time env), and all
            28 builtins via an initial-env (`builtins` attrset), mirroring
            native_builtins.
      - [x] Full 63-case parity with the interpreter on the Python prototype:
            PASSED 63/63. Compile-time env maps each pnix name to a Python
            identifier; let/lambda use unique walrus temps, rec attrsets route
            sibling refs through a runtime dict, builtins are `_C`-wrapped.
- [x] Phase 4 — Ported the emitter to Hy INSIDE stage7 and locked the self-host
      fixed point. pnix is now an interpreter (mirror/spec) AND a compiler
      (host-speed, self-host) with three lanes all at 63/63 parity.
      - [x] 4a. PoC done. Inside stage7, Hy emits Python source, `compile`s and
            `exec`s it, returning values matching the interpreter ([2,10,3]).
            Constraint nailed down: stage7's top-level `for` AND `while` fail to
            capture/update enclosing-block vars (NameError; `while` hangs) — same
            family as the for+subscript bug. So the compiler uses recursion +
            defn args for every loop (driver and node traversal); NO top-level
            `for`/`while`.
      - [x] 4b. Ported the emitter + prelude to a `HY_AST_COMPILER_SOURCE` block
            alongside `HY_AST_EVALUATOR_SOURCE`; added a `hy_compiler_*` bridge
            mirroring `hy_runtime_source_for_asts`.
            - [x] Hy emitter for all 16 nodes ported to recursion-only Hy;
                  validated 63/63 compiler-lane parity through stage7. The emitted
                  Python runs at host speed (compile+exec inside stage7), NOT
                  tree-walked.
            - [x] Found another stage7 constraint: a `do`/`setv` in a NON-tail
                  expression position (e.g. RHS of an outer `setv`) gets lifted to
                  a function that fails to capture the enclosing local
                  (UnboundLocalError) — same closure-capture family as for/while.
                  Fix: keep `do`/`setv` in tail position; factor non-tail helpers
                  into named defns (did this for str-interp).
            - [x] Integrated into pnix_runtime.py (`HY_AST_COMPILER_SOURCE` +
                  `COMPILER_PRELUDE` + `hy_compiler_source_for_asts`) and
                  pnix_mirror.py (`hy_compiler_batch`). Compiler-lane parity 63/63
                  through the real stage7 subprocess bridge.
      - [x] 4c. Compiler-lane parity wired into `pnix_mirror.self_test_report()`
            as `hy-compiler-parity`. Full report GREEN: runtime-self-test,
            mirror-chain-converges-7, lambda-mirror-converges,
            stage-tower-equivalent, hy-meta-stage7-light-probe, hy-runtime-parity
            63, hy-source-runtime-parity 63, hy-compiler-parity 63. Three lanes
            (interpreter / source-parsed / compiler) all match the interpreter.
      - [x] 4d. Self-host fixed point verified via `hy_compiler_emit_for_asts`
            (emit-only driver returning generated Python source per AST):
            (a) deterministic emit -- same AST -> same source across runs (63
            stable); (b) generated source is valid Python with stable `ast.dump`;
            (c) host-exec parity 63/63 -- the stage7-emitted source executed by
            the HOST outside stage7 matches the interpreter, so the compiled code
            is plain host bytecode, not stage7-bound. E.g. `rec-forward` emits a
            late-bound `_d1['y']` reference.

## stage7 Kernel Capability Map (probed systematically)

Booted the kernel once and `eval_source`-d each feature. The kernel supports far
more of Hy/Python than first assumed.

Supported: `lfor`/`dfor`/`gfor` comprehensions; `for`/`while` INSIDE a defn;
`try`/`except`; `raise`; `with`; `defclass`; `yield`/generators; f-strings;
`.format`; `%`-format; set literals `#{}`; `#(...)` tuples; `import` (re,
functools, itertools, collections); `#*` args / `#**` kwargs; list unpacking;
nested-fn closures; global list/dict mutation; `ord`/`chr`; `cut` (slicing);
keyword args; `ast`/`compile`/`exec`; `repr`; `sorted`.

NOT supported (only two real gaps):
- top-level `for`/`while` in a `(do ...)` block cannot capture that block's
  `setv` vars (NameError; `while` hangs). They work fine INSIDE a defn. (Earlier
  notes overgeneralized this; the real boundary is the top-level do-block.)
- `(, ...)` comma-tuple is unsupported; use `#(...)`.

Correction to an earlier boundary note: `ord`/`chr`/`re` ARE available, so JSON
control-char escaping (`\uXXXX`, `\b`, `\f`) was actually feasible; the
`canonical-json` identifier/plain-value limit was a simplification, not a hard
kernel limit.

## Compiler source-lane: pnix parsed inside stage7 (Python boundary reduced)

The compiler now has a **source-lane**: it reuses the evaluator's Hy parser
(`PNIX_PARSER_DEFS`, sliced out of `HY_AST_EVALUATOR_SOURCE` so there is ONE
parser definition, not two copies) and runs tokenize + parse + compile + exec
ALL inside stage7. On this path Python supplies only the source strings and
checks the result -- it never parses or evaluates.

- [x] `HY_AST_COMPILER_SOURCE` gained an `__PARSER__` injection slot and a
      source/ast-select driver (`parse-source-list` when sources are present,
      else the Python-provided AST JSON).
- [x] `hy_compiler_source_for_sources` injects the parser; the ast-lane
      (`hy_compiler_source_for_asts`) injects `__PARSER__=""`.
- [x] Verified through stage7: ast-lane 63/63 (no regression) AND source-lane
      63/63. So `pnix source -> stage7 parse -> stage7 compile -> host exec`
      equals the interpreter, end to end.
- [x] `pnix_mirror.hy_compiler_source_batch()` + `hy-compiler-source-parity`
      wired into `self_test_report()`.

Irreducible Python boundary (by design, NOT removable):
- `compile()` / `exec()` run on host CPython. hy-meta itself compiles Hy to
  Python AST that CPython executes, so host CPython is the floor for every lane
  (interpreter, compiler, hy-meta). The pnix Python evaluator stays on as the
  reference oracle (Compatibility rule above). "Fully self-contained, Python-free
  pnix" is therefore out of scope by construction, not a missing feature.

## CPython boundary — meta-circular introspection (DESIGN ONLY, not built)

Idea: the last host step — CPython's `compile()` (Python AST -> bytecode) and
`exec()` (bytecode -> value) — cannot be *removed* (something must run
natively), but it CAN be made introspectable + mirrored, repeating the pnix
pattern (mirror + cross-check) one level down. Then CPython stops being a black
box and becomes "a host component with a verified Hy mirror".

Probed: stage7 can fully introspect CPython internals (all returned OK):
- `import dis`; `dis.get_instructions(code)` -> instruction stream
- `compile(src, ..., "eval"/"exec")` -> code object (`type.__name__ == "code"`)
- code-object fields: `co_code` (raw bytes), `co_consts`, `co_names`,
  `co_varnames`, `co_stacksize`
- instruction fields: `.opname`, `.argval`; `dis.opname` / `dis.opmap` tables
- `import marshal` -> `marshal.dumps(code)` (serialize a code object)
- `types.CodeType`, `sys.settrace` available; `eval(codeobj)` works
- `(lfor i (dis.get_instructions ...) i.opname)` works; e.g. `40+2`
  disassembles to `['RESUME','LOAD_CONST','RETURN_VALUE']`
- => all materials for a mirror VM exist INSIDE stage7.

Layered plan (later):
- A. Introspect: expose the generated code's bytecode via `dis` (read-only).
- B. Mirror-VM: a tiny bytecode interpreter in stage7 Hy for ONLY the opcode
     subset our emitter produces (name load, call, make-function, build
     dict/list, subscript, compare, store/walrus, conditional); cross-check its
     value against CPython `exec`. This is the key step and is small because our
     emitted Python is a small subset.
- C. Mirror-compile (harder): emit a code object ourselves, replacing
     `compile()` (CPython code-object layout + marshal format).
- D. Bypass (pure mirror, slow): evaluate the Python AST with our own VM,
     skipping compile/exec entirely.

Performance constraint (IMPORTANT, per user): introspection must NOT cost speed.
A slow mirror VM is NOT an acceptable end state — do not "settle" for it because
it is introspectable. Introspection and execution are SEPARABLE, so keep
execution on host CPython and get introspection for (near-)free. Find the fast
paths, not the slow-but-pure one.

Fast, introspectable paths (preferred over a slow mirror VM):
- Static introspection (cost 0): `dis` / code-object inspection runs before or
  after execution, never on the hot path. Always available, always fast.
- Mirror-compile + host execute (path C promoted to the FAST main route): we emit
  the bytecode (own the `compile()` step), but a real host-CPython code object
  runs it at host speed. We own/inspect the code; CPython still executes it =
  introspectable AND fast. `marshal` (probed OK) can build/serialize the code
  object; `dis` reverse-checks it.
- Trace-based (`sys.settrace`, probed OK): observe a host run; execution stays
  native, observation is a side channel.
- AOT / caching: compile once, `marshal.dumps` the code object, reuse across the
  stage tower — no recompile cost.

The slow mirror VM (path B) is therefore demoted to a VERIFICATION / spec lane
only: run once for parity, cache, never on the production path (exactly like the
pnix interpreter mirror). Resolution: own the compile step for introspection,
keep execution native for speed, treat the slow VM as spec — not as the runtime.

Recommended order: A (static) -> C (own the bytecode, host runs it) -> parity;
B/D are spec lanes only. Even hy-meta's stage7 runs on CPython, so this boundary
is the shared floor — goal: make the floor *inspectable WITHOUT giving up host
speed*.

Open question to solve later: can path C reach full host speed while we own the
emit? Candidate answers to explore — (1) emit a code object via `types.CodeType`
/ `marshal` and let CPython eval it; (2) build bytecode with a known assembler
shape and verify by round-tripping through `dis`; (3) keep CPython compile but
make it transparent via cached `dis` + a parity-checked spec VM. Pick whichever
keeps the hot path native.

## IMPLEMENTATION PLAN: meta-circular pnix that compiles/runs `.px` (Python/Hy)

NORTH STAR (user): a pnix runtime that **compiles & runs `.px` files**,
Python/Hy based, meta-circular. Plan only here (no implementation). Synthesized
from a deep-research pass over `~/pnix` (original Rust + `.px` implementation).

### How the original ~/pnix runs a `.px` (ground truth)

- Direct: `pnix-eval` (Rust) **tree-walks** the AST, lazy + `deep_force`. CLI:
  `pnixc --emit eval <file>.px` -> JSON. (crates/pnix-eval/src/interpret.rs)
- Meta-circular: `pnixc-meta` (Rust) is the Stage-1 host -- it uses `pnix-eval`
  to boot `pnixc-pnix/eval/evaluator.px` (a 482-line `.px`-WRITTEN evaluator =
  Stage 2), which parses (via `pnixc-pnix/exec/runtime.px::parse_expr`, also
  `.px`) and evaluates the user's `.px`. So **`.px` evaluates `.px`**. CLI:
  `pnixc-meta <file>.px` -> JSON.
- Full pipeline (pnix-core): parse -> AST -> UnifiedExpr -> FxCore IR -> SSA ->
  build-IR -> codegen; AOT via pnix-seto-aot (LLVM). Semantic law lives in
  `.px`; Rust is the bootstrap ferry. Stage0/1/2 + repro-manifest (bit-for-bit).

### Correspondence  original <-> pnix-hy  (we already have the substrate)

| original ~/pnix | pnix-hy |
|---|---|
| `pnixc-pnix/eval/evaluator.px` (.px evaluator = Stage 2) | `HY_AST_EVALUATOR_SOURCE` (Hy interp in stage7) |
| `exec/runtime.px::parse_expr` (.px parser) | Hy parser in stage7 (`PNIX_PARSER_DEFS`) |
| `pnixc-meta` (Rust Stage-1 host boots .px evaluator) | hy-meta stage7 (Hy self-host host) |
| `pnix-eval` tree-walker (Rust fast path) | Python interpreter (oracle) + host compiler (entry A) |
| `pnixc --emit eval <f>.px` -> JSON | `run_px(path)` (entry A) |
| `pnix-mirror-runtime` (.px project via pnixc-meta) | `pnix_mirror` (introspection, entry B) |

KEY: original's "Stage 2 = .px evaluates .px" is EXACTLY our "HY_AST_EVALUATOR/
COMPILER runs inside stage7". The meta-circular core EXISTS; remaining work is
breadth (full grammar + builtins) + `.px` file/import entry + matching original
semantics. Difference: original makes `.px` the semantic owner (Rust = ferry);
we make Hy the owner (hy-meta stage7), with a host-Python compiler as fast path.

### Stop checkpoint (2026-06-28)

Implementation code checkpoint before this todo-only stop commit:
`5d7250a Add pnix fixture parity report`.

Implemented in this pass:
- `fromTOML`, `genericClosure`.
- XML/HTML parse+emit subset.
- Generic schema subset: `schemaValidate`, `schemaNormalize`, `schemaExplain`.
- Relative `.px` imports for `run_px` / compiler path, plus import cache/cycle
  coverage in self-test.
- Bitwise builtins: `bitAnd`, `bitOr`, `bitXor`.
- `addErrorContext` context-string guard and `unsafeGetAttrPos` fixture-compatible
  behavior for present/missing attrs.
- Reusable external fixture oracle: `pnix_runtime.fixture_report()`.

Last known verification:
- `pnix_runtime.self_test_report()` ready, `504/504`.
- Stage7 lanes ready, `249/249` each:
  `hy_runtime_batch`, `hy_source_runtime_batch`, `hy_compiler_batch`,
  `hy_compiler_source_batch`.
- Original expected fixtures via `fixture_report()`: `32/32` against
  `~/pnix/fixtures/pnix_expr/scenario*.expected.json`.
- Working tree was clean after the code commits; this document records the
  remaining plan before stopping.

### Roadmap (remaining, later, in order)

- P0. Exact semantic alignment against original `~/pnix`.
  - [x] Current corpus alignment: polymorphic `+`; `//` null identity; `->`
        implication; original-style `toString`; current stage7 evaluator/compiler
        sync; fixture parity `32/32`.
  - [x] Wire a first-class ORIGINAL-pnix oracle (`original_oracle_report`) around
        `pnixc-meta <file>.px` (the live v0 evaluator; `pnixc --emit eval` does
        NOT exist in this build). 86 agree / 0 disagree / 167 unsupported. Keeps
        `fixture_report()` as the external expected-json oracle. (2026-06-29)
  - [~] Import Rust eval tests into the corpus instead of relying on ad hoc reads.
        Done (2026-06-29): `eval_basics.rs` (41) + `builtin_parity.rs` (51) +
        `eval_arith_builtin_overflow.rs` (10) -> `RUST_EVAL_CORPUS` /
        `RUST_BUILTIN_CORPUS` / `RUST_OVERFLOW_CORPUS` + `rust_corpus_report()`,
        later extended with targeted grammar, guard/error/overflow, and
        source-position cases to 178/178 on interp+compiler. Surfaced + fixed three real bugs (string-interp
        `__toString`/`outPath`; attrset-formal strictness; i64 checked overflow);
        documents one gap (`builtins.ontologyLift`). Still to import:
        `eval_list_bounds.rs`, `eval_string_ops.rs`, `eval_seq_any_all.rs`,
        `eval_filter_elem_listtoattrs.rs`, `eval_addcontext_pos_bitops_guards.rs`,
        `eval_regex_ops.rs`, `eval_json_builtins.rs`, `eval_path_ops.rs`,
        `eval_source_position.rs`, `eval_px_files.rs`.
  - [x] Tighten exact guard/error/overflow/laziness semantics. Done (2026-06-29):
        checked integer overflow on +/-/*// and add/sub/mul/div (i64 bound, all 4
        lanes) via `eval_arith_builtin_overflow.rs`; followed by modulo-by-zero,
        pow overflow-to-float boundary, genList/take/drop negative guards,
        any/all/filter predicate type errors with index, sort comparator type
        errors, fromJSON integer bounds, and JSON special-float policy.
  - [x] Replace placeholder source positions with real tracked positions:
        `__curPos`, `unsafeGetAttrPos`, literal attr slot line/column, file labels,
        generated attrs with no position -> `null`.
  - [x] Implement real string-context propagation instead of stubs:
        `getContext`, `hasContext`, `appendContext`,
        `unsafeDiscardStringContext`, `addDrvOutputDependencies`,
        `unsafeDiscardOutputDependency`, `unsafeAddOutputDependency`,
        `unsafeAddOutputName`, derivation output contexts.

- P1. `.px` file/module/import breadth.
  - [x] `run_px(path)`, `run_px_source`, relative `import ./rel.px` /
        `../rel.px`, cache, cycle guard, nested relative imports.
  - [~] Make `~/pnix/stdlib/*.px` loadable as a normal library corpus.
        Blocked here because `~/pnix/stdlib` is absent.
  - [~] Mirror original import registry / preparsed import behavior used by
        `pnixc-meta` for `runtime.px` and `evaluator.px`.
        Deferred until upstream exposes that registry surface.
  - [x] Add `builtins.import` / `scopedImport` compatibility if original/nixpkgs
        code expects those surfaces.
  - [x] Add a repo-local fixture corpus or documented external-fixture gate so CI
        does not silently skip `~/pnix/fixtures/pnix_expr`.
  - [~] Add CLI/bin wrappers only if needed later; current module APIs are
        `run_px`, `run_px_source`, `fixture_report`, `introspect_px`.

- P2. Grammar breadth and exact grammar compatibility.
  - [x] Current scenario grammar covered: float, path, select default, index,
        with, assert, dynamic select/default/hasAttr, Construct/Match, Import,
        lambda attr/list patterns, `@`, defaults, ellipsis.
  - [x] Add `inherit` and `inherit (expr)` in attrsets/lets.
  - [x] Add exact duplicate-attribute diagnostics and recursive attr conflict rules.
  - [x] Add full string syntax coverage: indented strings, interpolation edge
        cases, path/string context interaction, escaped antiquotation parity.
  - [x] Add full path grammar/normalization parity: absolute paths, `~`, path
        concatenation, path interpolation/context behavior.
  - [x] Audit lambda/formal grammar against original: nested patterns, defaults
        depending on earlier bindings, extra args with/without ellipsis, error
        shape for missing/unexpected args.
  - [~] Audit ADT `Construct/Match` semantics beyond scenario23-26: nested
        constructors, repeated bindings, non-exhaustive errors, constructor arity.

- P3. Builtins breadth and exact behavior.
  - [x] Broad Nix/pnix core subset is implemented and stage7-checked:
        list/attr/string/math/control/filesystem/json/toml/regex/derivation stubs,
        genericClosure, bitops, schema, XML/HTML.
  - [⛔] NON-GOAL — Domain markup/schema families (`xmlSchema*`, `mathmlSchema*`,
        `openmathSchema*`, `svg*`, `x3d*`, `cellml*`, `sbml*`, `sedml*`,
        `collada*`, `programSchema*`, … XML-family normalize/validate/explain in
        `interpret.rs`) are added extension libraries, NOT the pnix language. See
        the ⛔ NON-GOALS section at the top. DO NOT implement.
  - [⛔] NON-GOAL — SVG/X3D/MathML/OpenMath emit/parse/render packet semantics
        (`markup.rs`, `svg.rs`, `x3d.rs`, `math_markup.rs`). DO NOT implement.
  - [x] Exact derivation behavior: required attrs, output paths, contexts,
        attr preservation, `derivationStrict`, JSON representation, no
        `builtins.isDerivation` unless original exposes it.
  - [x] Exact filesystem/path builtins: `pathExists`, `readFile`, `readDir`,
        `readFileType`, `hashFile`, `toPath`, `storePath`, `baseNameOf`, `dirOf`,
        plus error messages and path-context strings.
  - [x] Exact regex semantics for `match`/`split`: invalid regex errors, empty
        split pattern, optional captures as `null`, Unicode behavior.
  - [x] Exact data parser semantics: `fromJSON` bounds/types/errors, TOML edge
        cases, `toJSON` function/path/context errors.
  - [~] Keep adding Rust test slices as builtins are tightened; no new builtin is
        done until host, compiler, stage7 source, stage7 compiler-source all agree.

- P4. Meta-circular finish.
  - [x] Core exists: Hy evaluator/compiler runs inside stage7, analogous to
        original `.px evaluates .px`.
  - [x] Re-sync stage7 Hy compiler emitter with host strictness + partial-eval.
        `compiler_emit_shape_report` now checks emitted Python shape directly.
  - [x] Decide whether the semantic owner should remain Hy-in-stage7 or move
        toward a `.px` evaluator/compiler owner like original `pnixc-meta`.
  - [~] If moving toward `.px` ownership, port/import `evaluator.px` /
        `runtime.px` concepts carefully as reference, not blind copy.
  - [x] Keep host compiler Entry A as production fast path; interpreter and
        stage7 are spec/parity lanes.

- P5. Performance work.
  - [x] Build a benchmark harness over the same inputs for:
        original `pnixc --emit eval`, Python interpreter, host compiler Entry A
        compile-once/exec-many, and stage7 lanes separately.
  - [x] Report parse/emit/compile separately from exec-many; exclude process
        startup and Rust build time.
  - [x] Track generated Python size and bytecode op count through `hy_mirror`
        CPython introspection.
  - [~] Try only measurable wins: type/shape-specialized `_bin`/select/apply
        emission, call-site specialization, possibly a tiny self-tracing JIT.
        Keep changes only if parity stays green and speed improves.

### Verification infrastructure (oracles)

1. Nix oracle (`scratchpad/nixparity.py`): ours vs `nix --eval --strict --json`.
   42/42 on the base set. Standing check for the Nix-compatible subset.
2. ORIGINAL-pnix oracle (BUILT 2026-06-29, highest fidelity):
   `pnix_runtime.original_oracle_report()` wraps `pnixc-meta <file.px>` as a
   parity batch -- write source to temp `.px`, get original JSON, compare to
   pnix-hy `run_px`. 86 agree / 0 disagree / 167 unsupported. CAVEAT: the
   installed binary is the v0 meta-circular interpreter (core subset only); it
   does NOT yet cover the pnix EXTENSIONS / advanced grammar (rec/with/match/
   dynamic-select/most builtins) -- those show as `unsupported`, so the Nix oracle
   (#1) and fixtures (#3) remain necessary for the broader surface. `pnixc --emit
   eval` does not exist in this build.
3. Fixtures corpus: `~/pnix/fixtures/pnix_expr/scenario*.px` (45) +
   `*.expected.json` (32). Run each through interp + compiler (A) + un-folded
   introspect (B); assert == expected.
4. Rust test suites as case sources: `eval_basics.rs` (41) + `builtin_parity.rs`
   (51) + `eval_arith_builtin_overflow.rs` (10) BUILT 2026-06-29 ->
   `pnix_runtime.RUST_{EVAL,BUILTIN,OVERFLOW}_CORPUS` + `rust_corpus_report()`
   plus targeted grammar, guard/error/overflow, and source-position cases (178/178, interp +
   compiler), vendored statically (no `~/pnix` runtime dep).
   Still to adapt: `eval_list_bounds.rs`, `eval_string_ops.rs`, etc. -- same
   {name, source, expect} shape.

### Performance-comparison methodology

Compare on the SAME inputs (fixtures + a few hot loops):
- Engines: (a) original `pnixc --emit eval` (Rust tree-walker), (b) our Python
  interpreter, (c) our host compiler entry A (compile-once / exec-many), (d)
  stage7 lane (bootstrap-heavy -- report separately).
- Measure: wall-clock per input; for the compiler split compile-once vs
  exec-many (the production metric); warm up; report median (us/iter style).
- Fairness: exclude process startup & Rust build; compiler metric = exec-many
  (parse/emit/compile amortized). Expect Rust >> our compiler >> our interp;
  our compiler + partial-eval should close the gap on closed/pure programs
  (fold -> constant, runtime 0).
- Accuracy gate FIRST: every engine must match `.expected.json` before timing.
- Also track (we have CPython introspection in hy_mirror): generated-Python size
  + bytecode op count per program.

### Concrete imports from ~/pnix

- `fixtures/pnix_expr/scenario01..15.{px,expected.json}` -> corpus + perf set.
- `crates/pnix-eval/tests/{eval_basics,builtin_parity,eval_list_bounds}.rs` ->
  adapted corpus cases (with expected values).
- READ (reference, don't copy): `pnixc-pnix/eval/evaluator.px` (482) +
  `exec/runtime.px` (2141) -- the coverage target for our Hy evaluator/parser.
- Optionally wire `~/pnix/target/release/pnixc --emit eval` as the parity oracle.

---

## pnix == original ~/pnix alignment (THE real ground truth)

User: pnix follows the original `~/pnix` implementation (Rust crates + `.px`
stdlib + lang-test), which EXTENDS Nix. `~/pnix` is on this machine. Use it as
the oracle; Nix is a secondary check (real Nix is here too:
`/run/current-system/sw/bin/nix`, `--eval --strict --json`). Nix-parity harness
in scratchpad/nixparity.py (current base set `42/42`; earlier baseline was
`35/42`). Surveyed
`~/pnix/crates/pnix-eval/src/interpret.rs` and `crates/pnix-core/.../syntax.rs`.

Confirmed original semantics (interpret.rs):
- `+` is POLYMORPHIC (5933-6060): string concat, LIST concat, ATTRSET merge,
  path concat, else numeric. (Verified in code, not guessed.)
- `++` list-only; `//` attrset merge (right wins) + null (`null // x`->x,
  `x // null`->x); `==`/`!=` deep equality (depth 64); `< <= > >=` order over
  number / string (text-only lexicographic) / list (lexicographic) / path
  (normalized); `&&`/`||`/`->` boolean, `->` = implication (`!a || b`).
- `/` integer division truncates (checked_div).
- `toString` (5498-5567): int/float->str, true->"1", false->"", null->"",
  list->space-join (recursive), attrset.__toString/outPath->call, plain attrset
  & function -> ERROR.
- string interpolation coercion is STRICTER: int/float/bool/null/list/function
  -> ERROR ("use builtins.toString"); only string/path/__toString/outPath.

Alignment tasks status at stop:
- [x] `+` string concat + list concat + attrset merge (polymorphic, per
      interpret.rs); `<`/`<=`/`>`/`>=` order over number/string/list
      (`nix_less_than`/`nix_compare`) in apply_binary.
- [x] `value_to_string` -> original toString (true->"1", false/null->"",
      list->space-join, set/function->error); compiler prelude `_vts` synced;
      corpus toString cases updated (list->"1 2 3"; set/nested replaced by
      toString-true / toString-null).
- [x] VERIFIED at the earlier alignment point: Nix-parity harness 42/42
      (was 35/42); run_px compiler==interpreter 63/63; interpreter self-test
      126/126.
- [x] `//` null handling (`null // x`->x, `x // null`->x) and `->`
      implication operator.
- [x] Stage7 Hy interpreter/compiler semantics synced for the current corpus:
      four stage7 lanes ready `249/249`.
- [x] External expected-json fixture oracle wired:
      `pnix_runtime.fixture_report()` ready `32/32`.
- [x] Live ORIGINAL-pnix oracle wired (`original_oracle_report`, 2026-06-29):
      86 agree / 0 disagree vs `pnixc-meta`.
- [x] Grammar parity fix (2026-06-29): quoted dotted attr keys are single literal
      names, not nested paths (matches real Nix + original); fixed in both parser
      lanes, with corpus + fixture expectations corrected.
- [x] `eval_basics.rs` imported (2026-06-29): `RUST_EVAL_CORPUS` /
      `rust_corpus_report()` 41/41. Surfaced + fixed string-interp
      `__toString`/`outPath` coercion and attrset-formal strictness (all 4 lanes).
- [x] Import the remaining language-scope Rust eval slices available in this checkout (see P0). The current `~/pnix/crates/pnix-eval` has `src/interpret.rs` but no `tests/*.rs` directory; domain/product tests remain non-goals.

Current residual scope gaps:
- [x] Exact grammar parity beyond the current fixtures: `inherit`,
      duplicate-attr diagnostics, full string/path grammar (quoted-dot keys done),
      lambda formal edge cases (no-ellipsis unexpected-attr now done), and ADT
      `Construct/Match` edge cases.
- [x] Exact builtin parity beyond the current implemented subset: source
      positions, string contexts, derivation/path/filesystem/regex/data-parser
      edge cases, and the XML-family domain builtins listed in P3.

Tests / .px to import as ground truth:
- `~/pnix/crates/pnix-eval/tests/eval_basics.rs` (core semantics: arithmetic,
  let, lambda+currying, attrset destructure, with, string interp, list ops,
  attrset merge) -- adapt #[test] eval_expr cases into our corpus.
- `eval_arith_builtin_overflow.rs`, `eval_*toString*.rs`, other
  `crates/pnix-eval/tests/*.rs`.
- `~/pnix/stdlib/{list,string,attrset,default}.px` show the core surface.
- 4832 `.px` files exist; `~/pnix/pnixc-pnix/pnixc.px` is a pnix-written compiler.

Priority from here: follow P0-P5 in `## IMPLEMENTATION PLAN`. Stop here until
the user resumes implementation later.

## CPython introspection API — implemented in hy_mirror (host-direct + mirror)

Per user: implement EVERY CPython (C-level) introspection facility in
`hy_mirror`, in BOTH host-direct (fast, no stage7) and stage7-mirror forms, and
cross-check them. Host-direct is the fast path — introspection is a side channel
with zero hot-path cost (compile/eval stay native). The mirror re-runs the same
introspection INSIDE the kernel as the spec / verification lane.

Surface to cover (CPython 3.11 / 3.14):

- [x] **Code object**: `compile()`; `co_name`/`co_qualname`/`co_filename`/
      `co_firstlineno`; `co_argcount`/`co_posonlyargcount`/`co_kwonlyargcount`/
      `co_nlocals`; `co_stacksize`/`co_flags`; `co_code`/`co_consts`/`co_names`/
      `co_varnames`/`co_freevars`/`co_cellvars`; `co_exceptiontable`;
      `co_lines()`; `code.replace()`.
- [x] **Bytecode (dis)**: `get_instructions` (`Instruction.opname`/`opcode`/
      `arg`/`argval`/`argrepr`/`offset`/`starts_line`/`is_jump_target`);
      `opname`/`opmap`/`cmp_op`/`hasconst`/`hasname`/`hasjrel`/`hasjabs`/
      `haslocal`/`hascompare`/`hasfree`; `HAVE_ARGUMENT`/`EXTENDED_ARG`;
      `stack_effect`; `findlinestarts`/`findlabels`.
- [x] **marshal**: `dumps`/`loads`/`version`; round-trip a code object.
- [x] **Code building**: `types.CodeType`; `code.replace()`; `types.FunctionType`.
- [x] **Function**: `__code__`/`__globals__`/`__closure__`(cell_contents)/
      `__defaults__`/`__kwdefaults__`/`__annotations__`/`__name__`/`__qualname__`;
      `inspect.signature`/`getfullargspec`/`getclosurevars`/`unwrap`.
- [x] **Object/type**: `type`/`__class__`/`__dict__`/`__mro__`/`__bases__`/
      `__subclasses__`/`__slots__`; `dir`/`vars`/`id`/`hash`;
      `inspect.getmembers`/`getmro`/`is*`.
- [x] **Frame/exec**: `sys._getframe` (`f_code`/`f_locals`/`f_globals`/
      `f_lineno`/`f_lasti`/`f_back`); `inspect.currentframe`/`stack`/
      `getframeinfo`; `sys.settrace`/`gettrace`; `traceback.extract_stack`/
      `walk_stack`.
- [x] **Memory/GC**: `sys.getsizeof`/`getrefcount`/`intern`; `gc.get_objects`/
      `get_referrers`/`get_referents`/`get_stats`/`get_count`/`get_threshold`/
      `is_tracked`.
- [x] **Source/AST/symbols**: `ast.parse`/`dump`/`walk`/`literal_eval`;
      `symtable.symtable` (`Symbol.is_global`/`is_local`/`is_free`/
      `is_parameter`); `tokenize`/`token`.
- [x] **Import system**: `sys.modules`; `importlib`; `inspect.getmodule`/
      `getfile`.

Implementation status:
- [x] host-direct forms in `hy_mirror.py`: `compile_source`, `code_object_info`,
      `disassemble`, `opcode_tables`, `stack_effect`, `line_starts`, `jump_labels`,
      `marshal_code`, `rebuild_code`, `function_info`, `object_info`, `ast_info`,
      `symtable_info`, `tokenize_info`, `frame_info`, `trace_run`, `gc_info`,
      `gc_referrers`, `sys_info`, `traceback_info`, `module_info`,
      `full_introspection`. All verified working on host (incl. the compiler's
      own emitted Python: opnames RESUME/PUSH_NULL/LOAD_NAME/MAKE_FUNCTION/CALL/
      RETURN_VALUE, marshal round-trip, rebuild via code.replace eval->42).
- [x] stage7-mirror form (`mirror_full_introspection`) runs the same code-object
      + bytecode + marshal introspection INSIDE the kernel; `introspection_parity`
      cross-checks host vs mirror.
- [x] verified: host-direct and stage7-mirror AGREE bit-for-bit (co_code_hex,
      co_names, co_varnames, co_stacksize, co_flags, opnames, marshal_len) on
      `40 + 2`, `x = 1 + 2`, and the compiler's own emitted Python.
- Perf note honored: host-direct is the fast path (side channel, zero hot-path
      cost); the stage7 mirror is the spec/verification lane only.

## Entry points: `.px` compiler (A = run, B = introspect)

User decision: pnix is a COMPILER, not an interpreter (the interpreter stays as
the mirror, exactly as established). A `.px` file is pnix source. Two entry
points over the hy(py)-mirror -> pnix-runtime -> pnix-mirror chain; A is the real
execution path, B is introspection.

- [x] Entry point A (`pnix_runtime`, "the real one"): `compile_px_source`,
      `run_px_source`, `run_px(path)`. Host-direct compiler -- pnix source ->
      Python source -> `compile` -> `exec` on host CPython, NO stage7 bootstrap.
      The emit (`_px_emit`) is the SAME algorithm as the stage7 Hy emitter
      (`HY_AST_COMPILER_SOURCE`): this host form is the fast production path, the
      Hy form is the self-host mirror, bound by `hy_compiler_batch` parity.
      Verified: 63/63 parity vs the interpreter; a `.px` file runs (-> 42);
      compile-once-then-exec is ~1.9x the interpreter on a sample.
- [x] Entry point B (`pnix_mirror`, "introspection from a pnix statement"):
      `introspect_px(source)`, `introspect_px_file(path)`, `introspect_px_parity`.
      Returns the pnix AST, the generated Python, compiled-vs-interpreter value
      agreement, and full host-direct CPython introspection (code object /
      bytecode / marshal) of the emitted code. stage7 only for the parity lane.
      Verified: values agree (42==42), `.px` file introspects, bytecode exposed
      (`BUILD_MAP`/`STORE_NAME`/`MAKE_FUNCTION`/`CALL`).

Boundary now specified (was deferred under "Remaining Work"): `.px` file reading
is in scope as the compiler entry. CLI/bin wrappers stay out -- these are module
functions (`run_px`, `introspect_px`), not scripts.

Performance follow-up (this is the "find a fast way, don't settle for slow"
item): the remaining overhead is the lazy thunk wrapping (`_T(lambda: ...)`) the
emitter inserts everywhere to preserve pnix laziness; it caps the sample at
~1.9x. Next: a strictness analysis to drop thunks at positions that are always
forced (arith/comparison operands, `if` conditions, `select`/`?` bases, builtin
strict args) and emit the value directly, so the hot path approaches native
Python. Keep thunks only where laziness is observable (rec/let cells, list/
attrset elements, function args). The interpreter mirror stays as the oracle.

## Performance: strictness emit + PyPy execution (the "fast implementation")

User: find a fast implementation; look into PyPy-like engines. Key insight: the
compiler emits STANDARD Python, so the execution engine is a free choice. Two
orthogonal wins, both keeping the interpreter as the mirror:

- [x] Strictness emit (helps even on CPython): `_px_emit` (strict value, no
      thunk) split from `_px_t` (lazy thunk). Thunks survive ONLY at genuinely
      lazy positions -- function args and list/attrset/let cells. Verified 63/63
      parity vs the interpreter. `1 + 2 + 3` now emits
      `_bin('+',_bin('+',1,2),3)` (ZERO thunks); interpreter-relative speedup
      rose ~x1.9 -> x2.21. (Note: this diverges the host emitter from the stage7
      Hy emitter, which stays lazy; they still agree by VALUE -- hy_compiler_batch
      checks the Hy lane, run_px_source checks the host lane. Re-sync the Hy
      emitter to strict later if we want the self-host mirror identical again.)
- [~] Self-implement PyPy-STYLE techniques in OUR meta-circular compiler (NOT
      external PyPy; output stays standard Python, CPython/Hy compatible, zero
      external deps). Measured each; KEEP ONLY THE FAST ONES.
      * [x] Strictness emit (= static escape analysis): thunks removed at strict
        positions (arith / if-cond / unary / select base / function position);
        thunks survive ONLY at lazy positions (function args, list/attrset/let
        cells). 63/63 parity, ~x1.9 -> x2.21.
      * [x] Partial evaluation (pnix is PURE -> PyPy's runtime specialization
        pulled to COMPILE TIME; no runtime JIT): closed scalar terms fold to a
        constant. `1+2+3`->`6`, `rec{x=1;y=x+41;}.y`->`42`, `foldl' ...`->`6`
        (runtime 0). Native/closure markers and complex (list/attrset/closure)
        results fall through to real code. 63/63 parity. Guard: skip realized
        `#<pnix-hy-*>` markers so native functions are not mis-folded.
      * [x] Fold ON for entry A (run, fast), OFF for entry B (introspect) via
        `_PX_FOLD`, so the FULL emitted code stays introspectable -- every grammar
        node keeps meta-circular coverage. `introspect_px` returns both
        `generated_python` (full) and `folded_python` (what A runs) +
        `partial_evaluated`. Verified: A run 63/63, B introspect 63/63 with
        bytecode for all 15 grammar tags the corpus exercises.
      * REJECTED (measured slower / wrong shape): `functools.cache` thunks
        (x94.5 vs custom `_T` x19.9 vs native x1); external PyPy (a dependency --
        we implement the ideas ourselves, the installed brew pypy3 is unused).
        list-cell thunks beat `_T` (x13.4) but strictness + partial-eval dominate,
        so not adopted.
      * [~] Future perf idea (keep only if fast): specialization -- emit type/shape-
        specialized code where the shape is known, instead of generic `_bin`/
        `_apply`; (ambitious) self-tracing mini-JIT. Stage7 Hy emitter strict+fold
        re-sync is done; keep future work measured, not as an active unchecked item.
- Other engines noted for reference only (NOT used): Cython/mypyc (Python->C),
      Numba, CPython 3.13+ JIT. We implement the ideas ourselves; we do not
      depend on any of them. The installed brew pypy3 is unused.

Direction for "Python/Hy-speed pnix": fast path = strictness + partial-evaluation
+ specialization, emitted ONCE as standard Python the compiler owns end to end
(parse -> emit -> introspect), run on plain CPython. The interpreter stays the
mirror; introspection stays the slow spec lane (fine per the perf rule). The
whole point is that the meta-circular compiler does what PyPy does, itself.
