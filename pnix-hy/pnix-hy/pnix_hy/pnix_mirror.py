"""Mirror chain checks for the pnix-hy runtime, plus hy-meta proof-ladder evidence.

The pnix mirror runs the convergence chain locally and federates two kinds of
hy-meta evidence: the stage7 eval-kernel probe (the substrate Hy runs in) and the
stage15/stageN closure verdict (hy-meta is closed well past stage7; see
`hy_mirror`)."""

from __future__ import annotations

import json as _json
import sys as _sys
import dis as _dis
import time as _time
import re as _re
import hashlib as _hashlib
from typing import Any

# `from . import hy_mirror` would resolve via `hasattr(pnix_hy, "hy_mirror")`,
# which (since pnix_hy/__init__.py defines a lazy module __getattr__) pulls in
# the ENTIRE .meta/.proof graph just to answer that probe -- and that graph
# transitively re-enters this module (pnix_mirror) before it has finished
# executing past this point, breaking on names defined further down (e.g.
# gate.py's `_sha256`, stage.py's `cached_eval`). An absolute dotted import
# bypasses the package's __getattr__ entirely (it goes straight through the
# ordinary submodule-import path), so it can't trigger that cascade.
import pnix_hy.hy_mirror as hy_mirror
from . import interop as _interop
from . import pnix_runtime as rt


MIRROR_SELF_TEST_SCHEMA = "pnix-hy.mirror.self-test.v0"
ROUNDTRIP_STATUS_VOCAB = ("lossless", "lossy-ok", "held", "rejected")


def _eval_parsed_for_mirror(source: str, ast: dict[str, Any]) -> tuple[Any, list[dict[str, Any]]]:
    """Evaluate a parsed pnix AST once, preserving the runtime's native mirror events."""
    events: list[dict[str, Any]] = []
    ctx = rt.runtime_context(None)
    ctx["events"] = events
    ctx["source_text"] = source
    env = rt.initial_env(ctx)
    value = rt.realize_value(rt.eval_ast(ast, env, ctx))
    return value, events


def _value_sha256(value: Any) -> str:
    try:
        return _sha256(rt.to_json_string_value(value))
    except Exception:  # noqa: BLE001 - non-JSON values still get a stable data hash
        return _sha256(_json.dumps(rt.stable_data(value), sort_keys=True, separators=(",", ":")))


def singleton_mirror_run(source: str, *, trace: bool = False) -> dict[str, Any]:
    """Canonical singleton pnix mirror run.

    This is the shared implementation behind `pnix_hy.mirror.mirror_run` and the legacy
    `run_once`/`mirror_chain`/`run_mirror` views. One parse/emit/reparse/eval route emits
    all mirror facets; older surfaces project fields out of this record instead of owning
    a second mirror implementation.
    """
    facets: list[dict[str, Any]] = []

    def emit(kind: str, **payload: Any) -> None:
        facets.append({"facet": kind, **payload})

    result: dict[str, Any] = {"schema": "pnix-hy.mirror-run.v0", "source": source, "facets": facets}
    emit(":mirror/source", source=source, source_sha256=_sha256(source))

    try:
        tokens = rt.tokenize(source)
        emit(":mirror/token", count=len(tokens))
        ast = rt.parse(source)
    except Exception as exc:  # noqa: BLE001
        emit(":mirror/error", phase="tokenize", message=f"{type(exc).__name__}: {exc}")
        result.update(run_id=_sha256(source), ok=False, phase="tokenize")
        return result

    try:
        emitted = rt.emit_source(ast)
        reparsed = rt.parse(emitted)
        value, events = _eval_parsed_for_mirror(source, ast)
    except Exception as exc:  # noqa: BLE001
        emit(":mirror/error", phase="eval", message=f"{type(exc).__name__}: {exc}")
        result.update(run_id=rt.ast_hash(ast), ok=False, phase="eval")
        return result

    ast_hash = rt.ast_hash(ast)
    emitted_ast_hash = rt.ast_hash(reparsed)
    result_hash = rt.term_hash(value)
    run_id = ast_hash
    emit(":mirror/ast", root_tag=ast.get("tag"), ast_sha256=ast_hash,
         emitted_ast_sha256=emitted_ast_hash)
    emit(":mirror/ir", canonical=emitted, ir_sha256=ast_hash)

    purity = _static_purity_check_ast(source, ast)
    emit(":mirror/effect", pure=purity.get("pure"), impure_uses=purity.get("impure_uses"))

    emit(":mirror/value", value=value, result_hash=result_hash, value_sha256=_value_sha256(value))
    rec = _interop.to_host(value)[1]
    emit(":mirror/interop", direction=rec.direction, loss_status=rec.loss_status,
         effect_class=rec.effect_class, output_kind=rec.output_kind,
         witness_id=rec.witness_id)

    emit(":mirror/eval-step", event_count=len(events), events=(events if trace else events[:8]))
    result_sha = _sha256(ast_hash + "::" + result_hash)
    emit(":mirror/witness", result_sha256=result_sha, run_id=run_id)

    result.update(
        run_id=run_id,
        ok=True,
        emitted_source=emitted,
        ast_hash=ast_hash,
        emitted_ast_hash=emitted_ast_hash,
        result_hash=result_hash,
        value=value,
        events=events,
        result_sha256=result_sha,
    )
    return result


def run_once(source: str) -> dict[str, Any]:
    run = singleton_mirror_run(source, trace=True)
    if not run.get("ok"):
        raise rt.PnixError(run.get("facets", [{}])[-1].get("message", "mirror run failed"))
    return {
        "source": source,
        "emitted_source": run["emitted_source"],
        # ast_hash strips source-position metadata (pos/path_positions): emit
        # canonicalizes layout, so reparsing changes byte offsets but not the
        # AST's semantic shape. term_hash would wrongly diverge on positions.
        "ast_hash": run["ast_hash"],
        "emitted_ast_hash": run["emitted_ast_hash"],
        "result_hash": run["result_hash"],
        "value": run["value"],
        "events": run["events"],
        "facets": run["facets"],
        "run_id": run["run_id"],
        "result_sha256": run["result_sha256"],
    }


def mirror_chain(source: str, n: int = 7) -> dict[str, Any]:
    runs = [run_once(source) for _ in range(n)]
    first_hash = runs[0]["result_hash"] if runs else None
    diverged_at = None
    for index, run in enumerate(runs):
        if run["result_hash"] != first_hash:
            diverged_at = index
            break
        if run["ast_hash"] != run["emitted_ast_hash"]:
            diverged_at = index
            break
    return {
        "schema": "pnix-hy.mirror.chain.v0",
        "source": source,
        "n": n,
        "converged": diverged_at is None,
        "diverged_at": diverged_at,
        "result_hash": first_hash,
        "run_hashes": [run["result_hash"] for run in runs],
        "value": runs[-1]["value"] if runs else None,
        "runs": [
            {
                "ast_hash": run["ast_hash"],
                "emitted_ast_hash": run["emitted_ast_hash"],
                "result_hash": run["result_hash"],
                "event_count": len(run["events"]),
            }
            for run in runs
        ],
    }


def run_mirror(source: str, n: int = 7, include_hy_host: bool = True) -> dict[str, Any]:
    chain = mirror_chain(source, n)
    report: dict[str, Any] = {
        "schema": "pnix-hy.mirror.run.v0",
        "ready": chain["converged"],
        "runtime": rt.RUNTIME_SCHEMA,
        "mirror_stage_count": n,
        "source": source,
        "value": chain["value"],
        "result_hash": chain["result_hash"],
        "chain": chain,
    }
    if include_hy_host:
        report["hy_host"] = hy_mirror.host_summary(light=True)
        report["ready"] = bool(report["ready"] and report["hy_host"]["kernel_probe"]["ready"])
    return report


def stage_tower(source: str) -> dict[str, Any]:
    chain = mirror_chain(source, 7)
    direct = chain["value"]
    emitted = rt.eval_normalized_source(source)
    hashes = {
        "stage1": rt.term_hash(direct),
        "stage2": rt.term_hash(emitted),
        "stage3": chain["runs"][0]["ast_hash"],
        "stage4": chain["runs"][0]["emitted_ast_hash"],
        "stage5": chain["runs"][0]["result_hash"],
        "stage6": chain["runs"][1]["result_hash"],
        "stage7": chain["runs"][-1]["result_hash"],
    }
    value_hashes = [hashes["stage1"], hashes["stage2"], hashes["stage5"], hashes["stage6"], hashes["stage7"]]
    ast_hash_ok = hashes["stage3"] == hashes["stage4"]
    return {
        "schema": "pnix-hy.stage-tower.v0",
        "source": source,
        "equivalent": ast_hash_ok and len(set(value_hashes)) == 1 and chain["converged"],
        "hashes": hashes,
        "value": chain["value"],
        "chain": chain,
    }


def hy_runtime_batch(cases: list[dict[str, Any]] | None = None) -> dict[str, Any]:
    cases = cases or rt.HY_RUNTIME_CORPUS
    asts = [rt.parse(case["source"]) for case in cases]
    hy_values = hy_mirror.stage7_eval_json(rt.hy_runtime_source_for_asts(asts))
    checks = []
    for case, hy_value in zip(cases, hy_values):
        python_value = rt.eval_source(case["source"])
        checks.append(
            {
                "name": case["name"],
                "source": case["source"],
                "python_value": python_value,
                "hy_value": hy_value,
                "python_hash": rt.term_hash(python_value),
                "hy_hash": rt.term_hash(hy_value),
                "ok": rt.stable_data(python_value) == rt.stable_data(hy_value),
            }
        )
    return {
        "schema": "pnix-hy.hy-runtime.parity.v0",
        "runtime": rt.HY_RUNTIME_SCHEMA,
        "count": len(checks),
        "ready": len(checks) == len(cases) and bool(checks) and all(check["ok"] for check in checks),
        "cases": checks,
    }


def hy_source_runtime_batch(cases: list[dict[str, Any]] | None = None) -> dict[str, Any]:
    cases = cases or rt.HY_RUNTIME_CORPUS
    sources = [case["source"] for case in cases]
    hy_values = hy_mirror.stage7_eval_json(rt.hy_runtime_source_for_sources(sources))
    checks = []
    for case, hy_value in zip(cases, hy_values):
        python_value = rt.eval_source(case["source"])
        checks.append(
            {
                "name": case["name"],
                "source": case["source"],
                "python_value": python_value,
                "hy_value": hy_value,
                "python_hash": rt.term_hash(python_value),
                "hy_hash": rt.term_hash(hy_value),
                "ok": rt.stable_data(python_value) == rt.stable_data(hy_value),
            }
        )
    return {
        "schema": "pnix-hy.hy-source-runtime.parity.v0",
        "runtime": rt.HY_RUNTIME_SCHEMA,
        "count": len(checks),
        "ready": len(checks) == len(cases) and bool(checks) and all(check["ok"] for check in checks),
        "cases": checks,
    }


def hy_compiler_batch(cases: list[dict[str, Any]] | None = None) -> dict[str, Any]:
    """Parity of the pnix->Python compiler lane (compile+exec inside stage7)
    against the Python interpreter, on the runtime corpus."""
    cases = cases or rt.HY_RUNTIME_CORPUS
    asts = [rt.parse(case["source"]) for case in cases]
    hy_values = hy_mirror.stage7_eval_json(rt.hy_compiler_source_for_asts(asts))
    checks = []
    for case, hy_value in zip(cases, hy_values):
        python_value = rt.eval_source(case["source"])
        checks.append(
            {
                "name": case["name"],
                "source": case["source"],
                "python_value": python_value,
                "hy_value": hy_value,
                "python_hash": rt.term_hash(python_value),
                "hy_hash": rt.term_hash(hy_value),
                "ok": rt.stable_data(python_value) == rt.stable_data(hy_value),
            }
        )
    return {
        "schema": "pnix-hy.hy-compiler.parity.v0",
        "runtime": rt.HY_RUNTIME_SCHEMA,
        "count": len(checks),
        "ready": len(checks) == len(cases) and bool(checks) and all(check["ok"] for check in checks),
        "cases": checks,
    }


def hy_compiler_source_batch(cases: list[dict[str, Any]] | None = None) -> dict[str, Any]:
    """Source-lane parity: stage7 tokenizes, parses, compiles and execs straight
    from pnix source strings (Python never parses), matched against the Python
    interpreter on the runtime corpus."""
    cases = cases or rt.HY_RUNTIME_CORPUS
    sources = [case["source"] for case in cases]
    hy_values = hy_mirror.stage7_eval_json(rt.hy_compiler_source_for_sources(sources))
    checks = []
    for case, hy_value in zip(cases, hy_values):
        python_value = rt.eval_source(case["source"])
        checks.append(
            {
                "name": case["name"],
                "source": case["source"],
                "python_value": python_value,
                "hy_value": hy_value,
                "python_hash": rt.term_hash(python_value),
                "hy_hash": rt.term_hash(hy_value),
                "ok": rt.stable_data(python_value) == rt.stable_data(hy_value),
            }
        )
    return {
        "schema": "pnix-hy.hy-compiler-source.parity.v0",
        "runtime": rt.HY_RUNTIME_SCHEMA,
        "count": len(checks),
        "ready": len(checks) == len(cases) and bool(checks) and all(check["ok"] for check in checks),
        "cases": checks,
    }


COMPILER_EMIT_SHAPE_CASES: list[dict[str, Any]] = [
    {"name": "fold-arith", "source": "1 + 2 * 3"},
    {"name": "fold-if", "source": "if true then 1 + 2 else 9"},
    {"name": "fold-let", "source": "let x = 1 + 2; in x + 4"},
    {"name": "fold-builtin", "source": "builtins.add 1 2"},
    {"name": "lazy-lambda-shape", "source": "x: x + 1"},
    {"name": "attrset-child-fold", "source": "{ a = 1 + 2; b = \"x\"; }"},
]


def compiler_emit_shape_report(cases: list[dict[str, Any]] | None = None) -> dict[str, Any]:
    """Shape parity for the host Python emitter and the stage7 Hy emitter.

    Value parity alone misses whether the stage7 self-host compiler has the same
    strictness and partial-eval surface as the host fast path. This report compares
    emitted Python expressions directly on a small representative set.
    """
    cases = cases or COMPILER_EMIT_SHAPE_CASES
    asts = [rt.parse(case["source"]) for case in cases]
    host_exprs = [_compiler_expr_for_perf(ast, case["source"]) for ast, case in zip(asts, cases)]
    stage7_exprs = hy_mirror.stage7_eval_json(rt.hy_compiler_emit_for_asts(asts))
    checks = []
    for case, host_expr, stage7_expr in zip(cases, host_exprs, stage7_exprs):
        checks.append(
            {
                "name": case["name"],
                "source": case["source"],
                "host_expr": host_expr,
                "stage7_expr": stage7_expr,
                "ok": host_expr == stage7_expr,
            }
        )
    return {
        "schema": "pnix-hy.compiler-emit-shape.v0",
        "count": len(checks),
        "ready": len(checks) == len(cases) and bool(checks) and all(check["ok"] for check in checks),
        "cases": checks,
    }


def self_test_report(include_hy_host: bool = True) -> dict[str, Any]:
    runtime = rt.self_test_report()
    simple = run_mirror("rec { x = 1; y = x + 41; }.y", include_hy_host=include_hy_host)
    lambda_case = run_mirror("let f = x: x + 1; in f 41", include_hy_host=False)
    tower = stage_tower('let name = "world"; in "hi ${name}!"')
    cases = [
        {
            "name": "runtime-self-test",
            "ok": runtime["ready"],
        },
        {
            "name": "mirror-chain-converges-7",
            "ok": simple["ready"] and simple["value"] == 42 and simple["mirror_stage_count"] == 7,
        },
        {
            "name": "lambda-mirror-converges",
            "ok": lambda_case["ready"] and lambda_case["value"] == 42,
        },
        {
            "name": "stage-tower-equivalent",
            "ok": tower["equivalent"] and tower["value"] == "hi world!",
        },
    ]
    runtime_parity: dict[str, Any] | None = None
    source_parity: dict[str, Any] | None = None
    compiler_parity: dict[str, Any] | None = None
    compiler_source_parity: dict[str, Any] | None = None
    closure: dict[str, Any] | None = None
    if include_hy_host:
        runtime_parity = hy_runtime_batch()
        source_parity = hy_source_runtime_batch()
        compiler_parity = hy_compiler_batch()
        compiler_source_parity = hy_compiler_source_batch()
        closure = hy_mirror.closure_probe()
        cases.append(
            {
                "name": "hy-meta-kernel-eval-probe",
                "ok": simple["hy_host"]["kernel_probe"]["ready"],
            }
        )
        cases.append(
            {
                "name": "hy-meta-closure-stage15n",
                "ok": closure["ready"],
                "stages": closure["stages"],
                "status": closure["status"],
            }
        )
        cases.append(
            {
                "name": "hy-runtime-parity",
                "ok": runtime_parity["ready"],
                "count": runtime_parity["count"],
            }
        )
        cases.append(
            {
                "name": "hy-source-runtime-parity",
                "ok": source_parity["ready"],
                "count": source_parity["count"],
            }
        )
        cases.append(
            {
                "name": "hy-compiler-parity",
                "ok": compiler_parity["ready"],
                "count": compiler_parity["count"],
            }
        )
        cases.append(
            {
                "name": "hy-compiler-source-parity",
                "ok": compiler_source_parity["ready"],
                "count": compiler_source_parity["count"],
            }
        )
    return {
        "schema": MIRROR_SELF_TEST_SCHEMA,
        "ready": all(case["ok"] for case in cases),
        "runtime": runtime,
        "mirror": simple,
        "lambda_mirror": lambda_case,
        "tower": tower,
        "runtime_parity": runtime_parity,
        "source_parity": source_parity,
        "compiler_parity": compiler_parity,
        "compiler_source_parity": compiler_source_parity,
        "closure": closure,
        "cases": cases,
    }


# ---------------------------------------------------------------------------
# Entry point B: pnix mirror -- introspection FROM a pnix statement.
#
# Same .px source that entry point A runs, but here we expose what the compiler
# produced and how it behaves: the pnix AST, the generated Python, the compiled
# value vs the interpreter value, and full host-direct CPython introspection of
# the generated code. Fast (no stage7); stage7 is only the parity/spec lane.
# ---------------------------------------------------------------------------


def _emit_full(node: dict[str, Any]) -> str:
    """Emit WITHOUT partial-evaluation folding, so every grammar node survives in
    the generated code and is introspectable (meta-circular coverage)."""
    rt._PX_CTR[0] = 0
    rt._PX_FOLD[0] = False
    try:
        return rt._px_emit(node, {"builtins": "v_builtins"})
    finally:
        rt._PX_FOLD[0] = True


def introspect_px(source: str) -> dict[str, Any]:
    """Introspect a pnix statement: AST, the FULL generated Python (un-folded, so
    every node is inspectable), the folded Python entry A actually runs,
    compiled-vs-interpreter value agreement, and host-direct CPython
    introspection of the full emitted code."""
    node = rt.parse(source)
    full_expr = _emit_full(node)
    rt._PX_CTR[0] = 0
    folded_expr = rt._px_emit(node, {"builtins": "v_builtins"})  # what entry A runs
    interp_value = rt.eval_source(source)
    compiled_value = rt.run_px_source(source)
    return {
        "schema": "pnix-hy.introspect-px.v0",
        "source": source,
        "pnix_ast": node,
        "generated_python": full_expr,
        "folded_python": folded_expr,
        "partial_evaluated": folded_expr != full_expr,
        "interp_value": interp_value,
        "compiled_value": compiled_value,
        "values_agree": rt.stable_data(interp_value) == rt.stable_data(compiled_value),
        "host_introspection": hy_mirror.full_introspection(full_expr, "eval"),
    }


def introspect_px_file(path: str) -> dict[str, Any]:
    """Entry point B for a `.px` file."""
    with open(path, "r", encoding="utf-8") as handle:
        return introspect_px(handle.read())


def introspect_px_parity(source: str) -> dict[str, Any]:
    """Spec/verification lane: cross-check host vs stage7 CPython introspection
    of the full (un-folded) Python generated for a pnix statement."""
    return hy_mirror.introspection_parity(_emit_full(rt.parse(source)), "eval")


def _compiler_expr_for_perf(ast: dict[str, Any], source: str) -> str:
    rt._PX_CTR[0] = 0
    emit_ctx = rt.runtime_context({"source_text": source})
    emit_ctx["source_text"] = source
    rt._PX_EMIT_CTX[0] = emit_ctx
    return rt._px_emit(ast, rt._px_initial_env(emit_ctx.get("compiler_env_names", {})))


def _time_iterations(iterations: int, fn: Any) -> dict[str, Any]:
    n = max(1, int(iterations))
    start = _time.perf_counter_ns()
    last = None
    for _ in range(n):
        last = fn()
    total = _time.perf_counter_ns() - start
    last_data = rt.stable_data(last)
    try:
        _json.dumps(last_data, ensure_ascii=False)
    except TypeError:
        last_data = repr(last)
    return {
        "iterations": n,
        "total_ns": total,
        "per_iter_ns": total // n,
        "last": last_data,
    }


def _exec_compiled_module_for_perf(code: Any, source: str) -> Any:
    ctx = rt.runtime_context(None)
    namespace: dict[str, Any] = {}
    namespace["_PX_IMPORT"] = lambda path: rt.import_value(path, ctx, "run", namespace)
    namespace["_PX_SCOPED_IMPORT"] = lambda scope, path: rt.scoped_import_value(scope, path, ctx, "run", namespace)
    namespace["_PX_SOURCE_PATH"] = "<pnix-perf>"
    namespace["_PX_SOURCE_TEXT"] = source
    namespace["_PX_BASE_DIR"] = str(ctx.get("base_dir"))
    exec(code, namespace)  # noqa: S102 - benchmark executes the compiler output under test.
    return namespace["_RESULT"]


