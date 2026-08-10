# 감사 원장 — 2026-07-02 deep-research (meta-circular 체크리스트 대조 + 버그 헌트)

> 다중 에이전트 감사(84 agents) 결과. 버그는 **전부 2인 반박 투표를 통과한 확정 건**(refuted 0)이며,
> 대부분 실측 repro가 포함된다. 미구현 항목은 반박 검증을 통과한 **진짜 gap**만 남긴 것.
> SCOPE_LOCK §3/§4가 금지하는 항목 16건은 정확히 차단되어 여기 '구현 금지'로만 기록한다.
> 수정 상태는 이 파일이 아니라 git 이력이 정본이다.

## A. 확정 버그 26건 (high 6 · med 11 · low 9)

### A1. [HIGH] `pnix-hy/pnix_hy/deploy.py:26`

deployment_info() reports the projection/full_gate tiers as available whenever hm.hy_python() merely finds a Python *binary* on disk. hy_python() (hy_mirror.py:74-83) only does candidate.exists() — it never checks that Hy 1.3.0 (or any Hy) is importable in that interpreter. So proof_ok is set True (line 27) for a Hy-less Python, projection_ok becomes True (line 30), tiers.projection/tiers.full_gate report True (lines 42-43) and the corrective hint is suppressed (None, lines 45-48). This is a false-ready report from a module whose entire job is 'which capability tiers work HERE'.

**Repro:** PNIX_HY_PYTHON=/usr/bin/python3 (system python with NO hy) then call deploy.deployment_info(): observed proof_python_found=True, tiers={'core':True,'projection':True,'full_gate':True}, hint=None — yet `python3 -c 'import hy'` fails (ModuleNotFoundError), so --gate / projection actually break at runtime.

### A2. [HIGH] `pnix-hy/pnix_hy/hy_mirror.py:313`

Stage7 worker protocol desyncs permanently on any fragment that writes to stdout: _STAGE7_WORKER_SCRIPT does not redirect stdout during kernel.eval_source (unlike the projection worker's contextlib.redirect_stdout), so printed text is read as the response line; json.loads(line) at line 313 raises a raw JSONDecodeError (not HyMirrorError, so stage7_eval's fallback at lines 334-338 neither catches it nor kills the worker), and the real JSON response stays buffered, shifting the protocol so EVERY subsequent stage7_eval silently returns the PREVIOUS request's value.

**Repro:** Verified live: stage7_eval('(do (print "HELLO") 43)') -> JSONDecodeError crash; then stage7_eval('(+ 1 2)') -> '43' (stale response, wrong value); stage7_eval('(+ 10 10)') -> '3'. One-shot path (PNIX_HY_NO_WORKER=1) returns mixed stdout instead, so worker and fallback also disagree.

### A3. [HIGH] `pnix-hy/pnix_hy/interop.py:568`

_required_positional_count returns None ("variadic -> unary") the moment it sees a VAR_POSITIONAL parameter, discarding required positional params already counted; host_callable_to_pnix then treats the callable as unary and invokes fn() after the FIRST pnix arg, so any host function with >=2 required positionals plus *args (max, os.path.join, lambda a, b, *rest) crashes with a raw uncaptured TypeError inside pnix eval instead of currying — contradicts the docstring 'Multi-arg callables are curried by required-positional arity'.

**Repro:** ctx = rt.runtime_context(None); ctx['env'] = {'f': host_callable_to_pnix(lambda a, b, *rest: a + b)}; rt.eval_source_raw('f 1 2', ctx, realize=True) -> TypeError: <lambda>() missing 1 required positional argument: 'b' (verified; expected 3)

### A4. [HIGH] `pnix-hy/pnix_hy/pnix_mirror.py:2414`

_pe's `let` handler folds bindings strictly in textual order into a sequential env, but pnix let is recursive: a binding that references a name bound (or shadowed) later in the same let is resolved against the OUTER static env, producing a silently wrong fully-static value with zero gaps. Contradicts the module's own claim that static folding 'can't diverge from pnix semantics'; the wrong value also flows into meta_circular_tower's collapse stage and the _specialize_cache.

**Repro:** specialize_pnix('let x = 5; in let y = x + 1; x = 10; in y') -> {fully_static: True, value: 6, gaps: []} while rt.eval_source(same) == 11. Also specialize_pnix('let b = a + 1; a = 2; in b') emits residual '(let [b (+ a 1)] b)' with `a` free (NameError when run) though pnix evaluates it to 3 — no gap recorded either way.

### A5. [HIGH] `pnix-hy/pnix_hy/pnix_mirror.py:2437`

_pe's `attrset` branch `break`s out of the bindings loop on the first multi-segment path (len(path) != 1), so the residual dict silently drops that binding AND every later binding, records no gap, and returns a wrong dynamic residual instead of residualizing opaquely like other unhandled constructs.

