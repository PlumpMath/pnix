"""Convenience functions for evaluating inline and file-backed PNIX source."""

from __future__ import annotations

from pathlib import Path
from pprint import pprint
import sys
from typing import Any, Sequence


class PxFormatError(ValueError):
    """The number of `%s` source-template slots and arguments differs."""


def _runtime() -> Any:
    from . import pnix_runtime

    return pnix_runtime


def format_source(source: str, *arguments: object) -> str:
    """Replace `%s` source slots; this is not a type or wire encoding."""
    if not isinstance(source, str):
        raise TypeError("PNIX source must be text")
    formatted = source
    for argument in arguments:
        before, slot, after = formatted.partition("%s")
        if not slot:
            raise PxFormatError("too many PNIX source format arguments")
        formatted = before + str(argument) + after
    if "%s" in formatted:
        raise PxFormatError("missing PNIX source format argument")
    return formatted


def px(source: str, *format_arguments: object) -> Any:
    """Evaluate PNIX source and return the actual host guest value."""
    return _runtime().eval_source(format_source(source, *format_arguments))


def px_import(path: str | Path, *format_arguments: object) -> Any:
    """Evaluate a UTF-8 `.px` file relative to its own directory."""
    resolved = Path(path).expanduser().resolve()
    source = resolved.read_text(encoding="utf-8")
    opts = {
        "base_dir": str(resolved.parent),
        "source_path": str(resolved),
        "path_literals_absolute": True,
    }
    return _runtime().eval_source(format_source(source, *format_arguments), opts)


def _at_least_one_argument(argv: Sequence[str], usage: str) -> tuple[str, list[str]]:
    if not argv:
        raise SystemExit(usage)
    return argv[0], list(argv[1:])


def main_px() -> None:
    source, arguments = _at_least_one_argument(
        sys.argv[1:], "usage: px TEMPLATE [FORMAT-ARG ...]"
    )
    pprint(px(source, *arguments), sort_dicts=True)


def main_px_import() -> None:
    path, arguments = _at_least_one_argument(
        sys.argv[1:], "usage: px-import FILE.px [FORMAT-ARG ...]"
    )
    pprint(px_import(path, *arguments), sort_dicts=True)


__all__ = [
    "PxFormatError",
    "format_source",
    "main_px",
    "main_px_import",
    "px",
    "px_import",
]
