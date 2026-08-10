"""pnix-hy -- a meta-circular projection toolkit between the Python-language ecosystem
(Hy, Python) and pnix (a pure, lazy, Nix-like functional language).

This package's mission is to project LANGUAGE EXPRESSIVENESS in both directions so a
developer can study meta-circular evaluation; it is also usable in production as a pure,
deterministic, sandboxable evaluation/projection substrate.

Curated public API (stable; see each function's docstring + its `*_report()` self-check):

Production
    safe_eval(source, *, timeout_s, max_steps, max_output_bytes, pure_only, granted)
        Evaluate untrusted pnix under resource bounds and optional capability gates.
    static_purity_check(source)
        Statically classify a program's impurity (filesystem/env/import) before eval.
    cached_eval(source)
        Content-addressed memoized eval of PURE pnix (sound; deterministic).

Bidirectional projection (Hy/Python <-> pnix)
    pnix_to_hy_form(pnix_source) / synthesize_pnix_from_hy(hy_source)
    projection_value_roundtrip / hy_to_pnix_value_roundtrip   (meaning preservation)
    pnix_projection_closure / hy_projection_closure           (involution by value)
    correspondence_table()                                    (the AST/value map)
    reify_pnix / roundtrip_status / explain_pnix              (capability allocation surfaces)

Meta-circular staging tower (Hy)
    hy_quasiquote_projection / hy_defmacro_projection / hy_reader_macro_projection
    hy_macro_step_trace / hy_import_projection
    meta_circular_tower(hy_source)    -- the whole read->compile->run->collapse journey

Specialization (Futamura) & observation
    specialize_pnix / specialization_roundtrip
    pnix_evaluation_trace / execution_trace

Run every self-check: `python -m pnix_hy.cli --check` (or bin/pnix-hy-project --check).
"""

# Legacy compatibility aggregate. Meta-circular compiler APIs are canonically
# owned by pnix_hy.meta; this module additionally assembles proof, admission,
# deployment, and report APIs for explicit verification callers.
from __future__ import annotations

__version__ = "0.1.0"