def performance_report(
    source: str = "let xs = [ 1 2 3 4 5 ]; in builtins.foldl' (acc: x: acc + x) 0 xs",
    *,
    iterations: int = 10,
    include_stage7: bool = False,
) -> dict[str, Any]:
    """Benchmark pnix's shared runtime lanes without process-startup noise.

    Separates parse, canonical emit, compiler emit, Python compile, interpreter
    eval, compiler compile+exec, and compiler compile-once/exec-many. Stage7 uses
    the existing four-lane projection only when requested because kernel boot is slow.
    """
    result: dict[str, Any] = {
        "schema": "pnix-hy.performance-report.v0",
        "source": source,
        "iterations": max(1, int(iterations)),
        "stage7_included": include_stage7,
    }
    try:
        ast = rt.parse(source)
        canonical = rt.emit_source(ast)
        compiler_expr = _compiler_expr_for_perf(ast, source)
        module_source = rt.COMPILER_PRELUDE + "\n_RESULT=_realize(" + compiler_expr + ")\n"
        code = compile(module_source, "<pnix-perf>", "exec")

        timings = {
            "parse": _time_iterations(iterations, lambda: rt.parse(source)),
            "canonical_emit": _time_iterations(iterations, lambda: rt.emit_source(ast)),
            "compiler_emit": _time_iterations(iterations, lambda: _compiler_expr_for_perf(ast, source)),
            "python_compile": _time_iterations(iterations, lambda: compile(module_source, "<pnix-perf>", "exec")),
            "interpreter_eval": _time_iterations(iterations, lambda: rt.eval_source(source)),
            "compiler_compile_exec": _time_iterations(iterations, lambda: rt.run_px_source(source)),
            "compiler_exec_many": _time_iterations(iterations, lambda: _exec_compiled_module_for_perf(code, source)),
        }

        interp_value = timings["interpreter_eval"]["last"]
        compiled_value = timings["compiler_compile_exec"]["last"]
        exec_many_value = timings["compiler_exec_many"]["last"]
        values_agree = interp_value == compiled_value == exec_many_value

        introspection = hy_mirror.full_introspection(module_source, "exec")
        bytecode = introspection.get("bytecode") or []
        code_object = introspection.get("code_object") or {}
        result.update(
            ready=values_agree and bool(bytecode),
            canonical_source=canonical,
            generated_python_bytes=len(module_source.encode("utf-8")),
            generated_python_sha256=_sha256(module_source),
            bytecode_op_count=len(bytecode),
            bytecode_code_len=code_object.get("co_code_len"),
            timings=timings,
            values={
                "interpreter": interp_value,
                "compiler_compile_exec": compiled_value,
                "compiler_exec_many": exec_many_value,
                "agree": values_agree,
            },
            original_pnixc_meta={
                "available": bool(rt.original_pnixc_meta_path()),
                "timed": False,
                "reason": "pnixc-meta is only exposed as an external process here; startup is excluded by this harness",
            },
            stage7={
                "included": False,
                "reason": "set include_stage7=True to run the slow existing four-lane projection",
            },
        )
        if include_stage7:
            stage_timing = _time_iterations(1, lambda: pnix_meta_circular_projection(source))
            stage_value = (stage_timing["last"].get("values") or {}).get("host_interpreter")
            result["stage7"] = {
                "included": True,
                "timing": stage_timing,
                "value": stage_value,
                "agrees_with_host": stage_value == interp_value,
            }
            result["ready"] = bool(result["ready"]) and bool(result["stage7"]["agrees_with_host"])
        return result
    except Exception as exc:  # noqa: BLE001 - report benchmark setup failures.
        result.update(ready=False, error=f"{type(exc).__name__}: {exc}")
        return result


def performance_report_check() -> dict[str, Any]:
    report = performance_report("let xs = [ 1 2 3 ]; in builtins.foldl' (acc: x: acc + x) 0 xs", iterations=1)
    return {
        "schema": "pnix-hy.performance-report.check.v0",
        "ready": bool(report.get("ready")) and bool((report.get("values") or {}).get("agree")),
        "report": report,
    }


# ============================================================================
# pnix form projection (the pnix-side mirror of hy_mirror.hy_form_projection)
#
# Mission: project language expressiveness between the Python-ecosystem meta-circular
# and the pnix meta-circular. hy_mirror.hy_form_projection surfaces a Hy fragment's
# reader-form tree + the Python it lowers to. This is the symmetric pnix front: a pnix
# fragment's parsed AST (the tagged node tree: binary/int/attrset/lambda/let/var/...),
# its canonical re-emission (pnix self-hosting: AST -> source round-trip), and the value
# it evaluates to on BOTH host lanes (interp + compiler). Together the two projections
# put the two meta-circular substrates side by side in the same shape.
# ============================================================================

PNIX_FORM_PROJECTION_SCHEMA = "pnix-hy.pnix-form-projection.v0"

_PNIX_POS_KEYS = frozenset({"pos", "path_positions", "positions", "name_pos"})


def _strip_pos(node: Any) -> Any:
    """Drop source-position metadata so the AST node tree reads as pure structure
    (same canonicalization ast_hash uses)."""
    if isinstance(node, dict):
        return {k: _strip_pos(v) for k, v in node.items() if k not in _PNIX_POS_KEYS}
    if isinstance(node, list):
        return [_strip_pos(x) for x in node]
    return node


def pnix_form_projection(source: str) -> dict[str, Any]:
    """Project a pnix source fragment: its parsed AST node tree, canonical re-emission,
    and evaluated value on both host lanes. The pnix-side mirror of
    `hy_mirror.hy_form_projection`. Schema `pnix-hy.pnix-form-projection.v0`."""
    ast = rt.parse(source)
    raw_value = rt.eval_source(source)
    interp_value = rt.stable_data(raw_value)
    try:
        compiler_value: Any = rt.stable_data(rt.run_px_source(source))
        compiler_error = None
    except Exception as exc:  # noqa: BLE001 - report, don't crash the projection
        compiler_value = None
        compiler_error = f"{type(exc).__name__}: {exc}"
    try:
        # JSON from the already-computed value (no re-parse/re-eval of the source).
        value_json: Any = rt.to_json_string_value(raw_value)
    except Exception:  # noqa: BLE001 - JSON is best-effort (non-serializable values)
        value_json = None
    return {
        "schema": PNIX_FORM_PROJECTION_SCHEMA,
        "source": source,
        "ast": _strip_pos(ast),
        "ast_root_tag": ast.get("tag") if isinstance(ast, dict) else None,
        "ast_hash": rt.ast_hash(ast),
        "emitted_source": rt.emit_source(ast),
        "value": interp_value,
        "value_json": value_json,
        "compiler_value": compiler_value,
        "compiler_error": compiler_error,
        "lanes_agree": compiler_error is None and interp_value == compiler_value,
    }


def pnix_form_projection_report() -> dict[str, Any]:
    """Self-check: a representative pnix expression parses to the expected tagged AST,
    both lanes agree on the value, and the canonical re-emission round-trips (same
    ast_hash)."""
    probe = "let inc = x: x + 1; in inc 41"
    proj = pnix_form_projection(probe)
    reparsed_hash = rt.ast_hash(rt.parse(proj["emitted_source"]))
    ready = (
        proj["ast_root_tag"] == "let"
        and proj["value"] == 42
        and proj["lanes_agree"] is True
        and reparsed_hash == proj["ast_hash"]
    )
    return {
        "schema": "pnix-hy.pnix-form-projection.report.v0",
        "ready": ready,
        "available": True,
        "probe": probe,
        "ast_root_tag": proj["ast_root_tag"],
        "value": proj["value"],
        "lanes_agree": proj["lanes_agree"],
        "emit_roundtrip_ok": reparsed_hash == proj["ast_hash"],
    }


# ============================================================================
# Structural correspondence table (#2): Python/Hy AST <-> pnix AST/value taxonomy
#
# The curated map between the two meta-circular substrates' node + value taxonomies.
# Each row pairs a Python AST node class and the Hy reader form/macro that lowers to it
# with the pnix AST `tag` (from rt.parse) and the pnix runtime value type (from
# rt._type_of) it corresponds to. `differs` flags rows where the projection is NOT a
# clean 1:1 — the interesting research content (e.g. pnix `with` is dynamic-scope
# injection, NOT Python's context-manager `with`; pnix `path` is a native type with no
# Python/Hy literal). pnix tags are verified live against rt.parse so the table cannot
# silently drift from the runtime.
# ============================================================================

CORRESPONDENCE_SCHEMA = "pnix-hy.meta-circular-correspondence.v0"

# Sentinel pnix_tag for host constructs with NO direct pnix form (exceptions, classes,
# loops, comprehensions, generators/async). They are recorded with the closest analogue
# in the `note`; correspondence_report does not live-probe these (there is nothing to
# parse), it only requires they carry an explanatory note.
_NO_PNIX_TAG = "(no direct pnix form)"

# (category, python_ast, hy_form, pnix_tag, pnix_value_type, differs, note)
_CORRESPONDENCE: tuple[tuple[str, str, str, str, str | None, bool, str], ...] = (
    ("integer literal", "Constant(int)", "Integer", "int", "int", False,
     "i64 with checked overflow in pnix; Python int is arbitrary-precision"),
    ("float literal", "Constant(float)", "Float", "float", "float", False, ""),
    ("string literal", "Constant(str)", "String", "string", "string", False,
     "pnix strings additionally carry build string-context"),
    ("string interpolation", "JoinedStr/FormattedValue", "FString/FComponent", "str_interp", "string", False,
     "pnix ${} coerces via __toString; Python f-strings call __format__"),
    ("boolean literal", "Constant(bool)", "Symbol True/False", "bool", "bool", False, ""),
    ("null / none", "Constant(None)", "Symbol None", "null", "null", False, ""),
    ("list / vector", "List", "List [ ... ]", "list", "list", False,
     "pnix lists are lazy (thunked elements); Python lists are strict"),
    ("map / attrset", "Dict", "Dict { ... }", "attrset", "set", True,
     "pnix attrset keys are identifiers/strings only and values are lazy; "
     "Hy/Python dict keys are arbitrary hashables"),
    ("function abstraction", "Lambda / FunctionDef", "fn / defn", "lambda", "lambda", True,
     "pnix lambdas are single-param curried + attrset/list patterns; "
     "Python functions are multi-arg with *args/**kwargs/defaults"),
    ("application / call", "Call", "(f x)", "apply", None, True,
     "pnix application is curried juxtaposition; Python Call is n-ary with keywords"),
    ("binary operator", "BinOp / BoolOp / Compare", "(+ a b) (and ..) (< ..)", "binary", None, False,
     "pnix + is polymorphic (num/str/list/path/attrset); && || -> are the bool ops"),
    ("unary operator", "UnaryOp", "(- x) / (not x)", "unary", None, False, ""),
    ("conditional", "If / IfExp", "if / when / cond", "if", None, False,
     "pnix if is an expression (always has else); Hy cond -> nested IfExp"),
    ("local binding", "Assign (+ scope) / NamedExpr", "setv / let", "let", None, True,
     "pnix let is recursive lazy bindings in one scope; Hy let -> gensym'd assigns"),
    ("name reference", "Name", "Symbol", "var", None, False, ""),
    ("attribute / index access", "Attribute / Subscript", "(. o a) / (get o k)", "select", None, True,
     "pnix .a selects attrset members (with `or` default); Subscript is index/key"),
    ("membership test", "Compare(In)", "(in k coll)", "has_attr", "bool", True,
     "pnix `?` tests attrset key presence (no force of the value); `in` is general"),
    ("assertion", "Assert", "(assert c)", "assert", None, False,
     "pnix assert is an expression `assert c; body`; Python assert is a statement"),
    ("dynamic scope / namespace", "ImportFrom(*) (closest)", "(require) (closest)", "with", None, True,
     "pnix `with attrs; body` injects attrset keys into dynamic scope -- NOT Python's "
     "context-manager `with` (no __enter__/__exit__ correspondence)"),
    ("module import", "Import / ImportFrom", "(import ...) / require", "import", None, True,
     "pnix import evaluates a .px file to a value; Python import binds a module object"),
    ("pattern match", "Match / match_case (3.10+)", "(match ...) (cond-ish)", "match", None, True,
     "pnix match has guard arms `| pat if c => body`; Python match is statement-form"),
    ("filesystem path", "(no literal; pathlib.Path)", "(no literal)", "path", "path", True,
     "pnix has a native path LITERAL + type (./x, /abs, <search>); Python/Hy have none"),
    # --- host constructs pnix-hy "should mirror" (design directive) that have NO direct
    # pnix form: recorded honestly with the closest pnix analogue, pnix_tag = sentinel.
    ("exception handling", "Try / ExceptHandler / Raise", "(try ...) / (raise)", _NO_PNIX_TAG, None, True,
     "pnix is pure & exception-free; builtins.tryEval / throw / abort are the closest (recoverable vs hard-fail)"),
    ("context manager", "With / AsyncWith", "(with [mgr] ...)", _NO_PNIX_TAG, None, True,
     "Python context-manager with (__enter__/__exit__) has no pnix equivalent -- and is NOT pnix `with` (dynamic scope)"),
    ("class definition", "ClassDef", "(defclass ...)", _NO_PNIX_TAG, None, True,
     "pnix has no classes/OO; an attrset of values+functions (rec { }) is the data analogue"),
    ("list/set comprehension", "ListComp / SetComp / GeneratorExp", "(lfor ...) / (gfor ...)", _NO_PNIX_TAG, None, True,
     "pnix expresses these via builtins.genList / map + filter / concatMap (no comprehension syntax)"),
    ("dict comprehension", "DictComp", "(dfor ...)", _NO_PNIX_TAG, None, True,
     "pnix: builtins.listToAttrs over a mapped list of {name=..; value=..;}"),
    ("statement loop", "For / AsyncFor / While", "(for ...) / (while ...)", _NO_PNIX_TAG, None, True,
     "pnix is expression/recursion-based; iteration is fold / genList / recursion, no statement loop"),
    ("generator / async", "Yield / YieldFrom / Await", "(yield) / (await)", _NO_PNIX_TAG, None, True,
     "pnix has no generators/coroutines/async; laziness (thunks) is the closest evaluation-deferral"),
)


def correspondence_table() -> dict[str, Any]:
    """The curated Python/Hy AST <-> pnix AST/value correspondence, as structured rows.
    Schema `pnix-hy.meta-circular-correspondence.v0`."""
    rows = [
        {
            "category": cat,
            "python_ast": py,
            "hy_form": hy,
            "pnix_tag": tag,
            "pnix_value_type": vtype,
            "differs": differs,
            "note": note,
        }
        for (cat, py, hy, tag, vtype, differs, note) in _CORRESPONDENCE
    ]
    return {
        "schema": CORRESPONDENCE_SCHEMA,
        "row_count": len(rows),
        "clean_1to1": sum(1 for r in rows if not r["differs"]),
        "differs": sum(1 for r in rows if r["differs"]),
        "rows": rows,
    }


CORRESPONDENCE_ABI_VERSION = "pnix-hy.correspondence-abi.v0"


def correspondence_abi() -> dict[str, Any]:
    """D3: the correspondence table as a content-hashed, versioned ABI artifact for downstream
    pinning + drift detection. Re-emits `correspondence_table()` with normalized rows
    (source_node/hy_form/pnix_tag/value_type/loss/supported) and a stable sha256 over them; a
    future lane (pnix-hs/pnix-rs) can pin `abi_sha256` and detect table drift. Does NOT replace
    `correspondence_table` or its `_TAG_PROBES` drift-guard."""
    table = correspondence_table()
    rows = [
        {
            "source_node": r["python_ast"],
            "hy_form": r["hy_form"],
            "pnix_tag": r["pnix_tag"],
            "value_type": r["pnix_value_type"],
            "loss": "lossy" if r["differs"] else "lossless",
            "supported": r["pnix_tag"] != _NO_PNIX_TAG,
        }
        for r in table["rows"]
    ]
    digest = _sha256(_json.dumps(rows, sort_keys=True))
    return {
        "schema": "pnix-hy.correspondence-abi.report.v0",
        "abi_version": CORRESPONDENCE_ABI_VERSION,
        "abi_sha256": digest,
        "row_count": len(rows),
        "supported": sum(1 for r in rows if r["supported"]),
        "rows": rows,
    }


def correspondence_abi_report() -> dict[str, Any]:
    """Self-check: the correspondence ABI is deterministic (stable hash), matches the live table
    row count, and every row carries a valid loss + a supported flag."""
    try:
        a = correspondence_abi()
        b = correspondence_abi()
        table = correspondence_table()
        loss_ok = all(r["loss"] in ("lossless", "lossy") for r in a["rows"])
        ready = (a["abi_sha256"] == b["abi_sha256"] and len(a["abi_sha256"]) == 64
                 and a["row_count"] == table["row_count"] and a["row_count"] > 0 and loss_ok
                 and all("supported" in r for r in a["rows"]))
        return {"schema": "pnix-hy.correspondence-abi.self.v0", "ready": bool(ready), "available": True,
                "abi_version": a["abi_version"], "abi_sha256": a["abi_sha256"],
                "row_count": a["row_count"], "supported": a["supported"]}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.correspondence-abi.self.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


# Live probes that must parse to each table row's pnix_tag (grounds the table in the
# real rt.parse output -- the table fails its report if pnix's tag taxonomy drifts).
_TAG_PROBES: dict[str, str] = {
    "int": "1", "float": "1.5", "string": '"x"', "str_interp": '"${a}"',
    "bool": "true", "null": "null", "list": "[ 1 2 ]", "attrset": "{ a = 1; }",
    "lambda": "(x: x)", "apply": "f x", "binary": "1 + 2", "unary": "-1",
    "if": "if true then 1 else 2", "let": "let a = 1; in a", "var": "a",
    "select": "{ a = 1; }.a", "has_attr": "{ a = 1; } ? a", "assert": "assert true; 1",
    "with": "with { }; 1", "import": "import ./x", "match": "match 1 with | _ => 1",
    "path": "./x",
}


def correspondence_report() -> dict[str, Any]:
    """Validate the correspondence table against the live runtime: every pnix_tag a row
    references must actually be produced by rt.parse for a representative probe, and
    every pnix_value_type must be a real rt._type_of name. Catches table drift."""
    table = correspondence_table()
    real_value_types = {"int", "float", "string", "bool", "null", "list", "set", "lambda", "path"}
    tag_mismatches: list[dict[str, str]] = []
    type_mismatches: list[dict[str, str]] = []
    note_mismatches: list[dict[str, str]] = []
    for row in table["rows"]:
        tag = row["pnix_tag"]
        if tag == _NO_PNIX_TAG:
            # intentionally non-pnix: nothing to parse; just require an analogue note.
            if not (row.get("note") or "").strip():
                note_mismatches.append({"category": row["category"], "reason": "no analogue note"})
            continue
        probe = _TAG_PROBES.get(tag)
        if probe is None:
            tag_mismatches.append({"pnix_tag": tag, "reason": "no probe"})
        else:
            parsed = rt.parse(probe)
            got = parsed.get("tag") if isinstance(parsed, dict) else None
            if got != tag:
                tag_mismatches.append({"pnix_tag": tag, "probe": probe, "parsed_tag": str(got)})
        vtype = row["pnix_value_type"]
        if vtype is not None and vtype not in real_value_types:
            type_mismatches.append({"pnix_value_type": vtype, "category": row["category"]})
    ready = not tag_mismatches and not type_mismatches and not note_mismatches
    return {
        "schema": "pnix-hy.meta-circular-correspondence.report.v0",
        "ready": ready,
        "available": True,
        "row_count": table["row_count"],
        "clean_1to1": table["clean_1to1"],
        "differs": table["differs"],
        "tag_mismatches": tag_mismatches,
        "type_mismatches": type_mismatches,
        "note_mismatches": note_mismatches,
        "no_pnix_form_rows": sum(1 for r in table["rows"] if r["pnix_tag"] == _NO_PNIX_TAG),
    }


# ============================================================================
# Hy/Python AST -> pnix term aligner (#next)
#
# Uses the correspondence table to actually LABEL a lowered Python AST: project a Hy
# fragment to Python (hy_form_projection), parse that Python in-process, walk it, and
# tag every node with the pnix construct it corresponds to (+ whether it `differs`).
# The concrete realization of the projection: "this Hy form, once lowered, is built from
# these Python constructs, each corresponding to these pnix terms."
# ============================================================================

import ast as _pyast

ALIGNMENT_SCHEMA = "pnix-hy.hy-pnix-alignment.v0"

# Actual Python ast class name -> pnix AST tag. Grounded in _CORRESPONDENCE; Constant
# and Compare are resolved by inspecting the node (polymorphic), handled in _node_tag.
_PY_CLASS_TO_TAG: dict[str, str] = {
    "List": "list", "Tuple": "list", "Set": "list",
    "Dict": "attrset",
    "Lambda": "lambda", "FunctionDef": "lambda", "AsyncFunctionDef": "lambda",
    "Call": "apply",
    "BinOp": "binary", "BoolOp": "binary",
    "UnaryOp": "unary",
    "If": "if", "IfExp": "if",
    "Assign": "let", "NamedExpr": "let", "AnnAssign": "let",
    "Name": "var",
    "Attribute": "select", "Subscript": "select",
    "Assert": "assert",
    "Import": "import", "ImportFrom": "import",
    "JoinedStr": "str_interp", "FormattedValue": "str_interp",
    "Match": "match",
    # host constructs with no direct pnix form (recognized, not "unmapped")
    "Try": _NO_PNIX_TAG, "ExceptHandler": _NO_PNIX_TAG, "Raise": _NO_PNIX_TAG,
    "With": _NO_PNIX_TAG, "AsyncWith": _NO_PNIX_TAG,
    "ClassDef": _NO_PNIX_TAG,
    "ListComp": _NO_PNIX_TAG, "SetComp": _NO_PNIX_TAG, "GeneratorExp": _NO_PNIX_TAG,
    "DictComp": _NO_PNIX_TAG,
    "For": _NO_PNIX_TAG, "AsyncFor": _NO_PNIX_TAG, "While": _NO_PNIX_TAG,
    "Yield": _NO_PNIX_TAG, "YieldFrom": _NO_PNIX_TAG, "Await": _NO_PNIX_TAG,
}


def _node_tag(node: Any) -> str | None:
    cls = type(node).__name__
    if cls == "Constant":
        v = node.value
        if isinstance(v, bool):
            return "bool"
        if isinstance(v, int):
            return "int"
        if isinstance(v, float):
            return "float"
        if isinstance(v, str):
            return "string"
        if v is None:
            return "null"
        return None
    if cls == "Compare":
        if any(isinstance(op, (_pyast.In, _pyast.NotIn)) for op in node.ops):
            return "has_attr"
        return "binary"
    return _PY_CLASS_TO_TAG.get(cls)


