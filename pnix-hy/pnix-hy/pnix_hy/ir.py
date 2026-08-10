"""pnix_hy.ir -- the explicit pnix IR (canonical runtime representation) layer (§3.4).

The canonical definition distinguishes AST -> IR -> canonical form. pnix is a small core
functional language, so its surface AST is already close to a core IR: the IR here is the
NORMALIZED (position-free, structurally canonical) AST -- and crucially it is DIRECTLY
EVALUABLE (`eval_from_ast(ir)`) and value-equivalent to evaluating the source, so it is a
genuine runtime representation, not just a relabeled AST.

Key principle (from the definition): pnix IR is the canonical pnix representation; the
host Python that the compiler lane emits (`_px_*`) is an EXECUTION ARTIFACT/CACHE, NOT the
IR. The IR is content-addressed (`ir_sha256`) so identical programs share one IR identity.

(Further desugaring -- attrset-path folding, sugar expansion into fewer core tags -- is a
documented future refinement; the current IR is the normalized core AST.)
"""

from __future__ import annotations

from typing import Any

from . import pnix_runtime as rt


def lower_to_ir(source_or_ast: Any) -> dict[str, Any]:
    """Lower pnix source (str) or a parsed AST (dict) to the canonical IR (position-free,
    structurally normalized AST)."""
    ast = rt.parse(source_or_ast) if isinstance(source_or_ast, str) else source_or_ast
    return rt.ast_stable(ast)


def ir_hash(ir_or_ast: Any) -> str:
    """Content hash of an IR/AST (sha256 over the stable JSON; identical programs match)."""
    return rt.ast_hash(ir_or_ast)


def eval_ir(ir: dict[str, Any], opts: dict[str, Any] | None = None) -> Any:
    """Evaluate the IR directly -- proof that the IR is a real runtime representation."""
    return rt.eval_from_ast(ir, opts) if opts is not None else rt.eval_from_ast(ir)


def ir_of(source: str) -> dict[str, Any]:
    """The IR bundle for a pnix source fragment. Schema `pnix-hy.ir.v0`."""
    ast = rt.parse(source)
    ir = rt.ast_stable(ast)
    return {
        "schema": "pnix-hy.ir.v0",
        "source": source,
        "root_tag": ast.get("tag"),
        "ir": ir,
        "ir_sha256": rt.ast_hash(ast),
    }


def ir_roundtrip(source: str) -> dict[str, Any]:
    """Check the IR is a faithful canonical representation: evaluating the IR equals
    evaluating the source, and the IR hash is stable across a parse->emit->reparse
    roundtrip. Schema `pnix-hy.ir.roundtrip.v0`."""
    result: dict[str, Any] = {"schema": "pnix-hy.ir.roundtrip.v0", "source": source}
    try:
        ast = rt.parse(source)
        ir = rt.ast_stable(ast)
        h1 = rt.ast_hash(ast)
        h2 = rt.ast_hash(rt.parse(rt.emit_source(ast)))  # canonical roundtrip
        v_src = rt.stable_data(rt.eval_source(source))
        v_ir = rt.stable_data(rt.eval_from_ast(ir))
        result.update(ir_sha256=h1, hash_stable=h1 == h2, value=v_src,
                      ir_evaluable=True, meaning_preserved=v_src == v_ir, comparable=True)
    except Exception as exc:  # noqa: BLE001 - functions / eval errors -> not comparable
        result.update(comparable=False, reason=f"{type(exc).__name__}: {exc}")
    return result


def ir_report() -> dict[str, Any]:
    """Self-check: representative programs lower to an evaluable, meaning-preserving IR with
    a stable content hash."""
    try:
        probes = ["let a = 10; b = 2; in a * b + 1", "{ a.b.c = 1; }", "(x: x + 1) 41",
                  "[1 2 (3 + 4)]"]
        results = []
        all_ok = True
        for p in probes:
            r = ir_roundtrip(p)
            ok = r.get("comparable") and r.get("meaning_preserved") and r.get("hash_stable")
            all_ok = all_ok and bool(ok)
            results.append({"source": p, "ir_sha256": (r.get("ir_sha256") or "")[:12],
                            "value": r.get("value"), "ok": ok})
        # determinism: same source -> same IR hash
        det = ir_of("1 + 2")["ir_sha256"] == ir_of("1 + 2")["ir_sha256"]
        return {"schema": "pnix-hy.ir.report.v0", "ready": bool(all_ok and det),
                "available": True, "deterministic": det, "results": results}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.ir.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


# --- 0018 (G1+P3): structural IR diff + nanopass-style pass-pipeline reification ---

# The declared IR tag vocabulary (nanopass define-language analogue). ir_pipeline() CHECKS the
# lowered corpus against this set -- an undeclared tag appearing in lowered IR is a pipeline
# invariant violation (a silently grown language), which is exactly what nanopass guards against.
IR_TAGS = frozenset({
    "int", "float", "string", "path", "bool", "null", "var", "list", "attrset", "rec_attrset",
    "let", "if", "lambda", "apply", "select", "has_attr", "binary", "unary", "with", "assert",
    "import", "inherit", "str_interp", "string_interp", "or_default", "concat", "update", "and", "or",
    "implies", "not", "neg", "call", "attrpath",
})


