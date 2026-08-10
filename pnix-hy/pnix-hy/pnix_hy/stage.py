"""pnix_hy.stage -- the pnix RUNTIME stage ladder (SEP4).

Distinct from hy-meta's stage8/stage9, which prove the *host* (Hy/Python) compiler. These
stages prove the *pnix runtime* is stable: a program's value survives normalization,
content-addressed store eval, AST roundtrip, the singleton mirror route, deterministic
replay, and the interp-vs-compiler closure. All host-only and fast; the FULL 4-lane
convergence (host interp/compiler + stage7 runtime/compiler) remains the `--gate` proof.

    pnix-stage1  direct eval
    pnix-stage2  parse -> normalized AST -> eval
    pnix-stage3  content-addressed store-backed eval
    pnix-stage4  AST roundtrip integrity (parse -> emit -> reparse, hash-stable)
    pnix-stage5  singleton mirror_run route
    pnix-stage6  deterministic replay (in-process)
    pnix-stage7  runtime closure: interpreter == compiler (host 2-lane)
"""

from __future__ import annotations

import json as _json
from typing import Any

from . import pnix_runtime as rt
from . import pnix_mirror as _pm
from .pnix_mirror import cached_eval


def _canon(raw: Any) -> str:
    return _json.dumps(_json.loads(rt.to_json_string_value(raw)), sort_keys=True)


def pnix_stage_ladder(source: str) -> dict[str, Any]:
    """The pnix runtime stage ladder. REUSES the existing `pnix_mirror.stage_tower` (which
    already proves the stage1-7 equivalence: direct / normalized / AST-roundtrip / mirror /
    replay value+hash convergence) and ADDS the two checks stage_tower lacks -- a
    content-addressed STORE-backed eval and the interpreter==compiler CLOSURE (host 2-lane;
    the full 4-lane convergence stays the `--gate` proof). Schema
    `pnix-hy.pnix-stage-ladder.v0`."""
    stages: list[dict[str, Any]] = []

    def record(stage: str, ok: bool, detail: Any) -> None:
        stages.append({"stage": stage, "ok": bool(ok), "detail": detail})

    # core: reuse the existing stage_tower (stages 1-7 equivalence), don't reimplement
    try:
        tower = _pm.stage_tower(source)
        value = tower.get("value")
        base = _canon(rt.eval_source(source))
        record("stage-tower-1..7-equivalence", bool(tower.get("equivalent")), tower.get("hashes"))
    except Exception as exc:  # noqa: BLE001
        record("stage-tower-1..7-equivalence", False, {"error": f"{type(exc).__name__}: {exc}"})
        return {"schema": "pnix-hy.pnix-stage-ladder.v0", "source": source,
                "passed": False, "stages": stages}

    # added: content-addressed store-backed eval (stage_tower has no store stage)
    try:
        ce = cached_eval(source)
        record("store-backed", ce.get("value") == value,
               {"cacheable": ce.get("cacheable"), "cached": ce.get("cached")})
    except Exception as exc:  # noqa: BLE001
        record("store-backed", False, {"error": f"{type(exc).__name__}: {exc}"})

    # added: runtime closure -- interpreter == compiler (host 2-lane; 4-lane = --gate)
    try:
        record("compiler-closure", _canon(rt.run_px_source(source)) == base,
               {"interp_eq_compiler": True})
    except Exception as exc:  # noqa: BLE001
        record("compiler-closure", False, {"error": f"{type(exc).__name__}: {exc}"})

    passed = all(s["ok"] for s in stages)
    return {"schema": "pnix-hy.pnix-stage-ladder.v0", "source": source,
            "passed": passed, "value": value, "stages": stages}


def pnix_stage_ladder_report() -> dict[str, Any]:
    """Self-check: a representative program passes the stage-tower equivalence plus the
    added store-backed + compiler-closure stages."""
    try:
        r = pnix_stage_ladder("let a = 10; b = 2; in a * b + 1")
        names = [s["stage"] for s in r.get("stages", [])]
        ready = (r.get("passed") is True and r.get("value") == 21
                 and "stage-tower-1..7-equivalence" in names and "compiler-closure" in names)
        return {"schema": "pnix-hy.pnix-stage-ladder.report.v0", "ready": bool(ready),
                "available": True, "passed": r.get("passed"), "value": r.get("value"),
                "stages": names}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.pnix-stage-ladder.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}
