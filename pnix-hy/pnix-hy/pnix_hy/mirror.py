"""pnix_hy.mirror -- the SINGLETON pnix runtime mirror (SEP3).

The design correction: the pnix runtime must have ONE observation entrypoint, not many
competing mirrors. Today the surface is fragmented (run_mirror / mirror_chain /
stage_tower / self_test_report's 4 parity lanes + ~32 *_report facilities). This module
adds the canonical `mirror_run(source, opts)` that funnels a single pnix evaluation through
ONE route and emits the different observations as TRACE FACETS of that one run:

    :mirror/source :mirror/token :mirror/ast :mirror/ir :mirror/eval-step
    :mirror/value :mirror/effect :mirror/interop :mirror/error :mirror/witness

It produces one canonical result hash + one witness. The 4-lane parity mirror
(`pnix_mirror.self_test_report`) stays as the convergence GATE, but the legacy local mirror
surfaces now project out of the same singleton core (`pnix_mirror.singleton_mirror_run`)
instead of owning parallel parse/eval implementations.

Deterministic by content (run_id = source hash; no timestamp) -- the same program yields
the same mirror run. Works with no Hy/stage7 dependency (pure host runtime + interop).
"""

from __future__ import annotations

from typing import Any

# Absolute import: bypasses the package's lazy __getattr__ (see the note in
# pnix_mirror.py).
import pnix_hy.pnix_mirror as _pm


def mirror_run(source: str, *, trace: bool = False) -> dict[str, Any]:
    """Run ONE pnix evaluation through the canonical mirror, emitting trace facets.

    This delegates to `pnix_mirror.singleton_mirror_run`, the shared core used by the legacy
    `run_once` / `mirror_chain` / `run_mirror` projections. Facets (each
    `{"facet": ":mirror/<kind>", ...}`): source, token, ast, ir, effect, value, eval-step
    (native runtime mirror events), interop, witness -- or error if a phase fails. Returns
    `{schema, run_id, source, ok, value, result_sha256, facets}`. Schema
    `pnix-hy.mirror-run.v0`."""
    return _pm.singleton_mirror_run(source, trace=trace)


def mirror_run_report() -> dict[str, Any]:
    """Self-check: one run emits all expected facets, is deterministic by content, and a
    parse error yields an error facet instead of throwing."""
    try:
        r = mirror_run("let a = 10; b = 2; in a * b + 1", trace=True)
        kinds = [f["facet"] for f in r.get("facets", [])]
        expected = [":mirror/source", ":mirror/token", ":mirror/ast", ":mirror/ir",
                    ":mirror/effect", ":mirror/value", ":mirror/interop", ":mirror/eval-step",
                    ":mirror/witness"]
        r2 = mirror_run("let a = 10; b = 2; in a * b + 1")
        bad = mirror_run("1 +")
        ready = (
            r.get("ok") is True and r.get("value") == 21
            and all(k in kinds for k in expected)
            and len(r.get("result_sha256", "")) == 64
            and r.get("run_id") == r2.get("run_id")              # deterministic by content
            and r.get("result_sha256") == r2.get("result_sha256")
            and bad.get("ok") is False
            and any(f["facet"] == ":mirror/error" for f in bad.get("facets", []))
        )
        return {"schema": "pnix-hy.mirror-run.report.v0", "ready": bool(ready), "available": True,
                "facets": kinds, "value": r.get("value"), "result_sha256": (r.get("result_sha256") or "")[:12],
                "deterministic": r.get("run_id") == r2.get("run_id")}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.mirror-run.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}
