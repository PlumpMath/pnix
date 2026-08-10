"""pnix_hy.incremental -- definition-granular content addressing + realisation early cutoff
(proposal 0023; Unison identity model + Nix CA resolved derivations).

R1: each top-level `let` definition gets a DEPENDENCY-SUBSTITUTED content hash -- references to
sibling definitions are replaced by those definitions' hashes before hashing, so a definition's
identity depends on its meaning, not on sibling NAMES (alpha-renaming a dependency does not
invalidate anything: names are metadata). `incremental_eval` caches each pure, data-valued
definition by that hash and recomputes ONLY definitions whose hash changed (plus dependents).

R3: `realisation_record` is the Nix-CA analogue -- a store mapping `ir_sha256` (the drv) to the
produced `value_hash` (the out) with a witness; a known ir hash short-circuits evaluation
entirely (early cutoff).

Soundness first: anything unsupported (non-let programs, cyclic definitions, impure source,
non-data definition values) falls back to a full evaluation -- a cache miss is safe, a wrong
hit is not.
"""

from __future__ import annotations

import hashlib
import json
from typing import Any

from . import pnix_runtime as rt

_DEF_CACHE: dict[str, Any] = {}
_REALISATIONS: dict[str, dict[str, Any]] = {}


def clear_incremental_cache() -> None:
    _DEF_CACHE.clear()
    _REALISATIONS.clear()


def _collect_refs(node: Any, names: frozenset[str], out: set[str]) -> None:
    if isinstance(node, dict):
        if node.get("tag") == "var" and node.get("name") in names:
            out.add(node["name"])
        for v in node.values():
            _collect_refs(v, names, out)
    elif isinstance(node, list):
        for v in node:
            _collect_refs(v, names, out)


def _substitute_refs(node: Any, hashes: dict[str, str]) -> Any:
    """Replace references to sibling definitions by their HASHES (Unison: dependencies are
    replaced by their hashes before hashing, so names become metadata)."""
    if isinstance(node, dict):
        if node.get("tag") == "var" and node.get("name") in hashes:
            return {"tag": "__ref__", "hash": hashes[node["name"]]}
        return {k: _substitute_refs(v, hashes) for k, v in node.items()}
    if isinstance(node, list):
        return [_substitute_refs(v, hashes) for v in node]
    return node


def definition_hashes(source: str) -> dict[str, Any]:
    """Dependency-substituted content hash per top-level let definition. `supported: False`
    for non-let programs, multi-segment paths, or cyclic definition graphs."""
    try:
        ast = rt.parse(source)
    except Exception as exc:  # noqa: BLE001
        return {"supported": False, "reason": f"parse: {exc}"}
    if not isinstance(ast, dict) or ast.get("tag") != "let":
        return {"supported": False, "reason": "not a top-level let"}
    bindings = ast.get("bindings") or []
    if any(len(b.get("path", [])) != 1 or not isinstance(b["path"][0], str) for b in bindings):
        return {"supported": False, "reason": "multi-segment binding path"}
    names = frozenset(b["path"][0] for b in bindings)
    values = {b["path"][0]: b["value"] for b in bindings}
    deps: dict[str, set[str]] = {}
    for name, value in values.items():
        refs: set[str] = set()
        _collect_refs(value, names, refs)
        deps[name] = refs - {name}  # a self-reference would be recursive -> cycle below
    hashes: dict[str, str] = {}
    order: list[str] = []
    remaining = set(names)
    while remaining:
        ready = sorted(n for n in remaining if deps[n] <= set(hashes))
        if not ready:
            return {"supported": False, "reason": f"cyclic definitions: {sorted(remaining)}"}
        for name in ready:
            substituted = _substitute_refs(values[name], hashes)
            hashes[name] = rt.ast_hash(substituted)
            order.append(name)
            remaining.discard(name)
    return {"supported": True, "hashes": hashes, "order": order, "deps": {k: sorted(v) for k, v in deps.items()},
            "values": values, "body": ast.get("body")}


def _is_data(value: Any) -> bool:
    try:
        json.dumps(value)
        return True
    except (TypeError, ValueError):
        return False


