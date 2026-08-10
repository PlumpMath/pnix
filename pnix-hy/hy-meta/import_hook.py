"""pnix import-hook service for hy-meta SR4."""

from __future__ import annotations

import importlib
import importlib.abc
import importlib.machinery
import builtins
import sys
import tempfile
from pathlib import Path
from types import ModuleType
from typing import Any, Callable


class PnixModuleLoader(importlib.abc.Loader):
    """Importlib loader that delegates .px module execution to a pnix-owned callable."""

    def __init__(self, pnix_loader: Any, path: Path, is_package: bool) -> None:
        self.pnix_loader = pnix_loader
        self.path = path
        self.is_package = is_package

    def create_module(self, spec: importlib.machinery.ModuleSpec) -> ModuleType | None:
        return None

    def get_filename(self, fullname: str) -> str:
        return str(self.path)

    def exec_module(self, module: ModuleType) -> None:
        module.__file__ = str(self.path)
        module.__loader__ = self
        module.__package__ = module.__name__ if self.is_package else module.__name__.rpartition(".")[0]
        if self.is_package:
            module.__path__ = [str(self.path.parent)]  # type: ignore[attr-defined]
        result = self._run_loader(module)
        if isinstance(result, ModuleType):
            module.__dict__.update(result.__dict__)
        elif isinstance(result, dict):
            module.__dict__.update(result)
        elif result is not None:
            module.__dict__["__pnix_result__"] = result

    def _run_loader(self, module: ModuleType) -> Any:
        try:
            return self.pnix_loader(
                module=module,
                path=self.path,
                fullname=module.__name__,
                is_package=self.is_package,
            )
        except TypeError:
            return self.pnix_loader(self.path, module.__name__)


class SuffixModuleFinder(importlib.abc.MetaPathFinder):
    """Shared finder for source-file imports under explicit roots."""

    def __init__(
        self,
        roots: list[str | Path],
        *,
        suffix: str,
        loader_factory: Callable[[Path, bool], importlib.abc.Loader],
    ) -> None:
        self.search_roots = [Path(root).resolve() for root in roots]
        self.roots = self.search_roots
        self.suffix = suffix
        self._loader_factory = loader_factory

    def find_spec(
        self,
        fullname: str,
        path: object = None,
        target: ModuleType | None = None,
    ) -> importlib.machinery.ModuleSpec | None:
        del target
        roots, parts = self._candidate_roots_and_parts(fullname, path)
        for root in roots:
            package_path = root.joinpath(*parts, "__init__" + self.suffix)
            if package_path.is_file():
                loader = self._loader_factory(package_path, True)
                return importlib.machinery.ModuleSpec(
                    fullname,
                    loader,
                    origin=str(package_path),
                    is_package=True,
                )
            module_path = root.joinpath(*parts).with_suffix(self.suffix)
            if module_path.is_file():
                loader = self._loader_factory(module_path, False)
                return importlib.machinery.ModuleSpec(
                    fullname,
                    loader,
                    origin=str(module_path),
                    is_package=False,
                )
        return None

    def _candidate_roots_and_parts(self, fullname: str, path: object) -> tuple[list[Path], list[str]]:
        if path is None:
            return self.roots, fullname.split(".")
        if isinstance(path, (str, bytes)):
            return [], []
        try:
            roots = [Path(entry).resolve() for entry in path]  # type: ignore[union-attr]
        except TypeError:
            return [], []
        return roots, [fullname.rpartition(".")[2]]


class ImportHookContext:
    """Context manager that temporarily installs a finder."""

    def __init__(self, finder: importlib.abc.MetaPathFinder) -> None:
        self.finder = finder

    def __enter__(self) -> importlib.abc.MetaPathFinder:
        sys.meta_path.insert(0, self.finder)
        return self.finder

    def __exit__(self, exc_type: object, exc: object, tb: object) -> None:
        while self.finder in sys.meta_path:
            sys.meta_path.remove(self.finder)


class PnixModuleFinder(SuffixModuleFinder):
    """Finder for pnix .px modules under explicit search roots."""

    def __init__(self, pnix_loader: Any, roots: list[str | Path]) -> None:
        self.pnix_loader = pnix_loader
        super().__init__(
            roots,
            suffix=".px",
            loader_factory=lambda path, is_package: PnixModuleLoader(
                self.pnix_loader,
                path,
                is_package,
            ),
        )


class PnixImportHook(ImportHookContext):
    """Context manager that temporarily installs a pnix-backed import finder."""

    def __init__(self, pnix_loader: Any, roots: list[str | Path]) -> None:
        super().__init__(PnixModuleFinder(pnix_loader, roots))

    def __enter__(self) -> PnixModuleFinder:
        return super().__enter__()  # type: ignore[return-value]


def install_pnix_import_hook(pnix_loader: Any, roots: list[str | Path]) -> PnixImportHook:
    """Temporarily import .px files through pnix-owned module semantics."""
    return PnixImportHook(pnix_loader, roots)


def snapshot_sys_modules(prefixes: tuple[str, ...] | list[str] | None = None) -> dict[str, Any]:
    """Snapshot sys.modules for scoped rollback around import-hook probes."""
    return {
        "schema": "hy-meta.sys-modules.snapshot.v0",
        "prefixes": list(prefixes or ()),
        "modules": dict(sys.modules),
    }


def _matches_prefix(name: str, prefixes: list[str]) -> bool:
    return not prefixes or any(name == prefix or name.startswith(prefix + ".") for prefix in prefixes)


