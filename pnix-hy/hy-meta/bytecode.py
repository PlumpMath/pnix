"""Bytecode host-artifact facade for hy-meta SR2."""

from __future__ import annotations

from host_introspect import (  # noqa: F401
    disassemble,
    jump_labels,
    line_starts,
    opcode_tables,
    stack_effect,
)

__all__ = [
    "disassemble",
    "jump_labels",
    "line_starts",
    "opcode_tables",
    "stack_effect",
]
