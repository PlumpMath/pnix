"""Canonical host artifact API.

Thin facade over `host_exec.py`, which in turn lazy-reuses the artifact functions already
implemented in `bootstrap.py`.
"""

from __future__ import annotations

from host_exec import (  # noqa: F401
    artifact_from_ast,
    artifact_from_source,
    artifact_summary,
    compare_artifacts,
    stable_code_payload,
)

__all__ = [
    "artifact_from_ast",
    "artifact_from_source",
    "artifact_summary",
    "compare_artifacts",
    "stable_code_payload",
]