def rollback_sys_modules(snapshot: dict[str, Any]) -> None:
    """Restore sys.modules entries captured by snapshot_sys_modules."""
    prefixes = list(snapshot.get("prefixes") or [])
    before = snapshot["modules"]
    for name in list(sys.modules):
        if _matches_prefix(name, prefixes) and name not in before:
            sys.modules.pop(name, None)
    for name, module in before.items():
        if _matches_prefix(name, prefixes):
            sys.modules[name] = module


def snapshot_host_state(prefixes: tuple[str, ...] | list[str] | None = None) -> dict[str, Any]:
    """Snapshot host mutation surfaces for scoped rollback probes."""
    return {
        "schema": "hy-meta.host-state.snapshot.v0",
        "modules": snapshot_sys_modules(prefixes),
        "sys_path": list(sys.path),
        "meta_path": list(sys.meta_path),
        "builtins": dict(vars(builtins)),
    }


def diff_host_state(snapshot: dict[str, Any]) -> dict[str, Any]:
    """Summarize mutations since `snapshot_host_state`."""
    module_snapshot = snapshot["modules"]
    prefixes = list(module_snapshot.get("prefixes") or [])
    before_modules = module_snapshot["modules"]
    module_names = sorted({
        name for name in set(before_modules) | set(sys.modules)
        if _matches_prefix(name, prefixes)
    })
    before_builtins = snapshot["builtins"]
    current_builtins = vars(builtins)
    builtin_names = sorted(set(before_builtins) | set(current_builtins))
    return {
        "schema": "hy-meta.host-state.diff.v0",
        "modules_added": [name for name in module_names if name not in before_modules and name in sys.modules],
        "modules_removed": [name for name in module_names if name in before_modules and name not in sys.modules],
        "modules_changed": [
            name for name in module_names
            if name in before_modules and name in sys.modules and sys.modules[name] is not before_modules[name]
        ],
        "sys_path_changed": list(sys.path) != list(snapshot["sys_path"]),
        "meta_path_changed": list(sys.meta_path) != list(snapshot["meta_path"]),
        "builtins_added": [name for name in builtin_names if name not in before_builtins and name in current_builtins],
        "builtins_removed": [name for name in builtin_names if name in before_builtins and name not in current_builtins],
        "builtins_changed": [
            name for name in builtin_names
            if name in before_builtins and name in current_builtins
            and current_builtins[name] is not before_builtins[name]
        ],
    }


def rollback_host_state(snapshot: dict[str, Any]) -> None:
    """Restore host mutation surfaces captured by `snapshot_host_state`."""
    rollback_sys_modules(snapshot["modules"])
    sys.path[:] = list(snapshot["sys_path"])
    sys.meta_path[:] = list(snapshot["meta_path"])
    before_builtins = snapshot["builtins"]
    current_builtins = vars(builtins)
    for name in list(current_builtins):
        if name not in before_builtins:
            delattr(builtins, name)
    for name, value in before_builtins.items():
        setattr(builtins, name, value)


def import_hook_report() -> dict[str, Any]:
    try:
        original_meta_path = list(sys.meta_path)
        state_snapshot = snapshot_host_state(["probe_state"])
        state_module = ModuleType("probe_state")
        sys.modules["probe_state"] = state_module
        sys.path.insert(0, "__hy_meta_probe_path__")
        sys.meta_path.insert(0, object())
        setattr(builtins, "__hy_meta_probe_builtin__", 42)
        state_diff = diff_host_state(state_snapshot)
        rollback_host_state(state_snapshot)
        state_restored = (
            "probe_state" not in sys.modules
            and "__hy_meta_probe_path__" not in sys.path
            and not hasattr(builtins, "__hy_meta_probe_builtin__")
            and sys.meta_path == original_meta_path
        )
        with tempfile.TemporaryDirectory(prefix="hy-meta-pnix-hook-") as temp_dir:
            root = Path(temp_dir)
            (root / "probe.px").write_text("42\n", encoding="utf-8")
            snapshot = snapshot_sys_modules(["probe"])

            def loader(path: Path, fullname: str) -> dict[str, Any]:
                return {"VALUE": int(path.read_text(encoding="utf-8").strip()), "FULLNAME": fullname}

            with install_pnix_import_hook(loader, [root]) as finder:
                hook_installed = finder in sys.meta_path
                module = importlib.import_module("probe")
            hook_removed = finder not in sys.meta_path
            rollback_sys_modules(snapshot)
            module_removed = "probe" not in sys.modules
        ready = (
            hook_installed
            and hook_removed
            and module.VALUE == 42
            and module.FULLNAME == "probe"
            and module_removed
            and sys.meta_path == original_meta_path
            and state_diff["modules_added"] == ["probe_state"]
            and state_diff["sys_path_changed"] is True
            and state_diff["meta_path_changed"] is True
            and "__hy_meta_probe_builtin__" in state_diff["builtins_added"]
            and state_restored
        )
        return {
            "schema": "hy-meta.pnix-import-hook.report.v0",
            "ready": ready,
            "available": True,
            "hook_installed": hook_installed,
            "hook_removed": hook_removed,
            "module_removed": module_removed,
            "meta_path_restored": sys.meta_path == original_meta_path,
            "host_state_restored": state_restored,
        }
    except Exception as exc:  # noqa: BLE001
        return {
            "schema": "hy-meta.pnix-import-hook.report.v0",
            "ready": False,
            "available": False,
            "error": f"{type(exc).__name__}: {exc}",
        }


__all__ = [
    "ImportHookContext",
    "PnixImportHook",
    "PnixModuleFinder",
    "PnixModuleLoader",
    "SuffixModuleFinder",
    "import_hook_report",
    "install_pnix_import_hook",
    "diff_host_state",
    "rollback_sys_modules",
    "rollback_host_state",
    "snapshot_host_state",
    "snapshot_sys_modules",
]