**Repro:** specialize_pnix('{ a.b = 1; c = 2; }') -> {fully_static: False, residual_hy: '{}', gaps: []} — the residual evaluates to an empty dict, while rt.eval_source(same) == {'a': {'b': 1}, 'c': 2}; specialization consumers get {} with no warning.

### A6. [HIGH] `pnix-hy/scripts/ci-local.sh:18`

find_hy_python probes candidates with `"$c" -c 'import hy'` from the pnix-hy PACKAGE root, but the supported proof Pythons (the /tmp venvs hy_mirror._candidate_pythons targets) have no pip-installed hy — they import the VENDORED hy only via cwd=HY_ROOT (the repo-root `hy` symlink), exactly how every hy_mirror subprocess runs (cwd=str(HY_ROOT)). So the script rejects the canonical PNIX_HY_PYTHON and exits 2 'no Python with Hy 1.3.0 found' on a configuration where --check/--gate fully pass; it also silently overrides an explicitly set PNIX_HY_PYTHON with a later candidate and never verifies the found hy is actually 1.3.0 despite claiming so.

**Repro:** Verified live: /tmp/pnix-hy-py311-venv/bin/python -c 'import hy' from ~/pnix-hy/pnix-hy -> ModuleNotFoundError (same python imports vendored hy 1.3.0 fine from HY_ROOT). Hence PNIX_HY_PYTHON=/tmp/pnix-hy-py311-venv/bin/python bash scripts/ci-local.sh -> exit 2 (false CI failure) unless some PATH python happens to have pip hy.

### A7. [MED] `pnix-hy/pnix_hy/action.py:170`

