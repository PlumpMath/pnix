"""PNIX Python/Hy host executor and meta-circular compiler facade.

Runtime, host interop, and meta-circular compiler capability are basic.
Service admission, deployment policy, and proof verdicts remain explicit.
"""

from __future__ import annotations

from importlib import import_module
from typing import Any

__version__ = "0.1.0"

pnix_runtime = import_module(".pnix_runtime", __name__)
interop = import_module(".interop", __name__)
from .interop import (
    CapabilityHandle,
    InteropError,
    apply_effect_request,
    apply_host_method,
    call_host,
    call_host_method,
    declare_opaque_invariants,
    from_host,
    grant_capability,
    harden_opaque,
    host_callable_arity,
    host_callable_to_pnix,
    host_module_to_pnix,
    inspect_opaque,
    install_pnix_import_hook,
    interop_context,
    interop_error_of,
    is_interop_error,
    lend_opaque,
    make_opaque_ref,
    numeric_fits,
    opaque_allowed_methods,
    opaque_call_method,
    opaque_lifecycle,
    opaque_ref_id,
    release_opaque,
    roundtrip_host_value,
    to_host,
    to_host_eval,
    try_call_host,
)
from .pnix_runtime import (
    PnixCatchableError,
    PnixError,
    eval_ast,
    eval_from_ast,
    eval_normalized_source,
    eval_source,
    eval_source_raw,
    parse,
    run_px,
    run_px_source,
    run_px_source_raw,
    runtime_context,
)

# Host-language import of a `.px` file. Alias of run_px for a stable name
# parallel to clj eval-file / rs eval_file / C# Eval.File.
eval_file = run_px


def load_proof_api() -> Any:
    """Explicitly load service/proof verification APIs."""
    return import_module(".proof", __name__)


def load_meta_api() -> Any:
    """Load the basic meta-circular compiler/evaluator facade."""
    return import_module(".meta", __name__)


def __getattr__(name: str) -> Any:
    """Compatibility access, preferring basic meta capability over proof."""
    try:
        value = getattr(load_meta_api(), name)
    except AttributeError:
        try:
            value = getattr(load_proof_api(), name)
        except AttributeError as exc:
            raise AttributeError(
                f"module {__name__!r} has no attribute {name!r}"
            ) from exc
    globals()[name] = value
    return value


__all__ = [
    "__version__",
    "pnix_runtime",
    "interop",
    "PnixError",
    "PnixCatchableError",
    "parse",
    "eval_ast",
    "eval_source",
    "eval_source_raw",
    "eval_from_ast",
    "eval_normalized_source",
    "runtime_context",
    "run_px",
    "eval_file",
    "run_px_source",
    "run_px_source_raw",
    "to_host",
    "from_host",
    "to_host_eval",
    "roundtrip_host_value",
    "call_host",
    "try_call_host",
    "call_host_method",
    "apply_host_method",
    "host_callable_to_pnix",
    "host_callable_arity",
    "host_module_to_pnix",
    "make_opaque_ref",
    "opaque_ref_id",
    "inspect_opaque",
    "opaque_allowed_methods",
    "opaque_call_method",
    "opaque_lifecycle",
    "release_opaque",
    "lend_opaque",
    "numeric_fits",
    "install_pnix_import_hook",
    "grant_capability",
    "CapabilityHandle",
    "interop_context",
    "harden_opaque",
    "declare_opaque_invariants",
    "apply_effect_request",
    "InteropError",
    "is_interop_error",
    "interop_error_of",
    "load_meta_api",
    "load_proof_api",
]
