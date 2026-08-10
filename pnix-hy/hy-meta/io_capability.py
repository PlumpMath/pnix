"""Pnix-agnostic, capability-gated read-only host I/O substrate."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Iterable

FILE_READ_CAPABILITY = "file-read"


class MetaIOError(RuntimeError):
    def __init__(self, error_class: str, message: str) -> None:
        super().__init__(message)
        self.error_class = error_class


def _require_file_read(granted: Iterable[str]) -> None:
    if FILE_READ_CAPABILITY not in {str(item) for item in granted}:
        raise MetaIOError("capability-denied", "file-read capability denied")


def _path(path: str | Path) -> Path:
    return Path(path)


def path_exists(path: str | Path, granted: Iterable[str]) -> bool:
    _require_file_read(granted)
    return _path(path).exists()


def _classify(path: Path) -> str:
    if path.is_symlink():
        return "symlink"
    if path.is_dir():
        return "directory"
    if path.is_file():
        return "regular"
    return "unknown"


def file_type(path: str | Path, granted: Iterable[str]) -> str:
    _require_file_read(granted)
    target = _path(path)
    if not target.exists() and not target.is_symlink():
        raise MetaIOError("not-found", "file type target not found")
    return _classify(target)


def read_utf8(path: str | Path, granted: Iterable[str]) -> str:
    _require_file_read(granted)
    try:
        return _path(path).read_bytes().decode("utf-8")
    except FileNotFoundError as exc:
        raise MetaIOError("not-found", "read target not found") from exc
    except UnicodeDecodeError as exc:
        raise MetaIOError("invalid-utf8", "read target is not UTF-8") from exc
    except OSError as exc:
        raise MetaIOError("io-error", "read failed") from exc


def read_dir(path: str | Path, granted: Iterable[str]) -> dict[str, str]:
    _require_file_read(granted)
    target = _path(path)
    try:
        return {entry.name: _classify(entry) for entry in sorted(target.iterdir(), key=lambda p: p.name)}
    except FileNotFoundError as exc:
        raise MetaIOError("not-found", "directory not found") from exc
    except NotADirectoryError as exc:
        raise MetaIOError("not-directory", "target is not a directory") from exc
    except OSError as exc:
        raise MetaIOError("io-error", "directory read failed") from exc


def report() -> dict[str, object]:
    denied = False
    try:
        path_exists("README.md", ())
    except MetaIOError as exc:
        denied = exc.error_class == "capability-denied"
    granted = (FILE_READ_CAPABILITY,)
    return {
        "schema": "hy-meta.io.v1",
        "ready": denied and path_exists("README.md", granted) and file_type("README.md", granted) == "regular",
        "capability": FILE_READ_CAPABILITY,
        "effects": ["path-exists", "open", "file-type", "read-dir"],
    }


if __name__ == "__main__":
    result = report()
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    raise SystemExit(0 if result["ready"] else 1)