from . import action, capabilities, check_cache, cogen, compartment, compiled, deploy, gate, hy_mirror, incremental, interop, ir, mirror, phase, pnix_mirror, pnix_runtime, stage, tower  # noqa: F401
from .action import action_report, begin_action, check_action, verify_action  # noqa: F401
from .capabilities import capability_index, docs_drift_report  # noqa: F401
from .deploy import deployment_info  # noqa: F401
from .interop import (  # noqa: F401
    apply_effect_request,
    apply_host_method,
    call_host,
    call_host_method,
    CapabilityHandle,
    declare_opaque_invariants,
    from_host,
    grant_capability,
    harden_opaque,
    interop_context,
    host_callable_arity,
    host_callable_to_pnix,
    host_module_to_pnix,
    inspect_opaque,
    install_pnix_import_hook,
    InteropError,
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
from .mirror import mirror_run  # noqa: F401
from .stage import pnix_stage_ladder  # noqa: F401
from .gate import gate_check, make_witness, predicate_of, typed_witness  # noqa: F401
from .ir import eval_ir, ir_diff, ir_of, ir_pipeline, lower_to_ir  # noqa: F401

# --- production ---
from .pnix_mirror import (  # noqa: F401
    cached_eval,
    classify_drift,
    clear_eval_cache,
    diagnose,
    eval_cache_stats,
    eval_receipt,
    explain_pnix,
    reify_hy,
    ROUNDTRIP_STATUS_VOCAB,
    safe_eval,
    reify_pnix,
    roundtrip_status,
    static_purity_check,
)

# --- bidirectional projection + meaning preservation ---
from .pnix_mirror import (  # noqa: F401
    correspondence_abi,
    correspondence_table,
    hy_projection_closure,
    hy_to_pnix_value_roundtrip,
    pnix_projection_closure,
    pnix_to_hy_form,
    projection_value_roundtrip,
    synthesize_pnix_from_hy,
)

# --- meta-circular tower + capstones ---
from .pnix_mirror import hygiene_report, jones_optimality_report  # noqa: F401
from .pnix_mirror import (  # noqa: F401
    hy_macro_over_pnix,
    hy_macro_quasiquote_over_pnix_report,
    hy_pnix_projection,
    hy_quasiquote_over_pnix,
    hy_reader_embed_pnix,
    meta_circular_tower,
    pnix_meta_circular_projection,
    quasiquote_specialize_correspondence,
)
from .hy_mirror import (  # noqa: F401
    hy_defmacro_projection,
    hy_import_projection,
    hy_macro_step_trace,
    hy_quasiquote_projection,
    hy_reader_macro_projection,
)

# --- specialization (Futamura) + observation ---
from .pnix_mirror import (  # noqa: F401
    pnix_evaluation_trace,
    specialization_roundtrip,
    specialize_pnix,
)
from .hy_mirror import execution_trace  # noqa: F401
from .repl import run_repl  # noqa: F401
from .compiled import compiled_bench, compiled_differential_report, compiled_eval, compiled_runtime_report, differential_corpus, evaluate, evaluate_report, subset_supported  # noqa: F401
from .cogen import cogen, cogen_report, compile_with, compiler_from_interpreter, compiler_source, generating_extension, pe_size_report  # noqa: F401
from .compartment import Compartment  # noqa: F401
from .phase import phase_of  # noqa: F401
from .incremental import definition_hashes, incremental_eval, realisation_record  # noqa: F401
from .pnix_mirror import assumptions_valid, respecialize_if_drifted  # noqa: F401
from .tower import (  # noqa: F401
    binding_time_analysis,
    build_cogen,
    cek_resume,
    cek_run,
    collapse_interpreter,
    futamura_ladder,
    run_cogen,
    em,
    em_stepwise,
    mix_in_pnix,
    poly_mix_in_pnix,
    poly_specialize,
    reflect_to_stage7,
    reify_computation,
    self_generation_witness,
    stage_poly_compile,
    stage_poly_interpret,
)

__all__ = [
    "__version__",
    "hy_mirror", "pnix_mirror", "pnix_runtime", "interop", "mirror", "stage", "gate", "ir",
    "action", "deploy", "deployment_info", "capabilities",
    "capability_index", "docs_drift_report",
    "begin_action", "check_action", "verify_action", "action_report",
    "to_host", "from_host", "to_host_eval", "make_opaque_ref", "install_pnix_import_hook",
    "roundtrip_host_value",
    "call_host", "try_call_host", "is_interop_error", "interop_error_of", "InteropError",
    "host_callable_to_pnix", "host_callable_arity", "host_module_to_pnix",
    "opaque_lifecycle", "release_opaque", "lend_opaque", "numeric_fits", "correspondence_abi",
    "opaque_ref_id", "inspect_opaque", "opaque_allowed_methods", "call_host_method",
    "apply_host_method", "opaque_call_method",
    "mirror_run",
    "pnix_stage_ladder", "gate_check", "make_witness", "lower_to_ir", "ir_of", "eval_ir",
    "ir_diff", "ir_pipeline", "check_cache", "jones_optimality_report", "hygiene_report",
    # PHASE C (0020-0026)
    "grant_capability", "CapabilityHandle", "interop_context", "harden_opaque",
    "declare_opaque_invariants", "compartment", "Compartment", "phase", "phase_of",
    "incremental", "definition_hashes", "incremental_eval", "realisation_record",
    "typed_witness", "predicate_of", "assumptions_valid", "respecialize_if_drifted",
    "tower", "compiled", "compiled_eval", "compiled_bench", "compiled_runtime_report",
    "compiled_differential_report", "differential_corpus", "subset_supported", "evaluate", "evaluate_report",
    "cogen", "cogen_report", "compiler_from_interpreter", "generating_extension", "pe_size_report",
    "compiler_source", "compile_with",
    "stage_poly_interpret", "stage_poly_compile", "mix_in_pnix",
    "self_generation_witness", "reify_computation", "reflect_to_stage7", "em",
    "collapse_interpreter", "cek_run", "cek_resume", "em_stepwise", "binding_time_analysis", "poly_specialize", "poly_mix_in_pnix", "build_cogen", "run_cogen", "futamura_ladder",
    # production
    "safe_eval", "static_purity_check", "cached_eval", "eval_cache_stats", "clear_eval_cache",
    "diagnose", "eval_receipt", "reify_pnix", "reify_hy", "classify_drift",
    "roundtrip_status", "explain_pnix",
    "ROUNDTRIP_STATUS_VOCAB",
    # bidirectional projection
    "pnix_to_hy_form", "synthesize_pnix_from_hy",
    "projection_value_roundtrip", "hy_to_pnix_value_roundtrip",
    "pnix_projection_closure", "hy_projection_closure", "correspondence_table",
    # tower + capstones
    "hy_quasiquote_projection", "hy_defmacro_projection", "hy_reader_macro_projection",
    "hy_macro_step_trace", "hy_import_projection",
    "hy_pnix_projection", "meta_circular_tower", "pnix_meta_circular_projection",
    "hy_macro_over_pnix", "hy_quasiquote_over_pnix", "quasiquote_specialize_correspondence",
    "hy_reader_embed_pnix",
    # specialization + observation
    "specialize_pnix", "specialization_roundtrip",
    "pnix_evaluation_trace", "execution_trace",
]