def _tag_rows() -> dict[str, dict[str, Any]]:
    rows: dict[str, dict[str, Any]] = {}
    for row in correspondence_table()["rows"]:
        rows.setdefault(row["pnix_tag"], row)
    return rows


def _strip_hy_scaffolding(tree: Any) -> Any:
    """Drop the leading `import hy` / `import hy.*` statements the Hy compiler injects, so
    they don't show up as a spurious `Import -> import` node in alignments."""
    if not isinstance(tree, _pyast.Module):
        return tree
    kept = []
    for stmt in tree.body:
        if isinstance(stmt, _pyast.Import) and stmt.names and all(
            a.name == "hy" or a.name.startswith("hy.") for a in stmt.names
        ):
            continue
        if isinstance(stmt, _pyast.ImportFrom) and (
            (stmt.module or "") == "hy" or (stmt.module or "").startswith("hy.")
        ):
            continue
        kept.append(stmt)
    tree.body = kept
    return tree


def align_python_to_pnix(python_source: str) -> dict[str, Any]:
    """Parse Python source and label every node that has a pnix correspondence. Returns
    a flat list of aligned nodes + a summary. Schema `pnix-hy.hy-pnix-alignment.v0`."""
    tree = _strip_hy_scaffolding(_pyast.parse(python_source))
    rows = _tag_rows()
    aligned: list[dict[str, Any]] = []
    unmapped: dict[str, int] = {}
    for node in _pyast.walk(tree):
        tag = _node_tag(node)
        cls = type(node).__name__
        if tag is None:
            # skip pure-scaffolding nodes (Module, Load, arguments, ...) silently;
            # only count "expression/statement-ish" ones as unmapped for visibility
            if cls not in _SCAFFOLD_NODES:
                unmapped[cls] = unmapped.get(cls, 0) + 1
            continue
        row = rows.get(tag, {})
        aligned.append({
            "python_node": cls,
            "pnix_tag": tag,
            "pnix_value_type": row.get("pnix_value_type"),
            "differs": bool(row.get("differs")),
            "lineno": getattr(node, "lineno", None),
        })
    tags_hit = sorted({a["pnix_tag"] for a in aligned})
    return {
        "schema": ALIGNMENT_SCHEMA,
        "python_source": python_source,
        "aligned_count": len(aligned),
        "aligned_nodes": aligned,
        "pnix_tags_hit": tags_hit,
        "differs_tags": sorted({a["pnix_tag"] for a in aligned if a["differs"]}),
        "unmapped_nodes": unmapped,
    }


_SCAFFOLD_NODES = frozenset({
    "Module", "Expr", "Load", "Store", "Del", "arguments", "arg", "keyword",
    "Return", "alias", "withitem", "comprehension", "Starred",
    "And", "Or", "Add", "Sub", "Mult", "Div", "Mod", "Pow", "Not", "USub", "UAdd",
    "Eq", "NotEq", "Lt", "LtE", "Gt", "GtE", "In", "NotIn", "Is", "IsNot",
    # trivial no-op / control statements with no data-level pnix correspondence
    "Pass", "Break", "Continue", "Global", "Nonlocal",
})