def _diff_walk(a: Any, b: Any, path: list[Any], out: list[dict[str, Any]], limit: int = 50) -> None:
    if len(out) >= limit:
        return
    if type(a) is not type(b):
        out.append({"path": list(path), "kind": "changed", "a": _brief(a), "b": _brief(b)})
        return
    if isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a:
                out.append({"path": [*path, k], "kind": "added", "b": _brief(b[k])})
            elif k not in b:
                out.append({"path": [*path, k], "kind": "removed", "a": _brief(a[k])})
            else:
                _diff_walk(a[k], b[k], [*path, k], out, limit)
        return
    if isinstance(a, list):
        for i in range(max(len(a), len(b))):
            if i >= len(a):
                out.append({"path": [*path, i], "kind": "added", "b": _brief(b[i])})
            elif i >= len(b):
                out.append({"path": [*path, i], "kind": "removed", "a": _brief(a[i])})
            else:
                _diff_walk(a[i], b[i], [*path, i], out, limit)
        return
    if a != b:
        out.append({"path": list(path), "kind": "changed", "a": _brief(a), "b": _brief(b)})


def _brief(x: Any) -> Any:
    if isinstance(x, (dict, list)):
        s = str(x)
        return s[:80] + ("..." if len(s) > 80 else "")
    return x


def ir_diff(source_a: str, source_b: str) -> dict[str, Any]:
    """0018 (G1): a DETERMINISTIC node-level structural diff of two programs' canonical IRs --
    'where do they differ', not just 'do the hashes differ'. Formatting/whitespace differences
    vanish (the IR is normalized); real divergences come back as paths like
    ['body', 'rhs', 'value']. Schema `pnix-hy.ir-diff.v0`."""
    ia, ib = lower_to_ir(source_a), lower_to_ir(source_b)
    diffs: list[dict[str, Any]] = []
    _diff_walk(ia, ib, [], diffs)
    return {
        "schema": "pnix-hy.ir-diff.v0",
        "equal": not diffs,
        "hash_a": rt.ast_hash(ia), "hash_b": rt.ast_hash(ib),
        "first_divergence_path": diffs[0]["path"] if diffs else None,
        "diff_count": len(diffs),
        "diffs": diffs,
    }


def _collect_tags(node: Any, seen: set[str]) -> None:
    if isinstance(node, dict):
        tag = node.get("tag")
        if isinstance(tag, str):
            seen.add(tag)
        for v in node.values():
            _collect_tags(v, seen)
    elif isinstance(node, list):
        for v in node:
            _collect_tags(v, seen)


def ir_pipeline(sources: list[str] | None = None) -> dict[str, Any]:
    """0018 (P3): reify the lowering pipeline as nanopass-style pass DATA with per-pass
    invariants, checked against a corpus: parse (source -> positional AST), then lower
    (AST -> position-free canonical IR; invariant: every tag is in the DECLARED IR_TAGS
    vocabulary and no position keys survive). Schema `pnix-hy.ir-pipeline.v0`."""
    corpus = sources or [
        "let a = 10; b = 2; in a * b + 1", "{ a.b.c = 1; }", "(x: x + 1) 41",
        "[1 2 (3 + 4)]", "if true then 1 else 2", 'rec { x = 1; y = x + 41; }.y',
        "with { a = 1; }; a + 1", "assert true; 7", '"pre-${"mid"}-post"',
    ]
    seen: set[str] = set()
    pos_leak: list[str] = []
    parse_fail: list[str] = []
    for src in corpus:
        try:
            ir = lower_to_ir(src)
        except Exception as exc:  # noqa: BLE001
            parse_fail.append(f"{src[:40]}: {type(exc).__name__}")
            continue
        _collect_tags(ir, seen)
        if '"pos"' in str(ir) or "'pos'" in str(ir):
            pos_leak.append(src[:40])
    undeclared = sorted(seen - IR_TAGS)
    passes = [
        {"pass": "parse", "input": "source", "output": "ast",
         "invariant": "syntactically valid pnix -> positional AST"},
        {"pass": "lower", "input": "ast", "output": "ir",
         "invariant": "position-free; tags within the declared IR_TAGS vocabulary",
         "undeclared_tags": undeclared, "position_leaks": pos_leak},
    ]
    return {
        "schema": "pnix-hy.ir-pipeline.v0",
        "passes": passes,
        "corpus_size": len(corpus),
        "tags_seen": sorted(seen),
        "ok": not undeclared and not pos_leak and not parse_fail,
        "parse_failures": parse_fail,
    }


def ir_diff_report() -> dict[str, Any]:
    """Self-check (proposal 0018): identical sources diff as equal (formatting-insensitive);
    a single changed literal is located at a precise divergence path; the reified pipeline's
    invariants hold over the probe corpus."""
    try:
        same = ir_diff("let a = 1; in a + 2", "let a=1;in a+2")  # formatting-insensitive
        d = ir_diff("let a = 1; in a + 2", "let a = 1; in a + 3")
        located = (not same["equal"] is False) and same["equal"] and not d["equal"] \
            and d["first_divergence_path"] is not None and d["diff_count"] >= 1 \
            and d["diffs"][0]["kind"] == "changed"
        # the divergence path must point at the changed literal's value slot
        path_ok = any(str(p) == "value" or p == "value" for p in (d["first_divergence_path"] or []))
        structural = ir_diff("[1 2 3]", "[1 2]")
        structural_ok = not structural["equal"] and structural["diffs"][0]["kind"] in ("removed", "changed")
        pipe = ir_pipeline()
        ready = bool(located and path_ok and structural_ok and pipe["ok"])
        return {"schema": "pnix-hy.ir-diff.report.v0", "ready": ready, "available": True,
                "formatting_insensitive": same["equal"], "divergence_located": located,
                "divergence_path": d["first_divergence_path"], "path_points_at_literal": path_ok,
                "structural_diff": structural_ok, "pipeline_ok": pipe["ok"],
                "pipeline_tags": pipe["tags_seen"], "undeclared_tags": pipe["passes"][1]["undeclared_tags"]}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.ir-diff.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}
