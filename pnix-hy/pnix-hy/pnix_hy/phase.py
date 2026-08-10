"""pnix_hy.phase -- phase arithmetic + compile/run separation gates (proposal 0022).

P2 (Racket-style): the tower's staging operations carry INTEGER phase shifts that compose and
cancel algebraically -- for-syntax/quote/quasiquote are +1 (toward compile time), unquote/
for-template are -1 (back toward runtime), read/eval/collapse are 0. The report checks the
algebra (composition, cancellation, associativity) and pins the mapping from actual toolkit
surfaces to phase ops.

P4 (Flatt macromod): compilation and execution must be OBSERVATIONALLY separate -- lowering a
program to IR must not mutate any runtime state (the empty-store separation result), and
evaluating the lowered IR must equal evaluating the source (interleaving irrelevance).
"""

from __future__ import annotations

from typing import Any

from . import pnix_runtime as rt
from . import ir as ir_mod

PHASE_SHIFTS: dict[str, int] = {
    "read": 0,
    "eval": 0,
    "collapse": 0,          # specialization changes REPRESENTATION, not phase
    "quote": +1,
    "quasiquote": +1,
    "for-syntax": +1,
    "unquote": -1,
    "unquote-splice": -1,
    "for-template": -1,
}

# where each toolkit surface sits in the phase algebra (documentation-as-data, drift-visible)
SURFACE_PHASES: dict[str, str] = {
    "hy_quasiquote_projection": "quasiquote",     # builds code: +1
    "hy_defmacro_projection": "for-syntax",       # macro definitions live at +1
    "hy_macro_step_trace": "for-syntax",
    "specialize_pnix": "collapse",                # Futamura collapse: phase-preserving
    "safe_eval": "eval",
    "pnix_to_hy_form": "read",
}


def phase_of(ops: list[str] | tuple[str, ...]) -> int:
    """Compose a sequence of staging operations into one integer phase shift."""
    return sum(PHASE_SHIFTS[op] for op in ops)


def phase_separation_report() -> dict[str, Any]:
    """Self-check (proposal 0022): the phase algebra holds (compose/cancel/associate) and
    lowering is observationally irrelevant (no state mutation; value-equal to direct eval)."""
    try:
        # P2: algebra
        cancel = phase_of(["quote", "unquote"]) == 0 and phase_of(["quasiquote", "unquote-splice"]) == 0
        compose = phase_of(["for-syntax", "for-syntax", "for-template"]) == 1
        assoc = (phase_of(["quote", "quote"]) + phase_of(["unquote"])
                 == phase_of(["quote", "quote", "unquote"]))
        identity = phase_of(["read", "eval", "collapse"]) == 0
        surfaces_mapped = all(op in PHASE_SHIFTS for op in SURFACE_PHASES.values())
        algebra_ok = bool(cancel and compose and assoc and identity and surfaces_mapped)

        # P4: lowering mutates NO runtime state (empty-store separation)
        from . import interop as iop  # noqa: PLC0415
        from . import pnix_mirror as pm  # noqa: PLC0415
        corpus = ["let a = 1; in a + 2", "{ x.y = 1; }", "(f: f 20) (x: x + 22)",
                  "[1 2 3]", "if true then 1 else 2", 'rec { a = 1; b = a + 41; }.b']
        before = (dict(pm.eval_cache_stats()), dict(iop.opaque_lifecycle()))
        irs = [ir_mod.lower_to_ir(src) for src in corpus for _ in range(3)]
        after = (dict(pm.eval_cache_stats()), dict(iop.opaque_lifecycle()))
        no_mutation = before == after and len(irs) == len(corpus) * 3

        # P4: observational irrelevance -- eval(source) == eval(lower(source))
        agree = all(
            rt.stable_data(rt.eval_source(src)) == rt.stable_data(ir_mod.eval_ir(ir_mod.lower_to_ir(src)))
            for src in corpus
        )
        ready = bool(algebra_ok and no_mutation and agree)
        return {"schema": "pnix-hy.phase-separation.report.v0", "ready": ready, "available": True,
                "algebra": algebra_ok, "lowering_pure": no_mutation,
                "observational_irrelevance": agree,
                "surface_phases": SURFACE_PHASES}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.phase-separation.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


__all__ = ["PHASE_SHIFTS", "SURFACE_PHASES", "phase_of", "phase_separation_report"]
