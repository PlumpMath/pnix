"""Basic meta-circular compiler/evaluator capability for the Hy host.

This module owns mechanism, not service admission. Compiler generation,
staging, projection, and interpreter collapse are basic PNIX/host-language
capabilities. Proof receipts and deployment verdicts remain separate APIs.
"""

from __future__ import annotations

from importlib import import_module
from typing import Any

from . import cogen as cogen_module
from . import compiled as compiled_module
from . import tower as tower_module
from .cogen import (
    cogen,
    cogen_report,
    compile_with,
    compiler_from_interpreter,
    compiler_source,
    generating_extension,
    pe_size_report,
)
from .compiled import (
    compiled_bench,
    compiled_differential_report,
    compiled_eval,
    compiled_runtime_report,
    differential_corpus,
    evaluate,
    evaluate_report,
    subset_supported,
)
from .tower import (
    binding_time_analysis,
    build_cogen,
    cek_resume,
    cek_run,
    collapse_interpreter,
    em,
    em_stepwise,
    futamura_ladder,
    mix_in_pnix,
    poly_mix_in_pnix,
    poly_specialize,
    reflect_to_stage7,
    reify_computation,
    run_cogen,
    self_generation_witness,
    stage_poly_compile,
    stage_poly_interpret,
)

_PROJECTION_MODULES = (
    ".ir",
    ".mirror",
    ".stage",
    ".pnix_mirror",
    ".hy_mirror",
)


def load_projection_api() -> dict[str, Any]:
    """Load meta-circular projection modules without loading service policy."""
    return {
        name[1:]: import_module(name, __package__)
        for name in _PROJECTION_MODULES
    }


def __getattr__(name: str) -> Any:
    for module in load_projection_api().values():
        try:
            value = getattr(module, name)
        except AttributeError:
            continue
        globals()[name] = value
        return value
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


__all__ = [
    "compiled_module",
    "cogen_module",
    "tower_module",
    "compiled_eval",
    "compiled_bench",
    "compiled_runtime_report",
    "compiled_differential_report",
    "differential_corpus",
    "subset_supported",
    "evaluate",
    "evaluate_report",
    "cogen",
    "cogen_report",
    "compiler_from_interpreter",
    "generating_extension",
    "pe_size_report",
    "compiler_source",
    "compile_with",
    "stage_poly_interpret",
    "stage_poly_compile",
    "mix_in_pnix",
    "self_generation_witness",
    "reify_computation",
    "reflect_to_stage7",
    "em",
    "collapse_interpreter",
    "cek_run",
    "cek_resume",
    "em_stepwise",
    "binding_time_analysis",
    "poly_specialize",
    "poly_mix_in_pnix",
    "build_cogen",
    "run_cogen",
    "futamura_ladder",
    "load_projection_api",
]