The module docstring states this layer 'deliberately owns no evaluator, mirror, gate, backup system, or host machinery', but check_action()/verify_action() call pm.roundtrip_status(source), which via projection_value_roundtrip() -> hy_mirror.stage7_eval() spawns the hy-meta bootstrap Hy proof SUBPROCESS on every unique source. A supposedly thin action-checkpoint therefore blocks ~30s per call (measured). action_report()/verify_action run several such calls, so the self-check gate spends minutes; two sequential verify_action calls on an impure source did not finish within a 2-minute timeout. Callers assuming a lightweight verdict (e.g. under safe_eval's 5s budget elsewhere) get an unexpectedly heavy/blocking host dependency.

**Repro:** time pnix_mirror.roundtrip_status('1 + 2') -> 31.77s wall (Hy subprocess); roundtrip_status('builtins.pathExists "/etc/passwd"') -> 33.03s. Each check_action('1 + 2')/verify_action(...) inherits this cost; two verify_action calls back-to-back exceeded 120s.

### A8. [MED] `pnix-hy/pnix_hy/capabilities.py:23`

docs/proposals paths are derived from Path(__file__).resolve().parents[1], ignoring PNIX_HY_HOME — unlike hy_mirror.HY_ROOT/deploy which honor it for off-tree installs. On an installed (non-editable) pnix_hy with PNIX_HY_HOME pointing at the checkout, --capabilities silently renders counts proposals=0 and an empty proposals table (vs 13), so the documented regen command truncates the committed docs/CAPABILITIES.md; docs_drift_report likewise reports ready=True 'not applicable' even though the real docs tree is reachable. Bonus: _REPO_ROOT becomes site-packages' parent, so a stray docs/ or README.md there activates the gate against a foreign tree (capabilities_md_current=False -> --check exit 1).

**Repro:** cp -R pnix_hy to a scratch 'site-packages', run with PNIX_HY_HOME=~/pnix-hy PYTHONPATH=<scratch> from outside the repo: capability_index()['counts'] == {'symbols': 85, 'reports': 57, 'proposals': 0} and render_capabilities_md() contains only the header 'docs/proposals/' mention (0 proposal rows); docs_drift_report() -> {'ready': True, 'available': False, 'note': 'docs tree absent...'}. Piping that render into docs/CAPABILITIES.md per the file's own instruction loses all 13 proposals and then fails the in-repo CI drift gate.

### A9. [MED] `pnix-hy/pnix_hy/capabilities.py:213`

docs_drift_report resolves ANY 4-digit wikilink via re.fullmatch(r"[0-9]{4}", target), so links to nonexistent proposals are never flagged. The check is also redundant: real proposal ids (0000-0012) are already in `known` via _known_names, so the fullmatch branch only ever whitelists broken ids — a false-ready doc<->code gate for exactly the link class (proposal refs) the gate exists to guard.

**Repro:** Add 'see ⟦9999⟧' (no proposal 9999 exists) to any scanned doc, e.g. docs/notes.md -> docs_drift_report() returns ready=True, wikilinks_unresolved=[]. Verified by injecting a doc with '⟦9999⟧ and ⟦totally_missing_symbol⟧' (⟦·⟧ = 실제로는 이중대괄호 위키링크 문자): only totally_missing_symbol is reported unresolved.

### A10. [MED] `pnix-hy/pnix_hy/hy_mirror.py:309`

Worker request paths drop the timeout contract entirely: _stage7_worker_eval's proc.stdout.readline() (line 309) and _proj_worker_run's readline (line 469) have no deadline, and _stage7_ensure_worker/_proj_ensure_worker's drain loops only re-check their deadline between lines, so a child that emits nothing blocks readline forever. The one-shot paths enforce timeout=120/60 via subprocess.run, so the same input behaves differently depending on worker mode.

**Repro:** stage7_eval('(while True 1)') (or any long/hung fragment) with the worker enabled hangs the caller forever; with PNIX_HY_NO_WORKER=1 the identical call raises HyMirrorError after the 120s subprocess timeout. Similarly a wedged worker startup (kernel build stalls without printing) hangs _stage7_ensure_worker past its nominal 120s deadline.

### A11. [MED] `pnix-hy/pnix_hy/interop.py:388`

to_host misclassifies a genuine pnix STRING whose value equals a function sentinel: _is_pnix_function(raw) has already returned False (raw is a real string), yet the 'data in _FUNCTION_SENTINELS' branch converts it to an opaque ref with kind 'pnix-function' / input_kind 'pnix-function', so the string value never crosses to the host at all — wrong value, wrong kind, false capability requirement.

**Repro:** v, r = io.to_host(rt.eval_source('"#<pnix-hy-closure>"')) -> v is an opaque-ref dict (is_opaque_ref True, kind 'pnix-function', loss 'opaque') instead of the string '#<pnix-hy-closure>' with loss 'lossless' (verified)

### A12. [MED] `pnix-hy/pnix_hy/interop.py:402`

The A6 path/string-context loss detection only inspects the TOP-LEVEL forced value; a PnixPath (or context-carrying PnixString) nested inside an attrset/list is collapsed to a plain str by stable_data but the record is still marked lossless — a false fidelity claim that contradicts the module's own A6 contract ('fidelity is never silently claimed lossless') and is inconsistent with the top-level case.

**Repro:** raw = rt.eval_source_raw('{ p = ./foo; }', rt.runtime_context(None), realize=False); hv, rec = io.to_host(raw) -> hv == {'p': './foo'}, rec.loss_status == 'lossless' (verified), while to_host of top-level ./foo correctly reports loss 'lossy' / output_kind 'path'

### A13. [MED] `pnix-hy/pnix_hy/interop.py:720`

wrap_pnix_callable forces the closure (rt.force_value(closure_raw)) OUTSIDE the D1 try/except, so a pnix expression that errors at force time leaks a raw PnixError into host code, violating the InteropError contract stated in the class docstring ('raises this instead of leaking a raw pnix PnixError across the boundary into host code (D1)').

**Repro:** io.pnix_callable('throw "boom"') raises PnixError('boom') to the host caller instead of InteropError (verified: RAW PnixError, isinstance rt.PnixError True)

### A14. [MED] `pnix-hy/pnix_hy/pnix_mirror.py:1433`

_pnix_to_hy's `select` branch synthesizes `(. base attr)` — Python attribute access — for pnix attrset selection, but the base is projected as a Hy dict, so the synthesized Hy always raises AttributeError; unlike let/has_attr/rec-attrset it records NO gap, so pnix_to_hy_form reports clean=True and classify_drift reports drift_count=0 (a false 'no drift' verdict) for a projection that can never evaluate.

**Repro:** pnix_to_hy_form('{ a = 1; }.a') -> hy_source '(. {"a" 1} a)', gaps [], clean True; classify_drift('{ a = 1; }.a') -> {clean: True, drift_count: 0}; evaluating the projected form raises AttributeError: 'dict' object has no attribute 'a' (confirmed in the stage7 kernel via projection_value_roundtrip).

### A15. [MED] `pnix-hy/pnix_hy/pnix_mirror.py:2400`

_pe's `if` handler prunes on Python truthiness of a static condition (`if cond[1]`), but pnix requires the condition to be a bool and errors otherwise — so specialize_pnix returns a successful fully-static value for a program the real runtime rejects.

**Repro:** rt.eval_source('if 1 then 2 else 3') raises PnixError 'if condition: expected bool, got int', but specialize_pnix('if 1 then 2 else 3') -> {fully_static: True, value: 2, gaps: []} (verified).

### A16. [MED] `pnix-hy/pnix_hy/repl.py:35`

_BIND_RE only accepts [A-Za-z_][A-Za-z0-9_]* names, but pnix identifiers also allow `'` and `-` (pnix_runtime._is_ident_char, line 154 — e.g. foldl', my-var). Binding such a name silently falls through to expression evaluation and fails with 'unexpected trailing tokens' plus a caret pointing at the value; these valid pnix names can never be bound in the REPL (and `:let foldl' = 2` fails the same way).

**Repro:** In run_pnix_repl, input "foldl' = 2" -> 'error: PnixError: unexpected trailing tokens at 1: [Token(sym =, pos=7), Token(number 2, pos=9)]' with the excerpt caret under '2', instead of binding foldl' (verified via io.StringIO session).

### A17. [MED] `pnix-hy/pyproject.toml:24`

The `projection`/`full` extras install hy==1.3.0 into the consuming interpreter, but hy_mirror._candidate_pythons() (hy_mirror.py:55-70) never considers sys.executable — only PNIX_HY_PYTHON plus six hardcoded machine-specific paths (including world-writable /tmp venv paths that get silently preferred if present). deploy.py's own recovery hint ('install pnix-hy[projection] and set PNIX_HY_HOME=...') therefore does not work: following it exactly still yields projection=False / HyMirrorError because PNIX_HY_PYTHON also has to be set, which neither the extras nor the hint mention.

**Repro:** Fresh machine: pip install 'pnix-hy[projection]'; export PNIX_HY_HOME=/path/to/checkout; python -c "import pnix_hy.hy_mirror as hm; hm.hy_form_projection('(+ 1 2)')" -> HyMirrorError 'no supported Hy proof Python found' even though `import hy` succeeds in that very interpreter; deployment_info() reports tiers.projection False.

### A18. [LOW] `pnix-hy/Makefile:5`

`PY ?= python` makes check/gate/verify/capabilities-check run `python -m pnix_hy.cli`, which does not exist on stock macOS (only python3), so the documented entry point `make -C pnix-hy verify` fails with exit 127 out of the box; additionally `PYTHONPATH := $(CURDIR)` + `export PYTHONPATH` clobbers any caller-set PYTHONPATH rather than prepending, and capabilities-check truncates docs/CAPABILITIES.md via shell redirection before $(PY) runs, so a missing/failing $(PY) leaves the committed capability index emptied in the worktree (making the subsequent --check docs_drift gate fail for a manufactured reason).

**Repro:** On a machine without a `python` shim (default macOS): `make -C pnix-hy capabilities-check` -> 'python: No such file or directory' AND docs/CAPABILITIES.md is now 0 bytes (git diff shows the whole file deleted); `make check` similarly exits 127.

### A19. [LOW] `pnix-hy/pnix_hy/action.py:131`

granted_tuple = tuple(granted) (and gate.gate_check's set(granted)) silently mis-handle a bare str argument: tuple('file-read') -> ('f','i','l','e','-','r','e','a','d'), so a caller passing granted='file-read' (a plausible mistake given the union type hint) grants nine one-character capabilities and none of the real effect classes. The action is then wrongly denied/HELD instead of accepted. No guard rejects the str form.

**Repro:** check_action('builtins.pathExists "/etc/passwd"', granted='file-read') -> status 'held', gate.denials non-empty (file-read not granted), whereas granted=('file-read',) -> status 'accepted'.

### A20. [LOW] `pnix-hy/pnix_hy/capabilities.py:52`

_summary_of uses inspect.getdoc(obj) on plain-value public symbols, which falls back to the value's TYPE docstring, and the module fallback (line 72) attributes them to 'pnix_hy'; the committed capability index therefore documents ROUNDTRIP_STATUS_VOCAB as summary 'Built-in immutable sequence.' with owner module `pnix_hy` — wrong summary/owner in the anti-duplicate-development lookup table (any future tuple/dict/str constant in __all__ gets its builtin type's docstring too).

**Repro:** grep ROUNDTRIP_STATUS_VOCAB docs/CAPABILITIES.md -> '| `ROUNDTRIP_STATUS_VOCAB` | value | pnix-hy | `pnix_hy` | Built-in immutable sequence. |' (line 15 of the generated file, verified); the symbol's real owner and meaning (the roundtrip status vocabulary) are absent.

### A21. [LOW] `pnix-hy/pnix_hy/cli.py:253`

cmd_specialize (and cmd_hy_trace at line 286) do not catch parse/compile errors, so malformed input crashes the CLI with a raw multi-hundred-line traceback (PnixError from pnix_runtime.parse; SyntaxError from hy-meta execution_trace's compile()), while sibling commands (--explain, --diagnose, --safe-eval) return structured errors and exit 0.

**Repro:** pnix-hy-project --specialize '((' -> uncaught pnix_hy.pnix_runtime.PnixError traceback (verified); pnix-hy-project --hy-trace '(this is bad syntax' -> uncaught SyntaxError traceback from compile() in hy-meta/host_introspect.py (verified). Note --hy-trace '(undefined-name-xyz)' IS handled gracefully, so the error contract is inconsistent.

### A22. [LOW] `pnix-hy/pnix_hy/cli.py:397`

_split_capability_spec (used by --explain/--action-check/--action-explain/--specialize, duplicated inline in cmd_gate_check at line 380) splits the argument on the FIRST ';;' anywhere, including inside pnix string literals, silently truncating legal source and treating the string tail as capability grants — there is no escape or space-delimited requirement.

**Repro:** pnix-hy-project --safe-eval '"a;;b"' -> value 'a;;b' (ground truth), but pnix-hy-project --explain '"a;;b"' -> src becomes '"a', granted=('b"',) -> 'parse unterminated string literal' (verified). Same for --action-check/--gate-check/--specialize.

### A23. [LOW] `pnix-hy/pnix_hy/hy_mirror.py:337`

stage7_eval's `except HyMirrorError: _stage7_kill_worker()` cannot distinguish worker-infrastructure failures from ordinary eval errors (_stage7_worker_eval raises HyMirrorError for ok:False responses too), so ANY erroring Hy fragment kills the healthy warm worker, pays the ~27s one-shot kernel rebuild for this call, and forces another ~27s worker kernel rebuild on the next call — defeating the module's stated 'dominant speed lever' whenever user code merely raises.

**Repro:** With a warm worker (fast '(+ 1 2)' calls ~0s), call stage7_eval('(undefined-symbol)'): it raises after a ~27s one-shot run instead of the worker's instant error response, and the following stage7_eval('(+ 1 2)') pays another ~27s worker rebuild instead of ~0s.

### A24. [LOW] `pnix-hy/pnix_hy/interop.py:431`

from_host of a mixed-type (unorderable) set falls back to list(value), i.e. raw hash-iteration order, so the projected pnix list is nondeterministic across processes (PYTHONHASHSEED) — the same host value maps to different pnix values run-to-run, breaking reproducibility of the projection (the lossy flag documents lost set-ness, not nondeterminism).

**Repro:** PYTHONHASHSEED=1 python -c "...from_host({1,'a','b','c'})[0]" -> [1, 'a', 'b', 'c']; PYTHONHASHSEED=7 -> [1, 'b', 'a', 'c'] (verified)

### A25. [LOW] `pnix-hy/pnix_hy/interop.py:509`

roundtrip_host_value misclassifies a plain host dict containing the reserved '__pnix_opaque__' key: from_host converts it as ordinary data (loss 'lossless') but is_opaque_ref(pv) then matches, so the report claims roundtrip 'by-ref' / opaque True with loss_status 'lossless' while equal is False — a self-contradictory fidelity report; if the integer happens to equal a live registry id, resolve_opaque would even return an unrelated object.

**Repro:** io.roundtrip_host_value({'__pnix_opaque__': 5}) -> {'roundtrip': 'by-ref', 'from_host_loss': 'lossless', 'loss_status': 'lossless', 'equal': False, 'opaque': True} (verified) — loss 'lossless' with equal False is contradictory

### A26. [LOW] `pnix-hy/pnix_hy/repl.py:100`

run_pnix_repl assigns env['_'] = raw before realize_value(raw); when evaluation succeeds lazily but realization fails, the failed line clobbers `_` with a value that errors on every force, discarding the previous good result — the docs say 'result bound to _' but the line produced no result.

**Repro:** REPL session: '41 + 1' prints 42; '{ a = 1 / 0; }' prints 'error: PnixError: division by zero'; then '_' prints 'error: PnixError: division by zero' instead of the last result 42 (verified via io.StringIO session).

## B. 검증된 진짜 미구현(제안 가능) 5건

새 기능이므로 SCOPE_LOCK §7에 따라 구현 전 proposal로 다룬다 (0013 후보 카탈로그 참조).

- **[med] (pnix-ast-ir)** pnix-ir-diff (structural diff between two pnix IRs/ASTs showing where they differ)
  - 근거: No node-level structural diff of two pnix IRs/ASTs exists under any name in either lane. Only boolean equality via content hashes exists (ir.py ir_hash/ir_roundtrip h1==h2). All existing diff-like machinery serves other purposes: compare_stage8_artifact_bundles diffs host Python artifact bundle hash keys, classify_drift classifies projection gaps of one source, the correspondence-table 'differs' flags are a static Python/Hy<->pnix mapping. Not listed in SCOPE_LOCK §3 placeholders, §4 forbidden items, or §5 out-of-scope items, and no proposal 0000-0012 ships or declines it (0000 C5 is classify_drift, a different thing). Caveat: adding it would require a proposal per SCOPE_LOCK §7 process, but the capability itself is neither forbidden nor out-of-scope.
- **[low] (artifact-bundle-cache)** form_sha256: a Hy form/model-level hash in the artifact record distinct from source_sha256 and ast_sha256 (read-stage identity)
  - 근거: No Hy form/model-level hash exists under any name in either lane. artifact_from_ast (hy-meta/bootstrap.py:528-566) emits exactly source_sha256, ast_sha256, python_sha256, normalized_sha256, code_sha256, raw_code_sha256, pyc_sha256 — all computed from the source text, the Python AST dump, and compiled code objects; the intermediate hy.models read-stage layer is never serialized or hashed. Greps for form_sha/model_sha/form_hash/model_hash/hy_sha across hy-meta/ and pnix-hy/pnix_hy return nothing relevant (the sole 'model_hash' hit at hy-meta/bootstrap.py:5456 is an unrelated solver fixture string). SCOPE_LOCK §3 placeholder table does not cover this, §4 does not forbid it, §5 lists the artifact machinery as in-scope lane-local (hy-meta owns 'Python/Hy artifact'), and proposals 0000-0012 neither ship nor decline it. Caveat: the scope is CLOSED per SCOPE_LOCK §1/§7, so this is a proposal-level new-feature candidate, not an open defect in the closed scope.
- **[low] (artifact-bundle-cache)** entrypoint and dependency-hash as fields of the cache key itself
  - 근거: The claim survives refutation: host_exec.cache_key has no entrypoint or dependency-hash component, and dependency hashing (stage9_manifest) is never wired into _ARTIFACT_CACHE invalidation, so a dependency change does not change cache keys. No synonym/composed implementation exists in either lane or in CAPABILITIES.md; SCOPE_LOCK §3/§4/§5 neither declares it an intentional placeholder nor forbids it (host artifact cache is explicitly the hy-meta lane's own §20 item, still marked partial [◑]); no proposal 0000-0012 ships or declines it. Caveat: the implemented key matches the documented §20 spec exactly (source/compiler/stage/env/py/hy-version), so this is an unspecified extension rather than a regression, and per SCOPE_LOCK §7 it would need a new proposal rather than a todo.
- **[low] (quote-macro-hygiene)** dedicated named hygiene/symbol-capture self-check REPORT in the pnix-hy projection toolkit (hygiene evidence lives only in hy-meta test lane, not as a hy_mirror *_report projection)
  - 근거: No hygiene/gensym/symbol-capture self-check report exists under any name in the pnix-hy projection toolkit: the 57-report registry in docs/CAPABILITIES.md has no such entry, and every macro-adjacent report (hy_macroexpand_projection_report, hy_macro_step_trace_report, hy_defmacro_projection_report, hy_quasiquote_projection) asserts only expansion shape/step counts, never gensym uniqueness or capture avoidance. Hygiene evidence lives solely in the hy-meta test lane. It is not an intentional placeholder (SCOPE_LOCK §3 line 54 covers only pnix-side macro absence and explicitly allows Hy-side OBSERVE-via-projection, which is what such a report would be), not forbidden (§4 line 63 forbids pnix-side macro implementation only), not out-of-scope (§5 covers other repos), and no proposal 0000-0012 shipped or declined it. Note: adding it would require a proposal per SCOPE_LOCK §7 since the scope is closed, but the capability itself is genuinely absent.
- **[low] (env-replay-determinism)** per-variable env-snapshot diff (name-by-name report of WHICH env var/manifest entry changed, rather than hash-level manifest_sha256/hard_env_sha256 mismatch classified as kind=env)
  - 근거: No per-variable env-snapshot diff exists under any name in either lane. Env drift is detected and reported only at hash granularity, and I could not refute the claim: (a) hy-meta/bootstrap.py maps env_sha256/environment_sha256/manifest_sha256 keys to the single drift kind "env" (lines 1935-1937, sample at 1974) without naming any variable; (b) the hard_env dict ({PYTHONHASHSEED, LC_ALL, LANG, TZ, PYTHONNOUSERSITE}, bootstrap.py ~2163-2170) is folded into manifest_sha256 (line 2171) and hard_env_sha256 (lines 4320-4321) and compared only as whole-dict/hash equality; (c) the only key-by-key drift machinery is compare_stage8_artifact_bundles (lines 2024-2040, differing_keys over hash fields like source_sha256/pyc_sha256, not env vars) and stage13_replay_historical_verdict's stale_reason_by_key (lines 4110-4120, four binding hashes, again not env vars); stage13_replay_env_verdict reports only reason=f"{binding_kind}-changed" i.e. "environment-changed" with no variable name (~line 4386). pnix-hy side (gate.py:88) only reads env_hash. Grep for env_diff/snapshot_env/per-key environ diff across hy-meta/, pnix-hy/pnix_hy/, and docs/CAPABILITIES.md returned nothing. It is not a SCOPE_LOCK §3 intentional placeholder (the §3 table lists eight items, none about drift/env granularity), not §4 forbidden, and not §5 out-of-scope (env-replay determinism is core in-scope hy-meta proof machinery). No proposal 0000-0012 covers it (0000-0012 are interop/REPL/action-VM/module-dist/docs/CI topics). Caveat: SCOPE_LOCK §7 requires any new feature like this to enter via a docs/proposals/NNNN document, not todo.md — so it is missing, but implementation without a proposal would violate governance process.

## C. 구현 금지 재확인(SCOPE_LOCK §3/§4) 16건

체크리스트에 있어도 아래는 **미구현이 아니라 의도**다. 다시 열지 말 것.

- (reader-source) pnix-reader-boundary-check as a pnix-SIDE reader-macro boundary: pnix has no reader macros by design — SCOPE_LOCK.md:54 (§3 table row §9: pnix macro/quasiquote/defmacro/reader-macro/require are intentional placeholders, Hy side OBSERVEs via projection only) and SCOPE_LOCK.md:63 (§4 forbidden: implementing pnix macro/quasiquote/reader-macro since pnix is explicitly non-homoiconic). The boundary is instead proven on the Hy side (bootstrap run_reader_boundary_check + hy_reader_embed_pnix docstring: 'This is Hy's reader machinery embedding pnix — NOT a pnix reader-macro').
- (pnix-ast-ir) Further IR desugaring (attrset-path folding, sugar expansion into fewer core tags) — explicitly declared a documented future refinement in ir.py:13-14, consistent with SCOPE_LOCK's rule that intentional placeholders/documented deferrals are not missing work; new features must go via docs/proposals per SCOPE_LOCK, so treat as intentional-not-missing rather than a gap
- (artifact-bundle-cache) Persistent on-disk / store-backed artifact cache (derivation store-hashing): SCOPE_LOCK §3 lists derivation store-hashing as an intentional placeholder — all caches are deliberately in-process (host_exec _ARTIFACT_CACHE docstring; pnix_mirror PP4 comment 'The cache lives in-process', pnix_mirror.py:3375)
- (mirrors) pnix-side host-artifact facet/envelope (mirror-pnix-host-artifact): intentionally lane-separated — SCOPE_LOCK.md §6 'hy-meta owns host Python/Hy artifact·import hook·clean replay·introspection; pnix-hy owns pnix reader/parser/AST/IR/eval/value/builtins/mirror' (lines 88-90); docs/SEPARATION.md:64 same takeaway; docs/INTEROP_ROLE_MATRIX.md:33 'host artifact interop envelope (codex P9)' explicitly out of pnix-hy interop scope (needs its own hy-meta proposal); hy_mirror.py:1940 comment marks traceback/module/execution_trace as host artifacts not pnix runtime. The capability itself exists on the hy-meta side (hy-meta/artifact.py, code_object.py, bytecode.py, pyc.py, host_exec.py:350-369 content-addressed host artifact cache).
- (reify-debug-explain) Filling the #_pnix-gap markers that classify_drift reports (pnix_mirror.py classify_drift docstring: 'it does NOT fill them (SCOPE_LOCK: they are by design)') — SCOPE_LOCK §3 intentional placeholders
- (reify-debug-explain) pnix-side macro/quasiquote/reader-macro reification absence — SCOPE_LOCK §3 marks pnix-side macro machinery absence as intentional, so no reify surface for it is expected
- (quote-macro-hygiene) pnix-quasiquote / pnix-unquote / pnix-splice — SCOPE_LOCK §3 (line 54: '§9 pnix macro / quasiquote / defmacro / reader-macro / require' = intentional placeholders _QUASIQUOTE/_DEFMACRO/_READER_MACRO/_IMPORT_PNIX_NOTE in hy_mirror.py; pnix is not homoiconic, Hy side OBSERVEs by projection only) and §4 (line 63: implementing pnix macro/quasiquote/reader-macro is a forbidden scope-reopening unless a proposal adopts homoiconicity)
- (quote-macro-hygiene) pnix-side defmacro / macro table — same SCOPE_LOCK §3 row + §4; the shipped substitute is the Hy-side bridge (proposals 0003/0005: hy_macro_over_pnix, hy_quasiquote_over_pnix, hy_reader_embed_pnix)
- (env-replay-determinism) SCOPE_LOCK §3: fail-closed stage16 peer-review all-None record (bootstrap.py ~6437) — non-None value is by definition DRIFT; do not 'fix' by making the record deterministic-valued
- (env-replay-determinism) SCOPE_LOCK line 13: pre-reconcile open todos (stage8/9 CI wiring, standalone drift/diff tooling) are explicitly closed within current scope — absence of a separate drift-diff CLI tool is intentional, not missing work
- (roundtrip-witness) pnix IR -> python-AST -> IR structural roundtrip — The claim is literally true that no single function named/implementing "pnix IR -> python-AST -> IR structural-identity roundtrip" exists, but treating it as MISSING work is exactly what SCOPE_LOCK forbids. Both directional structural mappings ALREADY exist under other names: pnix->py-AST is the composition `pnix_to_hy_form` (pnix_mirror.py:1461, via `_pnix_to_hy` :1329) + `hy_form_projection` lowering to python_ast (used exactly this way in `pnix_to_hy_form_report`, pnix_mirror.py:1478-1487, which IS a pnix->Hy->python-AST roundtrip sanity check); py-AST->pnix is `_python_expr_to_pnix` (pnix_mirror.py:1710) / `synthesize_pnix_from_hy` (:1843), plus structural labeling `align_python_to_pnix`/`_tree` (:1035/:1231) and the content-hashed `correspondence_table`/`correspondence_abi` (:731-:902). The roundtrip CLOSURE is deliberately proven at the VALUE level in both directions (`projection_value_roundtrip` :1597, `hy_to_pnix_value_roundtrip`) rather than by structural identity, because pnix is non-homoiconic and the projection is documented as intentionally gapped: SCOPE_LOCK.md section 3 has a dedicated placeholder row naming precisely the two functions this capability would live in (`#_pnix-gap[...]` markers in `_pnix_to_hy`/`_python_expr_to_pnix` = intentional "no clean projection" markers), and section 4 explicitly forbids reinterpreting section-3 placeholders as gaps to fill. A total structural IR<->py-AST bijection cannot exist without filling those gaps; a subset check would be a new feature requiring a proposal per section 7, not a repo gap. The checklist's own 'candidate' label confirms it was never accepted scope.
- (roundtrip-witness) first-class witness fields stage-id / artifact-hash / rule-id / error-class / error-message / evidence-files — Promoting stage-id/artifact-hash/rule-id/error-class/error-message/evidence-files to top-level witness fields would change the locked witness FIELD SCHEMA, which SCOPE_LOCK.md §6 declares the SOLE shared ABI envelope between the two lanes; §7 requires any such change to go through a proposal document plus a both-lane + drift-guard update. This is not a gap: the schema is deliberately frozen and runtime-enforced by the witness_schema_ok drift-guard, and the "missing" data is not lost — make_witness embeds the full payload (bootstrap/interop payloads already carry artifact_hash, error_class, evidence_id, etc.), so the information is reachable via witness["payload"] without schema expansion. No proposal 0000-0012 requests these fields (0006, the error-contract proposal, explicitly states it does NOT touch the witness field schema), so it is neither shipped nor declined — it is simply a schema change forbidden without a new proposal.
- (roundtrip-witness) Extending the shared witness field schema (e.g. adding stage-id/rule-id/evidence-files to the envelope) without a proposal — SCOPE_LOCK §6 fixes '§14 witness FIELD SCHEMA (in_hash/out_hash/env_hash/status/loss + InteropRecord field names)' as the ONLY shared cross-lane contract, with the gate.gate_report witness_schema_ok drift-guard; §7 requires any envelope change to go through a proposal doc updating both lanes + drift-guard.
- (gates-sandbox-pollution) pnix-side macro / reader-macro / quasiquote gates or tables: SCOPE_LOCK §3 (row '§9 pnix macro / quasiquote / defmacro / reader-macro / require' — pnix is non-homoiconic, Hy side observes only) and §4 explicitly forbids implementing pnix macro/quasiquote/reader-macro; hence a pnix-side macro/reader-macro gate cannot exist and macro-table concerns are host(Hy)-side only (which ARE checked, bootstrap.py:1281-1283)
- (gates-sandbox-pollution) Fail-closed stage16 and 'unsupported-X error messages' are intentional placeholders per SCOPE_LOCK §3 — do not reinterpret unimplemented effectful builtins (e.g. real fetchurl/exec execution) as missing sandbox work
- (introspect-interop-opaque) Dedicated descriptor-protocol inspection and generator/coroutine/property CONTROL primitives (send/throw/fget etc.): hy-meta/README.md:316-318 explicitly keeps 'Python 예외, descriptor, context-manager 프로토콜, async 프로토콜, 일반 Python 객체 모델' as HOST runtime services (intentional boundary, not to be re-implemented in the proof lane); pnix side: pnix_hy/pnix_mirror.py:810 documents 'pnix has no generators/coroutines/async; laziness (thunks) is the closest evaluation-deferral' — a SCOPE_LOCK §3-style intentional gap marker (#_pnix-gap class). Generic inspection of such objects is still covered by object_info (mro/dir/is_routine) and inspect_opaque + opaque_call_method for controlled method-level access.
