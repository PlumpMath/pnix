"""Convenience facade for hy-meta host APIs consumed by pnix-hy.

The directory is named `hy-meta`, so callers commonly path-import this file instead of
using a package import. Keep this module thin: it only re-exports existing host services.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

_HERE = str(Path(__file__).resolve().parent)
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

import interop  # noqa: E402,F401
from interop import (  # noqa: E402,F401
    call_method,
    inspect_object,
    opaque_allowed_methods,
    opaque_call_method,
    opaque_ref_id,
)
from clean_replay import (  # noqa: E402,F401
    clean_env,
    clean_replay_report,
    compare_clean_probes,
    gate_check,
    run_clean_probe,
    sandbox_report,
    sandbox_run,
)
from host_exec import (  # noqa: E402,F401
    artifact_cache_report,
    artifact_from_ast,
    artifact_from_source,
    artifact_summary,
    cache_clear,
    cache_get,
    cache_key,
    cache_put,
    classify_drift,
    classify_drift_report,
    compare_artifacts,
    compile_artifact_witness,
    compile_artifact_witness_report,
    compile_python_ast,
    host_exec_report,
    reify_host,
    reify_host_report,
    roundtrip_code_result,
    roundtrip_python_ast,
    roundtrip_report,
    roundtrip_stage,
    roundtrip_status_vocabulary,
    run_code_object,
    run_python_source,
)
from import_hook import (  # noqa: E402,F401
    diff_host_state,
    import_hook_report,
    install_pnix_import_hook,
    rollback_host_state,
    rollback_sys_modules,
    snapshot_host_state,
    snapshot_sys_modules,
)
from witness import (  # noqa: E402,F401
    WITNESS_FIELD_SCHEMA,
    WITNESS_SCHEMA,
    conversion_witness,
    drift_witness,
    make_witness,
    replay_witness,
    resolve_witness,
    witness_fields,
    witness_report,
)


def host_api_report() -> dict[str, Any]:
    """Aggregate SR2-SR6 host API reports."""
    reports = {
        "host_exec": host_exec_report(),
        "classify_drift": classify_drift_report(),
        "reify_host": reify_host_report(),
        "artifact_cache": artifact_cache_report(),
        "compile_artifact_witness": compile_artifact_witness_report(),
        "roundtrip": roundtrip_report(),
        "clean_replay": clean_replay_report(),
        "sandbox": sandbox_report(),
        "import_hook": import_hook_report(),
        "interop": interop.interop_report(),
        "witness": witness_report(),
    }
    return {
        "schema": "hy-meta.host-api.report.v0",
        "ready": all(report.get("ready") for report in reports.values()),
        "reports": reports,
    }


__all__ = [
    "WITNESS_FIELD_SCHEMA",
    "WITNESS_SCHEMA",
    "artifact_from_ast",
    "artifact_from_source",
    "artifact_summary",
    "artifact_cache_report",
    "cache_clear",
    "cache_get",
    "cache_key",
    "cache_put",
    "classify_drift",
    "classify_drift_report",
    "clean_env",
    "clean_replay_report",
    "compare_artifacts",
    "compare_clean_probes",
    "compile_artifact_witness",
    "compile_artifact_witness_report",
    "compile_python_ast",
    "conversion_witness",
    "call_method",
    "diff_host_state",
    "drift_witness",
    "gate_check",
    "host_api_report",
    "host_exec_report",
    "inspect_object",
    "import_hook_report",
    "install_pnix_import_hook",
    "interop",
    "make_witness",
    "opaque_allowed_methods",
    "opaque_call_method",
    "opaque_ref_id",
    "reify_host",
    "reify_host_report",
    "rollback_host_state",
    "replay_witness",
    "resolve_witness",
    "rollback_sys_modules",
    "roundtrip_code_result",
    "roundtrip_python_ast",
    "roundtrip_report",
    "roundtrip_stage",
    "roundtrip_status_vocabulary",
    "run_clean_probe",
    "run_code_object",
    "run_python_source",
    "sandbox_report",
    "sandbox_run",
    "snapshot_host_state",
    "snapshot_sys_modules",
    "witness_report",
    "witness_fields",
]
