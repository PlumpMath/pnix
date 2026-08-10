"""Code-object host-artifact facade for hy-meta SR2."""

from __future__ import annotations

from host_exec import compile_python_ast, stable_code_payload  # noqa: F401
from host_introspect import (  # noqa: F401
    code_object_info,
    function_info,
    line_starts,
    marshal_code,
    rebuild_code,
)

__all__ = [
    "code_object_info",
    "compile_python_ast",
    "function_info",
    "line_starts",
    "marshal_code",
    "rebuild_code",
    "stable_code_payload",
]