def incremental_eval(source: str) -> dict[str, Any]:
    """Evaluate a top-level let with a per-definition content-addressed cache: unchanged
    definitions (by dependency-substituted hash) are reused; only changed definitions and
    their dependents are recomputed. Unsupported shapes fall back to full evaluation."""
    info = definition_hashes(source)
    from . import pnix_mirror as pm  # noqa: PLC0415 - lazy: avoid import cycle

    if not info.get("supported") or not pm.static_purity_check(source).get("pure", False):
        value = rt.stable_data(rt.eval_source(source))
        return {"schema": "pnix-hy.incremental-eval.v0", "value": value, "supported": False,
                "reason": info.get("reason", "impure source"), "hits": 0, "misses": 0}
    hits = misses = 0
    env: dict[str, Any] = {}
    for name in info["order"]:
        h = info["hashes"][name]
        if h in _DEF_CACHE:
            env[name] = _DEF_CACHE[h]
            hits += 1
            continue
        ctx = rt.runtime_context(None)
        ctx["env"] = dict(env)  # topological order: every dependency is already materialized
        val = rt.realize_value(rt.force_value(
            rt.eval_source_raw(rt.emit_source(info["values"][name]), ctx, realize=False)))
        if not _is_data(val):  # a function-valued definition cannot be cached as data -> full eval
            value = rt.stable_data(rt.eval_source(source))
            return {"schema": "pnix-hy.incremental-eval.v0", "value": value, "supported": False,
                    "reason": f"definition {name!r} is not data", "hits": hits, "misses": misses}
        _DEF_CACHE[h] = val
        env[name] = val
        misses += 1
    ctx = rt.runtime_context(None)
    ctx["env"] = env
    value = rt.realize_value(rt.force_value(
        rt.eval_source_raw(rt.emit_source(info["body"]), ctx, realize=False)))
    return {"schema": "pnix-hy.incremental-eval.v0", "value": value, "supported": True,
            "hits": hits, "misses": misses, "definition_hashes": info["hashes"]}


def realisation_record(source: str) -> dict[str, Any]:
    """R3 (Nix CA): the `ir_sha256 -> value_hash` realisation store. A known ir hash proves the
    result without evaluating (early cutoff); a miss evaluates once, records the realisation,
    and stamps a witness."""
    from . import gate  # noqa: PLC0415
    from . import ir as ir_mod  # noqa: PLC0415

    drv = ir_mod.ir_of(source)["ir_sha256"]
    if drv in _REALISATIONS:
        return {**_REALISATIONS[drv], "early_cutoff": True}
    value = rt.stable_data(rt.eval_source(source))
    value_hash = hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":"), default=repr).encode("utf-8")
    ).hexdigest()
    witness = gate.make_witness("realisation", {"drv": drv, "out": value_hash})
    rec = {"schema": "pnix-hy.realisation.v0", "ir_sha256": drv, "value_hash": value_hash,
           "value": value, "witness_id": witness["witness_id"], "early_cutoff": False}
    _REALISATIONS[drv] = rec
    return rec


def incremental_eval_report() -> dict[str, Any]:
    """Self-check (proposal 0023): full hit on repeat, partial recompute on a one-definition
    change, alpha-rename immunity (names are metadata), realisation early cutoff, and value
    agreement with ground-truth evaluation throughout."""
    try:
        clear_incremental_cache()
        src = "let big = 1000 * 1000; dep = big + 1; other = 7; in dep + other"
        truth = rt.stable_data(rt.eval_source(src))
        e1 = incremental_eval(src)
        e2 = incremental_eval(src)
        cold_warm = (e1["value"] == truth == e2["value"]
                     and e1["misses"] == 3 and e1["hits"] == 0
                     and e2["hits"] == 3 and e2["misses"] == 0)

        # change ONE independent definition -> exactly one recompute
        src_b = "let big = 1000 * 1000; dep = big + 1; other = 8; in dep + other"
        e3 = incremental_eval(src_b)
        partial = e3["value"] == rt.stable_data(rt.eval_source(src_b)) \
            and e3["misses"] == 1 and e3["hits"] == 2

        # alpha-rename a dependency -> ALL definitions still hit (names are metadata)
        src_r = "let huge = 1000 * 1000; dep = huge + 1; other = 8; in dep + other"
        e4 = incremental_eval(src_r)
        alpha = e4["value"] == e3["value"] and e4["hits"] == 3 and e4["misses"] == 0

        # unsupported/impure shapes fall back soundly
        fb = incremental_eval("(x: x + 1) 41")
        fallback_ok = fb["supported"] is False and fb["value"] == 42

        # R3: realisation early cutoff
        r1 = realisation_record("let a = 6; in a * 7")
        r2 = realisation_record("let a = 6; in a * 7")
        real_ok = (r1["early_cutoff"] is False and r2["early_cutoff"] is True
                   and r1["value_hash"] == r2["value_hash"] and r1["value"] == 42
                   and bool(r1["witness_id"]))

        ready = bool(cold_warm and partial and alpha and fallback_ok and real_ok)
        return {"schema": "pnix-hy.incremental-eval.report.v0", "ready": ready, "available": True,
                "cold_then_warm": cold_warm, "partial_recompute": partial,
                "alpha_rename_immune": alpha, "sound_fallback": fallback_ok,
                "realisation_early_cutoff": real_ok}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.incremental-eval.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


__all__ = ["definition_hashes", "incremental_eval", "realisation_record",
           "clear_incremental_cache", "incremental_eval_report"]