def align_hy_to_pnix(hy_source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Project a Hy fragment to Python, then label the lowered Python AST with its pnix
    correspondences -- the concrete Hy form -> pnix term alignment."""
    front = hy_mirror.hy_form_projection(hy_source, timeout=timeout)
    python_source = front.get("python_source", "")
    alignment = align_python_to_pnix(python_source)
    return {
        "schema": "pnix-hy.hy-to-pnix-alignment.v0",
        "hy_source": hy_source,
        "reader_forms": front.get("reader_forms"),
        "python_source": python_source,
        "alignment": alignment,
    }


def alignment_report() -> dict[str, Any]:
    """Self-check: a defn+arith Hy form aligns to the expected pnix tags."""
    try:
        out = align_hy_to_pnix("(defn f [x] (+ x 1))")
    except hy_mirror.HyMirrorError as exc:
        return {"schema": "pnix-hy.hy-pnix-alignment.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    hit = set(out["alignment"]["pnix_tags_hit"])
    ready = {"lambda", "binary", "var", "int"} <= hit
    return {
        "schema": "pnix-hy.hy-pnix-alignment.report.v0",
        "ready": ready,
        "available": True,
        "probe": "(defn f [x] (+ x 1))",
        "pnix_tags_hit": out["alignment"]["pnix_tags_hit"],
        "differs_tags": out["alignment"]["differs_tags"],
        "unmapped_nodes": out["alignment"]["unmapped_nodes"],
    }


# ============================================================================
# Capstone: one-call Hy fragment -> whole meta-circular journey + pnix correspondence
#
# The developer research entry point. For one Hy fragment, bundle every projection
# stage: reader-form models -> macroexpansion (macro layer) -> Python AST/source ->
# Python introspection (code object + bytecode) -> pnix correspondence alignment. Two
# Hy-proof subprocess calls (form + macroexpand); the rest in-process.
# ============================================================================

HY_PNIX_PROJECTION_SCHEMA = "pnix-hy.hy-pnix-projection.v0"


def hy_pnix_projection(source: str, *, timeout: int = 60) -> dict[str, Any]:
    """One call: a Hy fragment's full meta-circular journey + its pnix correspondence.
    reader_forms -> macro (is_macro/expansion) -> python_source/ast -> python
    introspection (code object + bytecode) -> pnix_alignment (correspondence-labelled
    Python AST). Schema `pnix-hy.hy-pnix-projection.v0`."""
    # B2: one Hy-proof spawn yields both the form (reader/lowering) and macro views.
    fm = hy_mirror.hy_form_and_macro_projection(source, timeout=timeout)
    form = fm
    macro = fm
    python_source = form.get("python_source", "")
    alignment = align_python_to_pnix(python_source)
    introspection: dict[str, Any] = {}
    introspection_error = None
    try:
        full = hy_mirror.full_introspection(python_source, "exec")
        introspection = {
            "code_object": full.get("code_object"),
            "bytecode_len": len(full.get("bytecode") or []),
        }
    except Exception as exc:  # noqa: BLE001 - report, don't crash
        introspection_error = f"{type(exc).__name__}: {exc}"
    return {
        "schema": HY_PNIX_PROJECTION_SCHEMA,
        "source": source,
        "reader_forms": form.get("reader_forms"),
        "macro": {
            "is_macro": macro.get("is_macro"),
            "changed_1": macro.get("changed_1"),
            "expanded_full": macro.get("expanded_full"),
            "fully_expanded_python": macro.get("fully_expanded_python"),
        },
        "python_source": python_source,
        "python_ast": form.get("python_ast"),
        "python_introspection": introspection,
        "python_introspection_error": introspection_error,
        "pnix_alignment": alignment,
    }


def hy_pnix_projection_report() -> dict[str, Any]:
    """Self-check: every stage of the capstone populates for a representative form."""
    probe = "(when (> x 0) (+ x 1))"
    try:
        proj = hy_pnix_projection(probe)
    except hy_mirror.HyMirrorError as exc:
        return {"schema": "pnix-hy.hy-pnix-projection.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    align = proj.get("pnix_alignment") or {}
    ready = (
        bool(proj.get("reader_forms"))
        and proj["macro"].get("is_macro") is True
        and "if" in proj.get("python_source", "").replace("\n", " ")
        and {"binary", "var"} <= set(align.get("pnix_tags_hit", []))
        and proj.get("python_introspection_error") is None
    )
    return {
        "schema": "pnix-hy.hy-pnix-projection.report.v0",
        "ready": ready,
        "available": True,
        "probe": probe,
        "is_macro": proj["macro"].get("is_macro"),
        "pnix_tags_hit": align.get("pnix_tags_hit"),
        "stages_present": {
            "reader_forms": bool(proj.get("reader_forms")),
            "macro": bool(proj.get("macro")),
            "python_source": bool(proj.get("python_source")),
            "python_introspection": bool(proj.get("python_introspection")),
            "pnix_alignment": bool(align),
        },
    }


# ============================================================================
# Nested (tree-shaped) alignment (#2-followup)
#
# align_python_to_pnix returns a FLAT list. This returns the same correspondence as a
# TREE mirroring the Python AST structure: each aligned node carries its aligned
# children, so you see which pnix construct nests inside which. Scaffolding nodes (no
# correspondence) are transparent -- their aligned descendants bubble up to the nearest
# aligned ancestor.
# ============================================================================


def _align_collect(node: Any, rows: dict[str, dict[str, Any]]) -> list[dict[str, Any]]:
    """Return this node as a 1-element aligned subtree if it has a pnix correspondence,
    else (scaffolding) the flattened aligned subtrees of its children."""
    children: list[dict[str, Any]] = []
    for child in _pyast.iter_child_nodes(node):
        children.extend(_align_collect(child, rows))
    tag = _node_tag(node)
    if tag is None:
        return children
    row = rows.get(tag, {})
    return [{
        "python_node": type(node).__name__,
        "pnix_tag": tag,
        "pnix_value_type": row.get("pnix_value_type"),
        "differs": bool(row.get("differs")),
        "lineno": getattr(node, "lineno", None),
        "children": children,
    }]


def align_python_to_pnix_tree(python_source: str) -> dict[str, Any]:
    """Tree-shaped correspondence alignment of Python source (mirrors the AST nesting).
    Schema `pnix-hy.hy-pnix-alignment.tree.v0`."""
    tree = _strip_hy_scaffolding(_pyast.parse(python_source))
    rows = _tag_rows()
    forest = _align_collect(tree, rows)

    def _count(nodes: list[dict[str, Any]]) -> int:
        return sum(1 + _count(n["children"]) for n in nodes)

    return {
        "schema": "pnix-hy.hy-pnix-alignment.tree.v0",
        "python_source": python_source,
        "tree": forest,
        "node_count": _count(forest),
    }


def align_hy_to_pnix_tree(hy_source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Project a Hy fragment to Python, then align it as a nested correspondence tree."""
    front = hy_mirror.hy_form_projection(hy_source, timeout=timeout)
    python_source = front.get("python_source", "")
    return {
        "schema": "pnix-hy.hy-to-pnix-alignment.tree.v0",
        "hy_source": hy_source,
        "python_source": python_source,
        "alignment_tree": align_python_to_pnix_tree(python_source),
    }


def alignment_tree_report() -> dict[str, Any]:
    """Self-check: a defn+arith Hy form aligns to a nested lambda > binary > {var,int}."""
    try:
        out = align_hy_to_pnix_tree("(defn f [x] (+ x 1))")
    except hy_mirror.HyMirrorError as exc:
        return {"schema": "pnix-hy.hy-pnix-alignment.tree.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    forest = out["alignment_tree"]["tree"]

    def _find(nodes, tag):
        for n in nodes:
            if n["pnix_tag"] == tag:
                return n
            hit = _find(n["children"], tag)
            if hit:
                return hit
        return None

    lam = _find(forest, "lambda")
    binary = _find(lam["children"], "binary") if lam else None
    inner = {c["pnix_tag"] for c in binary["children"]} if binary else set()
    ready = lam is not None and binary is not None and {"var", "int"} <= inner
    return {
        "schema": "pnix-hy.hy-pnix-alignment.tree.report.v0",
        "ready": ready,
        "available": True,
        "probe": "(defn f [x] (+ x 1))",
        "lambda_present": lam is not None,
        "binary_under_lambda": binary is not None,
        "binary_children_tags": sorted(inner),
    }


# ============================================================================
# Reverse projection: pnix term -> Hy form (best-effort synthesis)
#
# The other direction of the mirror. A pnix AST has no 1:1 Hy form for every construct,
# so this synthesizes representative Hy for the clean-correspondence constructs
# (literals/list/attrset/lambda/apply/binary/unary/if/let/var/select) and honestly
# records a `gap` for constructs with no direct Hy form (with = dynamic scope, path =
# native literal, import/match/has_attr/str_interp, the -> and // operators). Guided by
# the #2 correspondence table.
# ============================================================================

_PNIX_OP_TO_HY: dict[str, str] = {
    "+": "+", "-": "-", "*": "*", "/": "/", "%": "%",
    "<": "<", ">": ">", "<=": "<=", ">=": ">=", "==": "=", "!=": "!=",
    "&&": "and", "||": "or",
}


def _hy_str_literal(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def _match_literal_to_hy(value: Any) -> str:
    """A pnix match `literal` pattern carries a raw Python value, not an AST node."""
    if isinstance(value, bool):
        return "True" if value else "False"
    if isinstance(value, (int, float)):
        return str(value)
    if value is None:
        return "None"
    if isinstance(value, str):
        return _hy_str_literal(value)
    return repr(value)


def _pnix_to_hy(node: Any, gaps: list[dict[str, Any]]) -> str:
    if not isinstance(node, dict):
        return repr(node)
    tag = node.get("tag")
    if tag == "int" or tag == "float":
        return str(node.get("value"))
    if tag == "bool":
        return "True" if node.get("value") else "False"
    if tag == "null":
        return "None"
    if tag == "string":
        return _hy_str_literal(str(node.get("value", "")))
    if tag == "var":
        return str(node.get("name"))
    if tag == "list":
        return "[" + " ".join(_pnix_to_hy(x, gaps) for x in node.get("items", [])) + "]"
    if tag == "lambda":
        return "(fn [" + str(node.get("param")) + "] " + _pnix_to_hy(node.get("body"), gaps) + ")"
    if tag == "apply":
        # flatten the curried application spine: ((f x) y) -> (f x y)
        spine: list[Any] = []
        cur: Any = node
        while isinstance(cur, dict) and cur.get("tag") == "apply":
            spine.append(cur.get("arg"))
            cur = cur.get("func")
        spine.append(cur)
        parts = [_pnix_to_hy(p, gaps) for p in reversed(spine)]
        return "(" + " ".join(parts) + ")"
    if tag == "unary":
        op = node.get("op")
        hyop = {"-": "-", "!": "not"}.get(op)
        if hyop is None:
            gaps.append({"tag": "unary", "op": op, "reason": "no Hy unary op"})
            return f"#_pnix-unary[{op}]"
        return "(" + hyop + " " + _pnix_to_hy(node.get("arg"), gaps) + ")"
    if tag == "binary":
        op = node.get("op")
        hyop = _PNIX_OP_TO_HY.get(op)
        lhs = _pnix_to_hy(node.get("lhs"), gaps)
        rhs = _pnix_to_hy(node.get("rhs"), gaps)
        if hyop is None:
            gaps.append({"tag": "binary", "op": op,
                         "reason": "no Hy operator (pnix -> implication / // attrset-update)"})
            return f"#_pnix-binary[{op}]({lhs} {rhs})"
        return "(" + hyop + " " + lhs + " " + rhs + ")"
    if tag == "if":
        return ("(if " + _pnix_to_hy(node.get("cond"), gaps) + " "
                + _pnix_to_hy(node.get("then"), gaps) + " "
                + _pnix_to_hy(node.get("else"), gaps) + ")")
    if tag == "attrset":
        # fold possibly-multi-segment paths ({ a.b = 1; }) into a nested structure;
        # synthesized intermediate dicts have no "tag" key, pnix AST values do.
        root: dict[str, Any] = {}
        for b in node.get("bindings", []):
            path = b.get("path", [])
            if not path:
                continue
            cur = root
            for seg in path[:-1]:
                nxt = cur.get(seg)
                if not isinstance(nxt, dict) or "tag" in nxt:
                    nxt = {}
                    cur[seg] = nxt
                cur = nxt
            cur[path[-1]] = b.get("value")

        def _emit_dict(d: dict[str, Any]) -> str:
            pairs2: list[str] = []
            for k, v in d.items():
                vs = _emit_dict(v) if isinstance(v, dict) and "tag" not in v else _pnix_to_hy(v, gaps)
                pairs2.append(_hy_str_literal(str(k)) + " " + vs)
            return "{" + " ".join(pairs2) + "}"

        body = _emit_dict(root)
        if node.get("recursive"):
            gaps.append({"tag": "attrset", "reason": "rec {} is recursive; Hy dict is not"})
        return body
    if tag == "str_interp":
        out = 'f"'
        for part in node.get("parts", []):
            if "lit" in part:
                out += (part["lit"].replace("\\", "\\\\").replace('"', '\\"')
                        .replace("{", "{{").replace("}", "}}"))
            else:
                out += "{" + _pnix_to_hy(part.get("expr"), gaps) + "}"
        out += '"'
        return out
    if tag == "has_attr":
        path = node.get("path") or [node.get("attr")]
        gaps.append({"tag": "has_attr",
                     "reason": "pnix ? is a non-forcing key test; Hy `in` forces + is general membership"})
        if len(path) == 1:
            return "(in " + _hy_str_literal(str(path[0])) + " " + _pnix_to_hy(node.get("base"), gaps) + ")"
        return "#_pnix-has_attr[" + ".".join(str(p) for p in path) + "]"
    if tag == "let":
        binds: list[str] = []
        for b in node.get("bindings", []):
            path = b.get("path", [])
            if len(path) != 1:
                gaps.append({"tag": "let", "path": path, "reason": "nested-path let binding"})
                continue
            binds.append(str(path[0]) + " " + _pnix_to_hy(b.get("value"), gaps))
        gaps.append({"tag": "let", "reason": "pnix let is recursive+lazy; Hy let is sequential"})
        return "(let [" + " ".join(binds) + "] " + _pnix_to_hy(node.get("body"), gaps) + ")"
    if tag == "select":
        attr = _hy_str_literal(str(node.get("attr")))
        return "(get " + _pnix_to_hy(node.get("base"), gaps) + " " + attr + ")"
    if tag == "match":
        arms = node.get("arms", [])
        simple = bool(arms) and all(
            a.get("guard") is None
            and (a.get("pattern") or {}).get("tag") in ("literal", "wildcard")
            for a in arms
        )
        if not simple:
            gaps.append({"tag": "match",
                         "reason": "match arm has a guard or non-literal pattern; no clean Hy cond form"})
            return "#_pnix-match"
        scrut = _pnix_to_hy(node.get("scrutinee"), gaps)
        clauses: list[str] = []  # Hy 1.3 cond is flat: (cond test1 body1 test2 body2 ...)
        for a in arms:
            pat = a.get("pattern") or {}
            body = _pnix_to_hy(a.get("body"), gaps)
            if pat.get("tag") == "wildcard":
                clauses.append("True " + body)
            else:
                clauses.append("(= " + scrut + " " + _match_literal_to_hy(pat.get("value")) + ") " + body)
        return "(cond " + " ".join(clauses) + ")"
    # constructs with no direct Hy form
    gaps.append({"tag": tag, "reason": "no direct Hy form"})
    return f"#_pnix-{tag}"


def pnix_to_hy_form(pnix_source: str) -> dict[str, Any]:
    """Reverse projection: synthesize representative Hy source for a pnix fragment, with
    honest `gaps` for constructs that have no direct Hy form. Schema
    `pnix-hy.pnix-to-hy.v0`."""
    ast = rt.parse(pnix_source)
    gaps: list[dict[str, Any]] = []
    hy_source = _pnix_to_hy(ast, gaps)
    return {
        "schema": "pnix-hy.pnix-to-hy.v0",
        "pnix_source": pnix_source,
        "pnix_ast_root_tag": ast.get("tag") if isinstance(ast, dict) else None,
        "hy_source": hy_source,
        "gaps": gaps,
        "clean": not gaps,
    }


def pnix_to_hy_form_report() -> dict[str, Any]:
    """Self-check: a clean pnix expression synthesizes to runnable Hy that, projected
    back through hy_form_projection, lowers to Python (round-trip sanity); and a `with`
    form is flagged as a gap."""
    clean = pnix_to_hy_form("(x: x + 1)")
    roundtrip_ok = False
    try:
        proj = hy_mirror.hy_form_projection(clean["hy_source"])
        roundtrip_ok = "lambda" in proj.get("python_source", "")
    except hy_mirror.HyMirrorError:
        roundtrip_ok = False
    gapped = pnix_to_hy_form("with m; b")
    ready = (
        clean["hy_source"] == "(fn [x] (+ x 1))"
        and clean["clean"] is True
        and roundtrip_ok
        and any(g["tag"] == "with" for g in gapped["gaps"])
    )
    return {
        "schema": "pnix-hy.pnix-to-hy.report.v0",
        "ready": ready,
        "available": True,
        "clean_probe": "(x: x + 1)",
        "clean_hy": clean["hy_source"],
        "roundtrip_lowers_to_lambda": roundtrip_ok,
        "with_flagged_gap": any(g["tag"] == "with" for g in gapped["gaps"]),
    }


# ============================================================================
# pnix meta-circular projection: one pnix form across all FOUR substrates
#
# The deepest on-mission view: pnix's own meta-circular is that pnix is evaluated by a
# pnix-evaluator/compiler WRITTEN IN HY (stage7) -- i.e. the pnix evaluator is itself a
# Python-ecosystem artifact. This evaluates one pnix form on all four substrates and
# reports whether they converge (self-hosting closure for that form):
#   1. host interpreter (Python)         2. host compiler (Python)
#   3. stage7 runtime  (pnix-in-Hy)      4. stage7 compiler (pnix-in-Hy)
# (3) and (4) spawn the Hy proof Python, so this is slower than pnix_form_projection.
# ============================================================================


def pnix_meta_circular_projection(source: str, *, timeout: int = 180) -> dict[str, Any]:
    """Evaluate one pnix form on all four meta-circular substrates and report
    convergence. Schema `pnix-hy.pnix-meta-circular-projection.v0`."""
    ast = rt.parse(source)
    lanes: dict[str, Any] = {}
    errors: dict[str, str] = {}

    def _host(name: str, fn) -> None:
        try:
            lanes[name] = rt.stable_data(fn(source))
        except Exception as exc:  # noqa: BLE001
            errors[name] = f"{type(exc).__name__}: {exc}"

    _host("host_interp", rt.eval_source)
    _host("host_compiler", rt.run_px_source)
    try:
        lanes["stage7_runtime"] = hy_mirror.stage7_eval_json(
            rt.hy_runtime_source_for_sources([source]))[0]
    except Exception as exc:  # noqa: BLE001
        errors["stage7_runtime"] = f"{type(exc).__name__}: {exc}"
    try:
        lanes["stage7_compiler"] = hy_mirror.stage7_eval_json(
            rt.hy_compiler_source_for_sources([source]))[0]
    except Exception as exc:  # noqa: BLE001
        errors["stage7_compiler"] = f"{type(exc).__name__}: {exc}"

    values = list(lanes.values())
    converged = not errors and len(values) == 4 and all(v == values[0] for v in values[1:])
    return {
        "schema": "pnix-hy.pnix-meta-circular-projection.v0",
        "source": source,
        "ast_root_tag": ast.get("tag") if isinstance(ast, dict) else None,
        "lanes": lanes,
        "errors": errors,
        "converged": converged,
        "substrates": {
            "host_interp": "Python interpreter",
            "host_compiler": "Python compiler",
            "stage7_runtime": "pnix evaluator written in Hy",
            "stage7_compiler": "pnix compiler written in Hy",
        },
    }


def pnix_meta_circular_projection_report() -> dict[str, Any]:
    """Check value convergence and Nix's selective tryEval error boundary on
    all four meta-circular substrates."""
    probe = "let inc = x: x + 1; in inc 41"
    error_probe = "builtins.tryEval (1 / 0)"
    repeated_catch_probe = (
        'let s = { x = builtins.throw "boom"; }; '
        "in [ (builtins.tryEval s.x).success (builtins.tryEval s.x).success ]"
    )
    throw_type_probe = "builtins.tryEval (builtins.throw 42)"
    try:
        proj = pnix_meta_circular_projection(probe)
        error_proj = pnix_meta_circular_projection(error_probe)
        repeated_catch_proj = pnix_meta_circular_projection(repeated_catch_probe)
        throw_type_proj = pnix_meta_circular_projection(throw_type_probe)
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.pnix-meta-circular-projection.report.v0",
                "ready": False, "available": False, "error": str(exc)}
    error_ready = (
        not error_proj["lanes"]
        and len(error_proj["errors"]) == 4
        and all("division by zero" in error for error in error_proj["errors"].values())
    )
    repeated_catch_ready = (
        repeated_catch_proj["converged"]
        and repeated_catch_proj["lanes"].get("host_interp") == [False, False]
    )
    throw_type_ready = (
        not throw_type_proj["lanes"]
        and len(throw_type_proj["errors"]) == 4
        and all("string" in error for error in throw_type_proj["errors"].values())
    )
    ready = (
        proj["converged"]
        and proj["lanes"].get("host_interp") == 42
        and error_ready
        and repeated_catch_ready
        and throw_type_ready
    )
    return {
        "schema": "pnix-hy.pnix-meta-circular-projection.report.v0",
        "ready": ready,
        "available": True,
        "probe": probe,
        "lanes": proj["lanes"],
        "converged": proj["converged"],
        "errors": proj["errors"],
        "tryeval_error_probe": error_probe,
        "tryeval_error_ready": error_ready,
        "tryeval_errors": error_proj["errors"],
        "repeated_tryeval_probe": repeated_catch_probe,
        "repeated_tryeval_ready": repeated_catch_ready,
        "repeated_tryeval_lanes": repeated_catch_proj["lanes"],
        "repeated_tryeval_errors": repeated_catch_proj["errors"],
        "throw_type_probe": throw_type_probe,
        "throw_type_ready": throw_type_ready,
        "throw_type_errors": throw_type_proj["errors"],
    }


# ============================================================================
# Projection value-roundtrip: does the projection PRESERVE MEANING, not just shape?
#
# The correspondence/alignment facilities relate the two substrates STRUCTURALLY. This
# checks SEMANTICS: evaluate a pnix form (host), synthesize Hy from it (pnix_to_hy_form),
# evaluate that Hy in the stage7 Hy kernel, and compare the two values (via canonical
# JSON, so spacing/key-order don't matter). meaning_preserved=True means pnix X and its
# projected Hy compute the same value. Constructs with synthesis gaps or non-data values
# (functions) are reported not-comparable. (Spawns the Hy kernel -> slower.)
# ============================================================================


def projection_value_roundtrip(source: str) -> dict[str, Any]:
    """Evaluate a pnix form (host) and its synthesized Hy (stage7 kernel), compare values
    via canonical JSON. Schema `pnix-hy.projection-value-roundtrip.v0`.

    Note: a `gap` in pnix_to_hy_form is either a `#_pnix-...` PLACEHOLDER (unprojectable
    construct -- won't evaluate, so not comparable) or a SEMANTIC note (the Hy DID
    synthesize but its meaning may differ -- exactly what this checks). Only placeholders
    skip the roundtrip; semantic-note constructs are evaluated so any meaning divergence
    is surfaced."""
    result: dict[str, Any] = {
        "schema": "pnix-hy.projection-value-roundtrip.v0",
        "source": source,
    }
    try:
        pnix_value = rt.stable_data(rt.eval_source(source))
    except Exception as exc:  # noqa: BLE001 - pnix form itself errors (free var, etc.)
        result["comparable"] = False
        result["reason"] = f"pnix eval error: {type(exc).__name__}: {exc}"
        return result
    synth = pnix_to_hy_form(source)
    result["pnix_value"] = pnix_value
    result["hy_source"] = synth["hy_source"]
    result["gaps"] = synth["gaps"]
    if "#_pnix-" in synth["hy_source"]:
        result["comparable"] = False
        result["reason"] = "synthesized Hy has unprojectable placeholders (#_pnix-...)"
        return result
    try:
        raw = hy_mirror.stage7_eval("(do (import json) (json.dumps " + synth["hy_source"] + "))")
        hy_value = _json.loads(raw)
    except Exception as exc:  # noqa: BLE001 - non-data value (function) or eval error
        result["comparable"] = False
        result["reason"] = f"Hy value not data/JSON-serializable: {type(exc).__name__}: {exc}"
        return result
    result["hy_value"] = hy_value
    result["comparable"] = True
    result["meaning_preserved"] = pnix_value == hy_value
    return result


def projection_value_roundtrip_report() -> dict[str, Any]:
    """Self-check: several clean pnix forms must roundtrip meaning-preserving through the
    synthesized Hy (evaluated in the stage7 kernel)."""
    probes = ["1 + 2", "[ 1 2 3 ]", "{ a = 1; b = 2; }", "if true then 10 else 20",
              "let a = 1; in a + a"]
    results = []
    all_ok = True
    for p in probes:
        try:
            r = projection_value_roundtrip(p)
        except Exception as exc:  # noqa: BLE001
            return {"schema": "pnix-hy.projection-value-roundtrip.report.v0",
                    "ready": False, "available": False, "error": f"{p}: {exc}"}
        ok = r.get("comparable") and r.get("meaning_preserved")
        all_ok = all_ok and bool(ok)
        results.append({"source": p, "hy": r.get("hy_source"),
                        "pnix_value": r.get("pnix_value"), "hy_value": r.get("hy_value"),
                        "meaning_preserved": r.get("meaning_preserved")})
    return {
        "schema": "pnix-hy.projection-value-roundtrip.report.v0",
        "ready": all_ok,
        "available": True,
        "results": results,
    }


# ============================================================================
# Hy -> pnix SYNTHESIS (the reverse of pnix_to_hy_form) + forward value-roundtrip
#
# pnix_to_hy_form synthesizes Hy from pnix; align_*_to_pnix only LABELS Python AST nodes.
# Neither produces actual pnix SOURCE from a Hy fragment, so the FORWARD direction's
# meaning preservation was never checkable. This synthesizes pnix source from a Hy
# fragment's Python lowering (the expression subset), then hy_to_pnix_value_roundtrip
# evaluates the Hy lowering (host Python) AND the synthesized pnix (host pnix) and compares
# values -- the symmetric counterpart of projection_value_roundtrip, proving Hy X and its
# pnix projection compute the same value.
# ============================================================================

_PY_BINOP_TO_PNIX: dict[type, str] = {
    _pyast.Add: "+", _pyast.Sub: "-", _pyast.Mult: "*", _pyast.Div: "/", _pyast.Mod: "%",
}
_PY_CMP_TO_PNIX: dict[type, str] = {
    _pyast.Eq: "==", _pyast.NotEq: "!=", _pyast.Lt: "<", _pyast.Gt: ">",
    _pyast.LtE: "<=", _pyast.GtE: ">=",
}
_PY_BOOL_TO_PNIX: dict[type, str] = {_pyast.And: "&&", _pyast.Or: "||"}


def _pnix_str_literal(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"').replace("${", "\\${") + '"'


def _joinedstr_to_pnix(node: Any, gaps: list[dict[str, Any]]) -> str:
    """A Python f-string (JoinedStr) -> pnix string interpolation. Each hole is wrapped in
    `builtins.toString (...)` because Python f-strings apply str() while pnix interpolation
    requires a string (so int/string holes preserve value; bool/float coercion differs from
    Python and is left to surface in the value-roundtrip, not hidden)."""
    body = ""
    for part in node.values:
        if isinstance(part, _pyast.Constant) and isinstance(part.value, str):
            body += part.value.replace("\\", "\\\\").replace('"', '\\"').replace("${", "\\${")
        elif isinstance(part, _pyast.FormattedValue):
            if part.format_spec is not None or part.conversion not in (-1, 115):  # -1/!s = str()
                gaps.append({"node": "FormattedValue",
                             "reason": "format spec or !r/!a conversion has no pnix interpolation form"})
                return "#_pnix-gap[fstring-format]"
            body += "${builtins.toString (" + _python_expr_to_pnix(part.value, gaps) + ")}"
        else:
            gaps.append({"node": "JoinedStr", "reason": f"unsupported f-string part {type(part).__name__}"})
            return "#_pnix-gap[fstring]"
    return '"' + body + '"'


def _python_expr_to_pnix(node: Any, gaps: list[dict[str, Any]]) -> str:
    """Synthesize pnix source for the clean-correspondence Python expression subset that
    Hy fragments lower to. Honest `#_pnix-gap[...]` + a gap record for anything else."""
    if isinstance(node, _pyast.Constant):
        v = node.value
        if isinstance(v, bool):
            return "true" if v else "false"
        if v is None:
            return "null"
        if isinstance(v, str):
            return _pnix_str_literal(v)
        if isinstance(v, (int, float)):
            return str(v)
        gaps.append({"node": "Constant", "reason": f"unsupported literal type {type(v).__name__}"})
        return "#_pnix-gap[const]"
    if isinstance(node, _pyast.JoinedStr):  # f-string -> pnix string interpolation
        return _joinedstr_to_pnix(node, gaps)
    if isinstance(node, _pyast.Name):
        return node.id
    if isinstance(node, _pyast.BinOp) and type(node.op) in _PY_BINOP_TO_PNIX:
        return ("(" + _python_expr_to_pnix(node.left, gaps) + " "
                + _PY_BINOP_TO_PNIX[type(node.op)] + " "
                + _python_expr_to_pnix(node.right, gaps) + ")")
    if isinstance(node, _pyast.UnaryOp):
        if isinstance(node.op, _pyast.USub):
            return "(-" + _python_expr_to_pnix(node.operand, gaps) + ")"
        if isinstance(node.op, _pyast.Not):
            return "(!" + _python_expr_to_pnix(node.operand, gaps) + ")"
        gaps.append({"node": "UnaryOp", "reason": "no pnix unary op"})
        return "#_pnix-gap[unary]"
    if isinstance(node, _pyast.Compare) and len(node.ops) == 1 and type(node.ops[0]) in _PY_CMP_TO_PNIX:
        return ("(" + _python_expr_to_pnix(node.left, gaps) + " "
                + _PY_CMP_TO_PNIX[type(node.ops[0])] + " "
                + _python_expr_to_pnix(node.comparators[0], gaps) + ")")
    if isinstance(node, _pyast.BoolOp) and type(node.op) in _PY_BOOL_TO_PNIX:
        op = _PY_BOOL_TO_PNIX[type(node.op)]
        s = _python_expr_to_pnix(node.values[0], gaps)
        for v in node.values[1:]:
            s = "(" + s + " " + op + " " + _python_expr_to_pnix(v, gaps) + ")"
        return s
    if isinstance(node, _pyast.IfExp):
        return ("(if " + _python_expr_to_pnix(node.test, gaps) + " then "
                + _python_expr_to_pnix(node.body, gaps) + " else "
                + _python_expr_to_pnix(node.orelse, gaps) + ")")
    if isinstance(node, (_pyast.List, _pyast.Tuple)):
        return "[ " + " ".join(_python_expr_to_pnix(x, gaps) for x in node.elts) + " ]"
    if isinstance(node, _pyast.Lambda):
        if node.args.kwonlyargs or node.args.vararg or node.args.kwarg:
            gaps.append({"node": "Lambda", "reason": "non-positional params; pnix lambda is curried positional"})
            return "#_pnix-gap[lambda]"
        body = _python_expr_to_pnix(node.body, gaps)
        params = [a.arg for a in node.args.args]
        if not params:
            gaps.append({"node": "Lambda", "reason": "zero-arg lambda has no pnix form"})
            return "#_pnix-gap[lambda0]"
        for a in reversed(params):  # curry: (lambda x y: e) -> (x: (y: e))
            body = "(" + a + ": " + body + ")"
        return body
    if isinstance(node, _pyast.Call) and not node.keywords and not any(
        isinstance(a, _pyast.Starred) for a in node.args
    ):
        s = _python_expr_to_pnix(node.func, gaps)
        for a in node.args:  # curried juxtaposition: f(x, y) -> ((f x) y)
            s = "(" + s + " " + _python_expr_to_pnix(a, gaps) + ")"
        return s
    if isinstance(node, _pyast.Dict):
        pairs: list[str] = []
        for k, v in zip(node.keys, node.values):
            if isinstance(k, _pyast.Constant) and isinstance(k.value, str) and k.value.isidentifier():
                pairs.append(str(k.value) + " = " + _python_expr_to_pnix(v, gaps) + ";")
            else:
                gaps.append({"node": "Dict", "reason": "non-identifier-string key has no clean pnix attr"})
                return "#_pnix-gap[dict-key]"
        return "{ " + " ".join(pairs) + " }"
    gaps.append({"node": type(node).__name__, "reason": "no direct pnix form"})
    return f"#_pnix-gap[{type(node).__name__}]"


def _python_stmt_to_pnix_binding(stmt: Any, gaps: list[dict[str, Any]], seen: set[str]) -> str | None:
    """Synthesize a pnix let-binding from a Python statement (the defn/setv subset). pnix
    let is recursive, so `setv`->binding and `defn`->curried-lambda-binding both fit."""
    if isinstance(stmt, _pyast.Assign) and len(stmt.targets) == 1 and isinstance(stmt.targets[0], _pyast.Name):
        name = stmt.targets[0].id
        if name in seen:
            gaps.append({"node": "Assign", "reason": f"rebinding '{name}'; pnix let has no reassignment"})
            return None
        seen.add(name)
        return name + " = " + _python_expr_to_pnix(stmt.value, gaps) + ";"
    if isinstance(stmt, _pyast.FunctionDef):
        a = stmt.args
        if a.kwonlyargs or a.vararg or a.kwarg or a.defaults or a.kw_defaults or getattr(a, "posonlyargs", []):
            gaps.append({"node": "FunctionDef", "reason": "non-simple-positional params"})
            return None
        if len(stmt.body) != 1 or not isinstance(stmt.body[0], _pyast.Return) or stmt.body[0].value is None:
            gaps.append({"node": "FunctionDef", "reason": "body is not a single `return <expr>`"})
            return None
        params = [p.arg for p in a.args]
        if not params:
            gaps.append({"node": "FunctionDef", "reason": "zero-arg def has no pnix (curried) lambda form"})
            return None
        if stmt.name in seen:
            gaps.append({"node": "FunctionDef", "reason": f"rebinding '{stmt.name}'"})
            return None
        seen.add(stmt.name)
        body = _python_expr_to_pnix(stmt.body[0].value, gaps)
        for p in reversed(params):  # curry: def f(a, b) -> f = (a: (b: e))
            body = "(" + p + ": " + body + ")"
        return stmt.name + " = " + body + ";"
    gaps.append({"node": type(stmt).__name__, "reason": "no pnix let-binding form"})
    return None


def _python_module_to_pnix(body: list[Any], gaps: list[dict[str, Any]]) -> str:
    """A single expression -> that pnix expr; a sequence of (setv/defn ...) ending in an
    expression -> a pnix `let <bindings> in <result>`."""
    if len(body) == 1 and isinstance(body[0], _pyast.Expr):
        return _python_expr_to_pnix(body[0].value, gaps)
    if not body or not isinstance(body[-1], _pyast.Expr):
        gaps.append({"node": "Module", "reason": "module must end in an expression (the result value)"})
        return "#_pnix-gap[module-no-result]"
    binds: list[str] = []
    seen: set[str] = set()
    for stmt in body[:-1]:
        b = _python_stmt_to_pnix_binding(stmt, gaps, seen)
        if b is None:
            return "#_pnix-gap[stmt]"
        binds.append(b)
    result = _python_expr_to_pnix(body[-1].value, gaps)
    if not binds:
        return result
    return "let " + " ".join(binds) + " in " + result


def synthesize_pnix_from_hy(hy_source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Reverse of pnix_to_hy_form: synthesize pnix SOURCE from a Hy fragment (via its
    Python lowering, expression subset). Schema `pnix-hy.synthesize-pnix-from-hy.v0`."""
    front = hy_mirror.hy_form_projection(hy_source, timeout=timeout)
    python_source = front.get("python_source", "")
    tree = _strip_hy_scaffolding(_pyast.parse(python_source))
    gaps: list[dict[str, Any]] = []
    result: dict[str, Any] = {
        "schema": "pnix-hy.synthesize-pnix-from-hy.v0",
        "hy_source": hy_source,
        "python_source": python_source,
    }
    pnix_source = _python_module_to_pnix(tree.body, gaps)
    result["pnix_source"] = pnix_source
    result["gaps"] = gaps
    result["synthesizable"] = "#_pnix-gap" not in pnix_source
    return result


def synthesize_pnix_from_hy_report() -> dict[str, Any]:
    """Self-check: representative Hy expressions synthesize to evaluable pnix source."""
    cases = {
        "(+ 1 2)": "(1 + 2)",
        "[1 2 3]": "[ 1 2 3 ]",
        "(if (> 3 2) 10 20)": "(if (3 > 2) then 10 else 20)",
        "(setv x 5)\n(defn inc [n] (+ n 1))\n(inc x)":
            "let x = 5; inc = (n: (n + 1)); in (inc x)",
        'f"x={(+ 1 2)}"': '"x=${builtins.toString ((1 + 2))}"',  # f-string -> pnix interpolation
    }
    results = []
    all_ok = True
    for hy, expect in cases.items():
        try:
            r = synthesize_pnix_from_hy(hy)
        except Exception as exc:  # noqa: BLE001
            return {"schema": "pnix-hy.synthesize-pnix-from-hy.report.v0",
                    "ready": False, "available": False, "error": f"{hy}: {exc}"}
        ok = r.get("synthesizable") and r.get("pnix_source") == expect
        all_ok = all_ok and bool(ok)
        results.append({"hy": hy, "pnix": r.get("pnix_source"), "expected": expect, "ok": ok})
    return {
        "schema": "pnix-hy.synthesize-pnix-from-hy.report.v0",
        "ready": all_ok,
        "available": True,
        "results": results,
    }


def hy_to_pnix_value_roundtrip(hy_source: str, *, timeout: int = 60) -> dict[str, Any]:
    """The FORWARD meaning-preservation check (symmetric to projection_value_roundtrip):
    synthesize pnix from a Hy fragment, evaluate the Hy lowering (host Python) AND the
    synthesized pnix (host pnix), compare values via canonical JSON. Schema
    `pnix-hy.hy-to-pnix-value-roundtrip.v0`."""
    result: dict[str, Any] = {"schema": "pnix-hy.hy-to-pnix-value-roundtrip.v0", "hy_source": hy_source}
    synth = synthesize_pnix_from_hy(hy_source, timeout=timeout)
    result["python_source"] = synth["python_source"]
    result["pnix_source"] = synth["pnix_source"]
    result["gaps"] = synth["gaps"]
    if not synth["synthesizable"]:
        result["comparable"] = False
        result["reason"] = "Hy fragment has no clean pnix synthesis (#_pnix-gap)"
        return result
    # Hy value = the value of its Python lowering (Hy's semantics ARE the lowered Python).
    # A statement-level fragment (setv/defn ... result) execs the leading statements first.
    tree = _strip_hy_scaffolding(_pyast.parse(synth["python_source"]))
    try:
        body = tree.body
        ns: dict[str, Any] = {}
        if len(body) > 1:
            exec(compile(_pyast.Module(body=body[:-1], type_ignores=[]), "<hy-rt>", "exec"), ns)  # noqa: S102
        hy_value = eval(_pyast.unparse(body[-1].value), ns)  # noqa: S307 - Hy lowering
        hy_json = _json.dumps(hy_value, sort_keys=True)
    except Exception as exc:  # noqa: BLE001 - non-data value (function) or eval error
        result["comparable"] = False
        result["reason"] = f"Hy value not data/JSON-serializable: {type(exc).__name__}: {exc}"
        return result
    try:
        raw = rt.eval_source(synth["pnix_source"])
        pnix_json = _json.dumps(_json.loads(rt.to_json_string_value(raw)), sort_keys=True)
        pnix_value = rt.stable_data(raw)
    except Exception as exc:  # noqa: BLE001 - pnix eval error / non-data value
        result["comparable"] = False
        result["reason"] = f"pnix value not data/JSON-serializable: {type(exc).__name__}: {exc}"
        return result
    result["hy_value"] = hy_value
    result["pnix_value"] = pnix_value
    result["comparable"] = True
    result["meaning_preserved"] = hy_json == pnix_json
    return result


def hy_to_pnix_value_roundtrip_report() -> dict[str, Any]:
    """Self-check: several Hy expressions roundtrip meaning-preserving to pnix (the Hy
    lowering and the synthesized pnix compute the same value)."""
    probes = ["(+ 1 2)", "[1 2 3]", "(if (> 3 2) 10 20)", "((fn [x] (+ x 1)) 41)",
              "(and True False)",
              "(setv x 5)\n(defn inc [n] (+ n 1))\n(inc x)",
              "(defn add [a b] (+ a b))\n(add 3 4)",
              'f"v={(+ 20 1)}"']  # f-string -> pnix interpolation; "v21" both sides
    results = []
    all_ok = True
    for p in probes:
        try:
            r = hy_to_pnix_value_roundtrip(p)
        except Exception as exc:  # noqa: BLE001
            return {"schema": "pnix-hy.hy-to-pnix-value-roundtrip.report.v0",
                    "ready": False, "available": False, "error": f"{p}: {exc}"}
        ok = r.get("comparable") and r.get("meaning_preserved")
        all_ok = all_ok and bool(ok)
        results.append({"hy": p, "pnix": r.get("pnix_source"),
                        "hy_value": r.get("hy_value"), "pnix_value": r.get("pnix_value"),
                        "meaning_preserved": r.get("meaning_preserved")})
    return {
        "schema": "pnix-hy.hy-to-pnix-value-roundtrip.report.v0",
        "ready": all_ok,
        "available": True,
        "results": results,
    }


# ============================================================================
# Projection CLOSURE (involution): the two synthesis directions compose meaning-
# preservingly. pnix -> Hy -> pnix and Hy -> pnix -> Hy should each return a program
# whose value equals the original. This is a stronger property than either direction's
# single roundtrip -- it validates the toolkit's OWN internal consistency (pnix_to_hy_form
# and synthesize_pnix_from_hy are mutually inverse on the clean subset, up to value).
# ============================================================================

def _eval_hy_lowering_value(hy_source: str, *, timeout: int = 60) -> Any:
    """The value of a Hy fragment = the value of its Python lowering (statement-level
    fragments exec the leading statements first). Raises on non-expression-tailed source."""
    front = hy_mirror.hy_form_projection(hy_source, timeout=timeout)
    tree = _strip_hy_scaffolding(_pyast.parse(front.get("python_source", "")))
    body = tree.body
    if not body or not isinstance(body[-1], _pyast.Expr):
        raise ValueError("Hy fragment does not end in an expression")
    ns: dict[str, Any] = {}
    if len(body) > 1:
        exec(compile(_pyast.Module(body=body[:-1], type_ignores=[]), "<hy-rt>", "exec"), ns)  # noqa: S102
    return eval(_pyast.unparse(body[-1].value), ns)  # noqa: S307 - Hy lowering


def _canonical_pnix_value(raw: Any) -> str:
    return _json.dumps(_json.loads(rt.to_json_string_value(raw)), sort_keys=True)


def pnix_projection_closure(pnix_source: str, *, timeout: int = 60) -> dict[str, Any]:
    """pnix -> synthesized Hy -> synthesized pnix -> value; check the doubly-projected
    value equals the original. Schema `pnix-hy.pnix-projection-closure.v0`."""
    result: dict[str, Any] = {"schema": "pnix-hy.pnix-projection-closure.v0", "pnix_source": pnix_source}
    try:
        v0_raw = rt.eval_source(pnix_source)
        v0 = _canonical_pnix_value(v0_raw)
    except Exception as exc:  # noqa: BLE001
        result["comparable"] = False
        result["reason"] = f"pnix eval error: {type(exc).__name__}: {exc}"
        return result
    hy = pnix_to_hy_form(pnix_source)
    result["hy_source"] = hy["hy_source"]
    if "#_pnix-" in hy["hy_source"]:
        result["comparable"] = False
        result["reason"] = "Hy synth has unprojectable placeholders (#_pnix-...)"
        return result
    synth = synthesize_pnix_from_hy(hy["hy_source"], timeout=timeout)
    result["pnix_source_2"] = synth["pnix_source"]
    if not synth["synthesizable"]:
        result["comparable"] = False
        result["reason"] = "re-synthesized pnix has gaps (#_pnix-gap)"
        return result
    try:
        v2_raw = rt.eval_source(synth["pnix_source"])
        v2 = _canonical_pnix_value(v2_raw)
    except Exception as exc:  # noqa: BLE001
        result["comparable"] = False
        result["reason"] = f"re-synthesized pnix eval error: {type(exc).__name__}: {exc}"
        return result
    result["value"] = rt.stable_data(v0_raw)
    result["comparable"] = True
    result["closed"] = v0 == v2
    return result


def pnix_projection_closure_report() -> dict[str, Any]:
    """Self-check: clean pnix forms survive the full pnix -> Hy -> pnix cycle by value."""
    probes = ["1 + 2", "[ 1 2 3 ]", "if true then 10 else 20", "(x: x + 1) 41",
              "{ a = 1; b = 2; }"]
    results = []
    all_ok = True
    for p in probes:
        try:
            r = pnix_projection_closure(p)
        except Exception as exc:  # noqa: BLE001
            return {"schema": "pnix-hy.pnix-projection-closure.report.v0",
                    "ready": False, "available": False, "error": f"{p}: {exc}"}
        ok = r.get("comparable") and r.get("closed")
        all_ok = all_ok and bool(ok)
        results.append({"pnix": p, "hy": r.get("hy_source"),
                        "pnix_2": r.get("pnix_source_2"), "closed": r.get("closed")})
    return {
        "schema": "pnix-hy.pnix-projection-closure.report.v0",
        "ready": all_ok,
        "available": True,
        "results": results,
    }


def hy_projection_closure(hy_source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Hy -> synthesized pnix -> synthesized Hy -> value; check the doubly-projected value
    equals the original. Schema `pnix-hy.hy-projection-closure.v0`."""
    result: dict[str, Any] = {"schema": "pnix-hy.hy-projection-closure.v0", "hy_source": hy_source}
    try:
        v0 = _json.dumps(_eval_hy_lowering_value(hy_source, timeout=timeout), sort_keys=True)
    except Exception as exc:  # noqa: BLE001
        result["comparable"] = False
        result["reason"] = f"Hy value not data/JSON-serializable: {type(exc).__name__}: {exc}"
        return result
    synth = synthesize_pnix_from_hy(hy_source, timeout=timeout)
    result["pnix_source"] = synth["pnix_source"]
    if not synth["synthesizable"]:
        result["comparable"] = False
        result["reason"] = "synthesized pnix has gaps (#_pnix-gap)"
        return result
    hy2 = pnix_to_hy_form(synth["pnix_source"])
    result["hy_source_2"] = hy2["hy_source"]
    if "#_pnix-" in hy2["hy_source"]:
        result["comparable"] = False
        result["reason"] = "re-synthesized Hy has unprojectable placeholders (#_pnix-...)"
        return result
    try:
        v2 = _json.dumps(_eval_hy_lowering_value(hy2["hy_source"], timeout=timeout), sort_keys=True)
    except Exception as exc:  # noqa: BLE001
        result["comparable"] = False
        result["reason"] = f"re-synthesized Hy value error: {type(exc).__name__}: {exc}"
        return result
    result["comparable"] = True
    result["closed"] = v0 == v2
    return result


def hy_projection_closure_report() -> dict[str, Any]:
    """Self-check: clean Hy forms survive the full Hy -> pnix -> Hy cycle by value."""
    probes = ["(+ 1 2)", "[1 2 3]", "(if (> 3 2) 10 20)", "((fn [x] (+ x 1)) 41)"]
    results = []
    all_ok = True
    for p in probes:
        try:
            r = hy_projection_closure(p)
        except Exception as exc:  # noqa: BLE001
            return {"schema": "pnix-hy.hy-projection-closure.report.v0",
                    "ready": False, "available": False, "error": f"{p}: {exc}"}
        ok = r.get("comparable") and r.get("closed")
        all_ok = all_ok and bool(ok)
        results.append({"hy": p, "pnix": r.get("pnix_source"),
                        "hy_2": r.get("hy_source_2"), "closed": r.get("closed")})
    return {
        "schema": "pnix-hy.hy-projection-closure.report.v0",
        "ready": all_ok,
        "available": True,
        "results": results,
    }


# ============================================================================
# META-CIRCULAR TOWER — one call, the whole journey for a single Hy fragment
#
# Integrates the session's facilities into the staging-tower narrative so a developer can
# study a fragment's full meta-circular life in one shot:
#   READ     reader-form model tree (hy_form_projection)
#   COMPILE  macro tower + Python lowering (hy_macro_step_trace)
#   RUN      the bytecode that actually executes (execution_trace)
#   PNIX     synthesized pnix + value-roundtrip + closure (this module)
#   COLLAPSE specialize the pnix program (Futamura / specialize_pnix) -- the tower
#            of stages collapses into one folded value when the program is closed.
# ============================================================================

def meta_circular_tower(hy_source: str, *, timeout: int = 90) -> dict[str, Any]:
    """The whole meta-circular journey of a Hy fragment in one call, organized as the
    read->compile->run->pnix->collapse staging tower. Schema
    `pnix-hy.meta-circular-tower.v0`."""
    front = hy_mirror.hy_form_projection(hy_source, timeout=timeout)
    python_source = front.get("python_source", "")
    macro = hy_mirror.hy_macro_step_trace(hy_source, timeout=timeout)

    run: dict[str, Any] = {"traced": False}
    try:
        tree = _strip_hy_scaffolding(_pyast.parse(python_source))
        if len(tree.body) == 1 and isinstance(tree.body[0], _pyast.Expr):
            expr_src = _pyast.unparse(tree.body[0].value)
            tr = hy_mirror.execution_trace(expr_src, "eval")
            run = {
                "traced": True,
                "result": tr.get("result_repr"),
                "opcodes_executed": tr.get("executed_opcode_count"),
                "top_opcodes": list(tr.get("opcode_histogram", {}).items())[:5],
            }
        else:
            run["reason"] = "not a single expression"
    except Exception as exc:  # noqa: BLE001
        run = {"traced": False, "error": f"{type(exc).__name__}: {exc}"}

    synth = synthesize_pnix_from_hy(hy_source, timeout=timeout)
    roundtrip = hy_to_pnix_value_roundtrip(hy_source, timeout=timeout)
    closure = hy_projection_closure(hy_source, timeout=timeout)

    collapse: dict[str, Any] | None = None
    if synth.get("synthesizable"):
        sp = specialize_pnix(synth["pnix_source"])
        collapse = {
            "specialized_from": synth["pnix_source"],
            "residual_hy": sp.get("residual_hy"),
            "fully_static": sp.get("fully_static"),
            "value": sp.get("value"),
        }

    macro_towers = [
        {"head": f.get("head"), "is_macro": f.get("is_macro"),
         "tower": [s.get("head") for s in f.get("steps") or []]}
        for f in (macro.get("forms") or [])
    ]
    return {
        "schema": "pnix-hy.meta-circular-tower.v0",
        "hy_source": hy_source,
        "stages": {
            "read": {
                "reader_form_count": front.get("reader_form_count"),
                "reader_forms": front.get("reader_forms"),
            },
            "compile": {
                "python_source": python_source,
                "macro_towers": macro_towers,
            },
            "run": run,
            "pnix": {
                "synth_pnix": synth.get("pnix_source"),
                "synthesizable": synth.get("synthesizable"),
                "value_preserved": roundtrip.get("meaning_preserved"),
                "comparable": roundtrip.get("comparable"),
                "closed": closure.get("closed"),
            },
            "collapse": collapse,
        },
    }


def meta_circular_tower_report() -> dict[str, Any]:
    """Self-check: a closed arithmetic fragment populates EVERY tower stage, preserves
    meaning into pnix, and COLLAPSES (specializes) to its value with no residue."""
    probe = "(+ (* 2 3) 4)"
    try:
        t = meta_circular_tower(probe)
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.meta-circular-tower.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    s = t.get("stages") or {}
    read, comp, run, pnix, collapse = (s.get("read") or {}, s.get("compile") or {},
                                       s.get("run") or {}, s.get("pnix") or {},
                                       s.get("collapse") or {})
    ready = (
        bool(read.get("reader_forms"))
        and "*" in (comp.get("python_source") or "") and "+" in (comp.get("python_source") or "")
        and run.get("traced") is True and (run.get("opcodes_executed") or 0) > 0
        and pnix.get("synth_pnix") == "((2 * 3) + 4)"
        and pnix.get("value_preserved") is True
        and pnix.get("closed") is True
        and collapse.get("fully_static") is True
        and collapse.get("value") == 10
        and collapse.get("residual_hy") == "10"
    )
    return {
        "schema": "pnix-hy.meta-circular-tower.report.v0",
        "ready": bool(ready),
        "available": True,
        "read_forms": read.get("reader_form_count"),
        "run_opcodes": run.get("opcodes_executed"),
        "synth_pnix": pnix.get("synth_pnix"),
        "closed": pnix.get("closed"),
        "collapse_value": collapse.get("value"),
    }


# ============================================================================
# Module (multi-form) projection: every top-level Hy form + its pnix correspondence
#
# Lifts the projection from a single fragment to a whole .hy module (P4 / closes the
# forms[0]-only A3 gap). For each top-level form: reader model, its own Python lowering,
# its own macro layer (from hy_mirror.hy_module_projection), plus the pnix correspondence
# alignment of that form's lowered Python. The safe, opt-in realization of "project a
# module" (the MacroPy/PEP-451 import-hook pattern's payload) -- no global sys.meta_path
# mutation; you call it on a file/source explicitly.
# ============================================================================


def project_hy_module(source: str, *, timeout: int = 90) -> dict[str, Any]:
    """Project every top-level Hy form of a module + each form's pnix correspondence.
    Schema `pnix-hy.hy-module-pnix-projection.v0`."""
    mod = hy_mirror.hy_module_projection(source, timeout=timeout)
    forms_out: list[dict[str, Any]] = []
    for entry in mod.get("forms", []):
        py = entry.get("python_source")
        alignment = align_python_to_pnix(py) if py else None
        forms_out.append({
            "head": entry.get("head"),
            "reader_form": entry.get("reader_form"),
            "python_source": py,
            "is_macro": entry.get("is_macro"),
            "expanded_head": entry.get("expanded_head"),
            "pnix_alignment": alignment,
        })
    return {
        "schema": "pnix-hy.hy-module-pnix-projection.v0",
        "form_count": mod.get("form_count"),
        "forms": forms_out,
    }


def project_hy_file(path: str, *, timeout: int = 90) -> dict[str, Any]:
    """project_hy_module for a .hy file on disk."""
    with open(path, "r", encoding="utf-8") as handle:
        out = project_hy_module(handle.read(), timeout=timeout)
    out["path"] = path
    return out


def project_hy_module_report() -> dict[str, Any]:
    """Self-check: a 2-form module yields 2 per-form projections, each with a pnix
    alignment, and the first (a defn) aligns to a lambda."""
    probe = "(defn f [x] (+ x 1))\n(setv y (f 41))"
    try:
        proj = project_hy_module(probe)
    except hy_mirror.HyMirrorError as exc:
        return {"schema": "pnix-hy.hy-module-pnix-projection.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    forms = proj.get("forms") or []
    first_tags = (forms[0]["pnix_alignment"] or {}).get("pnix_tags_hit", []) if forms else []
    ready = (
        proj.get("form_count") == 2
        and all(f.get("pnix_alignment") for f in forms)
        and "lambda" in first_tags
    )
    return {
        "schema": "pnix-hy.hy-module-pnix-projection.report.v0",
        "ready": ready,
        "available": True,
        "probe": probe,
        "form_count": proj.get("form_count"),
        "heads": [f.get("head") for f in forms],
        "first_form_tags": first_tags,
    }


# ============================================================================
# P5: Futamura 1st-projection prototype — tag-based partial evaluation of pnix
#
# Specialize the pnix interpreter w.r.t. a fixed pnix program: treat chosen free vars as
# DYNAMIC (the program's "input") and everything else STATIC, then fold the static parts
# to values and emit only the residual computation as Hy. The static AST-walk / eval
# dispatch disappears -- compiler-by-specialization (the 1st Futamura projection), here
# as a source-to-source partial evaluator over pnix AST tags (no Scala Rep[T]; pnix tags
# ARE the binding-time structure). Static folding delegates to the real pnix runtime
# (rt.apply_binary / rt.eval_source) so it can't diverge from pnix semantics.
#
# Jones-optimality analogue: a CLOSED program (no dynamic vars) specializes to its VALUE
# (a literal) -- no interpretive residue. specialization_roundtrip checks the residual,
# given the dynamic inputs, computes the same value as the original.
# ============================================================================


class _Closure:
    """A statically-known pnix lambda captured during partial evaluation (PP7). Carries the
    param, body AST, and the static env captured at definition -- so `apply` can beta-reduce
    when the argument is also static (closed function-heavy programs then fold fully)."""

    __slots__ = ("param", "body", "env")

    def __init__(self, param: Any, body: Any, env: dict[str, Any]) -> None:
        self.param = param
        self.body = body
        self.env = env


def _closure_to_hy(clo: "_Closure", dyn: set[str], gaps: list[dict[str, Any]]) -> str:
    """Residualize a closure as a Hy `(fn [param] body)` -- the param stays free (a Hy
    lambda param), captured static vars in the body fold away."""
    env2 = {k: v for k, v in clo.env.items() if k != clo.param}  # param shadows any capture
    body = _pe(clo.body, env2, dyn, gaps)
    return "(fn [" + str(clo.param) + "] " + _side(body) + ")"


def _value_to_hy(value: Any) -> str:
    if isinstance(value, bool):
        return "True" if value else "False"
    if isinstance(value, (int, float)):
        return str(value)
    if value is None:
        return "None"
    if isinstance(value, str):
        return _hy_str_literal(value)
    if isinstance(value, list):
        return "[" + " ".join(_value_to_hy(x) for x in value) + "]"
    if isinstance(value, dict):
        return "{" + " ".join(_hy_str_literal(str(k)) + " " + _value_to_hy(v) for k, v in value.items()) + "}"
    if isinstance(value, _Closure):
        return _closure_to_hy(value, set(), [])
    return repr(value)


def _merge_nested_dict(target: dict[str, Any], path: list[Any], value: Any) -> None:
    cur = target
    for key in path[:-1]:
        k = str(key)
        next_value = cur.get(k)
        if not isinstance(next_value, dict) or "tag" in next_value:
            next_value = {}
            cur[k] = next_value
        cur = next_value
    cur[str(path[-1])] = value


def _side(pe_result: tuple[str, Any]) -> str:
    """Render a partial-eval result as Hy: static -> literal, dynamic -> its residual."""
    kind, payload = pe_result
    return _value_to_hy(payload) if kind == "S" else payload


def _pe(node: Any, env: dict[str, Any], dyn: set[str], gaps: list[dict[str, Any]]) -> tuple[str, Any]:
    if not isinstance(node, dict):
        return ("D", repr(node))
    tag = node.get("tag")
    if tag in ("int", "float", "string", "bool"):
        return ("S", node.get("value"))
    if tag == "null":
        return ("S", None)
    if tag == "var":
        name = node.get("name")
        if name in env and name not in dyn:
            return ("S", env[name])
        return ("D", str(name))
    if tag == "binary":
        lhs = _pe(node.get("lhs"), env, dyn, gaps)
        rhs = _pe(node.get("rhs"), env, dyn, gaps)
        op = node.get("op")
        if lhs[0] == "S" and rhs[0] == "S":
            try:
                return ("S", rt.stable_data(rt.apply_binary(op, lhs[1], rhs[1])))
            except Exception:  # noqa: BLE001 - non-foldable -> residualize
                pass
        hyop = _PNIX_OP_TO_HY.get(op)
        if hyop is None:
            gaps.append({"tag": "binary", "op": op, "reason": "no Hy operator"})
            return ("D", "#_pnix-binary[" + str(op) + "]")
        return ("D", "(" + hyop + " " + _side(lhs) + " " + _side(rhs) + ")")
    if tag == "unary":
        arg = _pe(node.get("arg"), env, dyn, gaps)
        op = node.get("op")
        if arg[0] == "S":
            try:
                if op == "-":
                    return ("S", rt.stable_data(rt.apply_binary("-", 0, arg[1])))
                if op == "!":
                    return ("S", not arg[1])
            except Exception:  # noqa: BLE001
                pass
        hyop = {"-": "-", "!": "not"}.get(op)
        if hyop is None:
            gaps.append({"tag": "unary", "op": op})
            return ("D", "#_pnix-unary[" + str(op) + "]")
        return ("D", "(" + hyop + " " + _side(arg) + ")")
    if tag == "if":
        cond = _pe(node.get("cond"), env, dyn, gaps)
        if cond[0] == "S":
            if not isinstance(cond[1], bool):
                gaps.append({"tag": "if", "reason": "if condition is not bool"})
                return ("D", "(if " + _side(cond) + " "
                        + _side(_pe(node.get("then"), env, dyn, gaps)) + " "
                        + _side(_pe(node.get("else"), env, dyn, gaps)) + ")")
            branch = node.get("then") if cond[1] else node.get("else")
            return _pe(branch, env, dyn, gaps)  # dead branch pruned for bool cond
        return ("D", "(if " + _side(cond) + " "
                + _side(_pe(node.get("then"), env, dyn, gaps)) + " "
                + _side(_pe(node.get("else"), env, dyn, gaps)) + ")")
    if tag == "let":
        bindings = list(node.get("bindings", []))
        path_binding_names: list[str] = []
        for b in bindings:
            path = b.get("path", [])
            if len(path) != 1:
                sub_gaps: list[dict[str, Any]] = []
                hy = _pnix_to_hy(node, sub_gaps)
                gaps.extend(sub_gaps)
                gaps.append({"tag": "let", "reason": "nested-path let binding not specialized"})
                return ("D", hy)
            path_binding_names.append(str(path[0]))

        let_names = set(path_binding_names)
        outer_env = dict(env)
        for name in let_names:
            outer_env.pop(name, None)
        unresolved = set(let_names)
        pending = {str(b.get("path", [None])[0]): b for b in bindings}
        resolved: dict[str, Any] = {}
        blocked = set(unresolved)

        while unresolved:
            progress = False
            for name in list(unresolved):
                binding = pending[name]
                value = _pe(binding.get("value"), {**outer_env, **resolved}, dyn | blocked, gaps)
                if value[0] == "S":
                    resolved[name] = value[1]
                    unresolved.remove(name)
                    blocked.remove(name)
                    progress = True
            if not progress:
                break

        for name in list(path_binding_names):
            if name not in unresolved:
                outer_env[name] = resolved[name]

        body = _pe(node.get("body"), outer_env, dyn | blocked, gaps)
        if not unresolved:
            return body

        residual: list[tuple[str, Any]] = []
        for name in path_binding_names:
            binding = pending[name]
            if name not in unresolved:
                continue
            value = _pe(binding.get("value"), outer_env, dyn | blocked, gaps)
            residual.append((name, value))
        residual_names = [name for name, _ in residual if name in unresolved]
        if not residual_names:
            gaps.append({"tag": "let", "reason": "let-recursive-not-static", "names": sorted(unresolved)})
            return body

        residual_lines = [str(name) + " " + _side(value) for name, value in residual]
        if unresolved:
            gaps.append({"tag": "let", "reason": "let-recursive-not-static", "names": sorted(unresolved)})
        return ("D", "(let [" + " ".join(residual_lines) + "] " + _side(body) + ")")

    if tag == "list":
        parts = [_pe(x, env, dyn, gaps) for x in node.get("items", [])]
        if all(p[0] == "S" for p in parts):
            return ("S", [p[1] for p in parts])
        return ("D", "[" + " ".join(_side(p) for p in parts) + "]")
    if tag == "attrset" and not node.get("recursive"):
        out: dict[str, Any] = {}
        res_pairs: list[str] = []
        all_static = True
        for b in node.get("bindings", []):
            path = b.get("path", [])
            if not path:
                all_static = False
                continue
            v = _pe(b.get("value"), env, dyn, gaps)
            if v[0] == "S":
                _merge_nested_dict(out, path, v[1])
            else:
                all_static = False
            res_pairs.append(_hy_str_literal(str(path[0])) + " " + _side(v))
        if all_static:
            return ("S", out)
        return ("D", "{" + " ".join(res_pairs) + "}")
    if tag == "lambda":
        # a lambda is a statically-known closure capturing the current static env
        return ("S", _Closure(node.get("param"), node.get("body"), dict(env)))
    if tag == "apply":
        func = _pe(node.get("func"), env, dyn, gaps)
        arg = _pe(node.get("arg"), env, dyn, gaps)
        if func[0] == "S" and isinstance(func[1], _Closure) and arg[0] == "S":
            clo = func[1]  # beta-reduce: bind param -> static arg, specialize the body
            new_env = {k: v for k, v in clo.env.items() if k != clo.param}
            new_env[clo.param] = arg[1]
            return _pe(clo.body, new_env, dyn, gaps)
        # otherwise residualize the application (curried juxtaposition in Hy)
        return ("D", "(" + _side(func) + " " + _side(arg) + ")")
    # not specialized: residualize the whole subtree opaquely (still correct Hy)
    sub_gaps: list[dict[str, Any]] = []
    hy = _pnix_to_hy(node, sub_gaps)
    gaps.extend(sub_gaps)
    gaps.append({"tag": tag, "reason": "not specialized; residualized opaquely"})
    return ("D", hy)


_specialize_cache: dict[tuple[str, tuple[str, ...]], dict[str, Any]] = {}


def specialize_pnix(source: str, dynamic_vars: tuple[str, ...] = (), *,
                    assumptions: dict[str, Any] | None = None,
                    boundaries: tuple[str, ...] = ()) -> dict[str, Any]:
    """Partially evaluate a pnix program with `dynamic_vars` held dynamic: fold the static
    parts to values (incl. beta-reducing lambda applications with static args), emit the
    residual computation as Hy. Deterministic, so results are memoized by (canonical source,
    dynamic_vars, assumptions, boundaries) -- the "compile a fixed DSL program once" path.

    0025 (T4, Truffle-style PE primitives; defaults keep the original behavior byte-identical):
    - `assumptions`: name -> ASSUMED value, treated static and recorded in the result -- a
      speculative specialization that `assumptions_valid`/`respecialize_if_drifted` can deopt.
    - `boundaries`: names the PE must NOT fold through (PEBoundary): forced dynamic even when
      statically known. Schema `pnix-hy.specialize.v0`."""
    assumptions = dict(assumptions or {})
    ast = rt.parse(source)
    extras = (tuple(sorted((k, repr(v)) for k, v in assumptions.items())), tuple(sorted(boundaries)))
    try:
        key = (rt.emit_source(ast), tuple(dynamic_vars), *extras)
    except Exception:  # noqa: BLE001
        key = (source, tuple(dynamic_vars), *extras)
    cached = _specialize_cache.get(key)
    if cached is not None:
        return dict(cached, cached=True)
    gaps: list[dict[str, Any]] = []
    # an assumption SPECULATIVELY makes an otherwise-dynamic name static; a boundary always wins
    dyn = (set(dynamic_vars) - set(assumptions)) | set(boundaries)
    seed_env = {k: v for k, v in assumptions.items() if k not in boundaries}
    kind, payload = _pe(ast, seed_env, dyn, gaps)
    result: dict[str, Any] = {
        "schema": "pnix-hy.specialize.v0",
        "source": source,
        "dynamic_vars": list(dynamic_vars),
        "assumptions": assumptions,
        "boundaries": sorted(boundaries),
        "fully_static": kind == "S",
        "residual_hy": _value_to_hy(payload) if kind == "S" else payload,
        "gaps": gaps,
    }
    if kind == "S":
        if isinstance(payload, _Closure):
            result["value_is_function"] = True  # a residual function, not a data value
        else:
            result["value"] = payload
    if len(_specialize_cache) < 4096:
        _specialize_cache[key] = result
    return dict(result, cached=False)


# ============================================================================
# Hy macro/quasiquote OVER pnix (proposal 0003, candidates C1/C2/C3)
#
# The SCOPE_LOCK-blessed macro/quote interop directions. pnix stays non-homoiconic (no pnix
# macro/quasiquote -- SCOPE_LOCK §3): every macro/quote below is HY's, either operating on a
# pnix-projected form (C1) or fed from a pnix value (C2), with the staging correspondence made
# executable (C3). Composes pnix_to_hy_form / _value_to_hy / specialize_pnix (pnix side) with
# hy_macroexpand_projection / hy_eval_form / hy_quasiquote_projection (Hy proof subprocess).
# ============================================================================

def hy_macro_over_pnix(pnix_source: str, macro_wrap: str = "(when True {form})", *,
                       timeout: int = 60) -> dict[str, Any]:
    """C1: project a pnix expression to a Hy form, apply a Hy MACRO over it, and return the
    macroexpansion (best-effort re-synthesizing the expansion back to pnix). `macro_wrap` is a
    Hy template with `{form}` where the projected form goes. Adds NO pnix macro -- the macro is
    Hy's, operating on pnix-projected code. Schema `pnix-hy.hy-macro-over-pnix.v0`."""
    proj = pnix_to_hy_form(pnix_source)
    hy_form = proj["hy_source"]
    composed = macro_wrap.replace("{form}", hy_form)  # not .format: Hy {..} literals aren't fields
    macro = hy_mirror.hy_macroexpand_projection(composed, timeout=timeout)
    pnix_of_expansion: str | None = None
    try:
        tree = _strip_hy_scaffolding(_pyast.parse(macro.get("fully_expanded_python", "")))
        egaps: list[dict[str, Any]] = []
        pnix_of_expansion = _python_module_to_pnix(tree.body, egaps)
    except Exception:  # noqa: BLE001 - re-synthesis of the expansion is best-effort
        pnix_of_expansion = None
    return {
        "schema": "pnix-hy.hy-macro-over-pnix.v0",
        "pnix_source": pnix_source,
        "projected_hy_form": hy_form,
        "projection_gaps": proj["gaps"],
        "composed_hy": composed,
        "is_macro": macro.get("is_macro"),
        "expanded_full": macro.get("expanded_full"),
        "fully_expanded_python": macro.get("fully_expanded_python"),
        "pnix_of_expansion": pnix_of_expansion,
        "expansion_synthesizable": bool(pnix_of_expansion) and "#_pnix-gap" not in pnix_of_expansion,
    }


def hy_quasiquote_over_pnix(template: str, holes: dict[str, str], *,
                            timeout: int = 60) -> dict[str, Any]:
    """C2: fill a Hy quasiquote TEMPLATE's unquote hole(s) with pnix VALUE(s) and evaluate it,
    returning the constructed Hy form. `holes` maps each unquoted hole var to a pnix source;
    each is evaluated (host pnix), converted to Hy via `_value_to_hy`, bound, then the
    quasiquote is evaluated in the Hy proof subprocess. pnix stays non-homoiconic; the
    quasiquote is Hy's, its holes fed from pnix values. Schema
    `pnix-hy.hy-quasiquote-over-pnix.v0`."""
    bound: dict[str, str] = {var: _value_to_hy(rt.eval_source(psrc)) for var, psrc in holes.items()}
    setvs = " ".join(f"(setv {var} {val})" for var, val in bound.items())
    program = f"(do {setvs} {template})"
    evaluated = hy_mirror.hy_eval_form(program, timeout=timeout)
    return {
        "schema": "pnix-hy.hy-quasiquote-over-pnix.v0",
        "template": template,
        "hole_bindings": bound,
        "program": program,
        "result": evaluated.get("result"),
        "result_repr": evaluated.get("result_repr"),
    }


def quasiquote_specialize_correspondence(hy_template: str, pnix_source: str,
                                         dynamic_vars: tuple[str, ...] = (), *,
                                         timeout: int = 60) -> dict[str, Any]:
    """C3: make the 'a quasiquote is manual staging; specialize_pnix is automatic staging'
    analogy (hy_mirror._QUASIQUOTE_PNIX_NOTE) EXECUTABLE. The Hy quasiquote hole vars must
    correspond to the pnix dynamic vars (dynamic side) and the static skeleton to the folded
    residual (static side). Schema `pnix-hy.quasiquote-specialize-correspondence.v0`."""
    qq = hy_mirror.hy_quasiquote_projection(hy_template, timeout=timeout)
    spec = specialize_pnix(pnix_source, tuple(dynamic_vars))
    hole_vars = sorted({
        (h.get("inserted") or {}).get("name")
        for h in (qq.get("holes") or [])
        if (h.get("inserted") or {}).get("model") == "Symbol"
    })
    dyn = sorted(set(dynamic_vars))
    holes_match = hole_vars == dyn
    static_ok = bool(qq.get("static_skeleton_nodes", 0)) and not spec.get("fully_static")
    return {
        "schema": "pnix-hy.quasiquote-specialize-correspondence.v0",
        "hy_template": hy_template,
        "pnix_source": pnix_source,
        "quasiquote_hole_vars": hole_vars,
        "pnix_dynamic_vars": dyn,
        "holes_match_dynamic_vars": holes_match,
        "quasiquote_static_nodes": qq.get("static_skeleton_nodes"),
        "pnix_residual_hy": spec.get("residual_hy"),
        "pnix_fully_static": spec.get("fully_static"),
        "static_skeleton_corresponds": static_ok,
        "corresponds": bool(holes_match and static_ok),
    }


def hy_macro_quasiquote_over_pnix_report() -> dict[str, Any]:
    """Self-check for proposal 0003 (C1/C2/C3): a Hy macro applies over a pnix-projected form; a
    pnix value fills a Hy quasiquote hole; the quasiquote<->specialize staging correspondence
    holds."""
    try:
        c1 = hy_macro_over_pnix("1 + 2")
        c1_head = ((c1.get("expanded_full") or {}).get("items") or [{}])[0].get("name")
        c1_ok = (c1.get("projected_hy_form") == "(+ 1 2)"
                 and c1.get("composed_hy") == "(when True (+ 1 2))"
                 and c1.get("is_macro") is True and c1_head == "if")
        c2 = hy_quasiquote_over_pnix("`(sum ~a ~b)", {"a": "1 + 2", "b": "10"})
        items = (c2.get("result") or {}).get("items") or []
        c2_ok = (len(items) == 3 and items[0].get("name") == "sum"
                 and items[1].get("repr") == "3" and items[2].get("repr") == "10")
        c3 = quasiquote_specialize_correspondence("`(+ ~x 10)", "x + 10", ("x",))
        c3_ok = (c3.get("corresponds") is True
                 and c3.get("quasiquote_hole_vars") == ["x"]
                 and c3.get("pnix_dynamic_vars") == ["x"])
        ready = bool(c1_ok and c2_ok and c3_ok)
        return {"schema": "pnix-hy.hy-macro-quasiquote-over-pnix.report.v0", "ready": ready,
                "available": True, "c1_macro_over_pnix": c1_ok,
                "c2_value_into_quasiquote": c2_ok, "c3_staging_correspondence": c3_ok}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.hy-macro-quasiquote-over-pnix.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def hy_reader_embed_pnix(hy_source: str, *, timeout: int = 60) -> dict[str, Any]:
    """C4: a Hy `#px "..."` reader macro that embeds pnix at READ time. In the Hy proof
    subprocess `#px <form>` reads to `(pnix-eval <form>)`; pnix-hy then supplies the pnix
    semantics -- evaluating each embedded fragment (rt.eval_source) and projecting it to its Hy
    form (pnix_to_hy_form). Hy's reader machinery embeds pnix; pnix stays non-homoiconic. Schema
    `pnix-hy.hy-reader-embed-pnix.v0`."""
    out = hy_mirror.hy_read_with_pnix_reader(hy_source, timeout=timeout)
    embeddings: list[dict[str, Any]] = []
    for pnix_src in out.get("embedded_pnix", []):
        entry: dict[str, Any] = {"pnix_source": pnix_src}
        try:
            entry["pnix_value"] = rt.stable_data(rt.eval_source(pnix_src))
        except Exception as exc:  # noqa: BLE001
            entry["pnix_value"] = {"error": f"{type(exc).__name__}: {exc}"}
        try:
            entry["hy_form"] = pnix_to_hy_form(pnix_src)["hy_source"]
        except Exception:  # noqa: BLE001
            entry["hy_form"] = None
        embeddings.append(entry)
    return {
        "schema": "pnix-hy.hy-reader-embed-pnix.v0",
        "hy_source": hy_source,
        "read_forms": out.get("read_forms"),
        "embedding_count": len(embeddings),
        "embeddings": embeddings,
    }


def hy_reader_embed_pnix_report() -> dict[str, Any]:
    """Self-check: `(+ 10 #px "1 + 2")` embeds pnix `1 + 2` (value 3, Hy form `(+ 1 2)`)."""
    try:
        r = hy_reader_embed_pnix('(+ 10 #px "1 + 2")')
        emb = r.get("embeddings") or []
        ready = (r.get("embedding_count") == 1 and emb[0].get("pnix_source") == "1 + 2"
                 and emb[0].get("pnix_value") == 3 and emb[0].get("hy_form") == "(+ 1 2)")
        return {"schema": "pnix-hy.hy-reader-embed-pnix.report.v0", "ready": bool(ready),
                "available": True, "embedding_count": r.get("embedding_count"),
                "embedding": emb[0] if emb else None}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.hy-reader-embed-pnix.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def specialization_roundtrip(source: str, bindings: dict[str, str] | None = None) -> dict[str, Any]:
    """Jones-optimality-style check: specialize `source` (with bindings' keys dynamic),
    then verify the residual -- given those inputs -- computes the same value as the
    original program with the same inputs. `bindings` maps a dynamic var name to a pnix
    literal source (e.g. {"x": "41"}); empty/None = closed-program optimality check.
    Schema `pnix-hy.specialize.roundtrip.v0`."""
    bindings = bindings or {}
    spec = specialize_pnix(source, tuple(bindings))
    result: dict[str, Any] = {
        "schema": "pnix-hy.specialize.roundtrip.v0",
        "source": source,
        "bindings": bindings,
        "fully_static": spec["fully_static"],
        "residual_hy": spec["residual_hy"],
    }
    # original value with the inputs bound
    if bindings:
        prefix = "".join(f"{k} = {v}; " for k, v in bindings.items())
        original_src = f"let {prefix}in {source}"
    else:
        original_src = source
    try:
        original_value = rt.stable_data(rt.eval_source(original_src))
    except Exception as exc:  # noqa: BLE001
        result["comparable"] = False
        result["reason"] = f"original pnix eval error: {type(exc).__name__}: {exc}"
        return result
    result["original_value"] = original_value
    if "#_pnix-" in spec["residual_hy"]:
        result["comparable"] = False
        result["reason"] = "residual has unprojectable placeholders"
        return result
    # evaluate the residual Hy in the stage7 kernel, with the dynamic inputs bound
    if bindings:
        hy_binds = " ".join(f"{k} {_value_to_hy(rt.stable_data(rt.eval_source(v)))}" for k, v in bindings.items())
        residual_prog = f"(let [{hy_binds}] {spec['residual_hy']})"
    else:
        residual_prog = spec["residual_hy"]
    try:
        raw = hy_mirror.stage7_eval("(do (import json) (json.dumps " + residual_prog + "))")
        residual_value = _json.loads(raw)
    except Exception as exc:  # noqa: BLE001
        result["comparable"] = False
        result["reason"] = f"residual eval error: {type(exc).__name__}: {exc}"
        return result
    result["residual_value"] = residual_value
    result["comparable"] = True
    result["meaning_preserved"] = original_value == residual_value
    return result


def specialize_report() -> dict[str, Any]:
    """Self-check: (a) Jones-optimality core -- a closed program specializes to its value
    literal; (b) a static `let` binding is folded away leaving only the dynamic residue;
    (c) the residual round-trips meaning-preserving."""
    closed = specialize_pnix("let a = 1; in a + (a * 2)")
    opt_ok = closed["fully_static"] and closed.get("value") == 3 and closed["residual_hy"] == "3"
    opened = specialize_pnix("let a = 1; in a + x", ("x",))
    fold_ok = (not opened["fully_static"]) and opened["residual_hy"] == "(+ 1 x)"
    # PP7: beta-reduce lambda applications -- closed function programs collapse fully.
    beta = specialize_pnix("(x: x + 1) 41")
    beta_ok = beta["fully_static"] and beta.get("value") == 42
    curry = specialize_pnix("((a: (b: a + b)) 3) 4")
    curry_ok = curry["fully_static"] and curry.get("value") == 7
    partial = specialize_pnix("(x: x + 1) y", ("y",))   # dynamic arg -> residual function applied
    partial_ok = (not partial["fully_static"]) and partial["residual_hy"] == "((fn [x] (+ x 1)) y)"
    return {
        "schema": "pnix-hy.specialize.report.v0",
        "ready": opt_ok and fold_ok and beta_ok and curry_ok and partial_ok,
        "available": True,
        "jones_optimality_closed": {"residual": closed["residual_hy"], "value": closed.get("value"), "ok": opt_ok},
        "static_let_folded": {"residual": opened["residual_hy"], "ok": fold_ok},
        "beta_reduce": {"residual": beta["residual_hy"], "value": beta.get("value"), "ok": beta_ok},
        "curried_fold": {"residual": curry["residual_hy"], "value": curry.get("value"), "ok": curry_ok},
        "lambda_partial": {"residual": partial["residual_hy"], "ok": partial_ok},
    }


# --- D2: live meta-circular observation -----------------------------------------
# The static toolkit shows what a fragment lowers TO (ast/dis/marshal). This shows
# what the pnix meta-circular evaluator actually DOES: trace the HOST evaluation of a
# pnix form at the Python-bytecode level, restricted to pnix_runtime's own code
# objects, so the developer sees the real execution footprint of "pnix is evaluated
# by a Python-ecosystem program" -- the tokenizer/parser/eval functions and opcodes
# that genuinely run for a given pnix source. The Hy counterpart is
# hy_mirror.execution_trace (lowered-Python opcode trace).

def pnix_evaluation_trace(source: str, *, top: int = 12) -> dict[str, Any]:
    """Trace the host pnix evaluation of `source` at the Python-opcode level,
    restricted to the pnix_runtime module (the meta-circular evaluator itself).

    schema pnix-hy.pnix-evaluation-trace.v0
    """
    rt_file = getattr(rt, "__file__", None)
    entered: dict[str, int] = {}
    op_hist: dict[str, int] = {}
    instr_cache: dict[Any, dict[int, str]] = {}
    state = {"calls": 0, "opcodes": 0}

    def _instrs(c: Any) -> dict[int, str]:
        m = instr_cache.get(c)
        if m is None:
            m = {i.offset: i.opname for i in _dis.get_instructions(c)}
            instr_cache[c] = m
        return m

    def tracer(frame, event, arg):  # noqa: ANN001
        if frame.f_code.co_filename != rt_file:
            return None
        if event == "call":
            name = frame.f_code.co_name
            entered[name] = entered.get(name, 0) + 1
            state["calls"] += 1
            frame.f_trace_opcodes = True
            return tracer
        if event == "opcode":
            op = _instrs(frame.f_code).get(frame.f_lasti, "?")
            op_hist[op] = op_hist.get(op, 0) + 1
            state["opcodes"] += 1
        return tracer

    value = None
    error = None
    prev = _sys.gettrace()
    _sys.settrace(tracer)
    try:
        raw = rt.eval_source(source)
        value = rt.stable_data(raw)
        value_json = rt.to_json_string_value(raw)
    except Exception as exc:  # noqa: BLE001
        error = f"{type(exc).__name__}: {exc}"
        value_json = None
    finally:
        _sys.settrace(prev)

    top_fns = sorted(entered.items(), key=lambda kv: (-kv[1], kv[0]))[:top]
    top_ops = sorted(op_hist.items(), key=lambda kv: (-kv[1], kv[0]))[:top]
    return {
        "schema": "pnix-hy.pnix-evaluation-trace.v0",
        "source": source,
        "value": value,
        "value_json": value_json,
        "error": error,
        "runtime_functions_distinct": len(entered),
        "runtime_calls_total": state["calls"],
        "runtime_opcodes_total": state["opcodes"],
        "top_functions": [{"name": n, "calls": c} for n, c in top_fns],
        "top_opcodes": [{"opname": o, "count": c} for o, c in top_ops],
    }


def pnix_evaluation_trace_report() -> dict[str, Any]:
    """Self-check: evaluating a closed pnix arith form drives the real evaluator --
    a nonzero opcode footprint over many runtime functions, correct value."""
    try:
        t = pnix_evaluation_trace("let a = 10; b = 2; in a * b + 1")
        fns = {f["name"] for f in t.get("top_functions", [])}
        ready = (
            t.get("error") is None
            and t.get("value") == 21
            and t.get("runtime_opcodes_total", 0) > 0
            and t.get("runtime_functions_distinct", 0) > 5
            and bool(fns)
        )
        return {"ready": bool(ready), "value": t.get("value"),
                "functions_distinct": t.get("runtime_functions_distinct"),
                "opcodes_total": t.get("runtime_opcodes_total")}
    except Exception as exc:  # noqa: BLE001
        return {"ready": False, "error": f"{type(exc).__name__}: {exc}"}


# ============================================================================
# PP1 -- SAFE EVALUATION SANDBOX (production: evaluate untrusted pnix logic)
#
# pnix is pure (no side effects), so evaluating a pnix term is a sandbox by construction --
# unlike Python `eval`. What pnix lacks for UNTRUSTED input is RESOURCE bounds: it has only
# piecemeal guards (self-recursion, genericClosure depth, genList cap). safe_eval adds, as a
# WRAPPER over rt.eval_source (pnix_runtime untouched), a step/fuel budget + wall-clock
# timeout (enforced via sys.settrace over the runtime's own frames, so it interrupts
# CPU-bound evaluation) + an output-size cap, returning a structured verdict instead of
# hanging/OOMing. This is the foundation of the "safe evaluation of user-supplied logic"
# production use-case.
# ============================================================================

class _StepBudgetExceeded(Exception):
    pass


class _TimeBudgetExceeded(Exception):
    pass


# PP2 -- impurity classifier. pnix's purity is the sandbox; these builtins are the only
# escape hatches (they touch the filesystem / environment / clock / nondeterminism). A
# pure-only gate REJECTS them statically before eval, so "pure sandbox" is provable.
_IMPURE_BUILTINS = _interop.IMPURE_BUILTINS


def _walk_ast(node: Any, fn) -> None:
    if isinstance(node, dict):
        fn(node)
        for v in node.values():
            _walk_ast(v, fn)
    elif isinstance(node, list):
        for x in node:
            _walk_ast(x, fn)


def _static_purity_check_ast(source: str, ast: dict[str, Any]) -> dict[str, Any]:
    result: dict[str, Any] = {"schema": "pnix-hy.static-purity-check.v0", "source": source}
    impure: list[dict[str, Any]] = []
    uncertain: list[dict[str, Any]] = []

    def visit(n: dict[str, Any]) -> None:
        tag = n.get("tag")
        if tag == "import":
            impure.append({"kind": "import", "name": n.get("path")})
        elif tag == "select":
            base = n.get("base") or {}
            if base.get("tag") == "var" and base.get("name") == "builtins":
                attr = n.get("attr")
                if attr in _IMPURE_BUILTINS:
                    impure.append({"kind": "builtin", "name": attr})
        elif tag == "with":
            env = n.get("env") or {}
            if isinstance(env, dict) and env.get("tag") == "var" and env.get("name") == "builtins":
                uncertain.append({"kind": "with-builtins",
                                  "reason": "bare names injected by `with builtins` can't be resolved statically"})

    _walk_ast(ast, visit)
    result.update(pure=not impure and not uncertain, impure_uses=impure, uncertain=uncertain)
    return result


def static_purity_check(source: str) -> dict[str, Any]:
    """Statically classify a pnix program's impurity WITHOUT evaluating it: flag `import`
    (reads+evals a file) and `builtins.<impure>` uses (filesystem/env/clock). `with
    builtins; ...` is reported as `uncertain` (bare names it injects can't be resolved
    statically). `pure` is true only when nothing impure or uncertain is found. Schema
    `pnix-hy.static-purity-check.v0`."""
    result: dict[str, Any] = {"schema": "pnix-hy.static-purity-check.v0", "source": source}
    try:
        ast = rt.parse(source)
    except Exception as exc:  # noqa: BLE001
        result.update(pure=False, parse_error=f"{type(exc).__name__}: {exc}")
        return result
    return _static_purity_check_ast(source, ast)


def static_purity_check_report() -> dict[str, Any]:
    """Self-check: a pure expr is pure; getEnv/readFile/import are flagged impure; a
    `with builtins;` is flagged uncertain."""
    try:
        pure = static_purity_check("let a = 1; in a + a * 2")
        env = static_purity_check('builtins.getEnv "PATH"')
        imp = static_purity_check("import ./x.px")
        wb = static_purity_check("with builtins; getEnv")
        ready = (
            pure.get("pure") is True
            and env.get("pure") is False and env.get("impure_uses", [{}])[0].get("name") == "getEnv"
            and imp.get("pure") is False and imp.get("impure_uses", [{}])[0].get("kind") == "import"
            and wb.get("pure") is False and bool(wb.get("uncertain"))
        )
        return {"schema": "pnix-hy.static-purity-check.report.v0", "ready": bool(ready),
                "available": True, "pure_case": pure.get("pure"),
                "getenv_flagged": env.get("impure_uses"), "import_flagged": imp.get("impure_uses"),
                "with_builtins_uncertain": bool(wb.get("uncertain"))}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.static-purity-check.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def safe_eval(source: str, *, timeout_s: float | None = 5.0, max_steps: int | None = 5_000_000,
              max_output_bytes: int | None = 1_000_000, pure_only: bool = False,
              granted: tuple[str, ...] | list[str] | None = None) -> dict[str, Any]:
    """Evaluate untrusted pnix `source` under resource bounds. Returns a structured verdict
    (never hangs/raises out): {ok, value, value_json, limit_exceeded, error, steps,
    elapsed_s}. `limit_exceeded` is one of timeout/max_steps/max_output_bytes/recursion/
    impure/capability when a bound or gate is hit. Bounds are enforced by tracing the
    runtime's own frames; set a bound to None to disable it (None on all three = fast path,
    no tracing). pure_only=True rejects impure programs (filesystem/env/import) BEFORE eval
    via static_purity_check. Pass granted=(...) to require capability-aware admission via
    gate.gate_check before eval.
    pnix_runtime untouched. Schema `pnix-hy.safe-eval.v0`."""
    result: dict[str, Any] = {"schema": "pnix-hy.safe-eval.v0", "source": source}
    if granted is not None:
        from . import gate as _gate  # noqa: PLC0415 - avoid import cycle at module load
        gate = _gate.gate_check(source, granted=tuple(granted))
        result["gate"] = gate
        if gate.get("parse_error"):
            result.update(ok=False, phase="parse", error=gate.get("parse_error"),
                          steps=0, elapsed_s=0.0)
            return result
        if not gate.get("allowed"):
            result.update(ok=False, limit_exceeded="capability",
                          required_effects=gate.get("required_effects"),
                          granted=gate.get("granted"),
                          denials=gate.get("denials"),
                          uncertain=gate.get("uncertain"),
                          steps=0, elapsed_s=0.0)
            return result
    if pure_only:
        purity = static_purity_check(source)
        if not purity.get("pure"):
            result.update(ok=False, limit_exceeded="impure",
                          impure_uses=purity.get("impure_uses"), uncertain=purity.get("uncertain"),
                          steps=0, elapsed_s=0.0)
            return result
    rt_file = getattr(rt, "__file__", None)
    state = {"steps": 0}
    start = _time.monotonic()
    deadline = (start + timeout_s) if timeout_s else None
    trace_on = bool(deadline) or bool(max_steps)

    def tracer(frame, event, arg):  # noqa: ANN001
        if frame.f_code.co_filename != rt_file:
            return None
        if event == "line":
            state["steps"] += 1
            if max_steps and state["steps"] > max_steps:
                raise _StepBudgetExceeded
            if deadline and (state["steps"] & 0x7FF) == 0 and _time.monotonic() > deadline:
                raise _TimeBudgetExceeded
        return tracer

    raw = None
    prev = _sys.gettrace()
    if trace_on:
        _sys.settrace(tracer)
    try:
        raw = rt.eval_source(source)
        ok = True
    except _StepBudgetExceeded:
        result.update(ok=False, limit_exceeded="max_steps")
        ok = False
    except _TimeBudgetExceeded:
        result.update(ok=False, limit_exceeded="timeout")
        ok = False
    except RecursionError:
        result.update(ok=False, limit_exceeded="recursion")
        ok = False
    except Exception as exc:  # noqa: BLE001 - the pnix program itself errored (a normal outcome)
        result.update(ok=False, error=f"{type(exc).__name__}: {exc}")
        ok = False
    finally:
        if trace_on:
            _sys.settrace(prev)
    result["steps"] = state["steps"]
    if not ok:
        result["elapsed_s"] = round(_time.monotonic() - start, 6)
        return result
    # success: serialize with a size cap (untrusted output must not blow memory downstream)
    try:
        value_json = rt.to_json_string_value(raw)
    except Exception as exc:  # noqa: BLE001 - non-data value (function) is fine, just not JSON
        result.update(ok=True, value=rt.stable_data(raw), value_json=None,
                      note=f"value not JSON-serializable: {type(exc).__name__}")
        result["elapsed_s"] = round(_time.monotonic() - start, 6)
        return result
    nbytes = len(value_json.encode("utf-8"))
    if max_output_bytes and nbytes > max_output_bytes:
        result.update(ok=False, limit_exceeded="max_output_bytes", output_bytes=nbytes)
        result["elapsed_s"] = round(_time.monotonic() - start, 6)
        return result
    result.update(ok=True, value=rt.stable_data(raw), value_json=value_json, output_bytes=nbytes)
    result["elapsed_s"] = round(_time.monotonic() - start, 6)
    return result


def safe_eval_report() -> dict[str, Any]:
    """Self-check: a normal eval succeeds; a tiny step budget, a tiny output cap, and
    infinite recursion each trip the right bound (structured, no hang/raise)."""
    try:
        ok = safe_eval("1 + 2")
        steps = safe_eval("1 + 2 + 3 + 4 + 5", max_steps=3)
        big = safe_eval("builtins.genList (i: i) 200", max_output_bytes=10)
        rec = safe_eval("let f = x: f x; in f 1", timeout_s=2.0)
        denied = safe_eval('builtins.pathExists "/etc/passwd"', granted=())
        granted = safe_eval('builtins.pathExists "/etc/passwd"', granted=("file-read",))
        ready = (
            ok.get("ok") is True and ok.get("value") == 3
            and steps.get("ok") is False and steps.get("limit_exceeded") == "max_steps"
            and big.get("ok") is False and big.get("limit_exceeded") == "max_output_bytes"
            and rec.get("ok") is False and rec.get("limit_exceeded") in ("recursion", "timeout")
            and denied.get("ok") is False and denied.get("limit_exceeded") == "capability"
            and granted.get("ok") is True
        )
        return {
            "schema": "pnix-hy.safe-eval.report.v0",
            "ready": bool(ready),
            "available": True,
            "normal": {"value": ok.get("value"), "steps": ok.get("steps")},
            "step_budget": steps.get("limit_exceeded"),
            "output_cap": big.get("limit_exceeded"),
            "runaway": rec.get("limit_exceeded"),
            "capability_denied": denied.get("limit_exceeded"),
            "capability_granted": granted.get("ok"),
        }
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.safe-eval.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


# ============================================================================
# Capability allocation surfaces -- reify / explain / roundtrip status vocabulary
#
# These are deliberately thin public surfaces over the singleton mirror, IR layer, gate,
# diagnostics, and receipts. They make the allocation checklist concrete without creating
# a second parser, evaluator, mirror, or proof system.
# ============================================================================

def _roundtrip_status(*, comparable: bool, preserved: bool | None = None,
                      gaps: list[dict[str, Any]] | None = None) -> str:
    if not comparable:
        return "held" if gaps else "rejected"
    return "lossless" if preserved else "lossy-ok"


def reify_pnix(source: str) -> dict[str, Any]:
    """Uniform pnix reification surface for source/form/AST/IR/value/effect/witness.

    This projects existing evidence from `singleton_mirror_run`, `pnix_runtime`,
    `pnix_hy.ir`, and `interop.to_host`; it does not own a second runtime path.
    Schema `pnix-hy.reify-pnix.v0`.
    """
    result: dict[str, Any] = {"schema": "pnix-hy.reify-pnix.v0", "source": source}
    try:
        tokens = rt.tokenize(source)
        ast = rt.parse(source)
        emitted = rt.emit_source(ast)
    except Exception as exc:  # noqa: BLE001
        result.update(ok=False, phase="parse", diagnostic=diagnose(source),
                      error=f"{type(exc).__name__}: {exc}")
        return result

    from . import ir as _ir  # noqa: PLC0415 - keep the IR layer independent at import time

    mirror = singleton_mirror_run(source, trace=True)
    purity = static_purity_check(source)
    ir_bundle = _ir.ir_of(source)
    reified: dict[str, Any] = {
        "source": {"text": source, "sha256": _sha256(source)},
        "form": {"canonical": emitted},
        "tokens": {"count": len(tokens)},
        "ast": {"root_tag": ast.get("tag"), "sha256": rt.ast_hash(ast),
                "data": rt.ast_stable(ast)},
        "ir": {"sha256": ir_bundle.get("ir_sha256"), "data": ir_bundle.get("ir")},
        "effect": {"pure": purity.get("pure"), "impure_uses": purity.get("impure_uses"),
                   "uncertain": purity.get("uncertain")},
        "witness": {"run_id": mirror.get("run_id"), "result_sha256": mirror.get("result_sha256")},
        "mirror": {"ok": mirror.get("ok"), "facet_count": len(mirror.get("facets", [])),
                   "facets": mirror.get("facets", [])},
    }
    if mirror.get("ok"):
        value = mirror.get("value")
        try:
            value_data = rt.stable_data(value)
        except Exception as exc:  # noqa: BLE001
            value_data = {"repr": repr(value)[:120], "note": f"not stable-data: {type(exc).__name__}"}
        host_value, record = _interop.to_host(value)
        reified["value"] = {"data": value_data, "sha256": _value_sha256(value)}
        reified["interop"] = {
            "host_value": host_value if not _interop.is_opaque_ref(host_value) else host_value,
            "record": record.to_dict(),
        }
    result.update(ok=bool(mirror.get("ok")), phase=("complete" if mirror.get("ok") else mirror.get("phase")),
                  reified=reified)
    return result


def reify_pnix_report() -> dict[str, Any]:
    try:
        r = reify_pnix("let a = 1; in a + 2")
        reified = r.get("reified") or {}
        ready = (
            r.get("ok") is True
            and (reified.get("ast") or {}).get("root_tag") == "let"
            and (reified.get("ir") or {}).get("sha256") == (reified.get("ast") or {}).get("sha256")
            and (reified.get("effect") or {}).get("pure") is True
            and (reified.get("value") or {}).get("data") == 3
            and (reified.get("mirror") or {}).get("facet_count", 0) >= 6
        )
        return {"schema": "pnix-hy.reify-pnix.report.v0", "ready": bool(ready),
                "available": True, "root_tag": (reified.get("ast") or {}).get("root_tag"),
                "value": (reified.get("value") or {}).get("data"),
                "facet_count": (reified.get("mirror") or {}).get("facet_count")}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.reify-pnix.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


_DRIFT_CATEGORIES = ("no-hy-operator", "no-projection-construct", "construct-gap", "other")


def _drift_category(gap: dict[str, Any]) -> str:
    reason = str(gap.get("reason") or "").lower()
    if "no hy operator" in reason:
        return "no-hy-operator"
    if "no direct" in reason or "not homoiconic" in reason or "no clean" in reason:
        return "no-projection-construct"
    if gap.get("tag"):
        return "construct-gap"
    return "other"


def classify_drift(pnix_source: str) -> dict[str, Any]:
    """C5: read-only reclassification of pnix->Hy projection `gaps` into a stable category enum
    (`_DRIFT_CATEGORIES`), with per-category counts. OBSERVES the honest `#_pnix-gap`/`gaps`
    markers from `pnix_to_hy_form` -- it does NOT fill them (SCOPE_LOCK: they are by design).
    Schema `pnix-hy.classify-drift.v0`."""
    try:
        proj = pnix_to_hy_form(pnix_source)
    except Exception as exc:  # noqa: BLE001 - unparseable source -> diagnostic, don't leak
        return {"schema": "pnix-hy.classify-drift.v0", "pnix_source": pnix_source, "ok": False,
                "diagnostic": diagnose(pnix_source), "error": f"{type(exc).__name__}: {exc}"}
    gaps = proj.get("gaps") or []
    classified = [dict(g, category=_drift_category(g)) for g in gaps]
    counts: dict[str, int] = {}
    for c in classified:
        counts[c["category"]] = counts.get(c["category"], 0) + 1
    return {
        "schema": "pnix-hy.classify-drift.v0",
        "pnix_source": pnix_source,
        "clean": not gaps,
        "drift_count": len(gaps),
        "categories": counts,
        "gaps": classified,
    }


def classify_drift_report() -> dict[str, Any]:
    """Self-check: a clean pnix expr has no drift; `with` classifies as no-projection-construct."""
    try:
        clean = classify_drift("(x: x + 1)")
        gapped = classify_drift("with m; b")
        ready = (clean.get("clean") is True and clean.get("drift_count") == 0
                 and gapped.get("clean") is False
                 and gapped.get("categories", {}).get("no-projection-construct") == 1)
        return {"schema": "pnix-hy.classify-drift.report.v0", "ready": bool(ready), "available": True,
                "clean_drift_count": clean.get("drift_count"), "gapped_categories": gapped.get("categories")}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.classify-drift.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def reify_hy(hy_source: str, *, timeout: int = 60) -> dict[str, Any]:
    """C7: uniform Hy-side reification surface, symmetric to `reify_pnix`. Packages the EXISTING
    Hy projections -- reader form, the Python AST/source it lowers to, the synthesized pnix and
    (when synthesizable) that pnix's IR + value -- into one envelope. Reuses `hy_form_projection`
    + `synthesize_pnix_from_hy`; owns no second projector. Schema `pnix-hy.reify-hy.v0`."""
    result: dict[str, Any] = {"schema": "pnix-hy.reify-hy.v0", "hy_source": hy_source}
    try:
        front = hy_mirror.hy_form_projection(hy_source, timeout=timeout)
    except Exception as exc:  # noqa: BLE001
        result.update(ok=False, phase="hy-form", error=f"{type(exc).__name__}: {exc}")
        return result
    synth = synthesize_pnix_from_hy(hy_source, timeout=timeout)
    pnix_source = synth.get("pnix_source")
    reified: dict[str, Any] = {
        "source": {"text": hy_source, "sha256": _sha256(hy_source)},
        "form": {"reader_forms": front.get("reader_forms"),
                 "reader_form_count": front.get("reader_form_count")},
        "python": {"ast": front.get("python_ast"), "source": front.get("python_source")},
        "pnix": {"source": pnix_source, "synthesizable": synth.get("synthesizable"),
                 "gaps": synth.get("gaps")},
    }
    if synth.get("synthesizable"):
        try:
            from . import ir as _ir  # noqa: PLC0415
            reified["ir"] = {"sha256": _ir.ir_of(pnix_source).get("ir_sha256")}
            value = rt.eval_source(pnix_source)
            reified["value"] = {"data": rt.stable_data(value), "sha256": _value_sha256(value)}
        except Exception as exc:  # noqa: BLE001 - value/IR of the synthesis is best-effort
            reified["value"] = {"note": f"not evaluable: {type(exc).__name__}"}
    result.update(ok=True, phase="complete", reified=reified)
    return result


def reify_hy_report() -> dict[str, Any]:
    """Self-check: `(+ 1 2)` reifies to synthesized pnix `(1 + 2)`, value 3, Python `1 + 2`."""
    try:
        r = reify_hy("(+ 1 2)")
        reified = r.get("reified") or {}
        ready = (r.get("ok") is True
                 and (reified.get("pnix") or {}).get("source") == "(1 + 2)"
                 and (reified.get("value") or {}).get("data") == 3
                 and "1 + 2" in ((reified.get("python") or {}).get("source") or ""))
        return {"schema": "pnix-hy.reify-hy.report.v0", "ready": bool(ready), "available": True,
                "pnix_source": (reified.get("pnix") or {}).get("source"),
                "value": (reified.get("value") or {}).get("data")}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.reify-hy.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def roundtrip_status(source: str) -> dict[str, Any]:
    """Apply the shared pnix roundtrip status vocabulary to the existing roundtrip proofs.
    Schema `pnix-hy.roundtrip-status.v0`."""
    projections = {
        "pnix_to_hy_value": projection_value_roundtrip(source),
    }
    try:
        from . import ir as _ir  # noqa: PLC0415
        projections["ir"] = _ir.ir_roundtrip(source)
    except Exception as exc:  # noqa: BLE001
        projections["ir"] = {"comparable": False, "reason": f"{type(exc).__name__}: {exc}"}
    statuses: dict[str, str] = {}
    pv = projections["pnix_to_hy_value"]
    statuses["pnix_to_hy_value"] = _roundtrip_status(
        comparable=bool(pv.get("comparable")),
        preserved=bool(pv.get("meaning_preserved")),
        gaps=pv.get("gaps"),
    )
    ir = projections["ir"]
    statuses["ir"] = _roundtrip_status(
        comparable=bool(ir.get("comparable")),
        preserved=bool(ir.get("meaning_preserved") and ir.get("hash_stable")),
    )
    return {
        "schema": "pnix-hy.roundtrip-status.v0",
        "source": source,
        "vocabulary": ROUNDTRIP_STATUS_VOCAB,
        "statuses": statuses,
        "projections": projections,
        "lossless": all(v == "lossless" for v in statuses.values()),
    }


def roundtrip_status_report() -> dict[str, Any]:
    try:
        good = roundtrip_status("1 + 2")
        held = roundtrip_status("x: x")
        ready = (
            good.get("lossless") is True
            and set(good.get("vocabulary") or ()) == set(ROUNDTRIP_STATUS_VOCAB)
            and (held.get("statuses") or {}).get("pnix_to_hy_value") in ROUNDTRIP_STATUS_VOCAB
        )
        return {"schema": "pnix-hy.roundtrip-status.report.v0", "ready": bool(ready),
                "available": True, "good_statuses": good.get("statuses"),
                "function_status": (held.get("statuses") or {}).get("pnix_to_hy_value")}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.roundtrip-status.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def explain_pnix(source: str, *, granted: tuple[str, ...] | list[str] | None = None,
                 include_receipt: bool = False) -> dict[str, Any]:
    """Unified pnix debug/explain surface over diagnostics, purity, gate, mirror, and
    optional receipt evidence. Schema `pnix-hy.explain-pnix.v0`."""
    result: dict[str, Any] = {
        "schema": "pnix-hy.explain-pnix.v0",
        "source": source,
        "diagnostic": diagnose(source),
        "purity": static_purity_check(source),
    }
    if granted is not None:
        from . import gate as _gate  # noqa: PLC0415
        result["gate"] = _gate.gate_check(source, granted=tuple(granted))
    if not (result["diagnostic"] or {}).get("ok"):
        result["ok"] = False
        result["phase"] = (result["diagnostic"] or {}).get("phase")
        return result
    if granted is not None and not (result.get("gate") or {}).get("allowed"):
        result["ok"] = False
        result["phase"] = "gate"
        return result
    mirror = singleton_mirror_run(source, trace=False)
    result["mirror"] = {"ok": mirror.get("ok"), "run_id": mirror.get("run_id"),
                        "result_sha256": mirror.get("result_sha256"),
                        "facet_count": len(mirror.get("facets", []))}
    result["safe_eval"] = safe_eval(source, granted=granted)
    if include_receipt:
        result["receipt"] = eval_receipt(source)
    result["ok"] = bool(mirror.get("ok")) and bool((result["safe_eval"] or {}).get("ok"))
    result["phase"] = "complete" if result["ok"] else "eval"
    return result


def explain_pnix_report() -> dict[str, Any]:
    try:
        ok = explain_pnix("1 + 2", granted=())
        parse = explain_pnix("1 +", granted=())
        denied = explain_pnix('builtins.pathExists "/etc/passwd"', granted=())
        ready = (
            ok.get("ok") is True
            and parse.get("ok") is False and parse.get("phase") == "parse"
            and denied.get("ok") is False and denied.get("phase") == "gate"
            and (denied.get("gate") or {}).get("required_effects") == ["file-read"]
        )
        return {"schema": "pnix-hy.explain-pnix.report.v0", "ready": bool(ready),
                "available": True, "ok_phase": ok.get("phase"),
                "parse_phase": parse.get("phase"),
                "denied_effects": (denied.get("gate") or {}).get("required_effects")}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.explain-pnix.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


# ============================================================================
# PP4 -- Content-addressed eval cache (production: cache deterministic pnix by content)
#
# pnix is pure + deterministic, so a PURE program's value is a pure function of its source:
# memoizing it by content is sound. Cache key = the CANONICAL emit of the parsed AST (so
# `1+2`, `1 + 2`, `1  +  2` share one entry). Only PURE programs are cached (PP2's
# static_purity_check gates it -- an impure program's value can change between runs).
# Enables config-build caching and repeated DSL evaluation. The cache lives in-process.
# ============================================================================

_eval_cache: dict[str, Any] = {}
_eval_cache_stats: dict[str, int] = {"hits": 0, "misses": 0, "uncacheable": 0}


def clear_eval_cache() -> None:
    _eval_cache.clear()
    _eval_cache_stats.update(hits=0, misses=0, uncacheable=0)


def eval_cache_stats() -> dict[str, Any]:
    return {"schema": "pnix-hy.eval-cache-stats.v0", "size": len(_eval_cache), **_eval_cache_stats}


def cached_eval(source: str, *, max_entries: int = 4096) -> dict[str, Any]:
    """Evaluate pnix `source`, memoized by canonical content -- but ONLY when the program is
    pure (impure programs are evaluated every time, never cached). Returns {value, cached,
    cacheable, cache_key}. Sound because pure pnix is deterministic. Schema
    `pnix-hy.cached-eval.v0`."""
    result: dict[str, Any] = {"schema": "pnix-hy.cached-eval.v0", "source": source}
    purity = static_purity_check(source)
    if not purity.get("pure"):
        _eval_cache_stats["uncacheable"] += 1
        raw = rt.eval_source(source)
        result.update(value=rt.stable_data(raw), cacheable=False, cached=False,
                      impure_uses=purity.get("impure_uses"))
        return result
    try:
        key = rt.emit_source(rt.parse(source))
    except Exception:  # noqa: BLE001 - fall back to the raw source as the key
        key = source
    if key in _eval_cache:
        _eval_cache_stats["hits"] += 1
        result.update(value=_eval_cache[key], cacheable=True, cached=True, cache_key=key)
        return result
    _eval_cache_stats["misses"] += 1
    raw = rt.eval_source(source)
    value = rt.stable_data(raw)
    if len(_eval_cache) < max_entries:
        _eval_cache[key] = value
    result.update(value=value, cacheable=True, cached=False, cache_key=key)
    return result


def cached_eval_report() -> dict[str, Any]:
    """Self-check: a pure program misses then hits (same value, normalized key); an impure
    program is uncacheable; whitespace-different pure sources share one cache entry."""
    try:
        clear_eval_cache()
        first = cached_eval("1 + 2 * 3")
        second = cached_eval("1 + 2 * 3")
        spaced = cached_eval("1  +  2  *  3")  # same canonical form -> hit
        impure = cached_eval('builtins.getEnv "PATH"')
        stats = eval_cache_stats()
        ready = (
            first.get("cached") is False and first.get("value") == 7
            and second.get("cached") is True and second.get("value") == 7
            and spaced.get("cached") is True
            and impure.get("cacheable") is False
            and stats.get("hits") == 2 and stats.get("misses") == 1
        )
        return {"schema": "pnix-hy.cached-eval.report.v0", "ready": bool(ready),
                "available": True, "first_cached": first.get("cached"),
                "second_cached": second.get("cached"), "whitespace_hit": spaced.get("cached"),
                "impure_cacheable": impure.get("cacheable"), "stats": stats}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.cached-eval.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


# ============================================================================
# PP6 -- User-facing diagnostics with source positions
#
# pnix evaluation errors are bare strings. For any user-supplied-logic UI you want
# {message, line, column} + an excerpt with a caret. pnix_runtime is untouched: parse-error
# messages embed the offending token's char offset (`pos=N`), which we map to (line, col)
# against the source; eval errors get the message (pnix doesn't attach a node position to
# them). A clean structured diagnostic either way.
# ============================================================================

def _offset_to_line_col(source: str, offset: int) -> tuple[int, int]:
    offset = max(0, min(offset, len(source)))
    line = source.count("\n", 0, offset) + 1
    col = offset - source.rfind("\n", 0, offset)  # rfind=-1 -> col=offset+1 (1-based)
    return line, col


def _excerpt(source: str, line: int, col: int) -> str:
    lines = source.splitlines()
    if not (1 <= line <= len(lines)):
        return ""
    text = lines[line - 1]
    caret = " " * max(0, col - 1) + "^"
    return text + "\n" + caret


def diagnose(source: str) -> dict[str, Any]:
    """Evaluate `source` and, on failure, return a structured diagnostic:
    {ok, phase (parse|eval), message, line, column, offset, excerpt}. Parse errors get a
    precise (line, column) + caret excerpt from the embedded token offset; eval errors get
    the message (pnix attaches no position to them). Schema `pnix-hy.diagnose.v0`."""
    result: dict[str, Any] = {"schema": "pnix-hy.diagnose.v0", "source": source}
    try:
        rt.parse(source)
    except Exception as exc:  # noqa: BLE001 - parse error
        msg = str(exc)
        hits = list(_re.finditer(r"pos=(\d+)", msg))
        result.update(ok=False, phase="parse", message=msg)
        if hits:
            offset = int(hits[-1].group(1))
            line, col = _offset_to_line_col(source, offset)
            result.update(offset=offset, line=line, column=col, excerpt=_excerpt(source, line, col))
        return result
    try:
        raw = rt.eval_source(source)
    except Exception as exc:  # noqa: BLE001 - eval error (no position available)
        result.update(ok=False, phase="eval", message=str(exc))
        return result
    result.update(ok=True, value=rt.stable_data(raw))
    return result


def diagnose_report() -> dict[str, Any]:
    """Self-check: a single-line parse error pins line 1; a multi-line one pins the right
    line; an eval error is phase=eval; a good program is ok."""
    try:
        p = diagnose("1 +")
        ml = diagnose("let a = 1\n  b = 2;\n  in a")  # missing `;` -> parse error on line 2
        ev = diagnose("undefined_var")
        ok = diagnose("1 + 2")
        ready = (
            p.get("ok") is False and p.get("phase") == "parse" and p.get("line") == 1 and p.get("column") == 3
            and ml.get("phase") == "parse" and ml.get("line") == 2
            and ev.get("ok") is False and ev.get("phase") == "eval" and "unknown variable" in ev.get("message", "")
            and ok.get("ok") is True and ok.get("value") == 3
        )
        return {"schema": "pnix-hy.diagnose.report.v0", "ready": bool(ready), "available": True,
                "single_line": {"line": p.get("line"), "col": p.get("column")},
                "multi_line": {"line": ml.get("line"), "col": ml.get("column")},
                "eval_phase": ev.get("phase")}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.diagnose.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


# ============================================================================
# PP8 -- Audit / reproducibility receipt
#
# One call bundles the evidence that a pnix computation is reproducible and correct:
# the content hash of the canonical program, its value (+ value hash), the 4-lane
# convergence verdict (4 independent meta-circular substrates agree), the execution
# footprint, and the purity classification. The receipt CONTENT is deterministic (no
# timestamp) -- the same program yields the same receipt, which IS the reproducibility
# property. For regulated / auditable / reproducible-science use.
# ============================================================================

def _sha256(text: str) -> str:
    return _hashlib.sha256(text.encode("utf-8")).hexdigest()


def eval_receipt(source: str, *, timeout: int = 180) -> dict[str, Any]:
    """A deterministic reproducibility receipt for a pnix computation: canonical emit +
    source hash, value + value hash, 4-lane convergence verdict, execution footprint, and
    purity. Schema `pnix-hy.eval-receipt.v0`."""
    result: dict[str, Any] = {"schema": "pnix-hy.eval-receipt.v0", "source": source}
    try:
        ast = rt.parse(source)
        canonical = rt.emit_source(ast)
    except Exception as exc:  # noqa: BLE001
        result.update(ok=False, phase="parse", error=str(exc))
        return result
    result["canonical_emit"] = canonical
    result["source_sha256"] = _sha256(canonical)
    purity = static_purity_check(source)
    result["pure"] = purity.get("pure")
    if not purity.get("pure"):
        result["impure_uses"] = purity.get("impure_uses")
    try:
        raw = rt.eval_source(source)
        value = rt.stable_data(raw)
        value_json = rt.to_json_string_value(raw)
        result.update(ok=True, value=value, value_json=value_json,
                      value_sha256=_sha256(_json.dumps(_json.loads(value_json), sort_keys=True)))
    except Exception as exc:  # noqa: BLE001
        result.update(ok=False, phase="eval", error=str(exc))
        return result
    # 4-lane convergence verdict (host interp/compiler + stage7 runtime/compiler)
    try:
        conv = pnix_meta_circular_projection(source, timeout=timeout)
        result["convergence"] = {"converged": conv.get("converged"), "lanes": conv.get("lanes"),
                                 "errors": conv.get("errors")}
    except Exception as exc:  # noqa: BLE001
        result["convergence"] = {"converged": None, "error": f"{type(exc).__name__}: {exc}"}
    # execution footprint (what the meta-circular evaluator actually ran)
    try:
        tr = pnix_evaluation_trace(source)
        result["trace"] = {
            "runtime_functions_distinct": tr.get("runtime_functions_distinct"),
            "runtime_calls_total": tr.get("runtime_calls_total"),
            "runtime_opcodes_total": tr.get("runtime_opcodes_total"),
        }
    except Exception as exc:  # noqa: BLE001
        result["trace"] = {"error": f"{type(exc).__name__}: {exc}"}
    return result


def eval_receipt_report() -> dict[str, Any]:
    """Self-check: a pure program's receipt has a stable source hash, the right value +
    value hash, 4-lane convergence, a nonzero trace, and is DETERMINISTIC (two receipts of
    the same program share both hashes)."""
    try:
        r1 = eval_receipt("let a = 10; b = 2; in a * b + 1")
        r2 = eval_receipt("let a = 10; b = 2; in a * b + 1")
        ready = (
            r1.get("ok") is True and r1.get("value") == 21 and r1.get("pure") is True
            and len(r1.get("source_sha256", "")) == 64 and len(r1.get("value_sha256", "")) == 64
            and r1.get("source_sha256") == r2.get("source_sha256")
            and r1.get("value_sha256") == r2.get("value_sha256")  # deterministic
            and (r1.get("convergence") or {}).get("converged") is True
            and (r1.get("trace") or {}).get("runtime_opcodes_total", 0) > 0
        )
        return {"schema": "pnix-hy.eval-receipt.report.v0", "ready": bool(ready),
                "available": True, "value": r1.get("value"),
                "source_sha256": (r1.get("source_sha256") or "")[:12],
                "value_sha256": (r1.get("value_sha256") or "")[:12],
                "converged": (r1.get("convergence") or {}).get("converged"),
                "deterministic": r1.get("source_sha256") == r2.get("source_sha256")}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.eval-receipt.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def jones_optimality_report() -> dict[str, Any]:
    """0014 (T1, Amin & Rompf POPL'18 Pink Prop 4.4/4.6): a corpus-wide Jones-optimality gate.
    For every parseable self-test program p: `ir(p) == ir(parse(emit(p)))` as CONTENT-HASH
    equality (compiling the canonical emission of p is exactly p again -- zero representational
    drift), plus the emit-parse FIXED POINT `emit(parse(emit(p))) == emit(p)` (one canonical
    emission; re-canonicalizing is idempotent). This quantifies the existing B==C fixed-point
    proof style over ALL corpus programs. Schema `pnix-hy.jones-optimality.report.v0`."""
    try:
        from . import ir as ir_mod  # noqa: PLC0415

        sources: list[str] = []
        seen: set[str] = set()
        for case in rt.SELF_TEST_CASES:
            src = case.get("source")
            if isinstance(src, str) and src not in seen:
                seen.add(src)
                sources.append(src)
        checked = 0
        skipped = 0
        hash_mismatches: list[dict[str, Any]] = []
        fixpoint_failures: list[str] = []
        for src in sources:
            try:
                ast = rt.parse(src)
            except Exception:  # noqa: BLE001 - error-fixture corpus entries are out of scope
                skipped += 1
                continue
            try:
                emitted = rt.emit_source(ast)
                reparsed = rt.parse(emitted)
                a, b = ir_mod.ir_hash(ast), ir_mod.ir_hash(reparsed)
                emitted2 = rt.emit_source(reparsed)
            except Exception as exc:  # noqa: BLE001 - an emit/reparse crash IS a gate failure
                hash_mismatches.append({"source": src[:80], "error": f"{type(exc).__name__}: {exc}"})
                continue
            checked += 1
            if a != b:
                hash_mismatches.append({"source": src[:80], "ir_a": a[:12], "ir_b": b[:12],
                                        "emitted": emitted[:80]})
            if emitted2 != emitted:
                fixpoint_failures.append(src[:80])
        ready = checked >= 500 and not hash_mismatches and not fixpoint_failures
        return {"schema": "pnix-hy.jones-optimality.report.v0", "ready": bool(ready),
                "available": True, "corpus": len(sources), "checked": checked, "skipped": skipped,
                "hash_mismatches": hash_mismatches[:10], "mismatch_count": len(hash_mismatches),
                "fixpoint_failures": fixpoint_failures[:10],
                "fixpoint_failure_count": len(fixpoint_failures)}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.jones-optimality.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


_HY_SYMBOL_RE = _re.compile(r"[A-Za-z_][A-Za-z0-9_.\-]*")
_HY_CORE_SYMS = frozenset({
    "let", "when", "if", "do", "setv", "fn", "defmacro", "quote", "quasiquote", "unquote",
    "True", "False", "None", "get", "print", "hy", "import",
})


def _hy_symbols(hy_source: str) -> set[str]:
    return {s for s in _HY_SYMBOL_RE.findall(hy_source or "") if s not in _HY_CORE_SYMS}


def _let_binders(hy_source: str) -> set[str]:
    """Names bound by top-level `(let [n v ...] ...)` forms in a Hy snippet (shallow scan)."""
    binders: set[str] = set()
    for m in _re.finditer(r"\(let\s*\[([^\]]*)\]", hy_source or ""):
        toks = _HY_SYMBOL_RE.findall(m.group(1))
        binders.update(toks[0::2])  # [name value name value ...]
    return binders


def hygiene_report() -> dict[str, Any]:
    """0017 (P1+G4, sets-of-scopes-inspired): a hygiene/symbol-capture self-check for the
    Hy-macro-over-pnix bridge. Scope-set reasoning reduced to a checkable set operation:
    a binder INTRODUCED by the wrapping Hy layer that collides with a FREE symbol of the
    pnix-projected form is a (potential) capture -- the detector must flag a planted collision
    and stay silent for a gensym-fresh binder. pnix stays non-homoiconic: everything analyzed
    here is Hy-side, OVER a pnix projection. Schema `pnix-hy.hygiene.report.v0`."""
    try:
        # projected form of `x + 1` has the free symbol x
        proj = pnix_to_hy_form("x + 1")
        free = _hy_symbols(proj["hy_source"])
        free_ok = "x" in free

        # case 1: PLANTED capture -- the wrap introduces a binder named x over a form free in x
        planted = hy_macro_over_pnix("x + 1", "(let [x 999] {form})")
        collisions1 = sorted(_let_binders(planted["composed_hy"]) & free)
        capture_detected = "x" in collisions1

        # case 2: gensym-style FRESH binder -- no collision with the form's free symbols
        fresh = hy_macro_over_pnix("x + 1", "(let [g-hyg-4821 999] {form})")
        collisions2 = sorted(_let_binders(fresh["composed_hy"]) & free)
        fresh_clean = not collisions2

        # case 3: a real macro expansion over a pnix projection still runs end-to-end, and the
        # introduced-symbol diff (expansion minus input) is reified as data
        mac = hy_macro_over_pnix("1 + 2", "(when True {form})")
        expanded = mac.get("fully_expanded_python") or ""
        introduced3 = sorted(_hy_symbols(str(mac.get("expanded_full") or "")) - _hy_symbols(mac["composed_hy"]))
        macro_ok = bool(mac.get("is_macro")) and bool(expanded)

        ready = bool(free_ok and capture_detected and fresh_clean and macro_ok)
        return {"schema": "pnix-hy.hygiene.report.v0", "ready": ready, "available": True,
                "free_symbols": sorted(free), "planted_collisions": collisions1,
                "capture_detected": capture_detected, "fresh_binder_clean": fresh_clean,
                "macro_expansion_ok": macro_ok, "macro_introduced_symbols": introduced3[:10]}
    except Exception as exc:  # noqa: BLE001 - Hy proof python unavailable -> degrade
        return {"schema": "pnix-hy.hygiene.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def assumptions_valid(record: dict[str, Any], env: dict[str, Any]) -> bool:
    """0025 (T4): True iff every assumption the specialization was built on still holds in
    `env` -- False is the DEOPT signal."""
    return all(env.get(k) == v for k, v in (record.get("assumptions") or {}).items())


_RESPECIALIZE_COUNT = {"n": 0}


def respecialize_if_drifted(source: str, dynamic_vars: tuple[str, ...] = (), *,
                            env: dict[str, Any] | None = None,
                            record: dict[str, Any]) -> dict[str, Any]:
    """0025 (T7, Purple-style): when a specialization's assumptions no longer hold, RE-specialize
    under the observed values (instead of permanently falling back to interpretation) and stamp
    a re-specialization witness. Valid assumptions reuse the existing record untouched."""
    env = env or {}
    if assumptions_valid(record, env):
        return {"respecialized": False, "record": record, "witness_id": None}
    from . import gate  # noqa: PLC0415

    old = dict(record.get("assumptions") or {})
    new_assumptions = {k: env.get(k) for k in old}
    fresh = specialize_pnix(source, dynamic_vars,
                            assumptions=new_assumptions,
                            boundaries=tuple(record.get("boundaries") or ()))
    _RESPECIALIZE_COUNT["n"] += 1
    witness = gate.make_witness("respecialize", {
        "source_sha256": rt.ast_hash(rt.parse(source)),
        "old_assumptions": {k: repr(v) for k, v in old.items()},
        "new_assumptions": {k: repr(v) for k, v in new_assumptions.items()},
    })
    return {"respecialized": True, "record": fresh, "witness_id": witness["witness_id"]}


def pe_annotations_report() -> dict[str, Any]:
    """Self-check (proposal 0025): assumption-seeded specialization folds correctly, boundaries
    force dynamic, deopt is detected and re-specializes with a witness, and the default path
    (no annotations) is regression-free (incl. the A4/A5/A15 soundness fixes)."""
    try:
        # default-path regression pins
        base = specialize_pnix("let x = 5; in let y = x + 1; x = 10; in y")
        a4_ok = (base.get("fully_static") and base.get("value") == 11) or bool(base.get("gaps"))
        nb = specialize_pnix("if 1 then 2 else 3")
        a15_ok = not (nb.get("fully_static") and nb.get("value") == 2 and not nb.get("gaps"))

        # T4 assumptions: a assumed 6, b stays dynamic -> residual folds a
        rec = specialize_pnix("a * b", ("a", "b"), assumptions={"a": 6})
        assume_ok = (not rec["fully_static"] and "6" in str(rec["residual_hy"])
                     and rec["assumptions"] == {"a": 6})
        # fully-static under assumption
        rec2 = specialize_pnix("a + 1", ("a",), assumptions={"a": 2})
        assume_static_ok = rec2["fully_static"] and rec2.get("value") == 3

        # T4 boundary: statically-known name is NOT folded through
        rec3 = specialize_pnix("let a = 2; in a + c", ("c",), boundaries=("a",))
        boundary_ok = not rec3["fully_static"] and "a" in str(rec3["residual_hy"])

        # T7 deopt + re-specialization
        valid = assumptions_valid(rec2, {"a": 2})
        invalid = not assumptions_valid(rec2, {"a": 5})
        keep = respecialize_if_drifted("a + 1", ("a",), env={"a": 2}, record=rec2)
        redo = respecialize_if_drifted("a + 1", ("a",), env={"a": 5}, record=rec2)
        deopt_ok = (valid and invalid
                    and keep["respecialized"] is False and keep["witness_id"] is None
                    and redo["respecialized"] is True and bool(redo["witness_id"])
                    and redo["record"].get("value") == 6)

        ready = bool(a4_ok and a15_ok and assume_ok and assume_static_ok and boundary_ok and deopt_ok)
        return {"schema": "pnix-hy.pe-annotations.report.v0", "ready": ready, "available": True,
                "default_path_sound": bool(a4_ok and a15_ok), "assumption_folds": assume_ok,
                "assumption_static": assume_static_ok, "boundary_forces_dynamic": boundary_ok,
                "deopt_respecializes": deopt_ok,
                "respecialize_count": _RESPECIALIZE_COUNT["n"]}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.pe-annotations.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}
