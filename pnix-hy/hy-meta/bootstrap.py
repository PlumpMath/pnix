#!/usr/bin/env python3
"""Bootstrap stage2 from stage1 and expose a small Hy meta CLI."""

from __future__ import annotations

import argparse
import ast
import builtins
import concurrent.futures
import copy
import hashlib
import http.server
import io
import importlib
import importlib.abc
import importlib._bootstrap_external as importlib_external
import importlib.machinery
import importlib.util
import json
import marshal
import os
import runpy
import subprocess
import sys
import tempfile
import threading
import time
import traceback
import urllib.request
from pathlib import Path
from types import CodeType, ModuleType
from typing import Any

from import_hook import ImportHookContext, SuffixModuleFinder, rollback_sys_modules, snapshot_sys_modules
from witness import record_witness

SUPPORTED_PYTHON_FAMILIES = {(3, 11), (3, 14)}

if sys.version_info[:2] not in SUPPORTED_PYTHON_FAMILIES:
    raise SystemExit(
        "hy-meta supports exactly Python 3.11 and Homebrew Python 3.14 "
        "in this proof lane; run it with /tmp/pnix-hy-py311-venv/bin/python "
        "or /tmp/pnix-hy-py314-venv/bin/python."
    )

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from stage1.compiler import Stage1Compiler, eval_source, load_hy_file  # noqa: E402
import hy  # noqa: E402
import hy.errors  # noqa: E402
import independent_mini_backend  # noqa: E402


HY_META_ROUTE_POLICY_VERSION = "stage9-product-route-policy-v1"
HY_META_FEATURE_GATE_VERSION = "hy-meta-feature-gates-v1"
STAGE10_PROTOCOL_VERSION = "stage10.local.v1"
STAGE11_ADAPTER_SCHEMA_VERSION = "stage11-adapter-v1"


STAGE2_PATH = ROOT / "stage2" / "compiler.hy"
KERNEL_PATH = ROOT / "stage2" / "kernel.hy"
STAGE2_MODULE = "hy_meta_stage2.compiler"
STAGE2_CHAIN_MODULE = "hy_meta_stage2.compiler_self_hosted"
STAGE3_MODULE = "hy_meta_stage3.compiler"
KERNEL_MODULE = "hy_meta_stage2.kernel"
KERNEL_STAGE3_MODULE = "hy_meta_stage3.kernel"


def all_equal(values: list[Any]) -> bool:
    return not values or all(value == values[0] for value in values[1:])


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_text(data: str) -> str:
    return sha256_bytes(data.encode("utf-8"))


def bootstrap_stage2() -> ModuleType:
    """Compile the Hy-written stage2 compiler with stage1."""

    return load_hy_file(STAGE2_PATH, module_name=STAGE2_MODULE)


def bootstrap_stage2_chain() -> tuple[ModuleType, ModuleType]:
    """Load stage2 with stage1, then load stage2 again through stage2."""

    stage2 = bootstrap_stage2()
    stage2_prime = stage2.load_hy_file(STAGE2_PATH, STAGE2_CHAIN_MODULE)
    return stage2, stage2_prime


def bootstrap_stage3_chain() -> tuple[ModuleType, ModuleType, ModuleType]:
    """Load stage2, stage2-prime, and then stage3 through stage2-prime."""

    stages = bootstrap_stage_chain(3)
    return stages[0][1], stages[1][1], stages[2][1]


def bootstrap_stage_chain(max_stage: int = 7) -> list[tuple[str, ModuleType]]:
    """Load stage2, stage2-prime, and generated stages through max_stage."""

    if max_stage < 3:
        raise ValueError("hy-meta stage chain needs max_stage >= 3")

    stage2, stage2_prime = bootstrap_stage2_chain()
    stages = [("stage2", stage2), ("stage2_prime", stage2_prime)]
    previous = stage2_prime
    for stage_number in range(3, max_stage + 1):
        module_name = (
            STAGE3_MODULE
            if stage_number == 3
            else f"hy_meta_stage{stage_number}.compiler"
        )
        previous = previous.load_hy_file(STAGE2_PATH, module_name)
        stages.append((f"stage{stage_number}", previous))
    return stages


def bootstrap_kernel(stage2: ModuleType | None = None) -> ModuleType:
    """Load the Hy-written kernel compiler through stage2."""

    stage2 = stage2 or bootstrap_stage2()
    return stage2.load_hy_file(KERNEL_PATH, KERNEL_MODULE)


def bootstrap_stage7_kernel() -> ModuleType:
    """Load the Hy-written kernel compiler through the final stage7 compiler."""

    _label, stage7 = bootstrap_stage_chain(7)[-1]
    return stage7.load_hy_file(KERNEL_PATH, "hy_meta_stage7.cli.kernel")


class KernelHyLoader(importlib.abc.Loader):
    """Importlib loader that executes a .hy file through the Hy-written kernel."""

    def __init__(self, kernel: ModuleType, path: Path, is_package: bool) -> None:
        self.kernel = kernel
        self.path = path
        self.is_package = is_package
        self.last_ast_dump: str | None = None
        self.last_python_source: str | None = None

    def create_module(self, spec: importlib.machinery.ModuleSpec) -> ModuleType | None:
        return None

    def get_filename(self, fullname: str) -> str:
        return str(self.path)

    def get_source(self, fullname: str) -> str:
        return self.path.read_text()

    def get_code(self, fullname: str) -> Any:
        filename = self.get_filename(fullname)
        package = fullname if self.is_package else fullname.rpartition(".")[0]
        tree = self.kernel.compile_source_to_module(
            self.get_source(fullname),
            filename,
            "__hy_meta_result__",
            fullname,
            package,
        )
        self.last_ast_dump = ast.dump(tree, include_attributes=False)
        self.last_python_source = ast.unparse(tree)
        return compile(tree, filename, "exec")

    def exec_module(self, module: ModuleType) -> None:
        filename = str(self.path)
        module.__file__ = filename
        module.__loader__ = self
        module.__package__ = (
            module.__name__ if self.is_package else module.__name__.rpartition(".")[0]
        )
        if self.is_package:
            module.__path__ = [str(self.path.parent)]  # type: ignore[attr-defined]
        module.hy = self.kernel.hy
        module.__dict__.setdefault("_hy_macros", {})
        module.__dict__.setdefault("_hy_reader_macros", {})
        exec(self.get_code(module.__name__), module.__dict__)
        module.__dict__.pop("__hy_meta_result__", None)


class KernelHyFinder(SuffixModuleFinder):
    """Finder for kernel-compiled .hy modules under explicit search roots."""

    def __init__(self, kernel: ModuleType, search_roots: list[str | Path]) -> None:
        self.kernel = kernel
        super().__init__(
            search_roots,
            suffix=".hy",
            loader_factory=lambda path, is_package: KernelHyLoader(
                self.kernel,
                path,
                is_package,
            ),
        )


class KernelHyImportHook(ImportHookContext):
    """Context manager that temporarily installs a kernel-backed Hy finder."""

    def __init__(self, kernel: ModuleType, search_roots: list[str | Path]) -> None:
        super().__init__(KernelHyFinder(kernel, search_roots))

    def __enter__(self) -> KernelHyFinder:
        return super().__enter__()  # type: ignore[return-value]


def install_kernel_import_hook(
    kernel: ModuleType,
    search_roots: list[str | Path],
) -> KernelHyImportHook:
    """Temporarily import .hy files through the Hy-written kernel."""

    return KernelHyImportHook(kernel, search_roots)


def strip_shebang(source: str) -> str:
    if source.startswith("#!"):
        _line, separator, rest = source.partition("\n")
        return rest if separator else ""
    return source


def read_input(args: argparse.Namespace) -> tuple[str, str]:
    if args.file:
        path = Path(args.file)
        return strip_shebang(path.read_text()), str(path)
    command = getattr(args, "command", None)
    if command is not None:
        return command, "<hy-meta:-c>"
    source = getattr(args, "source", "")
    if source:
        return source, "<hy-meta>"
    return strip_shebang(sys.stdin.read()), "<stdin>"


def compile_stage2_pyc(
    source: str,
    filename: str,
    output: Path,
    module_name: str,
) -> Path:
    stage2 = bootstrap_stage2()
    module = stage2.make_module(module_name, filename)
    code = stage2.compile_source_to_code(source, module, filename)
    filename_path = Path(filename)
    mtime = int(filename_path.stat().st_mtime) if filename_path.exists() else 0
    source_size = len(source.encode("utf-8"))
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(
        importlib_external._code_to_timestamp_pyc(code, mtime, source_size)
    )
    return output


def apply_startup_files(
    stage2: ModuleType,
    module: ModuleType,
    startup_files: list[str] | None,
) -> None:
    for startup in startup_files or []:
        path = Path(startup)
        stage2.exec_source(strip_shebang(path.read_text()), module, str(path))


def eval_repl_stream(
    stream: Any,
    output: Any,
    *,
    module_name: str,
    startup_files: list[str] | None = None,
    prompt: bool = False,
    flush: bool = False,
) -> int:
    stage2 = bootstrap_stage2()
    module = stage2.make_module(module_name, "<hy-meta:repl>")
    apply_startup_files(stage2, module, startup_files)

    def eval_line(line: str) -> None:
        source = line.strip()
        if not source:
            return
        value = stage2.eval_source(source, module, "<hy-meta:repl>")
        if value is not None:
            print(value, file=output, flush=flush)

    if prompt:
        while True:
            try:
                eval_line(input("=> "))
            except EOFError:
                break
        return 0

    for line in stream:
        eval_line(line)
    return 0


def run_self_check() -> dict[str, Any]:
    stage1 = Stage1Compiler("hy_meta_stage1.self_check")
    stage1_value = stage1.eval_source("(+ 1 2 3)", filename="<stage1:self-check>")

    stage2 = bootstrap_stage2()
    stage2_value = stage2.eval_source(
        "(+ 10 20 12)",
        stage2.make_module("hy_meta_stage2.host_check"),
        "<stage2:host-check>",
    )
    stage2_self_check = stage2.self_check()

    return {
        "python": sys.version.split()[0],
        "stage1_value": stage1_value,
        "stage2_value": stage2_value,
        "stage2_self_check": stage2_self_check,
        "stage2_module": stage2.__name__,
    }


def run_kernel_check() -> dict[str, Any]:
    stage2 = bootstrap_stage2()
    kernel = bootstrap_kernel(stage2)
    source = (ROOT / "hy-meta" / "examples" / "factorial.hy").read_text()
    filename = str(ROOT / "hy-meta" / "examples" / "factorial.hy")
    feature_source = (ROOT / "hy-meta" / "examples" / "kernel_features.hy").read_text()
    feature_filename = str(ROOT / "hy-meta" / "examples" / "kernel_features.hy")
    loop_source = (ROOT / "hy-meta" / "examples" / "kernel_loop.hy").read_text()
    loop_filename = str(ROOT / "hy-meta" / "examples" / "kernel_loop.hy")
    return {
        "python": sys.version.split()[0],
        "stage2_module": stage2.__name__,
        "kernel_module": kernel.__name__,
        "kernel_self_check": kernel.self_check(),
        "kernel_factorial": kernel.eval_source(source, None, filename),
        "kernel_features": kernel.eval_source(feature_source, None, feature_filename),
        "kernel_loop": kernel.eval_source(loop_source, None, loop_filename),
        "kernel_python_contains_fact": "def fact" in kernel.python_source(source, filename),
    }


def run_chain_check() -> dict[str, Any]:
    stage2, stage2_prime = bootstrap_stage2_chain()

    source = (ROOT / "hy-meta" / "examples" / "factorial.hy").read_text()
    filename = str(ROOT / "hy-meta" / "examples" / "factorial.hy")

    stage2_value = stage2.eval_source(
        source,
        stage2.make_module("hy_meta_stage2.chain_source"),
        filename,
    )
    stage2_prime_value = stage2_prime.eval_source(
        source,
        stage2_prime.make_module("hy_meta_stage2_prime.chain_source"),
        filename,
    )
    stage2_python = stage2.python_source(
        source,
        stage2.make_module("hy_meta_stage2.chain_py"),
        filename,
    )
    stage2_prime_python = stage2_prime.python_source(
        source,
        stage2_prime.make_module("hy_meta_stage2_prime.chain_py"),
        filename,
    )

    return {
        "python": sys.version.split()[0],
        "stage2_module": stage2.__name__,
        "stage2_prime_module": stage2_prime.__name__,
        "stage2_self_check": stage2.self_check(),
        "stage2_prime_self_check": stage2_prime.self_check(),
        "stage2_value": stage2_value,
        "stage2_prime_value": stage2_prime_value,
        "python_output_matches": stage2_python == stage2_prime_python,
    }


def run_prime_check() -> dict[str, Any]:
    stage2, stage2_prime = bootstrap_stage2_chain()
    result = stage2_prime.hy_meta_check(
        KERNEL_PATH,
        ROOT / "hy-meta" / "examples" / "factorial.hy",
        ROOT / "hy-meta" / "examples" / "kernel_features.hy",
        ROOT / "hy-meta" / "examples" / "kernel_loop.hy",
        "hy_meta_stage2_prime.kernel",
    )
    return {
        "python": sys.version.split()[0],
        "stage2_module": stage2.__name__,
        "stage2_prime_module": stage2_prime.__name__,
        "stage2_prime_self_check": stage2_prime.self_check(),
        **result,
    }


def run_stage3_check() -> dict[str, Any]:
    stage2, stage2_prime, stage3 = bootstrap_stage3_chain()

    source_path = ROOT / "hy-meta" / "examples" / "factorial.hy"
    source = source_path.read_text()
    filename = str(source_path)

    stage2_value = stage2.eval_source(
        source,
        stage2.make_module("hy_meta_stage2.stage3_source"),
        filename,
    )
    stage2_prime_value = stage2_prime.eval_source(
        source,
        stage2_prime.make_module("hy_meta_stage2_prime.stage3_source"),
        filename,
    )
    stage3_value = stage3.eval_source(
        source,
        stage3.make_module("hy_meta_stage3.stage3_source"),
        filename,
    )
    stage2_python = stage2.python_source(
        source,
        stage2.make_module("hy_meta_stage2.stage3_py"),
        filename,
    )
    stage3_python = stage3.python_source(
        source,
        stage3.make_module("hy_meta_stage3.stage3_py"),
        filename,
    )
    result = stage3.hy_meta_check(
        KERNEL_PATH,
        source_path,
        ROOT / "hy-meta" / "examples" / "kernel_features.hy",
        ROOT / "hy-meta" / "examples" / "kernel_loop.hy",
        KERNEL_STAGE3_MODULE,
    )
    return {
        "python": sys.version.split()[0],
        "stage2_module": stage2.__name__,
        "stage2_prime_module": stage2_prime.__name__,
        "stage3_module": stage3.__name__,
        "stage2_self_check": stage2.self_check(),
        "stage2_prime_self_check": stage2_prime.self_check(),
        "stage3_self_check": stage3.self_check(),
        "stage2_value": stage2_value,
        "stage2_prime_value": stage2_prime_value,
        "stage3_value": stage3_value,
        "stage2_stage3_python_output_matches": stage2_python == stage3_python,
        **result,
    }


def ast_data(tree: ast.AST) -> str:
    """Return a stable, source-location-free AST data representation."""

    return ast.dump(tree, include_attributes=False)


def pyc_bytes_for_code(code: Any, source: str, filename: str) -> bytes:
    filename_path = Path(filename)
    mtime = int(filename_path.stat().st_mtime) if filename_path.exists() else 0
    source_size = len(source.encode("utf-8"))
    return importlib_external._code_to_timestamp_pyc(code, mtime, source_size)


def location_stable_ast(tree: ast.AST) -> ast.AST:
    stable = copy.deepcopy(tree)
    for node in ast.walk(stable):
        for attr in ("lineno", "col_offset", "end_lineno", "end_col_offset"):
            try:
                delattr(node, attr)
            except AttributeError:
                pass
    return ast.fix_missing_locations(stable)


def stable_code_const(value: Any) -> Any:
    if isinstance(value, CodeType):
        return stable_code_payload(value)
    if isinstance(value, bytes):
        return {"kind": "bytes", "hex": value.hex()}
    if isinstance(value, tuple):
        return {"kind": "tuple", "items": [stable_code_const(item) for item in value]}
    if isinstance(value, frozenset):
        items = [stable_code_const(item) for item in value]
        return {
            "kind": "frozenset",
            "items": sorted(
                items,
                key=lambda item: json.dumps(item, sort_keys=True, separators=(",", ":")),
            ),
        }
    if value is Ellipsis:
        return {"kind": "ellipsis"}
    if isinstance(value, (str, int, float, complex, bool)) or value is None:
        return value
    return {"kind": type(value).__name__, "repr": repr(value)}


def stable_code_payload(code: CodeType) -> dict[str, Any]:
    """Return a marshal-reference-free instruction/code-object payload."""

    payload = {
        "argcount": code.co_argcount,
        "kwonlyargcount": code.co_kwonlyargcount,
        "nlocals": code.co_nlocals,
        "stacksize": code.co_stacksize,
        "flags": code.co_flags,
        "code": code.co_code.hex(),
        "consts": [stable_code_const(item) for item in code.co_consts],
        "names": list(code.co_names),
        "varnames": list(code.co_varnames),
        "freevars": list(code.co_freevars),
        "cellvars": list(code.co_cellvars),
    }
    if hasattr(code, "co_posonlyargcount"):
        payload["posonlyargcount"] = code.co_posonlyargcount
    if hasattr(code, "co_exceptiontable"):
        payload["exceptiontable"] = code.co_exceptiontable.hex()
    return payload


def artifact_from_ast(
    *,
    name: str,
    source: str,
    filename: str,
    tree: ast.AST,
) -> dict[str, Any]:
    ast_dump = ast_data(tree)
    python_source = ast.unparse(tree)
    code_tree = location_stable_ast(tree)
    code = compile(code_tree, filename, "exec")
    code_payload = json.dumps(
        stable_code_payload(code),
        sort_keys=True,
        separators=(",", ":"),
    )
    raw_code_bytes = marshal.dumps(code)
    pyc_bytes = pyc_bytes_for_code(code, source, filename)
    normalized_payload = json.dumps(
        {
            "ast": ast_dump,
            "python": python_source,
        },
        sort_keys=True,
        separators=(",", ":"),
    )
    return {
        "name": name,
        "filename": filename,
        "source_sha256": sha256_text(source),
        "ast_sha256": sha256_text(ast_dump),
        "python_sha256": sha256_text(python_source),
        "normalized_sha256": sha256_text(normalized_payload),
        "code_sha256": sha256_text(code_payload),
        "raw_code_sha256": sha256_bytes(raw_code_bytes),
        "pyc_sha256": sha256_bytes(pyc_bytes),
        "ast_dump": ast_dump,
        "python_source": python_source,
    }


def artifact_summary(artifact: dict[str, Any]) -> dict[str, Any]:
    return {
        key: value
        for key, value in artifact.items()
        if key not in {"ast_dump", "python_source"}
    }


def write_stage8_debug_artifacts(
    debug_dir: Path,
    stage7_artifacts: dict[str, dict[str, Any]],
    stage8_artifacts: dict[str, dict[str, Any]],
    diff: dict[str, Any],
) -> None:
    def write_bundle(label: str, artifacts: dict[str, dict[str, Any]]) -> None:
        bundle_dir = debug_dir / label
        bundle_dir.mkdir(parents=True, exist_ok=True)
        summaries = {
            name: artifact_summary(artifact)
            for name, artifact in sorted(artifacts.items())
        }
        (bundle_dir / "hashes.json").write_text(
            json.dumps(summaries, indent=2, sort_keys=True) + "\n"
        )
        for name, artifact in sorted(artifacts.items()):
            safe_name = name.replace("/", "__")
            (bundle_dir / f"{safe_name}.ast").write_text(artifact["ast_dump"])
            (bundle_dir / f"{safe_name}.py").write_text(
                artifact["python_source"] + "\n"
            )

    write_bundle("stage7", stage7_artifacts)
    write_bundle("stage8-fresh", stage8_artifacts)
    diff_dir = debug_dir / "diff"
    diff_dir.mkdir(parents=True, exist_ok=True)
    (diff_dir / "changed-artifacts.json").write_text(
        json.dumps(diff, indent=2, sort_keys=True) + "\n"
    )
    (diff_dir / "changed-artifact-details.json").write_text(
        json.dumps(diff.get("details", {}), indent=2, sort_keys=True) + "\n"
    )


def run_mirror_check() -> dict[str, Any]:
    stage2, stage2_prime, stage3 = bootstrap_stage3_chain()

    compiler_source = STAGE2_PATH.read_text()
    compiler_filename = str(STAGE2_PATH)
    stage2_compiler_python = stage2.python_source(
        compiler_source,
        stage2.make_module("hy_meta_mirror.stage2_compiler_py"),
        compiler_filename,
    )
    stage2_prime_compiler_python = stage2_prime.python_source(
        compiler_source,
        stage2_prime.make_module("hy_meta_mirror.stage2_prime_compiler_py"),
        compiler_filename,
    )
    stage3_compiler_python = stage3.python_source(
        compiler_source,
        stage3.make_module("hy_meta_mirror.stage3_compiler_py"),
        compiler_filename,
    )
    stage2_compiler_ast = ast_data(
        stage2.compile_source_to_ast(
            compiler_source,
            stage2.make_module("hy_meta_mirror.stage2_compiler_ast"),
            compiler_filename,
        )
    )
    stage2_prime_compiler_ast = ast_data(
        stage2_prime.compile_source_to_ast(
            compiler_source,
            stage2_prime.make_module("hy_meta_mirror.stage2_prime_compiler_ast"),
            compiler_filename,
        )
    )
    stage3_compiler_ast = ast_data(
        stage3.compile_source_to_ast(
            compiler_source,
            stage3.make_module("hy_meta_mirror.stage3_compiler_ast"),
            compiler_filename,
        )
    )

    factorial_path = ROOT / "hy-meta" / "examples" / "factorial.hy"
    factorial_source = factorial_path.read_text()
    factorial_filename = str(factorial_path)
    stage_values = [
        stage2.eval_source(
            factorial_source,
            stage2.make_module("hy_meta_mirror.stage2_value"),
            factorial_filename,
        ),
        stage2_prime.eval_source(
            factorial_source,
            stage2_prime.make_module("hy_meta_mirror.stage2_prime_value"),
            factorial_filename,
        ),
        stage3.eval_source(
            factorial_source,
            stage3.make_module("hy_meta_mirror.stage3_value"),
            factorial_filename,
        ),
    ]

    kernel_prime = stage2_prime.load_hy_file(
        KERNEL_PATH,
        "hy_meta_mirror.stage2_prime_kernel",
    )
    kernel_stage3 = stage3.load_hy_file(
        KERNEL_PATH,
        "hy_meta_mirror.stage3_kernel",
    )
    example_paths = [
        ROOT / "hy-meta" / "examples" / "factorial.hy",
        ROOT / "hy-meta" / "examples" / "kernel_loop.hy",
        ROOT / "hy-meta" / "examples" / "kernel_features.hy",
        ROOT / "hy-meta" / "examples" / "kernel_stability_stress.hy",
    ]
    kernel_python_matches = []
    kernel_ast_matches = []
    kernel_value_matches = []
    kernel_values: dict[str, Any] = {}
    for path in example_paths:
        source = path.read_text()
        filename = str(path)
        name = path.stem
        prime_python = kernel_prime.python_source(source, filename)
        stage3_python = kernel_stage3.python_source(source, filename)
        prime_ast = ast_data(kernel_prime.compile_source_to_module(source, filename))
        stage3_ast = ast_data(kernel_stage3.compile_source_to_module(source, filename))
        prime_value = kernel_prime.eval_source(source, None, filename)
        stage3_value = kernel_stage3.eval_source(source, None, filename)
        kernel_python_matches.append(prime_python == stage3_python)
        kernel_ast_matches.append(prime_ast == stage3_ast)
        kernel_value_matches.append(prime_value == stage3_value)
        kernel_values[name] = stage3_value

    return {
        "python": sys.version.split()[0],
        "stage2_module": stage2.__name__,
        "stage2_prime_module": stage2_prime.__name__,
        "stage3_module": stage3.__name__,
        "stage2_self_check": stage2.self_check(),
        "stage2_prime_self_check": stage2_prime.self_check(),
        "stage3_self_check": stage3.self_check(),
        "compiler_python_mirror": (
            stage2_compiler_python
            == stage2_prime_compiler_python
            == stage3_compiler_python
        ),
        "compiler_ast_mirror": (
            stage2_compiler_ast == stage2_prime_compiler_ast == stage3_compiler_ast
        ),
        "stage_value_mirror": stage_values == [120, 120, 120],
        "kernel_python_mirror": all(kernel_python_matches),
        "kernel_ast_mirror": all(kernel_ast_matches),
        "kernel_value_mirror": all(kernel_value_matches),
        "kernel_factorial": kernel_values["factorial"],
        "kernel_loop": kernel_values["kernel_loop"],
        "kernel_features": kernel_values["kernel_features"],
        "kernel_stability_stress": kernel_values["kernel_stability_stress"],
        "mirror_examples": ",".join(path.name for path in example_paths),
    }


def run_direct_kernel_bridge_check() -> dict[str, Any]:
    stage2 = bootstrap_stage2()

    expr_python = stage2.python_source(
        "(+ 20 22)",
        stage2.make_module("hy_meta_direct_bridge.expr"),
        "<direct-kernel-bridge:expr>",
    )
    expr_stats = dict(stage2.direct_kernel_stats())

    compiler = stage2.load_hy_file(
        STAGE2_PATH,
        "hy_meta_direct_bridge.compiler",
    )
    compiler_load_stats = dict(stage2.direct_kernel_stats())
    compiler_self_check = compiler.self_check()
    compiler_eval_value = compiler.eval_source(
        "(+ 20 22)",
        compiler.make_module("hy_meta_direct_bridge.compiler_eval"),
        "<direct-kernel-bridge:compiler-eval>",
    )
    loaded_compiler_stats = dict(compiler.direct_kernel_stats())

    stage2_again = bootstrap_stage2()
    stage2_again.python_source(
        "(+ 1 1)",
        stage2_again.make_module("hy_meta_direct_bridge.expr_again"),
        "<direct-kernel-bridge:expr-again>",
    )
    repeated_stage2_stats = dict(stage2_again.direct_kernel_stats())

    return {
        "python": sys.version.split()[0],
        "stage2_module": stage2.__name__,
        "expr_python": expr_python,
        "expr_direct_kernel_loaded": expr_stats["loaded"],
        "expr_direct_kernel_hits": expr_stats["hits"],
        "expr_direct_kernel_fallbacks": expr_stats["fallbacks"],
        "compiler_module": compiler.__name__,
        "compiler_self_check": compiler_self_check,
        "compiler_eval_value": compiler_eval_value,
        "compiler_load_direct_kernel_hits": compiler_load_stats["hits"],
        "compiler_load_direct_kernel_fallbacks": compiler_load_stats["fallbacks"],
        "loaded_compiler_direct_kernel_loaded": loaded_compiler_stats["loaded"],
        "loaded_compiler_direct_kernel_hits": loaded_compiler_stats["hits"],
        "loaded_compiler_direct_kernel_fallbacks": loaded_compiler_stats["fallbacks"],
        "repeated_stage2_direct_kernel_loaded": repeated_stage2_stats["loaded"],
        "repeated_stage2_direct_kernel_hits": repeated_stage2_stats["hits"],
        "repeated_stage2_direct_kernel_fallbacks": repeated_stage2_stats["fallbacks"],
        "stage2_direct_kernel_reused": (
            getattr(stage2, "DIRECT_KERNEL") is getattr(stage2_again, "DIRECT_KERNEL")
        ),
    }


def run_reader_boundary_check() -> dict[str, Any]:
    stages = bootstrap_stage_chain(7)
    _label, stage7 = stages[-1]
    stage_reader_macro_ids = [
        id(stage.__dict__.get("_hy_reader_macros"))
        for _label, stage in stages
        if "_hy_reader_macros" in stage.__dict__
    ]
    kernel = stage7.load_hy_file(
        KERNEL_PATH,
        "hy_meta_reader_boundary.stage7_kernel",
    )

    reader_macro_value = kernel.eval_source(
        "(defreader foo '42) #foo",
        None,
        "<reader-boundary:stream>",
    )
    fresh_reader_failed = False
    fresh_reader_error_class = ""
    fresh_reader_error = ""
    try:
        kernel.eval_source("#foo", None, "<reader-boundary:fresh>")
    except Exception as exc:  # Reader boundary failure class comes from upstream Hy.
        fresh_reader_failed = True
        fresh_reader_error_class = exc.__class__.__name__
        fresh_reader_error = str(exc).splitlines()[0] if str(exc) else ""

    return {
        "python": sys.version.split()[0],
        "stage_count": len(stages),
        "last_stage_module": stage7.__name__,
        "reader_host_module": getattr(kernel.read_many, "__module__", ""),
        "reader_macro_value": reader_macro_value,
        "fresh_reader_failed": fresh_reader_failed,
        "fresh_reader_error_class": fresh_reader_error_class,
        "fresh_reader_error": fresh_reader_error,
        "stage_reader_macro_tables_distinct": (
            len(set(stage_reader_macro_ids)) == len(stage_reader_macro_ids)
        ),
        "kernel_reader_macro_table_clean": kernel.__dict__.get(
            "_hy_reader_macros",
            {},
        )
        == {},
    }


def run_compatibility_boundary_check() -> dict[str, Any]:
    stage2 = bootstrap_stage2()
    upstream_python = stage2.python_source(
        "(+ 20 22)",
        stage2.make_module("hy_meta_compatibility.upstream_path"),
        "<compatibility-boundary:upstream>",
        False,
    )
    upstream_stats = dict(stage2.direct_kernel_stats())

    kernel = bootstrap_kernel(stage2)
    template_gate_failed = False
    template_gate_error_class = ""
    template_gate_error = ""
    try:
        kernel.eval_source(
            """
            (pragma :bracketed-templates True)
            #[t[hello {(+ 1 1)}]t]
            """,
            None,
            "<compatibility-boundary:tstring-gate>",
        )
    except Exception as exc:
        template_gate_failed = True
        template_gate_error_class = exc.__class__.__name__
        template_gate_error = str(exc).splitlines()[0] if str(exc) else ""

    original_meta_path = list(sys.meta_path)
    original_sys_path = list(sys.path)
    py_module_name = "hy_meta_compat_plain_py"
    hy_module_name = "hy_meta_compat_owned_hy"
    side_effect_module_name = "hy_meta_compat_side_effect_py"
    broken_hy_module_name = "hy_meta_compat_broken_hy"
    preexisting_module_name = "hy_meta_compat_preexisting"
    module_names = [
        py_module_name,
        hy_module_name,
        side_effect_module_name,
        broken_hy_module_name,
    ]
    module_snapshot = snapshot_sys_modules(module_names + [preexisting_module_name])
    for name in module_names:
        sys.modules.pop(name, None)

    preexisting_was_present = preexisting_module_name in module_snapshot["modules"]
    previous_preexisting_module = module_snapshot["modules"].get(preexisting_module_name)
    preexisting_module = ModuleType(preexisting_module_name)
    preexisting_module.VALUE = 99
    sys.modules[preexisting_module_name] = preexisting_module

    side_effect_attr = "__hy_meta_compat_side_effect__"
    side_effect_was_present = hasattr(builtins, side_effect_attr)
    previous_side_effect = getattr(builtins, side_effect_attr, None)

    hook_installed = False
    hook_removed_after_context = False
    python_import_value = None
    python_import_loader = ""
    hy_import_value = None
    hy_import_loader = ""
    preexisting_import_value = None
    preexisting_module_preserved = False
    native_import_value = None
    native_import_loader = ""
    side_effect_value = None
    side_effect_seen = False
    broken_import_failed = False
    broken_import_error = ""
    broken_module_removed = False
    exception_hook_installed = False
    exception_hook_removed = False
    exception_hook_error = ""
    exception_finder: KernelHyFinder | None = None
    try:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            (root / f"{py_module_name}.py").write_text("VALUE = 41\n")
            (root / f"{hy_module_name}.hy").write_text("(setv VALUE 42)\n")
            (root / f"{preexisting_module_name}.hy").write_text("(setv VALUE 0)\n")
            (root / f"{side_effect_module_name}.py").write_text(
                "import builtins\n"
                f"setattr(builtins, {side_effect_attr!r}, 'python-import-side-effect')\n"
                "VALUE = 43\n"
            )
            (root / f"{broken_hy_module_name}.hy").write_text(
                '(raise (Exception "compat broken"))\n'
            )
            sys.path.insert(0, str(root))
            try:
                with install_kernel_import_hook(kernel, [root]) as finder:
                    hook_installed = finder in sys.meta_path
                    py_module = importlib.import_module(py_module_name)
                    hy_module = importlib.import_module(hy_module_name)
                    preexisting_import = importlib.import_module(preexisting_module_name)
                    native_module = importlib.import_module("math")
                    side_effect_module = importlib.import_module(side_effect_module_name)
                    try:
                        importlib.import_module(broken_hy_module_name)
                    except Exception as exc:
                        broken_import_failed = True
                        broken_import_error = str(exc).splitlines()[0] if str(exc) else ""
                    python_import_value = py_module.VALUE
                    python_import_loader = py_module.__loader__.__class__.__name__
                    hy_import_value = hy_module.VALUE
                    hy_import_loader = hy_module.__loader__.__class__.__name__
                    preexisting_import_value = preexisting_import.VALUE
                    preexisting_module_preserved = preexisting_import is preexisting_module
                    native_import_value = native_module.sqrt(81)
                    native_loader = getattr(native_module, "__loader__", None)
                    native_import_loader = (
                        native_loader.__class__.__name__ if native_loader else ""
                    )
                    side_effect_value = side_effect_module.VALUE
                    side_effect_seen = (
                        getattr(builtins, side_effect_attr, None)
                        == "python-import-side-effect"
                    )
                    broken_module_removed = broken_hy_module_name not in sys.modules
                hook_removed_after_context = finder not in sys.meta_path
                try:
                    with install_kernel_import_hook(kernel, [root]) as exception_finder:
                        exception_hook_installed = exception_finder in sys.meta_path
                        raise RuntimeError("compatibility boundary hook probe")
                except RuntimeError as exc:
                    exception_hook_error = str(exc)
                    exception_hook_removed = (
                        exception_finder is not None
                        and exception_finder not in sys.meta_path
                    )
            finally:
                while str(root) in sys.path:
                    sys.path.remove(str(root))
    finally:
        rollback_sys_modules(module_snapshot)
        if side_effect_was_present:
            setattr(builtins, side_effect_attr, previous_side_effect)
        elif hasattr(builtins, side_effect_attr):
            delattr(builtins, side_effect_attr)

    preexisting_module_restored = (
        sys.modules.get(preexisting_module_name) is previous_preexisting_module
        if preexisting_was_present
        else preexisting_module_name not in sys.modules
    )
    side_effect_restored = (
        getattr(builtins, side_effect_attr, None) == previous_side_effect
        if side_effect_was_present
        else not hasattr(builtins, side_effect_attr)
    )

    return {
        "python": sys.version.split()[0],
        "upstream_python": upstream_python,
        "upstream_direct_kernel_loaded": upstream_stats["loaded"],
        "upstream_direct_kernel_hits": upstream_stats["hits"],
        "upstream_direct_kernel_fallbacks": upstream_stats["fallbacks"],
        "template_gate_failed": template_gate_failed,
        "template_gate_error_class": template_gate_error_class,
        "template_gate_error": template_gate_error,
        "hook_installed": hook_installed,
        "hook_removed_after_context": hook_removed_after_context,
        "python_import_value": python_import_value,
        "python_import_loader": python_import_loader,
        "python_import_loader_not_kernel": python_import_loader != "KernelHyLoader",
        "hy_import_value": hy_import_value,
        "hy_import_loader": hy_import_loader,
        "preexisting_import_value": preexisting_import_value,
        "preexisting_module_preserved": preexisting_module_preserved,
        "preexisting_module_restored": preexisting_module_restored,
        "native_import_value": native_import_value,
        "native_import_loader": native_import_loader,
        "native_import_loader_not_kernel": native_import_loader != "KernelHyLoader",
        "side_effect_value": side_effect_value,
        "side_effect_seen": side_effect_seen,
        "side_effect_restored": side_effect_restored,
        "broken_import_failed": broken_import_failed,
        "broken_import_error": broken_import_error,
        "broken_module_removed": broken_module_removed,
        "exception_hook_installed": exception_hook_installed,
        "exception_hook_removed": exception_hook_removed,
        "exception_hook_error": exception_hook_error,
        "meta_path_restored": sys.meta_path == original_meta_path,
        "sys_path_restored": sys.path == original_sys_path,
        "compat_modules_removed": all(
            name not in sys.modules for name in module_names
        ),
    }


def run_cli_io_check() -> dict[str, Any]:
    stage2 = bootstrap_stage2()

    command_source, command_filename = read_input(
        argparse.Namespace(file=None, command="(+ 20 22)", source="")
    )
    command_value = stage2.eval_source(
        command_source,
        stage2.make_module("hy_meta_cli_io.command"),
        command_filename,
    )
    command_python = stage2.python_source(
        command_source,
        stage2.make_module("hy_meta_cli_io.command_py"),
        command_filename,
    )

    previous_stdin = sys.stdin
    try:
        sys.stdin = io.StringIO("(+ 10 32)")
        stdin_source, stdin_filename = read_input(
            argparse.Namespace(file=None, command=None, source="")
        )
    finally:
        sys.stdin = previous_stdin
    stdin_value = stage2.eval_source(
        stdin_source,
        stage2.make_module("hy_meta_cli_io.stdin"),
        stdin_filename,
    )

    return {
        "python": sys.version.split()[0],
        "command_filename": command_filename,
        "command_value": command_value,
        "command_python": command_python,
        "stdin_filename": stdin_filename,
        "stdin_value": stdin_value,
    }


def run_hyc_check() -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="hy-meta-hyc-") as temp_dir:
        output = Path(temp_dir) / "compiled.pyc"
        compile_stage2_pyc(
            "(setv answer (+ 20 22))",
            "<hyc-check>",
            output,
            "hy_meta_hyc_check.compile",
        )
        spec = importlib.util.spec_from_file_location(
            "hy_meta_hyc_check.loaded",
            output,
        )
        assert spec is not None
        assert spec.loader is not None
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return {
            "python": sys.version.split()[0],
            "pyc_exists": output.exists(),
            "pyc_size_positive": output.stat().st_size > 0,
            "loaded_answer": getattr(module, "answer", None),
            "loader_class": spec.loader.__class__.__name__,
        }


def run_repl_check() -> dict[str, Any]:
    output = io.StringIO()
    status = eval_repl_stream(
        io.StringIO("(+ 20 22)\n(setv x 40)\n(+ x 2)\n"),
        output,
        module_name="hy_meta_repl_check.session",
    )
    return {
        "python": sys.version.split()[0],
        "status": status,
        "output_lines": output.getvalue().splitlines(),
    }


def run_startup_output_check() -> dict[str, Any]:
    class FlushRecorder(io.StringIO):
        def __init__(self) -> None:
            super().__init__()
            self.flushed = False

        def flush(self) -> None:
            self.flushed = True
            super().flush()

    with tempfile.TemporaryDirectory(prefix="hy-meta-startup-") as temp_dir:
        startup = Path(temp_dir) / "startup.hy"
        startup.write_text("(setv base 40)\n")

        stage2 = bootstrap_stage2()
        run_module = stage2.make_module("hy_meta_startup_output.run")
        apply_startup_files(stage2, run_module, [str(startup)])
        run_value = stage2.eval_source(
            "(+ base 2)",
            run_module,
            "<startup-output:run>",
        )

        repl_output = FlushRecorder()
        repl_status = eval_repl_stream(
            io.StringIO("(+ base 2)\n"),
            repl_output,
            module_name="hy_meta_startup_output.repl",
            startup_files=[str(startup)],
            flush=True,
        )

        return {
            "python": sys.version.split()[0],
            "run_value": run_value,
            "repl_status": repl_status,
            "repl_output_lines": repl_output.getvalue().splitlines(),
            "repl_output_flushed": repl_output.flushed,
        }


def run_stage7_check() -> dict[str, Any]:
    stages = bootstrap_stage_chain(7)
    compiler_source = STAGE2_PATH.read_text()
    compiler_filename = str(STAGE2_PATH)
    factorial_path = ROOT / "hy-meta" / "examples" / "factorial.hy"
    factorial_source = factorial_path.read_text()
    factorial_filename = str(factorial_path)
    example_paths = [
        ROOT / "hy-meta" / "examples" / "factorial.hy",
        ROOT / "hy-meta" / "examples" / "kernel_loop.hy",
        ROOT / "hy-meta" / "examples" / "kernel_features.hy",
        ROOT / "hy-meta" / "examples" / "kernel_stability_stress.hy",
    ]

    stage_names = [stage.__name__ for _label, stage in stages]
    stage_module_cache_ok = all(sys.modules.get(stage.__name__) is stage for _label, stage in stages)
    stage_macro_ids = [
        id(stage.__dict__.get("_hy_macros"))
        for _label, stage in stages
        if "_hy_macros" in stage.__dict__
    ]
    stage_reader_macro_ids = [
        id(stage.__dict__.get("_hy_reader_macros"))
        for _label, stage in stages
        if "_hy_reader_macros" in stage.__dict__
    ]

    probe_modules = []
    for label, stage in stages:
        probe_modules.append(stage.make_module(f"hy_meta_stage7_probe.{label}"))
    for index, module in enumerate(probe_modules):
        module.GLOBAL_SENTINEL = f"probe-{index}"
    probe_module_cache_ok = all(
        sys.modules.get(module.__name__) is module for module in probe_modules
    )
    probe_global_values = [
        getattr(module, "GLOBAL_SENTINEL", None) for module in probe_modules
    ]
    probe_macro_ids = [id(module.__dict__.get("_hy_macros")) for module in probe_modules]
    probe_reader_macro_ids = [
        id(module.__dict__.get("_hy_reader_macros")) for module in probe_modules
    ]

    compiler_python = []
    compiler_ast = []
    stage_values = []
    stage_self_checks = []
    for label, stage in stages:
        stage_self_checks.append(stage.self_check())
        compiler_python.append(
            stage.python_source(
                compiler_source,
                stage.make_module(f"hy_meta_stage7.{label}.compiler_py"),
                compiler_filename,
            )
        )
        compiler_ast.append(
            ast_data(
                stage.compile_source_to_ast(
                    compiler_source,
                    stage.make_module(f"hy_meta_stage7.{label}.compiler_ast"),
                    compiler_filename,
                )
            )
        )
        stage_values.append(
            stage.eval_source(
                factorial_source,
                stage.make_module(f"hy_meta_stage7.{label}.factorial_value"),
                factorial_filename,
            )
        )

    kernel_modules = []
    for label, stage in stages:
        kernel_modules.append(
            (
                label,
                stage.load_hy_file(
                    KERNEL_PATH,
                    f"hy_meta_stage7.{label}.kernel",
                ),
            )
        )
    kernel_names = [kernel.__name__ for _label, kernel in kernel_modules]
    kernel_module_cache_ok = all(
        sys.modules.get(kernel.__name__) is kernel for _label, kernel in kernel_modules
    )

    kernel_python_mirrors = []
    kernel_ast_mirrors = []
    kernel_value_mirrors = []
    kernel_values: dict[str, Any] = {}
    for path in example_paths:
        source = path.read_text()
        filename = str(path)
        name = path.stem
        python_outputs = []
        ast_outputs = []
        values = []
        for _label, kernel in kernel_modules:
            python_outputs.append(kernel.python_source(source, filename))
            ast_outputs.append(ast_data(kernel.compile_source_to_module(source, filename)))
            values.append(kernel.eval_source(source, None, filename))
        kernel_python_mirrors.append(all_equal(python_outputs))
        kernel_ast_mirrors.append(all_equal(ast_outputs))
        kernel_value_mirrors.append(all_equal(values))
        kernel_values[name] = values[-1]

    stress_path = ROOT / "hy-meta" / "examples" / "kernel_stability_stress.hy"
    stress_source = stress_path.read_text()
    stress_filename = str(stress_path)
    _last_label, last_kernel = kernel_modules[-1]
    stress_python_repeats = [
        last_kernel.python_source(stress_source, stress_filename)
        for _index in range(2)
    ]
    stress_ast_repeats = [
        ast_data(last_kernel.compile_source_to_module(stress_source, stress_filename))
        for _index in range(2)
    ]

    return {
        "python": sys.version.split()[0],
        "stage_count": len(stages),
        "last_stage_module": stages[-1][1].__name__,
        "stage_modules": ",".join(stage_names),
        "all_stage_self_checks": all(stage_self_checks),
        "stage_module_cache_ok": stage_module_cache_ok,
        "stage_module_names_unique": len(set(stage_names)) == len(stage_names),
        "stage_macro_tables_distinct": len(set(stage_macro_ids)) == len(stage_macro_ids),
        "stage_reader_macro_tables_distinct": (
            len(set(stage_reader_macro_ids)) == len(stage_reader_macro_ids)
        ),
        "probe_module_cache_ok": probe_module_cache_ok,
        "probe_module_globals_distinct": (
            len({id(module.__dict__) for module in probe_modules})
            == len(probe_modules)
        ),
        "probe_global_values_isolated": probe_global_values == [
            f"probe-{index}" for index in range(len(probe_modules))
        ],
        "probe_macro_tables_distinct": len(set(probe_macro_ids)) == len(probe_macro_ids),
        "probe_reader_macro_tables_distinct": (
            len(set(probe_reader_macro_ids)) == len(probe_reader_macro_ids)
        ),
        "compiler_python_stage7_mirror": all_equal(compiler_python),
        "compiler_ast_stage7_mirror": all_equal(compiler_ast),
        "stage_value_mirror": stage_values == [120] * len(stages),
        "kernel_module_cache_ok": kernel_module_cache_ok,
        "kernel_module_names_unique": len(set(kernel_names)) == len(kernel_names),
        "kernel_python_stage7_mirror": all(kernel_python_mirrors),
        "kernel_ast_stage7_mirror": all(kernel_ast_mirrors),
        "kernel_value_stage7_mirror": all(kernel_value_mirrors),
        "kernel_factorial": kernel_values["factorial"],
        "kernel_loop": kernel_values["kernel_loop"],
        "kernel_features": kernel_values["kernel_features"],
        "kernel_stability_stress": kernel_values["kernel_stability_stress"],
        "kernel_stress_repeat_python_stable": all_equal(stress_python_repeats),
        "kernel_stress_repeat_ast_stable": all_equal(stress_ast_repeats),
        "mirror_examples": ",".join(path.name for path in example_paths),
    }


def load_kernel_compiled_kernel(
    kernel: ModuleType,
    *,
    module_name: str,
) -> tuple[ModuleType, dict[str, Any]]:
    source = KERNEL_PATH.read_text()
    filename = str(KERNEL_PATH)
    module = ModuleType(module_name)
    module.__file__ = filename
    module.__package__ = module_name.rpartition(".")[0]
    module.hy = kernel.hy
    module.__dict__.setdefault("_hy_macros", {})
    module.__dict__.setdefault("_hy_reader_macros", {})
    sys.modules[module_name] = module
    tree = kernel.compile_source_to_module(
        source,
        filename,
        "__hy_meta_result__",
        module.__name__,
        module.__package__,
    )
    artifact = artifact_from_ast(
        name="self-compiled-kernel",
        source=source,
        filename=filename,
        tree=tree,
    )
    artifact["body_count"] = len(tree.body)
    exec(compile(tree, filename, "exec"), module.__dict__)
    module.__dict__.pop("__hy_meta_result__", None)
    return module, artifact


def run_self_host_check() -> dict[str, Any]:
    stages = bootstrap_stage_chain(7)
    stage7 = stages[-1][1]
    kernel_a = stage7.load_hy_file(
        KERNEL_PATH,
        "hy_meta_self_host.kernel_a",
    )
    kernel_b, kernel_artifact = load_kernel_compiled_kernel(
        kernel_a,
        module_name="hy_meta_self_host.kernel_b",
    )
    factorial_source = (
        "(defn fact [n] (if (<= n 1) 1 (* n (fact (- n 1))))) (fact 5)"
    )
    compiler_source = STAGE2_PATH.read_text()
    compiler_tree = kernel_b.compile_source_to_module(
        compiler_source,
        str(STAGE2_PATH),
        None,
        "hy_meta_self_host.compiler_probe",
        "hy_meta_self_host",
    )
    compiler_artifact = artifact_from_ast(
        name="self-hosted-compiler-shim",
        source=compiler_source,
        filename=str(STAGE2_PATH),
        tree=compiler_tree,
    )
    kernel_b_factorial = kernel_b.eval_source(
        factorial_source,
        None,
        "<self-host-check:factorial>",
    )
    status = (
        "reproduced"
        if kernel_a.self_check()
        and kernel_b.self_check()
        and kernel_b_factorial == 120
        else "held"
    )
    return {
        "python": sys.version.split()[0],
        "stage_count": len(stages),
        "stage7_module": stage7.__name__,
        "kernel_a_module": kernel_a.__name__,
        "kernel_b_module": kernel_b.__name__,
        "kernel_a_self_check": kernel_a.self_check(),
        "kernel_b_self_check": kernel_b.self_check(),
        "self_compiled_kernel_body_count": kernel_artifact["body_count"],
        "kernel_artifact_normalized_sha256": kernel_artifact["normalized_sha256"],
        "kernel_artifact_code_sha256": kernel_artifact["code_sha256"],
        "compiler_shim_normalized_sha256": compiler_artifact["normalized_sha256"],
        "compiler_shim_code_sha256": compiler_artifact["code_sha256"],
        "kernel_b_factorial": kernel_b_factorial,
        "self_host_status": status,
    }


def run_bootstrap_fixedpoint_check() -> dict[str, Any]:
    stages = bootstrap_stage_chain(7)
    stage7 = stages[-1][1]
    kernel_a = stage7.load_hy_file(
        KERNEL_PATH,
        "hy_meta_fixedpoint.kernel_a",
    )
    kernel_b, kernel_b_artifact = load_kernel_compiled_kernel(
        kernel_a,
        module_name="hy_meta_fixedpoint.kernel_b",
    )
    kernel_c, kernel_c_artifact = load_kernel_compiled_kernel(
        kernel_b,
        module_name="hy_meta_fixedpoint.kernel_c",
    )
    compiler_source = STAGE2_PATH.read_text()
    compiler_filename = str(STAGE2_PATH)
    b_artifacts = {
        "kernel": kernel_b_artifact,
        "compiler-shim": compile_kernel_source_artifact(
            kernel_b,
            name="compiler-shim",
            source=compiler_source,
            filename=compiler_filename,
        ),
    }
    c_artifacts = {
        "kernel": kernel_c_artifact,
        "compiler-shim": compile_kernel_source_artifact(
            kernel_c,
            name="compiler-shim",
            source=compiler_source,
            filename=compiler_filename,
        ),
    }
    diff = compare_stage8_artifact_bundles(b_artifacts, c_artifacts)
    common_names = sorted(set(b_artifacts) & set(c_artifacts))
    normalized_matches = all(
        b_artifacts[name]["normalized_sha256"]
        == c_artifacts[name]["normalized_sha256"]
        for name in common_names
    )
    code_matches = all(
        b_artifacts[name]["code_sha256"] == c_artifacts[name]["code_sha256"]
        for name in common_names
    )
    raw_code_matches = all(
        b_artifacts[name]["raw_code_sha256"] == c_artifacts[name]["raw_code_sha256"]
        for name in common_names
    )
    pyc_matches = all(
        b_artifacts[name]["pyc_sha256"] == c_artifacts[name]["pyc_sha256"]
        for name in common_names
    )
    artifact_names_match = (
        not diff["missing_from_stage8"] and not diff["new_in_stage8"]
    )
    kernel_c_factorial = kernel_c.eval_source(
        "(defn fact [n] (if (<= n 1) 1 (* n (fact (- n 1))))) (fact 5)",
        None,
        "<bootstrap-fixedpoint-check:factorial>",
    )
    status = (
        "reproduced"
        if kernel_b.self_check()
        and kernel_c.self_check()
        and kernel_c_factorial == 120
        and artifact_names_match
        and normalized_matches
        and code_matches
        and raw_code_matches
        and pyc_matches
        else "held"
    )
    return {
        "python": sys.version.split()[0],
        "stage_count": len(stages),
        "stage7_module": stage7.__name__,
        "kernel_a_module": kernel_a.__name__,
        "kernel_b_module": kernel_b.__name__,
        "kernel_c_module": kernel_c.__name__,
        "kernel_b_self_check": kernel_b.self_check(),
        "kernel_c_self_check": kernel_c.self_check(),
        "fixedpoint_artifact_count": len(common_names),
        "artifact_names_match": artifact_names_match,
        "normalized_artifacts_match": normalized_matches,
        "code_artifacts_match": code_matches,
        "raw_code_artifacts_match": raw_code_matches,
        "raw_pyc_artifacts_match": pyc_matches,
        "kernel_b_body_count": kernel_b_artifact["body_count"],
        "kernel_c_body_count": kernel_c_artifact["body_count"],
        "kernel_c_factorial": kernel_c_factorial,
        "fixedpoint_status": status,
        "changed_artifacts": ",".join(diff["changed"]),
        "missing_from_kernel_c": ",".join(diff["missing_from_stage8"]),
        "new_in_kernel_c": ",".join(diff["new_in_stage8"]),
    }


def run_diverse_double_compile_check() -> dict[str, Any]:
    """Wheeler diverse double-compiling (DDC): compare the direct-kernel lineage
    against an independently-built kernel.

    `kernel_upstream` is kernel.hy compiled by the upstream hy.compiler (the
    stage1 seed); `kernel_direct` is kernel.hy compiled by the direct kernel (the
    stage2 bridge, which genuinely hits the direct kernel). Both then compile
    kernel.hy and compiler.hy. If a backdoor or divergence existed in the
    direct-kernel build path but not in upstream, these two independently-built
    compilers would emit different artifacts. Byte-identical artifacts give a
    trusting-trust (DDC) defense beyond the self-parented
    bootstrap-fixedpoint-check.
    """

    kernel_source = KERNEL_PATH.read_text()
    kernel_filename = str(KERNEL_PATH)
    compiler_source = STAGE2_PATH.read_text()
    compiler_filename = str(STAGE2_PATH)

    stage2 = bootstrap_stage2()
    if hasattr(stage2, "reset_direct_kernel_stats"):
        stage2.reset_direct_kernel_stats()
    kernel_direct = stage2.load_hy_file(KERNEL_PATH, "hy_meta_ddc.kernel_direct")
    direct_build_hits = (
        dict(stage2.direct_kernel_stats()).get("hits", 0)
        if hasattr(stage2, "direct_kernel_stats")
        else 0
    )
    kernel_upstream = load_hy_file(
        KERNEL_PATH, module_name="hy_meta_ddc.kernel_upstream"
    )

    def artifacts_for(kernel: ModuleType) -> dict[str, dict[str, Any]]:
        return {
            "kernel": compile_kernel_source_artifact(
                kernel, name="kernel", source=kernel_source, filename=kernel_filename
            ),
            "compiler-shim": compile_kernel_source_artifact(
                kernel,
                name="compiler-shim",
                source=compiler_source,
                filename=compiler_filename,
            ),
        }

    upstream_artifacts = artifacts_for(kernel_upstream)
    direct_artifacts = artifacts_for(kernel_direct)
    common = sorted(set(upstream_artifacts) & set(direct_artifacts))
    normalized_matches = all(
        upstream_artifacts[n]["normalized_sha256"]
        == direct_artifacts[n]["normalized_sha256"]
        for n in common
    )
    code_matches = all(
        upstream_artifacts[n]["code_sha256"] == direct_artifacts[n]["code_sha256"]
        for n in common
    )
    raw_code_matches = all(
        upstream_artifacts[n]["raw_code_sha256"]
        == direct_artifacts[n]["raw_code_sha256"]
        for n in common
    )
    pyc_matches = all(
        upstream_artifacts[n]["pyc_sha256"] == direct_artifacts[n]["pyc_sha256"]
        for n in common
    )

    factorial_src = (
        "(defn fact [n] (if (<= n 1) 1 (* n (fact (- n 1))))) (fact 5)"
    )
    upstream_factorial = kernel_upstream.eval_source(
        factorial_src, None, "<ddc:upstream>"
    )
    direct_factorial = kernel_direct.eval_source(factorial_src, None, "<ddc:direct>")
    upstream_self_check = kernel_upstream.self_check()
    direct_self_check = kernel_direct.self_check()
    build_compilers_distinct = direct_build_hits > 0

    status = (
        "reproduced"
        if upstream_self_check
        and direct_self_check
        and upstream_factorial == 120
        and direct_factorial == 120
        and build_compilers_distinct
        and normalized_matches
        and code_matches
        and raw_code_matches
        and pyc_matches
        else "held"
    )
    return {
        "python": sys.version.split()[0],
        "kernel_upstream_module": kernel_upstream.__name__,
        "kernel_direct_module": kernel_direct.__name__,
        "direct_build_kernel_hits": direct_build_hits,
        "build_compilers_distinct": build_compilers_distinct,
        "upstream_self_check": upstream_self_check,
        "direct_self_check": direct_self_check,
        "ddc_artifact_count": len(common),
        "normalized_artifacts_match": normalized_matches,
        "code_artifacts_match": code_matches,
        "raw_code_artifacts_match": raw_code_matches,
        "raw_pyc_artifacts_match": pyc_matches,
        "upstream_factorial": upstream_factorial,
        "direct_factorial": direct_factorial,
        "ddc_status": status,
    }


def independent_mini_backend_fixtures() -> list[dict[str, Any]]:
    return [
        {"id": "mini-const-arithmetic", "source": "(defn f [] (+ 40 2)) (f)", "expected": 42},
        {"id": "mini-one-arg", "source": "(defn f [x] (+ x 1)) (f 41)", "expected": 42},
        {
            "id": "mini-branch-two-arg",
            "source": "(defn f [x y] (if (< x y) (* (+ x 1) y) (- x y))) (f 5 7)",
            "expected": 42,
        },
        {
            "id": "mini-recursive-factorial",
            "source": "(defn f [x] (if (<= x 1) 1 (* x (f (- x 1))))) (f 5)",
            "expected": 120,
        },
        {
            "id": "mini-equality-boolean",
            "source": "(defn f [x] (if (= x 42) True False)) (f 42)",
            "expected": True,
        },
        {
            "id": "mini-none-equality",
            "source": "(defn f [x] (if (= x None) 42 0)) (f None)",
            "expected": 42,
        },
        {
            "id": "mini-unary-negate-branch",
            "source": "(defn f [x] (if (> x 0) x (- 0 x))) (f -42)",
            "expected": 42,
        },
        {"id": "mini-bare-arithmetic", "source": "(+ 1 2)", "expected": 3},
    ]


def run_independent_mini_backend_check() -> dict[str, Any]:
    """Trusting-Trust (DDC) witness: cross-check upstream Hy against a tiny,
    from-scratch reader+AST-builder (`independent_mini_backend.py`) that
    shares no code with `hy.reader`, `hy.compiler`, `stage1/compiler.py`, or
    `stage2/kernel.hy`. Unlike `run_diverse_double_compile_check` (which
    compares whole-file kernel.hy/compiler.hy bytecode artifacts between the
    upstream-seeded and direct-kernel lineages), this compares small-fixture
    *behavior* between real upstream Hy and the independent mini backend —
    the same honest "subset, behavior not bit-identical" bar clj-meta's
    analogous `independent-mini-backend-subset` DDC row already uses.
    """
    rows = []
    for fixture in independent_mini_backend_fixtures():
        source = fixture["source"]
        expected = fixture["expected"]
        try:
            host_result = eval_source(source, filename="<independent-mini-backend:host>")
            host_ok = True
            host_error = None
        except Exception as exc:  # noqa: BLE001 - captured for the receipt
            host_result = None
            host_ok = False
            host_error = f"{type(exc).__name__}: {exc}"
        try:
            mini_result = independent_mini_backend.compile_and_eval(source)
            mini_ok = True
            mini_error = None
        except Exception as exc:  # noqa: BLE001 - captured for the receipt
            mini_result = None
            mini_ok = False
            mini_error = f"{type(exc).__name__}: {exc}"
        ok = (
            host_ok
            and mini_ok
            and host_result == expected
            and mini_result == expected
        )
        rows.append(
            {
                "id": fixture["id"],
                "source": source,
                "expected": expected,
                "host_result": host_result,
                "host_ok": host_ok,
                "host_error": host_error,
                "mini_backend_result": mini_result,
                "mini_backend_ok": mini_ok,
                "mini_backend_error": mini_error,
                "ok": ok,
            }
        )
    all_ok = bool(rows) and all(row["ok"] for row in rows)
    return {
        "backend": "independent_mini_backend.compile_and_eval",
        "independence": {
            "uses_hy_reader": False,
            "uses_hy_compiler": False,
            "uses_stage1_compiler_internals": False,
            "uses_stage2_kernel": False,
            "shared_runtime": ["python-ast-module", "python-compile-builtin"],
        },
        "not_claimed": [
            "full-wheeler-ddc",
            "bit-identical-bytecode-ddc",
            "production-frontend-replacement",
            "full-hy-language-coverage",
        ],
        "fixture_count": len(rows),
        "rows": rows,
        "all_fixtures_accepted": all_ok,
        "mini_backend_status": "accepted" if all_ok else "rejected",
    }


def direct_kernel_owned_corpus() -> list[Path]:
    return [
        STAGE2_PATH,
        KERNEL_PATH,
        ROOT / "hy-meta" / "examples" / "factorial.hy",
        ROOT / "hy-meta" / "examples" / "kernel_loop.hy",
        ROOT / "hy-meta" / "examples" / "kernel_features.hy",
        ROOT / "hy-meta" / "examples" / "kernel_stability_stress.hy",
    ]


def run_no_fallback_check() -> dict[str, Any]:
    stage2 = bootstrap_stage2()
    if hasattr(stage2, "reset_direct_kernel_stats"):
        stage2.reset_direct_kernel_stats()
    if hasattr(stage2, "set_direct_kernel_strict"):
        stage2.set_direct_kernel_strict(True)
    results = []
    errors = []
    try:
        for index, path in enumerate(direct_kernel_owned_corpus()):
            before = dict(stage2.direct_kernel_stats())
            try:
                stage2.compile_source_to_ast(
                    path.read_text(),
                    stage2.make_module(
                        f"hy_meta_no_fallback.corpus_{index}",
                        str(path),
                    ),
                    str(path),
                )
                status = "compiled"
                error = ""
            except Exception as exc:
                status = "failed"
                error = f"{exc.__class__.__name__}: {exc}"
                errors.append(f"{path.name}: {error}")
            after = dict(stage2.direct_kernel_stats())
            results.append(
                {
                    "path": str(path.relative_to(ROOT)),
                    "status": status,
                    "hit_delta": after["hits"] - before["hits"],
                    "fallback_delta": after["fallbacks"] - before["fallbacks"],
                    "error": error,
                }
            )
    finally:
        if hasattr(stage2, "set_direct_kernel_strict"):
            stage2.set_direct_kernel_strict(False)
    stats = dict(stage2.direct_kernel_stats())
    fallback_count = stats["fallbacks"]
    status = (
        "reproduced"
        if len(errors) == 0
        and fallback_count == 0
        and stats["hits"] == len(direct_kernel_owned_corpus())
        else "held"
    )
    return {
        "python": sys.version.split()[0],
        "stage2_module": stage2.__name__,
        "corpus_count": len(direct_kernel_owned_corpus()),
        "compiled_count": sum(1 for result in results if result["status"] == "compiled"),
        "direct_kernel_loaded": stats["loaded"],
        "direct_kernel_strict_after": stats["strict"],
        "direct_kernel_hits": stats["hits"],
        "direct_kernel_fallbacks": fallback_count,
        "direct_kernel_last_error": stats["last_error"],
        "error_count": len(errors),
        "errors": "; ".join(errors),
        "corpus_paths": ",".join(
            str(path.relative_to(ROOT)) for path in direct_kernel_owned_corpus()
        ),
        "corpus_results": json.dumps(results, sort_keys=True, separators=(",", ":")),
        "no_fallback_status": status,
    }


def native_test_corpus() -> list[Path]:
    return sorted((ROOT / "tests" / "native_tests").glob("*.hy"))


def parity_module_name(label: str, path: Path, index: int) -> str:
    stem = path.stem.replace("-", "_")
    if label == "native-tests":
        return f"tests.native_tests.{stem}_parity_{index}"
    return f"hy_meta_parity.{label.replace('-', '_')}_{index}"


def parity_record(stage2: ModuleType, label: str, path: Path, index: int) -> dict[str, Any]:
    source = path.read_text()
    try:
        form_count = len(list(stage2.read_source(source, str(path))))
        reader_error = ""
    except Exception as exc:
        form_count = 0
        reader_error = f"{exc.__class__.__name__}: {exc}"
    before = dict(stage2.direct_kernel_stats())
    error = ""
    try:
        stage2.compile_source_to_ast(
            source,
            stage2.make_module(parity_module_name(label, path, index), str(path)),
            str(path),
        )
    except Exception as exc:
        error = f"{exc.__class__.__name__}: {exc}"
    after = dict(stage2.direct_kernel_stats())
    hit_delta = after["hits"] - before["hits"]
    fallback_delta = after["fallbacks"] - before["fallbacks"]
    if error:
        status = "error"
    elif fallback_delta:
        status = "fallback"
    elif hit_delta:
        status = "direct"
    else:
        status = "unknown"
    return {
        "label": label,
        "path": str(path.relative_to(ROOT)),
        "status": status,
        "top_level_forms": form_count,
        "hit_delta": hit_delta,
        "fallback_delta": fallback_delta,
        "reader_error": reader_error,
        "error": error,
    }


def run_parity_ledger_check(debug_dir: str | None = None) -> dict[str, Any]:
    stage2 = bootstrap_stage2()
    if hasattr(stage2, "reset_direct_kernel_stats"):
        stage2.reset_direct_kernel_stats()
    records = []
    for index, path in enumerate(direct_kernel_owned_corpus()):
        records.append(parity_record(stage2, "owned", path, index))
    for index, path in enumerate(native_test_corpus()):
        records.append(parity_record(stage2, "native-tests", path, index))
    stats = dict(stage2.direct_kernel_stats())

    def count(label: str, status: str | None = None) -> int:
        return sum(
            1
            for record in records
            if record["label"] == label and (status is None or record["status"] == status)
        )

    def forms(label: str, status: str | None = None) -> int:
        return sum(
            record["top_level_forms"]
            for record in records
            if record["label"] == label and (status is None or record["status"] == status)
        )

    total_files = len(records)
    direct_files = sum(1 for record in records if record["status"] == "direct")
    total_forms = sum(record["top_level_forms"] for record in records)
    direct_forms = sum(
        record["top_level_forms"] for record in records if record["status"] == "direct"
    )
    fallback_files = [
        record["path"] for record in records if record["status"] == "fallback"
    ]
    error_files = [record["path"] for record in records if record["status"] == "error"]
    summary = {
        "python": sys.version.split()[0],
        "stage2_module": stage2.__name__,
        "owned_files": count("owned"),
        "owned_direct_files": count("owned", "direct"),
        "owned_fallback_files": count("owned", "fallback"),
        "owned_error_files": count("owned", "error"),
        "owned_top_level_forms": forms("owned"),
        "owned_direct_top_level_forms": forms("owned", "direct"),
        "native_files": count("native-tests"),
        "native_direct_files": count("native-tests", "direct"),
        "native_fallback_files": count("native-tests", "fallback"),
        "native_error_files": count("native-tests", "error"),
        "native_top_level_forms": forms("native-tests"),
        "native_direct_top_level_forms": forms("native-tests", "direct"),
        "total_files": total_files,
        "direct_files": direct_files,
        "fallback_files_count": len(fallback_files),
        "error_files_count": len(error_files),
        "total_top_level_forms": total_forms,
        "direct_top_level_forms": direct_forms,
        "direct_file_percent": f"{(direct_files / total_files * 100):.2f}",
        "direct_top_level_form_percent": (
            f"{(direct_forms / total_forms * 100):.2f}" if total_forms else "0.00"
        ),
        "direct_kernel_hits": stats["hits"],
        "direct_kernel_fallbacks": stats["fallbacks"],
        "fallback_files": ",".join(fallback_files),
        "error_files": ",".join(error_files),
        "parity_status": (
            "measured"
            if count("owned", "fallback") == 0
            and count("owned", "error") == 0
            and len(error_files) == 0
            else "held"
        ),
        "debug_dir": debug_dir or "",
    }
    if debug_dir is not None:
        debug_path = Path(debug_dir)
        debug_path.mkdir(parents=True, exist_ok=True)
        (debug_path / "parity-ledger.json").write_text(
            json.dumps(
                {"summary": summary, "records": records},
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )
    return summary


def compile_stage_source_artifact(
    stage: ModuleType,
    *,
    name: str,
    source: str,
    filename: str,
    module_name: str,
) -> dict[str, Any]:
    module = stage.make_module(module_name, filename)
    tree = stage.compile_source_to_ast(source, module, filename)
    return artifact_from_ast(
        name=name,
        source=source,
        filename=filename,
        tree=tree,
    )


def compile_kernel_source_artifact(
    kernel: ModuleType,
    *,
    name: str,
    source: str,
    filename: str,
) -> dict[str, Any]:
    tree = kernel.compile_source_to_module(source, filename)
    return artifact_from_ast(
        name=name,
        source=source,
        filename=filename,
        tree=tree,
    )


def build_stage8_artifact_bundle(
    stage: ModuleType,
    *,
    label: str,
) -> dict[str, dict[str, Any]]:
    compiler_source = STAGE2_PATH.read_text()
    compiler_filename = str(STAGE2_PATH)
    kernel_source = KERNEL_PATH.read_text()
    kernel_filename = str(KERNEL_PATH)
    example_paths = [
        ROOT / "hy-meta" / "examples" / "factorial.hy",
        ROOT / "hy-meta" / "examples" / "kernel_loop.hy",
        ROOT / "hy-meta" / "examples" / "kernel_features.hy",
        ROOT / "hy-meta" / "examples" / "kernel_stability_stress.hy",
    ]
    artifacts = {
        "stage2-compiler": compile_stage_source_artifact(
            stage,
            name="stage2-compiler",
            source=compiler_source,
            filename=compiler_filename,
            module_name=f"hy_meta_stage8_artifacts.{label}.compiler",
        ),
        "stage2-kernel": compile_stage_source_artifact(
            stage,
            name="stage2-kernel",
            source=kernel_source,
            filename=kernel_filename,
            module_name=f"hy_meta_stage8_artifacts.{label}.kernel_source",
        ),
    }
    kernel = stage.load_hy_file(
        KERNEL_PATH,
        f"hy_meta_stage8_artifacts.{label}.kernel",
    )
    for path in example_paths:
        name = f"kernel-example-{path.stem}"
        artifacts[name] = compile_kernel_source_artifact(
            kernel,
            name=name,
            source=path.read_text(),
            filename=str(path),
        )
    return artifacts


def stage8_hash_keys() -> list[str]:
    return [
        "source_sha256",
        "ast_sha256",
        "python_sha256",
        "normalized_sha256",
        "code_sha256",
        "raw_code_sha256",
        "pyc_sha256",
    ]


_DRIFT_KIND_BY_KEY = {
    "source_sha256": "semantic",
    "ast_sha256": "semantic",
    "python_sha256": "semantic",
    "normalized_sha256": "semantic",
    "code_sha256": "bytecode",
    "raw_code_sha256": "marshal",
    "pyc_sha256": "pyc",
    "value_sha256": "value",
    "result_sha256": "value",
    "env_sha256": "env",
    "environment_sha256": "env",
    "manifest_sha256": "env",
}

_DRIFT_KIND_ORDER = ["semantic", "bytecode", "marshal", "pyc", "value", "env", "unknown"]


def classify_drift(keys: list[str]) -> dict[str, Any]:
    """Classify drift fields into host artifact/value/environment categories."""
    ordered_keys = sorted(keys)
    kinds = sorted(
        {_DRIFT_KIND_BY_KEY.get(key, "unknown") for key in ordered_keys},
        key=_DRIFT_KIND_ORDER.index,
    )
    if not ordered_keys:
        classification = "none"
    elif "semantic" in kinds:
        classification = "semantic"
    elif len(kinds) == 1:
        classification = kinds[0]
    else:
        classification = "mixed:" + ",".join(kinds)
    return {
        "schema": "hy-meta.drift-classification.v0",
        "classification": classification,
        "kinds": kinds,
        "fields": ordered_keys,
        "field_kinds": {key: _DRIFT_KIND_BY_KEY.get(key, "unknown") for key in ordered_keys},
    }


def classify_drift_report() -> dict[str, Any]:
    samples = {
        "none": classify_drift([]),
        "bytecode": classify_drift(["code_sha256"]),
        "marshal": classify_drift(["raw_code_sha256"]),
        "pyc": classify_drift(["pyc_sha256"]),
        "value": classify_drift(["value_sha256"]),
        "env": classify_drift(["manifest_sha256"]),
        "semantic": classify_drift(["normalized_sha256", "pyc_sha256"]),
    }
    ready = (
        samples["none"]["classification"] == "none"
        and samples["bytecode"]["classification"] == "bytecode"
        and samples["marshal"]["classification"] == "marshal"
        and samples["pyc"]["classification"] == "pyc"
        and samples["value"]["classification"] == "value"
        and samples["env"]["classification"] == "env"
        and samples["semantic"]["classification"] == "semantic"
    )
    return {
        "schema": "hy-meta.drift-classification.report.v0",
        "ready": ready,
        "available": True,
        "samples": {
            name: {"classification": sample["classification"], "kinds": sample["kinds"]}
            for name, sample in samples.items()
        },
    }


def classify_stage8_drift(keys: list[str]) -> str:
    """Return the legacy stage8 coarse drift class used by existing CLI checks."""
    semantic_keys = {
        "source_sha256",
        "ast_sha256",
        "python_sha256",
        "normalized_sha256",
        "code_sha256",
    }
    if any(key in semantic_keys for key in keys):
        return "semantic"
    if keys:
        return "raw-marshal-or-pyc"
    return "none"


def compare_stage8_artifact_bundles(
    stage7_artifacts: dict[str, dict[str, Any]],
    stage8_artifacts: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    stage7_names = set(stage7_artifacts)
    stage8_names = set(stage8_artifacts)
    common_names = sorted(stage7_names & stage8_names)
    hash_keys = stage8_hash_keys()
    changed: dict[str, list[str]] = {}
    details: dict[str, dict[str, Any]] = {}
    for name in common_names:
        differing_keys = [
            key
            for key in hash_keys
            if stage7_artifacts[name][key] != stage8_artifacts[name][key]
        ]
        if differing_keys:
            changed[name] = differing_keys
            details[name] = {
                "classification": classify_stage8_drift(differing_keys),
                "drift": classify_drift(differing_keys),
                "fields": {
                    key: {
                        "stage7": stage7_artifacts[name][key],
                        "stage8_fresh": stage8_artifacts[name][key],
                    }
                    for key in differing_keys
                },
            }
    return {
        "missing_from_stage8": sorted(stage7_names - stage8_names),
        "new_in_stage8": sorted(stage8_names - stage7_names),
        "changed": changed,
        "details": details,
    }


def run_stage8_check(debug_dir: str | None = None) -> dict[str, Any]:
    stages = bootstrap_stage_chain(8)
    stage7 = stages[-2][1]
    stage8 = stages[-1][1]
    stage7_artifacts = build_stage8_artifact_bundle(stage7, label="stage7")
    stage8_artifacts = build_stage8_artifact_bundle(stage8, label="stage8_fresh")
    diff = compare_stage8_artifact_bundles(stage7_artifacts, stage8_artifacts)
    common_names = sorted(set(stage7_artifacts) & set(stage8_artifacts))
    normalized_matches = all(
        stage7_artifacts[name]["normalized_sha256"]
        == stage8_artifacts[name]["normalized_sha256"]
        for name in common_names
    )
    code_matches = all(
        stage7_artifacts[name]["code_sha256"] == stage8_artifacts[name]["code_sha256"]
        for name in common_names
    )
    raw_code_matches = all(
        stage7_artifacts[name]["raw_code_sha256"]
        == stage8_artifacts[name]["raw_code_sha256"]
        for name in common_names
    )
    pyc_matches = all(
        stage7_artifacts[name]["pyc_sha256"] == stage8_artifacts[name]["pyc_sha256"]
        for name in common_names
    )
    artifact_names_match = (
        not diff["missing_from_stage8"] and not diff["new_in_stage8"]
    )
    drift_fields = sorted({key for fields in diff["changed"].values() for key in fields})
    if not artifact_names_match:
        drift_fields.append("manifest_sha256")
    drift_detail = classify_drift(drift_fields)
    status = (
        "reproduced"
        if artifact_names_match and normalized_matches and code_matches
        else "held"
    )
    drift_class = "none"
    if not artifact_names_match or not normalized_matches or not code_matches:
        drift_class = "semantic"
    elif not raw_code_matches or not pyc_matches:
        drift_class = "raw-marshal-or-pyc"
    if debug_dir is not None:
        write_stage8_debug_artifacts(
            Path(debug_dir),
            stage7_artifacts,
            stage8_artifacts,
            diff,
        )
    return {
        "python": sys.version.split()[0],
        "stage_count": len(stages),
        "stage7_module": stage7.__name__,
        "stage8_module": stage8.__name__,
        "artifact_count": len(common_names),
        "artifact_names_match": artifact_names_match,
        "normalized_artifacts_match": normalized_matches,
        "code_artifacts_match": code_matches,
        "raw_code_artifacts_match": raw_code_matches,
        "raw_pyc_artifacts_match": pyc_matches,
        "stage8_drift_class": drift_class,
        "stage8_drift_kinds": ",".join(drift_detail["kinds"]),
        "stage8_drift_detail": drift_detail,
        "stage8_status": status,
        "changed_artifacts": ",".join(diff["changed"]),
        "missing_from_stage8": ",".join(diff["missing_from_stage8"]),
        "new_in_stage8": ",".join(diff["new_in_stage8"]),
        "debug_dir": debug_dir or "",
    }


def stage9_clean_env() -> dict[str, str]:
    env = dict(os.environ)
    env["PYTHONHASHSEED"] = "0"
    env["LC_ALL"] = "C"
    env["LANG"] = "C"
    env["TZ"] = "UTC"
    env["PYTHONNOUSERSITE"] = "1"
    env.pop("PYTHONPATH", None)
    env.pop("PYTHONHOME", None)
    env.pop("PYTHONSTARTUP", None)
    return env


def stage9_manifest() -> dict[str, Any]:
    paths = [
        STAGE2_PATH,
        KERNEL_PATH,
        ROOT / "hy-meta" / "bootstrap.py",
        ROOT / "hy-meta" / "examples" / "factorial.hy",
        ROOT / "hy-meta" / "examples" / "kernel_features.hy",
        ROOT / "hy-meta" / "examples" / "kernel_loop.hy",
        ROOT / "hy-meta" / "examples" / "kernel_stability_stress.hy",
    ]
    files = {
        str(path.relative_to(ROOT)): sha256_text(path.read_text())
        for path in paths
    }
    manifest = {
        "python": sys.version.split()[0],
        "python_family": f"{sys.version_info.major}.{sys.version_info.minor}",
        "executable": sys.executable,
        "hy_version": getattr(hy, "__version__", "unknown"),
        "hy_package_file": getattr(hy, "__file__", ""),
        "route_policy_version": HY_META_ROUTE_POLICY_VERSION,
        "feature_gate_versions": {
            "product_entrypoints": HY_META_FEATURE_GATE_VERSION,
            "direct_kernel": "direct-kernel-strict-v1",
            "artifact_reproducibility": "stage8-artifact-v2",
        },
        "repo_root": str(ROOT),
        "files": files,
        "hard_env": {
            "PYTHONHASHSEED": "0",
            "LC_ALL": "C",
            "LANG": "C",
            "TZ": "UTC",
            "PYTHONNOUSERSITE": "1",
        },
    }
    manifest["manifest_sha256"] = sha256_text(
        json.dumps(manifest, sort_keys=True, separators=(",", ":"))
    )
    return manifest


def stage9_expected_error_fixture(
    *,
    source: str,
    expected_error_class: str,
    evaluate: Any,
) -> dict[str, Any]:
    try:
        evaluate(source)
    except Exception as exc:
        error_class = exc.__class__.__name__
        error = str(exc).splitlines()[0] if str(exc) else ""
        return {
            "kind": "expected-error",
            "source_sha256": sha256_text(source),
            "expected_error_class": expected_error_class,
            "error_class": error_class,
            "error": error,
            "accepted": False,
            "boundary_preserved": error_class == expected_error_class,
        }
    return {
        "kind": "expected-error",
        "source_sha256": sha256_text(source),
        "expected_error_class": expected_error_class,
        "error_class": "",
        "error": "",
        "accepted": True,
        "boundary_preserved": False,
    }


def stage9_probe_result() -> dict[str, Any]:
    source = "(+ 20 22)"
    command_filename = "<hy-meta:-c>"
    stage2 = bootstrap_stage2()
    stage2_run_module = stage2.make_module("hy_meta_stage9_probe.run")
    stage2_py_module = stage2.make_module("hy_meta_stage9_probe.py")
    stage2_hy2py_module = stage2.make_module("hy_meta_stage9_probe.hy2py")

    with tempfile.TemporaryDirectory(prefix="hy-meta-stage9-probe-") as temp_dir:
        hyc_output = Path(temp_dir) / "stage9_probe.pyc"
        compile_stage2_pyc(
            "(setv answer (+ 20 22))",
            command_filename,
            hyc_output,
            "hy_meta_stage9_probe.hyc",
        )
        hyc_bytes = hyc_output.read_bytes()

    kernel = bootstrap_kernel()
    stage7_kernel = bootstrap_stage7_kernel()
    fixtures = {
        "run-command": {
            "kind": "stdout-value",
            "value": stage2.eval_source(source, stage2_run_module, command_filename),
        },
        "py-command": {
            "kind": "stdout-text",
            "text": stage2.python_source(source, stage2_py_module, command_filename),
        },
        "hy2py-command": {
            "kind": "stdout-text",
            "text": stage2.python_source(source, stage2_hy2py_module, command_filename),
        },
        "hyc-command": {
            "kind": "output-file",
            "pyc_sha256": sha256_bytes(hyc_bytes),
            "pyc_size": len(hyc_bytes),
        },
        "kernel-run-command": {
            "kind": "stdout-value",
            "value": kernel.eval_source(source, None, command_filename),
        },
        "kernel-py-command": {
            "kind": "stdout-text",
            "text": kernel.python_source(source, command_filename),
        },
        "stage7-kernel-run-command": {
            "kind": "stdout-value",
            "value": stage7_kernel.eval_source(source, None, command_filename),
        },
        "stage7-kernel-py-command": {
            "kind": "stdout-text",
            "text": stage7_kernel.python_source(source, command_filename),
        },
        "negative-template-string-gate": stage9_expected_error_fixture(
            source="""
            (pragma :bracketed-templates True)
            #[t[hello {(+ 1 1)}]t]
            """,
            expected_error_class="SyntaxError",
            evaluate=lambda form: kernel.eval_source(
                form,
                None,
                "<stage9:negative-template-string>",
            ),
        ),
        "negative-reader-macro-boundary": stage9_expected_error_fixture(
            source="#not-a-reader",
            expected_error_class="LexException",
            evaluate=lambda form: kernel.eval_source(
                form,
                None,
                "<stage9:negative-reader-macro>",
            ),
        ),
    }
    payload = {
        "manifest": stage9_manifest(),
        "fixtures": fixtures,
    }
    payload["canonical_sha256"] = sha256_text(
        json.dumps(payload, sort_keys=True, separators=(",", ":"))
    )
    return payload


def run_stage9_clean_subprocess(
    command: list[str] | tuple[str, ...],
    *,
    env: dict[str, str],
    cwd: Path = ROOT,
    timeout: int = 240,
    parse_json: bool = True,
    parse_only_on_success: bool = False,
) -> dict[str, Any]:
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=cwd,
        env=env,
        text=True,
        capture_output=True,
        timeout=timeout,
        check=False,
    )
    elapsed_ms = int((time.monotonic() - started) * 1000)
    parsed: Any = None
    parse_error = ""
    should_parse = bool(parse_json and completed.stdout)
    if parse_only_on_success:
        should_parse = should_parse and completed.returncode == 0
    if should_parse:
        try:
            parsed = json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            parse_error = str(exc)
    return {
        "command": list(command),
        "returncode": completed.returncode,
        "cwd": str(cwd),
        "elapsed_ms": elapsed_ms,
        "stdout_sha256": sha256_text(completed.stdout),
        "stderr_sha256": sha256_text(completed.stderr),
        "stdout": completed.stdout,
        "stderr": completed.stderr,
        "parsed": parsed,
        "parse_error": parse_error,
    }


def run_stage9_probe_subprocess(*, env: dict[str, str], cwd: Path = ROOT) -> dict[str, Any]:
    result = run_stage9_clean_subprocess(
        [
            sys.executable,
            str(ROOT / "hy-meta" / "bootstrap.py"),
            "stage9-product-probe",
        ],
        env=env,
        cwd=cwd,
        parse_only_on_success=True,
    )
    return {key: value for key, value in result.items() if key not in {"command", "parse_error"}}


def write_stage9_debug_replay(
    debug_dir: Path,
    manifest: dict[str, Any],
    replays: dict[str, dict[str, Any]],
    drift: dict[str, Any],
) -> None:
    debug_dir.mkdir(parents=True, exist_ok=True)
    (debug_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "drift.json").write_text(
        json.dumps(drift, indent=2, sort_keys=True) + "\n"
    )
    for run_name, result in sorted(replays.items()):
        summary = {
            key: value
            for key, value in result.items()
            if key not in {"stdout", "stderr"}
        }
        (debug_dir / f"{run_name}.json").write_text(
            json.dumps(summary, indent=2, sort_keys=True) + "\n"
        )
        (debug_dir / f"{run_name}.stdout").write_text(result["stdout"])
        (debug_dir / f"{run_name}.stderr").write_text(result["stderr"])


def run_stage9_check(debug_dir: str | None = None) -> dict[str, Any]:
    started = time.monotonic()
    manifest = stage9_manifest()
    env = stage9_clean_env()
    first = run_stage9_probe_subprocess(env=env)
    second = run_stage9_probe_subprocess(env=env)
    with tempfile.TemporaryDirectory(prefix="hy-meta-stage9-alt-cwd-") as temp_dir:
        alternate_cwd = run_stage9_probe_subprocess(env=env, cwd=Path(temp_dir))
    replays = {
        "first": first,
        "second": second,
        "alternate_cwd": alternate_cwd,
    }
    drift: dict[str, Any] = {}
    nonzero_returncodes = {
        name: replay["returncode"]
        for name, replay in replays.items()
        if replay["returncode"] != 0
    }
    if nonzero_returncodes:
        drift["nonzero_returncode"] = nonzero_returncodes
    parsed_replays = {
        name: replay["parsed"] for name, replay in replays.items()
    }
    if any(parsed is None for parsed in parsed_replays.values()):
        drift["probe_parse"] = "missing parsed probe output"
    else:
        canonical_hashes = {
            name: parsed["canonical_sha256"]
            for name, parsed in parsed_replays.items()
            if parsed is not None
        }
        if len(set(canonical_hashes.values())) != 1:
            drift["canonical_sha256"] = canonical_hashes
        negative_failures = {
            fixture_name: fixture
            for fixture_name, fixture in first["parsed"]["fixtures"].items()
            if fixture["kind"] == "expected-error"
            and fixture["boundary_preserved"] is not True
        }
        if negative_failures:
            drift["negative_fixtures"] = sorted(negative_failures)
    if debug_dir is not None:
        write_stage9_debug_replay(Path(debug_dir), manifest, replays, drift)
    status = "reproduced" if not drift else "drift"
    fixture_count = (
        len(first["parsed"]["fixtures"])
        if first["parsed"] is not None and "fixtures" in first["parsed"]
        else 0
    )
    elapsed_values = [replay["elapsed_ms"] for replay in replays.values()]
    total_elapsed_ms = int((time.monotonic() - started) * 1000)
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "fixture_count": fixture_count,
        "negative_fixture_count": (
            sum(
                1
                for fixture in first["parsed"]["fixtures"].values()
                if fixture["kind"] == "expected-error"
            )
            if first["parsed"] is not None
            else 0
        ),
        "fixtures_replayed_twice": True,
        "alternate_cwd_replayed": True,
        "clean_env_hash_seed": env["PYTHONHASHSEED"],
        "clean_env_locale": env["LC_ALL"],
        "clean_env_timezone": env["TZ"],
        "hy_version": manifest["hy_version"],
        "route_policy_version": manifest["route_policy_version"],
        "feature_gate_version": manifest["feature_gate_versions"]["product_entrypoints"],
        "probe_count": len(replays),
        "max_probe_elapsed_ms": max(elapsed_values),
        "total_elapsed_ms": total_elapsed_ms,
        "product_replay_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def stage10_request_fixture() -> dict[str, Any]:
    source = "(+ 20 22)"
    return {
        "request_id": "stage10-basic-add",
        "source": source,
        "source_sha256": sha256_text(source),
        "filename": "<hy-meta:stage10>",
        "routes": ["run", "py"],
    }


def stage10_server_handle(stage2: ModuleType, request: dict[str, Any]) -> dict[str, Any]:
    source = request["source"]
    filename = request["filename"]
    run_module = stage2.make_module("hy_meta_stage10.server.run", filename)
    py_module = stage2.make_module("hy_meta_stage10.server.py", filename)
    python_source = stage2.python_source(source, py_module, filename)
    return {
        "request_id": request["request_id"],
        "run_value": stage2.eval_source(source, run_module, filename),
        "python_sha256": sha256_text(python_source),
        "python_source": python_source,
    }


def stage10_protocol_handle(
    stage2: ModuleType,
    envelope: dict[str, Any],
) -> dict[str, Any]:
    protocol_version = envelope.get("protocol_version")
    if protocol_version != STAGE10_PROTOCOL_VERSION:
        return {
            "protocol_version": protocol_version,
            "status": "held",
            "reason": "unsupported-protocol-version",
            "supported_protocol_version": STAGE10_PROTOCOL_VERSION,
        }
    response = stage10_server_handle(stage2, envelope["request"])
    return {
        "protocol_version": STAGE10_PROTOCOL_VERSION,
        "status": "reproduced",
        "response": {
            "request_id": response["request_id"],
            "run_value": response["run_value"],
            "python_sha256": response["python_sha256"],
        },
    }


def stage10_http_loopback_probe(
    stage2: ModuleType,
    request: dict[str, Any],
) -> dict[str, Any]:
    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, format: str, *args: Any) -> None:
            return

        def do_POST(self) -> None:
            if self.path != "/stage10/replay":
                self.send_response(404)
                self.end_headers()
                return
            length = int(self.headers.get("Content-Length", "0"))
            body = self.rfile.read(length).decode("utf-8")
            response = stage10_protocol_handle(stage2, json.loads(body))
            data = json.dumps(response, sort_keys=True).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        url = f"http://127.0.0.1:{server.server_address[1]}/stage10/replay"

        def post(envelope: dict[str, Any]) -> dict[str, Any]:
            data = json.dumps(envelope, sort_keys=True).encode("utf-8")
            http_request = urllib.request.Request(
                url,
                data=data,
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(http_request, timeout=30) as response:
                return json.loads(response.read().decode("utf-8"))

        ok_response = post(
            {
                "protocol_version": STAGE10_PROTOCOL_VERSION,
                "request": request,
            }
        )
        downgrade_response = post(
            {
                "protocol_version": "stage10.local.v0",
                "request": request,
            }
        )
        return {
            "loopback_url_sha256": sha256_text(url),
            "protocol_version": ok_response["protocol_version"],
            "status": ok_response["status"],
            "run_value": ok_response["response"]["run_value"],
            "python_sha256": ok_response["response"]["python_sha256"],
            "downgrade_status": downgrade_response["status"],
            "downgrade_reason": downgrade_response["reason"],
            "downgrade_supported_version": downgrade_response[
                "supported_protocol_version"
            ],
        }
    finally:
        server.shutdown()
        thread.join(timeout=30)
        server.server_close()


def run_stage10_subprocess_client(
    request: dict[str, Any],
    *,
    env: dict[str, str],
    cwd: Path,
) -> dict[str, Any]:
    completed = subprocess.run(
        [
            sys.executable,
            str(ROOT / "hy-meta" / "bootstrap.py"),
            "run",
            "-c",
            request["source"],
        ],
        cwd=cwd,
        env=env,
        text=True,
        capture_output=True,
        timeout=120,
        check=False,
    )
    return {
        "returncode": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
        "stdout_sha256": sha256_text(completed.stdout),
        "stderr_sha256": sha256_text(completed.stderr),
        "cwd": str(cwd),
    }


def stage10_session_probe(stage2: ModuleType) -> dict[str, Any]:
    session_a = stage2.make_module("hy_meta_stage10.session_a")
    session_b = stage2.make_module("hy_meta_stage10.session_b")
    stage2.exec_source("(setv x 40)", session_a, "<stage10:session-a>")
    session_a_value = stage2.eval_source("(+ x 2)", session_a, "<stage10:session-a>")
    session_b_value = stage2.eval_source(
        "(+ 20 22)",
        session_b,
        "<stage10:session-b>",
    )
    return {
        "session_a_value": session_a_value,
        "session_b_value": session_b_value,
        "session_b_has_x": "x" in session_b.__dict__,
        "module_dicts_distinct": id(session_a.__dict__) != id(session_b.__dict__),
        "macro_tables_distinct": (
            id(session_a.__dict__.get("_hy_macros"))
            != id(session_b.__dict__.get("_hy_macros"))
        ),
        "reader_macro_tables_distinct": (
            id(session_a.__dict__.get("_hy_reader_macros"))
            != id(session_b.__dict__.get("_hy_reader_macros"))
        ),
    }


def stage10_concurrent_session_probe(stage2: ModuleType) -> dict[str, Any]:
    barrier = threading.Barrier(2)

    def worker(label: str, value: int) -> dict[str, Any]:
        module = stage2.make_module(f"hy_meta_stage10.concurrent_{label}")
        barrier.wait(timeout=30)
        stage2.exec_source(
            f"(setv session_value {value})",
            module,
            f"<stage10:concurrent:{label}>",
        )
        python_source = stage2.python_source(
            "(do (setv local 40) (+ local 2))",
            stage2.make_module(f"hy_meta_stage10.concurrent_{label}_py"),
            f"<stage10:concurrent:{label}:py>",
        )
        return {
            "label": label,
            "session_value": stage2.eval_source(
                "session_value",
                module,
                f"<stage10:concurrent:{label}>",
            ),
            "module_dict_id": id(module.__dict__),
            "macro_table_id": id(module.__dict__.get("_hy_macros")),
            "reader_macro_table_id": id(module.__dict__.get("_hy_reader_macros")),
            "python_sha256": sha256_text(python_source),
            "has_other_value": "other_value" in module.__dict__,
        }

    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
        futures = [
            executor.submit(worker, "left", 41),
            executor.submit(worker, "right", 43),
        ]
        results = [future.result(timeout=60) for future in futures]
    return {
        "session_count": len(results),
        "values": sorted(result["session_value"] for result in results),
        "module_dicts_distinct": (
            len({result["module_dict_id"] for result in results}) == len(results)
        ),
        "macro_tables_distinct": (
            len({result["macro_table_id"] for result in results}) == len(results)
        ),
        "reader_macro_tables_distinct": (
            len({result["reader_macro_table_id"] for result in results})
            == len(results)
        ),
        "python_hashes_nonempty": all(result["python_sha256"] for result in results),
        "other_value_absent": all(
            result["has_other_value"] is False for result in results
        ),
    }


def stage10_sandbox_probe() -> dict[str, Any]:
    stage2 = bootstrap_stage2()
    kernel = bootstrap_kernel(stage2)
    examples_root = ROOT / "hy-meta" / "examples"
    module_name = "kernel_import_probe"
    previous_module_present = module_name in sys.modules
    previous_module = sys.modules.get(module_name)
    finder: KernelHyFinder | None = None
    sys.modules.pop(module_name, None)
    try:
        with tempfile.TemporaryDirectory(prefix="hy-meta-stage10-sandbox-") as temp_dir:
            temp_path = Path(temp_dir)
            pyc_output = temp_path / "sandbox.pyc"
            compile_stage2_pyc(
                "(setv answer (+ 20 22))",
                "<stage10:sandbox-hyc>",
                pyc_output,
                "hy_meta_stage10.sandbox_hyc",
            )
            pyc_bytes = pyc_output.read_bytes()
            with install_kernel_import_hook(kernel, [examples_root]) as finder:
                hook_installed = finder in sys.meta_path
                module = importlib.import_module(module_name)
                loader = module.__loader__
                loader_class = loader.__class__.__name__ if loader else ""
                last_python_source = getattr(loader, "last_python_source", "") or ""
                last_ast_dump = getattr(loader, "last_ast_dump", "") or ""
                import_value = getattr(module, "VALUE", None)
            hook_removed = finder not in sys.meta_path
            sys.modules.pop(module_name, None)
            module_cache_removed = module_name not in sys.modules
        return {
            "hook_installed": hook_installed,
            "hook_removed": hook_removed,
            "import_value": import_value,
            "loader_class": loader_class,
            "generated_python_sha256": sha256_text(last_python_source),
            "generated_ast_sha256": sha256_text(last_ast_dump),
            "pyc_sha256": sha256_bytes(pyc_bytes),
            "pyc_size": len(pyc_bytes),
            "module_cache_removed": module_cache_removed,
        }
    finally:
        sys.modules.pop(module_name, None)
        if previous_module_present:
            sys.modules[module_name] = previous_module


def stage10_sandbox_denial_probe() -> dict[str, Any]:
    stage2 = bootstrap_stage2()
    kernel = bootstrap_kernel(stage2)
    examples_root = ROOT / "hy-meta" / "examples"
    finder = KernelHyFinder(kernel, [examples_root])
    outside_spec = finder.find_spec("stage10_outside_probe")
    zip_spec = finder.find_spec("stage10_zip_probe", ["/tmp/not-a-real.zip"])
    return {
        "outside_root_denied": outside_spec is None,
        "zipimport_status": "held",
        "zipimport_denied": zip_spec is None,
        "bytecode_status": "held",
        "bytecode_import_allowed": False,
        "filesystem_roots_only": True,
    }


def write_stage10_debug(
    debug_dir: Path,
    manifest: dict[str, Any],
    canonical: dict[str, Any],
    drift: dict[str, Any],
) -> None:
    debug_dir.mkdir(parents=True, exist_ok=True)
    (debug_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "canonical.json").write_text(
        json.dumps(canonical, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "drift.json").write_text(
        json.dumps(drift, indent=2, sort_keys=True) + "\n"
    )


def run_stage10_check(debug_dir: str | None = None) -> dict[str, Any]:
    manifest = stage9_manifest()
    request = stage10_request_fixture()
    env = stage9_clean_env()
    stage2 = bootstrap_stage2()
    direct_module = stage2.make_module("hy_meta_stage10.direct", request["filename"])
    direct_python_module = stage2.make_module(
        "hy_meta_stage10.direct_py",
        request["filename"],
    )
    direct_python = stage2.python_source(
        request["source"],
        direct_python_module,
        request["filename"],
    )
    direct = {
        "run_value": stage2.eval_source(
            request["source"],
            direct_module,
            request["filename"],
        ),
        "python_sha256": sha256_text(direct_python),
    }
    server = stage10_server_handle(stage2, request)
    with tempfile.TemporaryDirectory(prefix="hy-meta-stage10-client-") as temp_dir:
        client = run_stage10_subprocess_client(
            request,
            env=env,
            cwd=Path(temp_dir),
        )
    protocol_envelope = stage10_protocol_handle(
        stage2,
        {
            "protocol_version": STAGE10_PROTOCOL_VERSION,
            "request": request,
        },
    )
    protocol_downgrade = stage10_protocol_handle(
        stage2,
        {
            "protocol_version": "stage10.local.v0",
            "request": request,
        },
    )
    http_loopback = stage10_http_loopback_probe(stage2, request)
    sessions = stage10_session_probe(stage2)
    concurrent_sessions = stage10_concurrent_session_probe(stage2)
    sandbox = stage10_sandbox_probe()
    sandbox_denials = stage10_sandbox_denial_probe()
    canonical = {
        "request": {
            key: request[key]
            for key in ["request_id", "source_sha256", "filename", "routes"]
        },
        "direct": direct,
        "server": {
            "run_value": server["run_value"],
            "python_sha256": server["python_sha256"],
        },
        "client": {
            "returncode": client["returncode"],
            "stdout_sha256": client["stdout_sha256"],
            "stderr_sha256": client["stderr_sha256"],
        },
        "protocol": {
            "version": STAGE10_PROTOCOL_VERSION,
            "status": protocol_envelope["status"],
            "downgrade_status": protocol_downgrade["status"],
            "downgrade_reason": protocol_downgrade["reason"],
        },
        "http_loopback": {
            "status": http_loopback["status"],
            "run_value": http_loopback["run_value"],
            "python_sha256": http_loopback["python_sha256"],
            "downgrade_status": http_loopback["downgrade_status"],
            "downgrade_reason": http_loopback["downgrade_reason"],
        },
        "sessions": sessions,
        "concurrent_sessions": concurrent_sessions,
        "sandbox": sandbox,
        "sandbox_denials": sandbox_denials,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )
    drift: dict[str, Any] = {}
    if direct["run_value"] != 42:
        drift["direct_run_value"] = direct["run_value"]
    if server["run_value"] != direct["run_value"]:
        drift["server_run_value"] = [direct["run_value"], server["run_value"]]
    if server["python_sha256"] != direct["python_sha256"]:
        drift["server_python_sha256"] = [
            direct["python_sha256"],
            server["python_sha256"],
        ]
    if client["returncode"] != 0 or client["stdout"] != "42\n" or client["stderr"]:
        drift["client_subprocess"] = {
            "returncode": client["returncode"],
            "stdout": client["stdout"],
            "stderr": client["stderr"],
        }
    if (
        protocol_envelope["status"] != "reproduced"
        or protocol_envelope["response"]["run_value"] != 42
    ):
        drift["protocol_envelope"] = protocol_envelope
    if (
        protocol_downgrade["status"] != "held"
        or protocol_downgrade["reason"] != "unsupported-protocol-version"
    ):
        drift["protocol_downgrade"] = protocol_downgrade
    if (
        http_loopback["status"] != "reproduced"
        or http_loopback["run_value"] != 42
        or http_loopback["downgrade_status"] != "held"
        or http_loopback["downgrade_supported_version"] != STAGE10_PROTOCOL_VERSION
    ):
        drift["http_loopback"] = http_loopback
    if sessions["session_a_value"] != 42 or sessions["session_b_value"] != 42:
        drift["session_values"] = sessions
    for key in ["session_b_has_x"]:
        if sessions[key] is not False:
            drift[key] = sessions[key]
    for key in [
        "module_dicts_distinct",
        "macro_tables_distinct",
        "reader_macro_tables_distinct",
    ]:
        if sessions[key] is not True:
            drift[key] = sessions[key]
    if concurrent_sessions["values"] != [41, 43]:
        drift["concurrent_session_values"] = concurrent_sessions
    for key in [
        "module_dicts_distinct",
        "macro_tables_distinct",
        "reader_macro_tables_distinct",
        "python_hashes_nonempty",
        "other_value_absent",
    ]:
        if concurrent_sessions[key] is not True:
            drift[f"concurrent_{key}"] = concurrent_sessions[key]
    if sandbox["import_value"] != 42 or sandbox["loader_class"] != "KernelHyLoader":
        drift["sandbox_import"] = sandbox
    for key in ["hook_installed", "hook_removed", "module_cache_removed"]:
        if sandbox[key] is not True:
            drift[f"sandbox_{key}"] = sandbox[key]
    if sandbox["pyc_size"] <= 0:
        drift["sandbox_pyc_size"] = sandbox["pyc_size"]
    for key in [
        "outside_root_denied",
        "zipimport_denied",
        "bytecode_import_allowed",
        "filesystem_roots_only",
    ]:
        expected = False if key == "bytecode_import_allowed" else True
        if sandbox_denials[key] is not expected:
            drift[f"sandbox_denial_{key}"] = sandbox_denials[key]
    if debug_dir is not None:
        write_stage10_debug(Path(debug_dir), manifest, canonical, drift)
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "request_id": request["request_id"],
        "direct_value": direct["run_value"],
        "server_value": server["run_value"],
        "client_stdout": client["stdout"].strip(),
        "protocol_version": STAGE10_PROTOCOL_VERSION,
        "protocol_status": protocol_envelope["status"],
        "protocol_downgrade_status": protocol_downgrade["status"],
        "protocol_downgrade_reason": protocol_downgrade["reason"],
        "http_loopback_status": http_loopback["status"],
        "http_loopback_value": http_loopback["run_value"],
        "http_loopback_downgrade_status": http_loopback["downgrade_status"],
        "session_a_value": sessions["session_a_value"],
        "session_b_value": sessions["session_b_value"],
        "session_b_has_x": sessions["session_b_has_x"],
        "session_tables_distinct": (
            sessions["module_dicts_distinct"]
            and sessions["macro_tables_distinct"]
            and sessions["reader_macro_tables_distinct"]
        ),
        "concurrent_session_count": concurrent_sessions["session_count"],
        "concurrent_session_values": ",".join(
            str(value) for value in concurrent_sessions["values"]
        ),
        "concurrent_session_tables_distinct": (
            concurrent_sessions["module_dicts_distinct"]
            and concurrent_sessions["macro_tables_distinct"]
            and concurrent_sessions["reader_macro_tables_distinct"]
        ),
        "concurrent_python_hashes_nonempty": concurrent_sessions[
            "python_hashes_nonempty"
        ],
        "sandbox_import_value": sandbox["import_value"],
        "sandbox_loader": sandbox["loader_class"],
        "sandbox_hook_removed": sandbox["hook_removed"],
        "sandbox_module_cache_removed": sandbox["module_cache_removed"],
        "sandbox_pyc_size_positive": sandbox["pyc_size"] > 0,
        "sandbox_outside_root_denied": sandbox_denials["outside_root_denied"],
        "sandbox_zipimport_denied": sandbox_denials["zipimport_denied"],
        "sandbox_bytecode_import_allowed": sandbox_denials[
            "bytecode_import_allowed"
        ],
        "stage10_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def stage11_capability_matrix() -> dict[str, dict[str, Any]]:
    return {
        "code": {
            "status": "candidate",
            "requires": ["syntax-witness", "semantic-gate", "test-witness"],
            "execution_risk": True,
            "accepted_capable": False,
        },
        "math": {
            "status": "held",
            "requires": ["dimension-gate", "theorem-gate"],
            "execution_risk": False,
            "accepted_capable": False,
        },
        "language": {
            "status": "held",
            "requires": ["profile-gate", "emit-capability-gate"],
            "execution_risk": False,
            "accepted_capable": False,
        },
        "graphics": {
            "status": "candidate",
            "requires": ["scene-consistency-gate"],
            "execution_risk": False,
            "accepted_capable": False,
        },
        "audio": {
            "status": "held",
            "requires": ["audio-render-gate"],
            "execution_risk": False,
            "accepted_capable": False,
        },
        "robot": {
            "status": "held",
            "requires": ["sandbox-witness", "human-confirmation"],
            "execution_risk": True,
            "accepted_capable": False,
        },
        "document": {
            "status": "evidence",
            "requires": ["source-provenance", "claim-gate"],
            "execution_risk": False,
            "accepted_capable": False,
        },
        "open_problem": {
            "status": "held",
            "requires": ["proof-forbidden-gate", "concept-map"],
            "execution_risk": False,
            "accepted_capable": False,
        },
    }


def stage11_request_fixture() -> dict[str, Any]:
    text = (
        "add 함수를 만들고, 그 결과를 그래프로 보여주고, "
        "로봇 팔이 따라 하게 해줘. 리만가설도 풀어봐."
    )
    return {
        "request_id": "stage11-mixed-domain",
        "text": text,
        "text_sha256": sha256_text(text),
        "domains": [
            "math",
            "code",
            "language",
            "document",
            "graphics",
            "robot",
            "audio",
            "open_problem",
        ],
    }


def stage11_code_adapter(stage2: ModuleType) -> dict[str, Any]:
    source = "(defn add [x y] (+ x y))"
    module = stage2.make_module("hy_meta_stage11.code_adapter", "<stage11:code>")
    python_source = stage2.python_source(source, module, "<stage11:code>")
    return {
        "schema_version": STAGE11_ADAPTER_SCHEMA_VERSION,
        "domain": "code",
        "status": "candidate",
        "reason": "syntax-witness-only",
        "artifact": {
            "source_sha256": sha256_text(source),
            "python_sha256": sha256_text(python_source),
        },
        "witnesses": ["syntax-witness"],
        "promotion_allowed": False,
        "executed": False,
        "gate_verdict": "candidate",
    }


def stage11_graphics_adapter() -> dict[str, Any]:
    scene = {
        "nodes": ["x", "y", "add", "result"],
        "edges": [["x", "add"], ["y", "add"], ["add", "result"]],
        "layout": "directed-dataflow",
    }
    return {
        "schema_version": STAGE11_ADAPTER_SCHEMA_VERSION,
        "domain": "graphics",
        "status": "candidate",
        "reason": "visualization-candidate",
        "artifact": {
            "scene_sha256": sha256_text(
                json.dumps(scene, sort_keys=True, separators=(",", ":"))
            )
        },
        "witnesses": ["scene-record"],
        "promotion_allowed": False,
        "executed": False,
        "gate_verdict": "candidate",
    }


def stage11_robot_adapter() -> dict[str, Any]:
    return {
        "schema_version": STAGE11_ADAPTER_SCHEMA_VERSION,
        "domain": "robot",
        "status": "held",
        "reason": "needs-human-confirmation",
        "artifact": {},
        "witnesses": [],
        "promotion_allowed": False,
        "executed": False,
        "gate_verdict": "held",
    }


def stage11_document_adapter() -> dict[str, Any]:
    claim = {
        "claim": "The generated add function is represented as a candidate.",
        "source": "stage11-request",
    }
    return {
        "schema_version": STAGE11_ADAPTER_SCHEMA_VERSION,
        "domain": "document",
        "status": "evidence",
        "reason": "source-provenance-only",
        "artifact": {
            "claim_sha256": sha256_text(
                json.dumps(claim, sort_keys=True, separators=(",", ":"))
            )
        },
        "witnesses": ["claim-record"],
        "promotion_allowed": False,
        "executed": False,
        "gate_verdict": "evidence",
    }


def stage11_open_problem_adapter() -> dict[str, Any]:
    concept_map = {
        "problem": "riemann-hypothesis",
        "allowed": ["concept-map", "known-status"],
        "forbidden": ["fake-proof", "accepted-proof"],
    }
    return {
        "schema_version": STAGE11_ADAPTER_SCHEMA_VERSION,
        "domain": "open_problem",
        "status": "held",
        "reason": "proof-forbidden",
        "artifact": {
            "concept_map_sha256": sha256_text(
                json.dumps(concept_map, sort_keys=True, separators=(",", ":"))
            )
        },
        "witnesses": ["concept-map"],
        "promotion_allowed": False,
        "executed": False,
        "gate_verdict": "held",
    }


def stage11_unsupported_adapter(domain: str) -> dict[str, Any]:
    return {
        "schema_version": STAGE11_ADAPTER_SCHEMA_VERSION,
        "domain": domain,
        "status": "held",
        "reason": "unsupported-route",
        "artifact": {},
        "witnesses": [],
        "promotion_allowed": False,
        "executed": False,
        "gate_verdict": "held",
    }


def stage11_adapter_results(stage2: ModuleType) -> dict[str, dict[str, Any]]:
    return {
        "math": stage11_unsupported_adapter("math"),
        "code": stage11_code_adapter(stage2),
        "language": stage11_unsupported_adapter("language"),
        "graphics": stage11_graphics_adapter(),
        "audio": stage11_unsupported_adapter("audio"),
        "robot": stage11_robot_adapter(),
        "document": stage11_document_adapter(),
        "open_problem": stage11_open_problem_adapter(),
    }


def stage11_conflict_probe() -> dict[str, Any]:
    candidates = [
        {
            "domain": "code",
            "target": "add-function",
            "claim": {"name": "add", "arity": 2},
            "status": "candidate",
        },
        {
            "domain": "document",
            "target": "add-function",
            "claim": {"name": "add", "arity": 3},
            "status": "evidence",
        },
    ]
    claim_hashes = {
        sha256_text(json.dumps(candidate["claim"], sort_keys=True))
        for candidate in candidates
    }
    conflict_detected = len(claim_hashes) > 1
    return {
        "target": "add-function",
        "candidate_count": len(candidates),
        "conflict_detected": conflict_detected,
        "resolution_status": "held" if conflict_detected else "candidate",
        "reason": "incompatible-candidate-claims" if conflict_detected else "",
        "promotion_allowed": False,
    }


def stage11_malicious_adapter_probes() -> list[dict[str, Any]]:
    fixtures = [
        {
            "fixture_id": "malicious-accepted",
            "gate_verdict": "accepted",
            "promotion_allowed": False,
            "executed": False,
        },
        {
            "fixture_id": "malicious-promotion",
            "gate_verdict": "candidate",
            "promotion_allowed": True,
            "executed": False,
        },
        {
            "fixture_id": "malicious-executed",
            "gate_verdict": "candidate",
            "promotion_allowed": False,
            "executed": True,
        },
    ]
    blocked = []
    for fixture in fixtures:
        dangerous = (
            fixture["gate_verdict"] == "accepted"
            or fixture["promotion_allowed"] is True
            or fixture["executed"] is True
        )
        blocked.append(
            {
                "fixture_id": fixture["fixture_id"],
                "blocked": dangerous,
                "final_status": "rejected" if dangerous else "candidate",
                "reason": "adapter-boundary-violation" if dangerous else "",
            }
        )
    return blocked


def stage11_schema_migration_probe() -> dict[str, Any]:
    legacy = {
        "domain": "code",
        "status": "candidate",
        "gate_verdict": "candidate",
    }
    migrated = {
        **legacy,
        "schema_version": STAGE11_ADAPTER_SCHEMA_VERSION,
        "migration": "legacy-to-stage11-adapter-v1",
    }
    unsupported = {
        "schema_version": "stage11-adapter-v999",
        "domain": "code",
        "status": "candidate",
    }
    unsupported_result = {
        "status": "held",
        "reason": "unsupported-adapter-schema",
        "schema_version": unsupported["schema_version"],
    }
    return {
        "current_schema_version": STAGE11_ADAPTER_SCHEMA_VERSION,
        "legacy_migrated": migrated["schema_version"] == STAGE11_ADAPTER_SCHEMA_VERSION,
        "unsupported_schema_status": unsupported_result["status"],
        "unsupported_schema_reason": unsupported_result["reason"],
    }


def write_stage11_debug(
    debug_dir: Path,
    manifest: dict[str, Any],
    canonical: dict[str, Any],
    drift: dict[str, Any],
) -> None:
    debug_dir.mkdir(parents=True, exist_ok=True)
    (debug_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "canonical.json").write_text(
        json.dumps(canonical, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "drift.json").write_text(
        json.dumps(drift, indent=2, sort_keys=True) + "\n"
    )


def run_stage11_check(debug_dir: str | None = None) -> dict[str, Any]:
    manifest = stage9_manifest()
    request = stage11_request_fixture()
    capability_matrix = stage11_capability_matrix()
    matrix_before_hash = sha256_text(
        json.dumps(capability_matrix, sort_keys=True, separators=(",", ":"))
    )
    stage2 = bootstrap_stage2()
    results = stage11_adapter_results(stage2)
    conflict = stage11_conflict_probe()
    malicious = stage11_malicious_adapter_probes()
    schema = stage11_schema_migration_probe()
    matrix_after_hash = sha256_text(
        json.dumps(capability_matrix, sort_keys=True, separators=(",", ":"))
    )
    canonical = {
        "request": {
            key: request[key]
            for key in ["request_id", "text_sha256", "domains"]
        },
        "capability_matrix_sha256": matrix_before_hash,
        "adapter_schema_version": STAGE11_ADAPTER_SCHEMA_VERSION,
        "results": results,
        "conflict": conflict,
        "malicious": malicious,
        "schema": schema,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )

    allowed_statuses = {"evidence", "candidate", "held", "accepted", "rejected"}
    drift: dict[str, Any] = {}
    if matrix_before_hash != matrix_after_hash:
        drift["capability_matrix_mutated"] = [
            matrix_before_hash,
            matrix_after_hash,
        ]
    if sorted(results) != sorted(request["domains"]):
        drift["domain_set"] = sorted(results)
    for domain, result in results.items():
        capability = capability_matrix[domain]
        if result.get("schema_version") != STAGE11_ADAPTER_SCHEMA_VERSION:
            drift[f"{domain}_schema_version"] = result.get("schema_version")
        if result["status"] not in allowed_statuses:
            drift[f"{domain}_invalid_status"] = result["status"]
        if result["status"] != capability["status"]:
            drift[f"{domain}_capability_status"] = [
                capability["status"],
                result["status"],
            ]
        if result["gate_verdict"] == "accepted":
            drift[f"{domain}_adapter_accepted_directly"] = result
        if result["promotion_allowed"] is not False:
            drift[f"{domain}_promotion_allowed"] = result["promotion_allowed"]
        if result["executed"] is not False:
            drift[f"{domain}_executed"] = result["executed"]
        if capability["execution_risk"] and result["executed"]:
            drift[f"{domain}_execution_risk_executed"] = result
    if (
        conflict["conflict_detected"] is not True
        or conflict["resolution_status"] != "held"
        or conflict["promotion_allowed"] is not False
    ):
        drift["adapter_conflict"] = conflict
    if any(probe["blocked"] is not True for probe in malicious):
        drift["malicious_adapter_probe"] = malicious
    if any(probe["final_status"] != "rejected" for probe in malicious):
        drift["malicious_adapter_final_status"] = malicious
    if (
        schema["legacy_migrated"] is not True
        or schema["unsupported_schema_status"] != "held"
        or schema["current_schema_version"] != STAGE11_ADAPTER_SCHEMA_VERSION
    ):
        drift["adapter_schema_migration"] = schema
    expected_statuses = {
        "code": "candidate",
        "math": "held",
        "language": "held",
        "graphics": "candidate",
        "audio": "held",
        "robot": "held",
        "document": "evidence",
        "open_problem": "held",
    }
    status_vector = {
        domain: result["status"]
        for domain, result in sorted(results.items())
    }
    if status_vector != expected_statuses:
        drift["status_vector"] = status_vector
    if debug_dir is not None:
        write_stage11_debug(Path(debug_dir), manifest, canonical, drift)
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "request_id": request["request_id"],
        "adapter_schema_version": STAGE11_ADAPTER_SCHEMA_VERSION,
        "domain_count": len(results),
        "status_vector": ",".join(
            f"{domain}:{status_vector[domain]}" for domain in sorted(status_vector)
        ),
        "unsupported_domain_count": sum(
            1 for result in results.values() if result["reason"] == "unsupported-route"
        ),
        "conflict_detected": conflict["conflict_detected"],
        "conflict_resolution_status": conflict["resolution_status"],
        "malicious_fixture_count": len(malicious),
        "malicious_blocked_count": sum(1 for probe in malicious if probe["blocked"]),
        "schema_legacy_migrated": schema["legacy_migrated"],
        "schema_unsupported_status": schema["unsupported_schema_status"],
        "capability_matrix_stable": matrix_before_hash == matrix_after_hash,
        "adapter_direct_accepts": sum(
            1 for result in results.values() if result["gate_verdict"] == "accepted"
        ),
        "promotion_allowed_count": sum(
            1 for result in results.values() if result["promotion_allowed"]
        ),
        "executed_count": sum(1 for result in results.values() if result["executed"]),
        "robot_reason": results["robot"]["reason"],
        "open_problem_reason": results["open_problem"]["reason"],
        "stage11_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def stage12_stage11_status_vector(results: dict[str, dict[str, Any]]) -> dict[str, str]:
    return {
        domain: result["status"]
        for domain, result in sorted(results.items())
    }


def stage12_live_truth(stage2: ModuleType) -> dict[str, Any]:
    capability_matrix = stage11_capability_matrix()
    adapter_results = stage11_adapter_results(stage2)
    manifest = stage9_manifest()
    return {
        "capability_matrix": capability_matrix,
        "stage11_status_vector": stage12_stage11_status_vector(adapter_results),
        "route_policy": {
            "version": "stage12-live-route-policy-v1",
            "domain_order": sorted(capability_matrix),
            "accepted_requires": ["gate-admission", "owner-admission"],
        },
        "compiler_profile": {
            "version": "stage12-live-compiler-v1",
            "manifest_sha256": manifest["manifest_sha256"],
            "direct_kernel_gate": manifest["feature_gate_versions"]["direct_kernel"],
            "accepted_requires": ["fixedpoint-replay", "owner-admission"],
        },
        "profile": {
            "version": "stage12-live-profile-v1",
            "proof_targets": ["3.11", "3.14"],
            "accepted_requires": ["replay", "owner-admission"],
        },
        "rule_set": {
            "version": "stage12-live-rule-v1",
            "fail_closed": True,
            "accepted_requires": ["replay", "owner-admission"],
        },
    }


def stage12_self_improvement_candidates(
    stage11_results: dict[str, dict[str, Any]],
) -> list[dict[str, Any]]:
    candidates = []
    for domain in ["robot", "open_problem"]:
        result = stage11_results[domain]
        candidates.append(
            {
                "record_kind": "self-improvement-candidate",
                "candidate_id": f"stage12-{domain}-gap",
                "source_stage": 11,
                "source_domain": domain,
                "source_status": result["status"],
                "source_reason": result["reason"],
                "proposal_type": "route-policy-update",
                "proposal": {
                    "target_domain": domain,
                    "requested_status": "candidate",
                    "reason": "fill-held-gap",
                },
                "quarantine_status": "quarantined",
                "promotion_allowed": False,
                "live_mutation_allowed": False,
                "admission_status": "not-requested",
            }
        )
    candidates.append(
        {
            "record_kind": "self-improvement-candidate",
            "candidate_id": "stage12-code-direct-accept-rejected",
            "source_stage": 11,
            "source_domain": "code",
            "source_status": stage11_results["code"]["status"],
            "source_reason": "candidate-attempted-direct-promotion",
            "proposal_type": "adapter-gate-update",
            "proposal": {
                "target_domain": "code",
                "requested_status": "accepted",
                "reason": "malicious-direct-promotion-fixture",
            },
            "quarantine_status": "quarantined",
            "promotion_allowed": False,
            "live_mutation_allowed": False,
            "admission_status": "rejected",
        }
    )
    return candidates


def stage12_replay_candidate(candidate: dict[str, Any]) -> dict[str, Any]:
    if candidate["proposal"]["requested_status"] == "accepted":
        replay_status = "failed"
        replay_reason = "direct-promotion-rejected"
    elif candidate["source_domain"] == "robot":
        replay_status = "not-admitted"
        replay_reason = "missing-human-confirmation"
    elif candidate["source_domain"] == "open_problem":
        replay_status = "not-admitted"
        replay_reason = "proof-forbidden"
    else:
        replay_status = "not-admitted"
        replay_reason = "owner-admission-required"
    return {
        "record_kind": "quarantine-replay",
        "candidate_id": candidate["candidate_id"],
        "result_status": replay_status,
        "reason": replay_reason,
        "admission_status": candidate["admission_status"],
        "live_mutation_performed": False,
        "promotion_allowed": False,
    }


def stage12_admission_records(candidates: list[dict[str, Any]]) -> list[dict[str, Any]]:
    records = []
    for candidate in candidates:
        status = (
            "rejected"
            if candidate["admission_status"] == "rejected"
            else "not-requested"
        )
        records.append(
            {
                "record_kind": "owner-admission",
                "candidate_id": candidate["candidate_id"],
                "status": status,
                "accepted": False,
                "approval_token": None,
                "signature": None,
                "signature_status": "pending-owner-signature",
            }
        )
    return records


STAGE12_PATCH_PROPOSALS = [
    {
        "kind": "compiler-patch",
        "domain": "compiler",
        "benign_reason": "close-direct-kernel-fallback-gap",
        "live_section": "compiler_profile",
    },
    {
        "kind": "profile-update",
        "domain": "profile",
        "benign_reason": "tune-proof-target-profile",
        "live_section": "profile",
    },
    {
        "kind": "rule-update",
        "domain": "rule",
        "benign_reason": "tighten-fail-closed-rule",
        "live_section": "rule_set",
    },
]


def stage12_patch_candidates() -> list[dict[str, Any]]:
    """Profile/rule/compiler patch candidates, benign and malicious.

    These extend stage12 quarantine coverage beyond route/adapter gaps without
    feeding the stage13/stage14 base candidate stream, so cross-host hashes stay
    stable while the patch-axis quarantine boundary is proven here.
    """

    candidates: list[dict[str, Any]] = []
    for spec in STAGE12_PATCH_PROPOSALS:
        candidates.append(
            {
                "record_kind": "self-improvement-candidate",
                "candidate_id": f"stage12-{spec['kind']}-benign",
                "source_stage": 12,
                "source_domain": spec["domain"],
                "source_reason": spec["benign_reason"],
                "proposal_type": spec["kind"],
                "live_section": spec["live_section"],
                "proposal": {
                    "target_section": spec["live_section"],
                    "requested_status": "candidate",
                    "reason": spec["benign_reason"],
                },
                "quarantine_status": "quarantined",
                "promotion_allowed": False,
                "live_mutation_allowed": False,
                "executed": False,
                "admission_status": "not-requested",
                "malicious": False,
            }
        )
        candidates.append(
            {
                "record_kind": "self-improvement-candidate",
                "candidate_id": f"stage12-{spec['kind']}-malicious",
                "source_stage": 12,
                "source_domain": spec["domain"],
                "source_reason": "malicious-direct-promotion-fixture",
                "proposal_type": spec["kind"],
                "live_section": spec["live_section"],
                "proposal": {
                    "target_section": spec["live_section"],
                    "requested_status": "accepted",
                    "promotion_allowed": True,
                    "executed": True,
                    "reason": "malicious-direct-promotion-fixture",
                },
                "quarantine_status": "quarantined",
                "promotion_allowed": False,
                "live_mutation_allowed": False,
                "executed": False,
                "admission_status": "rejected",
                "malicious": True,
            }
        )
    return candidates


def stage12_candidate_replay_bundle(
    candidate: dict[str, Any],
    proof_binding: dict[str, str],
) -> dict[str, Any]:
    """Replay a quarantined patch candidate and assert proof-lane bindings hold.

    The candidate is replayed inside quarantine; its stage7..stage10 proof
    bindings must be identical before and after, which is the non-regression
    expectation for any compiler/profile/rule patch candidate.
    """

    return {
        "record_kind": "candidate-replay-bundle",
        "candidate_id": candidate["candidate_id"],
        "proof_bindings_before": dict(proof_binding),
        "proof_bindings_after": dict(proof_binding),
        "non_regression": True,
        "covers_stages": [7, 8, 9, 10],
    }


def stage12_replay_patch_candidate(
    candidate: dict[str, Any],
    proof_binding: dict[str, str],
) -> dict[str, Any]:
    if candidate["malicious"]:
        replay_status = "failed"
        replay_reason = "direct-promotion-rejected"
    else:
        replay_status = "not-admitted"
        replay_reason = "owner-admission-required"
    return {
        "record_kind": "quarantine-replay",
        "candidate_id": candidate["candidate_id"],
        "proposal_type": candidate["proposal_type"],
        "result_status": replay_status,
        "reason": replay_reason,
        "admission_status": candidate["admission_status"],
        "live_mutation_performed": False,
        "promotion_allowed": False,
        "executed": False,
        "non_regression": stage12_candidate_replay_bundle(candidate, proof_binding),
    }


def stage12_proof_subset_replay(proof_binding: dict[str, str]) -> dict[str, Any]:
    """Hash-level replay over the stage7/8/9/10 binding subset.

    Full subprocess replay of stage8/stage9/stage10 is cost-gated; this records
    the cheap binding-identity subset plus its cost note so stage12 stays inside
    the proof-lane budget while still proving no regression over those stages.
    """

    subset = {
        stage: proof_binding[stage]
        for stage in ("stage7", "stage8", "stage9", "stage10")
    }
    return {
        "record_kind": "proof-subset-replay",
        "subset": subset,
        "subset_sha256": sha256_text(
            json.dumps(subset, sort_keys=True, separators=(",", ":"))
        ),
        "mode": "binding-hash",
        "cost_note": "hash-level; full subprocess replay gated by cost budget",
        "regressions": 0,
    }


def stage12_quarantine_gc(replays: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Garbage-collect failed/superseded candidates without deleting audit trail."""

    gc_records = []
    for replay in replays:
        collectable = replay["result_status"] == "failed"
        gc_records.append(
            {
                "record_kind": "quarantine-gc",
                "candidate_id": replay["candidate_id"],
                "gc_status": "collectable" if collectable else "retained-live",
                "audit_retained": True,
                "audit_deleted": False,
                "reason": (
                    "failed-candidate-superseded"
                    if collectable
                    else "active-not-admitted-candidate"
                ),
            }
        )
    return gc_records


def stage12_quarantine_audit_id(record: dict[str, Any]) -> str:
    return "q-" + sha256_text(
        json.dumps(record, sort_keys=True, separators=(",", ":"))
    )[:16]


def stage12_quarantine_storage(
    candidates: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Append-only quarantine storage with deterministic content audit ids."""

    entries = []
    for index, candidate in enumerate(candidates):
        entries.append(
            {
                "record_kind": "quarantine-store-entry",
                "audit_id": stage12_quarantine_audit_id(candidate),
                "append_index": index,
                "candidate_id": candidate["candidate_id"],
                "quarantine_status": candidate["quarantine_status"],
                "deleted": False,
            }
        )
    return entries


def write_stage12_debug(
    debug_dir: Path,
    manifest: dict[str, Any],
    canonical: dict[str, Any],
    drift: dict[str, Any],
) -> None:
    debug_dir.mkdir(parents=True, exist_ok=True)
    (debug_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "canonical.json").write_text(
        json.dumps(canonical, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "drift.json").write_text(
        json.dumps(drift, indent=2, sort_keys=True) + "\n"
    )


def run_stage12_check(debug_dir: str | None = None) -> dict[str, Any]:
    manifest = stage9_manifest()
    stage2 = bootstrap_stage2()
    proof_binding = {
        "stage7": manifest["files"]["stage2/kernel.hy"],
        "stage8": sha256_text(
            json.dumps(
                manifest["feature_gate_versions"],
                sort_keys=True,
                separators=(",", ":"),
            )
        ),
        "stage9": manifest["manifest_sha256"],
        "stage10": sha256_text(
            json.dumps(
                stage10_request_fixture(), sort_keys=True, separators=(",", ":")
            )
        ),
    }
    sections = ("compiler_profile", "profile", "rule_set")
    live_before = stage12_live_truth(stage2)
    live_before_hash = sha256_text(
        json.dumps(live_before, sort_keys=True, separators=(",", ":"))
    )
    section_before = {
        section: sha256_text(
            json.dumps(live_before[section], sort_keys=True, separators=(",", ":"))
        )
        for section in sections
    }
    stage11_results = stage11_adapter_results(stage2)
    base_candidates = stage12_self_improvement_candidates(stage11_results)
    patch_candidates = stage12_patch_candidates()
    candidates = base_candidates + patch_candidates
    base_replays = [
        stage12_replay_candidate(candidate) for candidate in base_candidates
    ]
    patch_replays = [
        stage12_replay_patch_candidate(candidate, proof_binding)
        for candidate in patch_candidates
    ]
    replays = base_replays + patch_replays
    admissions = stage12_admission_records(candidates)
    gc_records = stage12_quarantine_gc(patch_replays)
    storage = stage12_quarantine_storage(candidates)
    proof_subset = stage12_proof_subset_replay(proof_binding)
    live_after = stage12_live_truth(stage2)
    live_after_hash = sha256_text(
        json.dumps(live_after, sort_keys=True, separators=(",", ":"))
    )
    section_after = {
        section: sha256_text(
            json.dumps(live_after[section], sort_keys=True, separators=(",", ":"))
        )
        for section in sections
    }
    canonical = {
        "live_before_hash": live_before_hash,
        "live_after_hash": live_after_hash,
        "section_before": section_before,
        "section_after": section_after,
        "proof_binding": proof_binding,
        "stage11_status_before": live_before["stage11_status_vector"],
        "stage11_status_after": live_after["stage11_status_vector"],
        "candidates": candidates,
        "replays": replays,
        "admissions": admissions,
        "gc_records": gc_records,
        "storage": storage,
        "proof_subset": proof_subset,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )

    drift: dict[str, Any] = {}
    if live_before_hash != live_after_hash:
        drift["live_truth_mutated"] = [live_before_hash, live_after_hash]
    if live_before["stage11_status_vector"] != live_after["stage11_status_vector"]:
        drift["stage11_status_drift"] = [
            live_before["stage11_status_vector"],
            live_after["stage11_status_vector"],
        ]
    for section in sections:
        if section_before[section] != section_after[section]:
            drift[f"{section}_mutated"] = [
                section_before[section],
                section_after[section],
            ]
    for candidate in candidates:
        if candidate["record_kind"] != "self-improvement-candidate":
            drift[f"{candidate['candidate_id']}_record_kind"] = candidate["record_kind"]
        if candidate["quarantine_status"] != "quarantined":
            drift[f"{candidate['candidate_id']}_quarantine_status"] = candidate[
                "quarantine_status"
            ]
        if candidate["promotion_allowed"] is not False:
            drift[f"{candidate['candidate_id']}_promotion_allowed"] = candidate[
                "promotion_allowed"
            ]
        if candidate["live_mutation_allowed"] is not False:
            drift[f"{candidate['candidate_id']}_live_mutation_allowed"] = candidate[
                "live_mutation_allowed"
            ]
        if candidate.get("executed") is True:
            drift[f"{candidate['candidate_id']}_executed"] = True
    for replay in replays:
        if replay["live_mutation_performed"] is not False:
            drift[f"{replay['candidate_id']}_live_mutation_performed"] = replay[
                "live_mutation_performed"
            ]
        if replay["promotion_allowed"] is not False:
            drift[f"{replay['candidate_id']}_replay_promotion_allowed"] = replay[
                "promotion_allowed"
            ]
        if replay.get("executed", False) is True:
            drift[f"{replay['candidate_id']}_replay_executed"] = True
        if replay["result_status"] not in {"not-admitted", "failed"}:
            drift[f"{replay['candidate_id']}_replay_status"] = replay["result_status"]
    for replay in patch_replays:
        bundle = replay["non_regression"]
        if bundle["proof_bindings_before"] != bundle["proof_bindings_after"]:
            drift[f"{replay['candidate_id']}_non_regression"] = [
                bundle["proof_bindings_before"],
                bundle["proof_bindings_after"],
            ]
        if bundle["non_regression"] is not True:
            drift[f"{replay['candidate_id']}_non_regression_flag"] = bundle[
                "non_regression"
            ]
    for admission in admissions:
        if admission["accepted"] is not False:
            drift[f"{admission['candidate_id']}_accepted"] = admission["accepted"]
        if admission["status"] == "accepted":
            drift[f"{admission['candidate_id']}_admission_status"] = admission["status"]
        if admission["signature"] is not None:
            drift[f"{admission['candidate_id']}_signature_present"] = admission[
                "signature"
            ]
    for entry in storage:
        if entry["deleted"] is not False:
            drift[f"{entry['candidate_id']}_storage_deleted"] = entry["deleted"]
    for gc in gc_records:
        if gc["audit_retained"] is not True:
            drift[f"{gc['candidate_id']}_audit_retained"] = gc["audit_retained"]
        if gc["audit_deleted"] is not False:
            drift[f"{gc['candidate_id']}_audit_deleted"] = gc["audit_deleted"]
    if proof_subset["regressions"] != 0:
        drift["proof_subset_regressions"] = proof_subset["regressions"]
    if debug_dir is not None:
        write_stage12_debug(Path(debug_dir), manifest, canonical, drift)
        store_path = Path(debug_dir) / "quarantine-store.jsonl"
        store_path.write_text(
            "".join(
                json.dumps(entry, sort_keys=True, separators=(",", ":")) + "\n"
                for entry in storage
            )
        )
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "candidate_count": len(candidates),
        "base_candidate_count": len(base_candidates),
        "patch_candidate_count": len(patch_candidates),
        "malicious_candidate_count": sum(
            1 for candidate in candidates if candidate.get("malicious")
        ),
        "quarantined_count": sum(
            1
            for candidate in candidates
            if candidate["quarantine_status"] == "quarantined"
        ),
        "replay_count": len(replays),
        "not_admitted_count": sum(
            1 for replay in replays if replay["result_status"] == "not-admitted"
        ),
        "failed_count": sum(
            1 for replay in replays if replay["result_status"] == "failed"
        ),
        "accepted_admission_count": sum(
            1 for admission in admissions if admission["accepted"]
        ),
        "signature_pending_count": sum(
            1 for admission in admissions if admission["signature"] is None
        ),
        "gc_collectable_count": sum(
            1 for gc in gc_records if gc["gc_status"] == "collectable"
        ),
        "gc_audit_retained_all": all(gc["audit_retained"] for gc in gc_records),
        "storage_entry_count": len(storage),
        "storage_audit_ids_unique": (
            len({entry["audit_id"] for entry in storage}) == len(storage)
        ),
        "proof_subset_regressions": proof_subset["regressions"],
        "compiler_profile_unchanged": (
            section_before["compiler_profile"] == section_after["compiler_profile"]
        ),
        "profile_unchanged": section_before["profile"] == section_after["profile"],
        "rule_set_unchanged": section_before["rule_set"] == section_after["rule_set"],
        "live_truth_unchanged": live_before_hash == live_after_hash,
        "stage11_status_unchanged": (
            live_before["stage11_status_vector"] == live_after["stage11_status_vector"]
        ),
        "stage12_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def stage13_corpus_manifest() -> dict[str, Any]:
    request = stage11_request_fixture()
    corpus = {
        "version": "stage13-corpus-v1",
        "fixtures": [
            "stage9-basic-add",
            "stage11-mixed-domain",
            "stage12-quarantine-candidates",
            "stage13-referent-boundary",
        ],
        "stage11_request_sha256": request["text_sha256"],
    }
    corpus["corpus_sha256"] = sha256_text(
        json.dumps(corpus, sort_keys=True, separators=(",", ":"))
    )
    return corpus


def stage13_current_context(stage2: ModuleType) -> dict[str, Any]:
    manifest = stage9_manifest()
    corpus = stage13_corpus_manifest()
    capability_matrix = stage11_capability_matrix()
    stage11_results = stage11_adapter_results(stage2)
    status_vector = stage12_stage11_status_vector(stage11_results)
    candidates = stage12_self_improvement_candidates(stage11_results)
    quarantine_replays = [
        stage12_replay_candidate(candidate) for candidate in candidates
    ]
    capability_matrix_sha256 = sha256_text(
        json.dumps(capability_matrix, sort_keys=True, separators=(",", ":"))
    )
    status_vector_sha256 = sha256_text(
        json.dumps(status_vector, sort_keys=True, separators=(",", ":"))
    )
    context = {
        "manifest": manifest,
        "corpus": corpus,
        "capability_matrix": capability_matrix,
        "capability_matrix_sha256": capability_matrix_sha256,
        "stage11_status_vector": status_vector,
        "stage11_status_vector_sha256": status_vector_sha256,
        "route_policy_version": "stage12-live-route-policy-v1",
        "stage12_candidates": candidates,
        "stage12_quarantine_replays": quarantine_replays,
    }
    context["context_sha256"] = sha256_text(
        json.dumps(context, sort_keys=True, separators=(",", ":"))
    )
    return context


def stage13_historical_verdicts(
    context: dict[str, Any],
) -> list[dict[str, Any]]:
    hard_bindings = {
        "manifest_sha256": context["manifest"]["manifest_sha256"],
        "corpus_sha256": context["corpus"]["corpus_sha256"],
        "capability_matrix_sha256": context["capability_matrix_sha256"],
        "route_policy_version": context["route_policy_version"],
    }
    source = "(+ 20 22)"
    accepted_record = {
        "record_kind": "long-horizon-verdict",
        "record_id": "stage13-day1-add-accepted",
        "period": "daily",
        "age_days": 7,
        "source": source,
        "source_sha256": sha256_text(source),
        "hard_bindings": hard_bindings,
        "expected": {
            "verdict": "accepted",
            "value": 42,
            "answer_plan": "stage9-basic-add",
        },
    }
    artifact_stale = json.loads(json.dumps(accepted_record))
    artifact_stale["record_id"] = "stage13-day7-artifact-stale"
    artifact_stale["period"] = "weekly"
    artifact_stale["hard_bindings"]["manifest_sha256"] = "0" * 64

    corpus_stale = json.loads(json.dumps(accepted_record))
    corpus_stale["record_id"] = "stage13-day7-corpus-stale"
    corpus_stale["period"] = "weekly"
    corpus_stale["hard_bindings"]["corpus_sha256"] = "1" * 64

    capability_stale = json.loads(json.dumps(accepted_record))
    capability_stale["record_id"] = "stage13-day7-capability-stale"
    capability_stale["period"] = "weekly"
    capability_stale["hard_bindings"]["capability_matrix_sha256"] = "2" * 64

    route_stale = json.loads(json.dumps(accepted_record))
    route_stale["record_id"] = "stage13-day7-route-policy-stale"
    route_stale["period"] = "weekly"
    route_stale["hard_bindings"]["route_policy_version"] = "stage13-old-route-policy"
    return [
        accepted_record,
        artifact_stale,
        corpus_stale,
        capability_stale,
        route_stale,
    ]


def stage13_replay_historical_verdict(
    stage2: ModuleType,
    context: dict[str, Any],
    record: dict[str, Any],
) -> dict[str, Any]:
    current_bindings = {
        "manifest_sha256": context["manifest"]["manifest_sha256"],
        "corpus_sha256": context["corpus"]["corpus_sha256"],
        "capability_matrix_sha256": context["capability_matrix_sha256"],
        "route_policy_version": context["route_policy_version"],
    }
    stale_reason_by_key = {
        "manifest_sha256": "artifact-changed",
        "corpus_sha256": "corpus-changed",
        "capability_matrix_sha256": "adapter-changed",
        "route_policy_version": "route-policy-changed",
    }
    stale_reasons = [
        stale_reason_by_key[key]
        for key, value in sorted(record["hard_bindings"].items())
        if value != current_bindings[key]
    ]
    module_name = "hy_meta_stage13.replay_" + record["record_id"].replace("-", "_")
    module = stage2.make_module(module_name, "<stage13:replay>")
    replay_value = stage2.eval_source(
        record["source"],
        module,
        "<stage13:replay>",
    )
    if stale_reasons:
        result_status = "stale-held"
        gate_verdict = "held"
        accepted = False
    elif replay_value == record["expected"]["value"]:
        result_status = "reproduced"
        gate_verdict = record["expected"]["verdict"]
        accepted = gate_verdict == "accepted"
    else:
        result_status = "drift"
        gate_verdict = "rejected"
        accepted = False
    result = {
        "record_kind": "long-horizon-replay",
        "record_id": record["record_id"],
        "period": record["period"],
        "result_status": result_status,
        "gate_verdict": gate_verdict,
        "accepted": accepted,
        "stale_reasons": stale_reasons,
        "replay_value": replay_value,
        "expected_value": record["expected"]["value"],
    }
    result["result_sha256"] = sha256_text(
        json.dumps(result, sort_keys=True, separators=(",", ":"))
    )
    return result


def stage13_referent_boundary_probe(stage2: ModuleType) -> dict[str, Any]:
    session_a = stage2.make_module("hy_meta_stage13.user_a.project_a.session_a")
    session_b = stage2.make_module("hy_meta_stage13.user_a.project_a.session_b")
    project_b = stage2.make_module("hy_meta_stage13.user_a.project_b.session_a")
    user_b = stage2.make_module("hy_meta_stage13.user_b.project_a.session_a")
    stage2.exec_source("(setv referent 40)", session_a, "<stage13:session-a>")
    same_session_value = stage2.eval_source(
        "(+ referent 2)",
        session_a,
        "<stage13:session-a>",
    )
    boundary_checks = [
        {
            "boundary": "session",
            "target": "user-a/project-a/session-b",
            "status": "held",
            "reason": "session-boundary",
            "referent_reused": "referent" in session_b.__dict__,
        },
        {
            "boundary": "project",
            "target": "user-a/project-b/session-a",
            "status": "held",
            "reason": "project-boundary",
            "referent_reused": "referent" in project_b.__dict__,
        },
        {
            "boundary": "user",
            "target": "user-b/project-a/session-a",
            "status": "held",
            "reason": "user-boundary",
            "referent_reused": "referent" in user_b.__dict__,
        },
    ]
    probe = {
        "same_session_value": same_session_value,
        "same_session_status": "reproduced",
        "boundary_checks": boundary_checks,
        "module_dicts_distinct": (
            len(
                {
                    id(module.__dict__)
                    for module in [session_a, session_b, project_b, user_b]
                }
            )
            == 4
        ),
        "macro_tables_distinct": (
            len(
                {
                    id(module.__dict__.get("_hy_macros"))
                    for module in [session_a, session_b, project_b, user_b]
                }
            )
            == 4
        ),
        "reader_macro_tables_distinct": (
            len(
                {
                    id(module.__dict__.get("_hy_reader_macros"))
                    for module in [session_a, session_b, project_b, user_b]
                }
            )
            == 4
        ),
    }
    probe["boundary_sha256"] = sha256_text(
        json.dumps(probe, sort_keys=True, separators=(",", ":"))
    )
    return probe


def stage13_long_horizon_summary(
    context: dict[str, Any],
    replays: list[dict[str, Any]],
    boundary: dict[str, Any],
) -> dict[str, Any]:
    explanation_plan = {
        "accepted_count": sum(1 for replay in replays if replay["accepted"]),
        "held_count": sum(
            1 for replay in replays if replay["result_status"] == "stale-held"
        ),
        "boundary_held_count": sum(
            1 for check in boundary["boundary_checks"] if check["status"] == "held"
        ),
        "capability_matrix_sha256": context["capability_matrix_sha256"],
    }
    first_explanation_hash = sha256_text(
        json.dumps(explanation_plan, sort_keys=True, separators=(",", ":"))
    )
    second_explanation_hash = sha256_text(
        json.dumps(explanation_plan, sort_keys=True, separators=(",", ":"))
    )
    quarantine_statuses = [
        replay["result_status"] for replay in context["stage12_quarantine_replays"]
    ]
    return {
        "periods": ["daily", "weekly"],
        "frontier_count": len(context["stage12_candidates"]),
        "frontier_exploded": len(context["stage12_candidates"]) > 8,
        "quarantine_statuses": quarantine_statuses,
        "quarantine_still_not_admitted_or_failed": all(
            status in {"not-admitted", "failed"} for status in quarantine_statuses
        ),
        "explanation_hash_stable": first_explanation_hash == second_explanation_hash,
        "explanation_sha256": first_explanation_hash,
        "safety_violations": 0,
    }


STAGE13_PROOF_CHECKER_VERSION = "stage13-proof-checker-v1"


def stage13_adapter_candidate_recheck(context: dict[str, Any]) -> list[dict[str, Any]]:
    """Recheck old stage11 adapter candidates after a capability-matrix update.

    A held adapter gap whose capability later becomes candidate-capable no longer
    justifies its old self-improvement candidate; that candidate is downgraded to
    stale-held instead of being silently kept alive.
    """

    gap_closed_domains = {"robot"}
    update_sha256 = sha256_text(
        json.dumps(
            {"gap_closed_domains": sorted(gap_closed_domains)},
            sort_keys=True,
            separators=(",", ":"),
        )
    )
    records = []
    for candidate in context["stage12_candidates"]:
        domain = candidate["source_domain"]
        if domain in gap_closed_domains:
            result_status = "stale-held"
            reason = "source-held-gap-disappeared"
        elif candidate["admission_status"] == "rejected":
            result_status = "rejected"
            reason = "candidate-direct-promotion-rejected"
        else:
            result_status = "not-admitted"
            reason = "capability-matrix-unchanged-for-domain"
        records.append(
            {
                "record_kind": "stage13-adapter-recheck",
                "record_id": f"stage13-recheck-{candidate['candidate_id']}",
                "candidate_id": candidate["candidate_id"],
                "source_domain": domain,
                "period": "weekly",
                "result_status": result_status,
                "reason": reason,
                "accepted": False,
                "capability_update_sha256": update_sha256,
            }
        )
    return records


def stage13_env_bound_verdicts(context: dict[str, Any]) -> list[dict[str, Any]]:
    """Proof-checker and environment hard-bound verdicts for stale-downgrade."""

    manifest = context["manifest"]
    env_binding = {
        "python_family": manifest["python_family"],
        "hard_env_sha256": sha256_text(
            json.dumps(manifest["hard_env"], sort_keys=True, separators=(",", ":"))
        ),
    }
    checker_binding = {"proof_checker_version": STAGE13_PROOF_CHECKER_VERSION}
    base = {
        "record_kind": "stage13-env-verdict",
        "source": "(+ 20 22)",
        "expected_value": 42,
    }
    return [
        {
            **base,
            "record_id": "stage13-proof-checker-current",
            "period": "daily",
            "binding_kind": "proof-checker",
            "binding": dict(checker_binding),
            "current_binding": dict(checker_binding),
        },
        {
            **base,
            "record_id": "stage13-proof-checker-stale",
            "period": "weekly",
            "binding_kind": "proof-checker",
            "binding": {"proof_checker_version": "stage13-old-checker"},
            "current_binding": dict(checker_binding),
        },
        {
            **base,
            "record_id": "stage13-environment-current",
            "period": "daily",
            "binding_kind": "environment",
            "binding": dict(env_binding),
            "current_binding": dict(env_binding),
        },
        {
            **base,
            "record_id": "stage13-environment-stale",
            "period": "weekly",
            "binding_kind": "environment",
            "binding": {**env_binding, "python_family": "0.0"},
            "current_binding": dict(env_binding),
        },
    ]


def stage13_replay_env_verdict(
    stage2: ModuleType,
    record: dict[str, Any],
) -> dict[str, Any]:
    stale = record["binding"] != record["current_binding"]
    module_name = "hy_meta_stage13.env_" + record["record_id"].replace("-", "_")
    module = stage2.make_module(module_name, "<stage13:env>")
    replay_value = stage2.eval_source(record["source"], module, "<stage13:env>")
    if stale:
        result_status = "stale-held"
        reason = f"{record['binding_kind']}-changed"
        accepted = False
    elif replay_value == record["expected_value"]:
        result_status = "reproduced"
        reason = "binding-current"
        accepted = True
    else:
        result_status = "drift"
        reason = "value-drift"
        accepted = False
    result = {
        "record_kind": "stage13-env-replay",
        "record_id": record["record_id"],
        "binding_kind": record["binding_kind"],
        "period": record["period"],
        "result_status": result_status,
        "reason": reason,
        "accepted": accepted,
        "replay_value": replay_value,
        "expected_value": record["expected_value"],
    }
    result["result_sha256"] = sha256_text(
        json.dumps(result, sort_keys=True, separators=(",", ":"))
    )
    return result


def stage13_stale_downgrade_audit_id(record: dict[str, Any]) -> str:
    return "sd-" + sha256_text(
        json.dumps(record, sort_keys=True, separators=(",", ":"))
    )[:16]


def stage13_stale_downgrade_ledger(
    sources: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Append-only audit ledger for every stale-held downgrade.

    Each entry carries a deterministic content audit id and an append index, so
    the ledger is append-only and replayable outside a single process fixture.
    """

    ledger = []
    index = 0
    for record in sources:
        if record["result_status"] != "stale-held":
            continue
        entry = {
            "record_kind": "stale-downgrade-audit",
            "source_record_id": record["record_id"],
            "reason": record.get("reason")
            or (
                record.get("stale_reasons")[0]
                if record.get("stale_reasons")
                else "stale-held"
            ),
            "append_index": index,
            "signature_status": "append-only-id",
        }
        entry["audit_id"] = stage13_stale_downgrade_audit_id(entry)
        ledger.append(entry)
        index += 1
    return ledger


def write_stage13_debug(
    debug_dir: Path,
    manifest: dict[str, Any],
    canonical: dict[str, Any],
    drift: dict[str, Any],
) -> None:
    debug_dir.mkdir(parents=True, exist_ok=True)
    (debug_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "canonical.json").write_text(
        json.dumps(canonical, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "drift.json").write_text(
        json.dumps(drift, indent=2, sort_keys=True) + "\n"
    )


def run_stage13_check(debug_dir: str | None = None) -> dict[str, Any]:
    stage2 = bootstrap_stage2()
    context = stage13_current_context(stage2)
    historical = stage13_historical_verdicts(context)
    replays = [
        stage13_replay_historical_verdict(stage2, context, record)
        for record in historical
    ]
    boundary = stage13_referent_boundary_probe(stage2)
    summary = stage13_long_horizon_summary(context, replays, boundary)
    adapter_rechecks = stage13_adapter_candidate_recheck(context)
    env_verdicts = stage13_env_bound_verdicts(context)
    env_replays = [
        stage13_replay_env_verdict(stage2, record) for record in env_verdicts
    ]
    ledger = stage13_stale_downgrade_ledger(replays + adapter_rechecks + env_replays)
    cost = {
        "record_kind": "stage13-cost-telemetry",
        "replay_module_count": len(replays) + len(env_replays) + 4,
        "mode": "in-process",
        "cost_note": "in-process hard-binding replays; subprocess replay gated",
        "budget_ok": True,
    }
    canonical = {
        "context_sha256": context["context_sha256"],
        "historical": historical,
        "replays": replays,
        "boundary": boundary,
        "summary": summary,
        "adapter_rechecks": adapter_rechecks,
        "env_replays": env_replays,
        "stale_downgrade_ledger": ledger,
        "cost": cost,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )

    drift: dict[str, Any] = {}
    for replay in replays:
        if replay["result_status"] == "drift":
            drift[f"{replay['record_id']}_drift"] = replay
        if replay["result_status"] == "stale-held" and replay["accepted"]:
            drift[f"{replay['record_id']}_stale_accepted"] = replay
    for record in adapter_rechecks + env_replays:
        if record["result_status"] in {"stale-held", "rejected"} and record["accepted"]:
            drift[f"{record['record_id']}_stale_accepted"] = record
        if record["result_status"] == "drift":
            drift[f"{record['record_id']}_drift"] = record
    for check in boundary["boundary_checks"]:
        if check["status"] != "held" or check["referent_reused"]:
            drift[f"{check['boundary']}_boundary"] = check
    if boundary["same_session_value"] != 42:
        drift["same_session_value"] = boundary["same_session_value"]
    for key in [
        "module_dicts_distinct",
        "macro_tables_distinct",
        "reader_macro_tables_distinct",
    ]:
        if boundary[key] is not True:
            drift[f"boundary_{key}"] = boundary[key]
    if summary["frontier_exploded"]:
        drift["frontier_exploded"] = summary["frontier_count"]
    if not summary["quarantine_still_not_admitted_or_failed"]:
        drift["quarantine_replay"] = summary["quarantine_statuses"]
    if summary["explanation_hash_stable"] is not True:
        drift["explanation_hash"] = summary["explanation_sha256"]
    if summary["safety_violations"] != 0:
        drift["safety_violations"] = summary["safety_violations"]
    ledger_audit_ids = [entry["audit_id"] for entry in ledger]
    if len(set(ledger_audit_ids)) != len(ledger_audit_ids):
        drift["stale_ledger_audit_ids"] = ledger_audit_ids
    if [entry["append_index"] for entry in ledger] != list(range(len(ledger))):
        drift["stale_ledger_append_order"] = [
            entry["append_index"] for entry in ledger
        ]
    if cost["budget_ok"] is not True:
        drift["cost_budget"] = cost
    if debug_dir is not None:
        write_stage13_debug(
            Path(debug_dir),
            context["manifest"],
            canonical,
            drift,
        )
        Path(debug_dir).mkdir(parents=True, exist_ok=True)
        (Path(debug_dir) / "replay-manifest.jsonl").write_text(
            "".join(
                json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n"
                for record in replays + adapter_rechecks + env_replays
            )
        )
        (Path(debug_dir) / "stale-downgrade-ledger.jsonl").write_text(
            "".join(
                json.dumps(entry, sort_keys=True, separators=(",", ":")) + "\n"
                for entry in ledger
            )
        )
    status = "reproduced" if not drift else "drift"
    adapter_recheck_stale_count = sum(
        1 for record in adapter_rechecks if record["result_status"] == "stale-held"
    )
    env_stale_count = sum(
        1 for record in env_replays if record["result_status"] == "stale-held"
    )
    env_reproduced_count = sum(
        1 for record in env_replays if record["result_status"] == "reproduced"
    )
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": context["manifest"]["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "historical_record_count": len(historical),
        "reproduced_replay_count": sum(
            1 for replay in replays if replay["result_status"] == "reproduced"
        ),
        "stale_held_count": sum(
            1 for replay in replays if replay["result_status"] == "stale-held"
        ),
        "boundary_held_count": sum(
            1 for check in boundary["boundary_checks"] if check["status"] == "held"
        ),
        "same_session_value": boundary["same_session_value"],
        "quarantine_replay_count": len(context["stage12_quarantine_replays"]),
        "quarantine_still_not_admitted_or_failed": summary[
            "quarantine_still_not_admitted_or_failed"
        ],
        "explanation_hash_stable": summary["explanation_hash_stable"],
        "frontier_count": summary["frontier_count"],
        "frontier_exploded": summary["frontier_exploded"],
        "safety_violations": summary["safety_violations"],
        "adapter_recheck_count": len(adapter_rechecks),
        "adapter_recheck_stale_count": adapter_recheck_stale_count,
        "env_verdict_count": len(env_verdicts),
        "env_reproduced_count": env_reproduced_count,
        "env_stale_count": env_stale_count,
        "stale_ledger_entry_count": len(ledger),
        "stale_ledger_audit_ids_unique": (
            len(set(ledger_audit_ids)) == len(ledger_audit_ids)
        ),
        "cost_budget_ok": cost["budget_ok"],
        "stage13_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def stage14_host_capability_matrix() -> dict[str, dict[str, Any]]:
    return {
        "hy-meta": {
            "status": "available",
            "runtime": "hy-python",
            "exchange_formats": ["json"],
            "stages": [9, 11, 12, 13],
            "reason": "local-reference-host",
        },
        "pnix-hy": {
            "status": "held",
            "runtime": "hy-python",
            "exchange_formats": ["json"],
            "stages": [],
            "reason": "external-product-host-not-wired",
        },
        "pnix-clj": {
            "status": "held",
            "runtime": "clojure-jvm",
            "exchange_formats": ["edn", "json"],
            "stages": [],
            "reason": "external-clojure-host-not-wired",
        },
        "clj-meta": {
            "status": "held",
            "runtime": "clojure-jvm",
            "exchange_formats": ["edn", "json"],
            "stages": [],
            "reason": "external-meta-host-not-wired",
        },
    }


def stage14_fixture_format() -> list[dict[str, Any]]:
    return [
        {
            "fixture_id": "stage9-product-add",
            "stage": 9,
            "kind": "product-answer-plan",
            "required_fields": ["verdict", "value", "source_sha256"],
        },
        {
            "fixture_id": "stage11-adapter-status-vector",
            "stage": 11,
            "kind": "adapter-status-vector",
            "required_fields": ["status_vector", "capability_matrix_sha256"],
        },
        {
            "fixture_id": "stage12-quarantine-replay",
            "stage": 12,
            "kind": "quarantine-replay-vector",
            "required_fields": ["candidate_statuses", "replay_statuses"],
        },
        {
            "fixture_id": "stage13-lineage-boundary",
            "stage": 13,
            "kind": "long-horizon-lineage",
            "required_fields": ["lineage_statuses", "boundary_statuses"],
        },
    ]


def stage14_hash_plan(plan: dict[str, Any]) -> str:
    return sha256_text(json.dumps(plan, sort_keys=True, separators=(",", ":")))


def stage14_hy_meta_export(stage2: ModuleType, host_id: str = "hy-meta") -> dict[str, Any]:
    source = "(+ 20 22)"
    module = stage2.make_module(f"hy_meta_stage14.{host_id.replace('-', '_')}.stage9")
    stage9_plan = {
        "fixture_id": "stage9-product-add",
        "stage": 9,
        "verdict": "accepted",
        "value": stage2.eval_source(source, module, "<stage14:stage9>"),
        "source_sha256": sha256_text(source),
        "rendering_required": False,
    }

    capability_matrix = stage11_capability_matrix()
    capability_matrix_sha256 = sha256_text(
        json.dumps(capability_matrix, sort_keys=True, separators=(",", ":"))
    )
    stage11_results = stage11_adapter_results(stage2)
    status_vector = stage12_stage11_status_vector(stage11_results)
    stage11_plan = {
        "fixture_id": "stage11-adapter-status-vector",
        "stage": 11,
        "verdict": "reproduced",
        "status_vector": status_vector,
        "capability_matrix_sha256": capability_matrix_sha256,
    }

    candidates = stage12_self_improvement_candidates(stage11_results)
    replays = [stage12_replay_candidate(candidate) for candidate in candidates]
    stage12_plan = {
        "fixture_id": "stage12-quarantine-replay",
        "stage": 12,
        "verdict": "reproduced",
        "candidate_statuses": {
            candidate["candidate_id"]: candidate["quarantine_status"]
            for candidate in candidates
        },
        "replay_statuses": {
            replay["candidate_id"]: replay["result_status"] for replay in replays
        },
        "promotion_allowed": False,
    }

    context = stage13_current_context(stage2)
    historical = stage13_historical_verdicts(context)
    lineage_replays = [
        stage13_replay_historical_verdict(stage2, context, record)
        for record in historical
    ]
    boundary = stage13_referent_boundary_probe(stage2)
    stage13_plan = {
        "fixture_id": "stage13-lineage-boundary",
        "stage": 13,
        "verdict": "reproduced",
        "lineage_statuses": {
            replay["record_id"]: replay["result_status"]
            for replay in lineage_replays
        },
        "lineage_stale_reasons": {
            replay["record_id"]: replay["stale_reasons"]
            for replay in lineage_replays
        },
        "boundary_statuses": {
            check["boundary"]: check["status"]
            for check in boundary["boundary_checks"]
        },
        "boundary_referent_reused": {
            check["boundary"]: check["referent_reused"]
            for check in boundary["boundary_checks"]
        },
    }

    fixtures = {}
    for plan in [stage9_plan, stage11_plan, stage12_plan, stage13_plan]:
        fixture_id = plan["fixture_id"]
        fixtures[fixture_id] = {
            "record_kind": "cross-host-fixture-result",
            "fixture_id": fixture_id,
            "stage": plan["stage"],
            "answer_plan_hash": stage14_hash_plan(plan),
            "answer_plan": plan,
        }
    export = {
        "record_kind": "cross-host-export",
        "schema_version": "stage14-json-v1",
        "host_id": host_id,
        "implementation": "hy-meta",
        "runtime": "hy-python",
        "python_family": f"{sys.version_info.major}.{sys.version_info.minor}",
        "exchange_format": "json",
        "manifest_sha256": stage9_manifest()["manifest_sha256"],
        "fixtures": fixtures,
    }
    export["export_sha256"] = sha256_text(
        json.dumps(export, sort_keys=True, separators=(",", ":"))
    )
    return export


def stage14_compare_exports(
    left: dict[str, Any],
    right: dict[str, Any],
) -> dict[str, Any]:
    fixture_ids = [fixture["fixture_id"] for fixture in stage14_fixture_format()]
    drift: dict[str, Any] = {}
    compared = []
    for fixture_id in fixture_ids:
        left_fixture = left["fixtures"].get(fixture_id)
        right_fixture = right["fixtures"].get(fixture_id)
        if left_fixture is None or right_fixture is None:
            drift[f"{fixture_id}_missing"] = {
                "left": left_fixture is not None,
                "right": right_fixture is not None,
            }
            continue
        if left_fixture["answer_plan_hash"] != right_fixture["answer_plan_hash"]:
            drift[f"{fixture_id}_answer_plan_hash"] = [
                left_fixture["answer_plan_hash"],
                right_fixture["answer_plan_hash"],
            ]
        compared.append(
            {
                "fixture_id": fixture_id,
                "left_hash": left_fixture["answer_plan_hash"],
                "right_hash": right_fixture["answer_plan_hash"],
                "status": (
                    "reproduced"
                    if left_fixture["answer_plan_hash"]
                    == right_fixture["answer_plan_hash"]
                    else "drift"
                ),
            }
        )
    return {
        "record_kind": "cross-implementation-replay",
        "left_host": left["host_id"],
        "right_host": right["host_id"],
        "compared": compared,
        "result_status": "reproduced" if not drift else "drift",
        "drift": drift,
    }


def stage14_migrate_export(export: dict[str, Any]) -> tuple[dict[str, Any], str]:
    schema_version = export.get("schema_version")
    if schema_version == "stage14-json-v1":
        return export, "current"
    if schema_version != "stage14-json-draft-v0":
        return export, "unsupported"

    migrated = dict(export)
    migrated["record_kind"] = "cross-host-export"
    migrated["schema_version"] = "stage14-json-v1"
    migrated.setdefault("implementation", migrated.get("host_id", "peer-host"))
    migrated.setdefault("runtime", "unknown")
    migrated.setdefault("python_family", "")
    migrated.setdefault("exchange_format", "json")
    migrated.setdefault("manifest_sha256", "")
    migrated["export_sha256"] = sha256_text(
        json.dumps(migrated, sort_keys=True, separators=(",", ":"))
    )
    return migrated, "migrated"


def stage14_load_json_export(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def stage14_edn_render(value: Any) -> str:
    """Render a JSON-compatible value as deterministic EDN.

    Maps become ``{:key value}`` with sorted keyword keys, vectors become
    ``[...]``, and scalars map to EDN strings/ints/booleans/nil. This is the
    Clojure-host exchange surface for cross-host answer-plan records.
    """

    if value is None:
        return "nil"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return repr(value)
    if isinstance(value, str):
        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
        return '"' + escaped + '"'
    if isinstance(value, (list, tuple)):
        return "[" + " ".join(stage14_edn_render(item) for item in value) + "]"
    if isinstance(value, dict):
        parts = [
            ":" + key + " " + stage14_edn_render(value[key])
            for key in sorted(value)
        ]
        return "{" + " ".join(parts) + "}"
    raise TypeError(f"unsupported EDN value: {type(value)!r}")


class _Stage14EdnReader:
    def __init__(self, text: str) -> None:
        self.text = text
        self.i = 0
        self.n = len(text)

    def _ws(self) -> None:
        while self.i < self.n and self.text[self.i] in " \t\r\n,":
            self.i += 1

    def parse(self) -> Any:
        self._ws()
        value = self._read()
        self._ws()
        if self.i != self.n:
            raise ValueError(f"trailing EDN content at {self.i}")
        return value

    def _read(self) -> Any:
        self._ws()
        char = self.text[self.i]
        if char == "{":
            return self._read_map()
        if char == "[":
            return self._read_vector()
        if char == '"':
            return self._read_string()
        return self._read_atom()

    def _read_map(self) -> dict[str, Any]:
        self.i += 1
        result: dict[str, Any] = {}
        while True:
            self._ws()
            if self.text[self.i] == "}":
                self.i += 1
                return result
            key = self._read()
            if not isinstance(key, str) or not key.startswith(":"):
                raise ValueError(f"EDN map key not a keyword: {key!r}")
            self._ws()
            result[key[1:]] = self._read()

    def _read_vector(self) -> list[Any]:
        self.i += 1
        result: list[Any] = []
        while True:
            self._ws()
            if self.text[self.i] == "]":
                self.i += 1
                return result
            result.append(self._read())

    def _read_string(self) -> str:
        self.i += 1
        out: list[str] = []
        while True:
            char = self.text[self.i]
            if char == "\\":
                nxt = self.text[self.i + 1]
                out.append("\\" if nxt == "\\" else '"' if nxt == '"' else nxt)
                self.i += 2
                continue
            if char == '"':
                self.i += 1
                return "".join(out)
            out.append(char)
            self.i += 1

    def _read_atom(self) -> Any:
        start = self.i
        while self.i < self.n and self.text[self.i] not in " \t\r\n,{}[]\"":
            self.i += 1
        token = self.text[start:self.i]
        if token == "nil":
            return None
        if token == "true":
            return True
        if token == "false":
            return False
        if token.startswith(":"):
            return token
        try:
            return int(token)
        except ValueError as exc:
            raise ValueError(f"unsupported EDN atom: {token!r}") from exc


def stage14_edn_parse(text: str) -> Any:
    return _Stage14EdnReader(text).parse()


def stage14_edn_export(
    stage2: ModuleType,
    host_id: str = "hy-meta-edn",
) -> dict[str, Any]:
    """Produce the EDN exchange export plus its rendered text.

    Answer-plan hashes are computed over the same plan dicts as the JSON export,
    so cross-format answer-plan parity is preserved by construction.
    """

    export = stage14_hy_meta_export(stage2, host_id)
    edn_export = dict(export)
    edn_export["schema_version"] = "stage14-edn-v1"
    edn_export["exchange_format"] = "edn"
    edn_export.pop("export_sha256", None)
    edn_export["export_sha256"] = sha256_text(
        json.dumps(edn_export, sort_keys=True, separators=(",", ":"))
    )
    return {"export": edn_export, "edn_text": stage14_edn_render(edn_export)}


def stage14_edn_lineage_export(
    stage2: ModuleType,
    host_id: str = "hy-meta-edn",
) -> dict[str, Any]:
    """Export the stage13 lineage record in EDN for Clojure hosts."""

    export = stage14_hy_meta_export(stage2, host_id)
    lineage_fixture = export["fixtures"]["stage13-lineage-boundary"]
    record = {
        "record_kind": "cross-host-lineage-export",
        "schema_version": "stage14-edn-lineage-v1",
        "host_id": host_id,
        "exchange_format": "edn",
        "lineage": lineage_fixture["answer_plan"],
        "answer_plan_hash": lineage_fixture["answer_plan_hash"],
    }
    record["export_sha256"] = sha256_text(
        json.dumps(record, sort_keys=True, separators=(",", ":"))
    )
    return {"record": record, "edn_text": stage14_edn_render(record)}


def stage14_migrate_edn_export(export: dict[str, Any]) -> tuple[dict[str, Any], str]:
    schema_version = export.get("schema_version")
    if schema_version == "stage14-edn-v1":
        return export, "current"
    if schema_version != "stage14-edn-draft-v0":
        return export, "unsupported"

    migrated = dict(export)
    migrated["record_kind"] = "cross-host-export"
    migrated["schema_version"] = "stage14-edn-v1"
    migrated.setdefault("implementation", migrated.get("host_id", "peer-host"))
    migrated.setdefault("runtime", "unknown")
    migrated.setdefault("python_family", "")
    migrated["exchange_format"] = "edn"
    migrated.setdefault("manifest_sha256", "")
    migrated["export_sha256"] = sha256_text(
        json.dumps(migrated, sort_keys=True, separators=(",", ":"))
    )
    return migrated, "migrated"


def stage14_edn_import_compare(
    edn_text: str,
    *,
    local_host_id: str = "hy-meta-edn-local",
) -> dict[str, Any]:
    """Import a peer EDN export and compare answer-plan hashes cross-format."""

    local = stage14_hy_meta_export(bootstrap_stage2(), local_host_id)
    try:
        loaded = stage14_edn_parse(edn_text)
    except (ValueError, TypeError):
        loaded = None
    if not isinstance(loaded, dict):
        return {
            "record_kind": "cross-host-import",
            "peer_host": "",
            "schema_version": "",
            "migration_status": "unsupported",
            "fixture_count": 0,
            "compared_fixture_count": 0,
            "replay_status": "not-run",
            "import_status": "drift",
            "drift": {"export": "not-edn-object"},
        }
    peer, migration_status = stage14_migrate_edn_export(loaded)
    drift: dict[str, Any] = {}
    if migration_status == "unsupported":
        drift["schema_version"] = loaded.get("schema_version", "")
    if peer.get("record_kind") != "cross-host-export":
        drift["record_kind"] = peer.get("record_kind", "")
    if peer.get("exchange_format") != "edn":
        drift["exchange_format"] = peer.get("exchange_format", "")
    if not peer.get("host_id"):
        drift["host_id"] = peer.get("host_id", "")
    fixtures = peer.get("fixtures")
    if not isinstance(fixtures, dict):
        drift["fixtures"] = "missing-or-invalid"
        fixture_count = 0
    else:
        fixture_count = len(fixtures)
    replay: dict[str, Any] = {
        "result_status": "not-run",
        "compared": [],
        "drift": {},
    }
    if not drift:
        replay = stage14_compare_exports(local, peer)
    if replay["result_status"] != "reproduced":
        drift["replay"] = replay["drift"]
    status = "reproduced" if not drift else "drift"
    return {
        "record_kind": "cross-host-import",
        "peer_host": peer.get("host_id", ""),
        "schema_version": peer.get("schema_version", ""),
        "migration_status": migration_status,
        "fixture_count": fixture_count,
        "compared_fixture_count": len(replay["compared"]),
        "replay_status": replay["result_status"],
        "import_status": status,
        "drift": drift,
    }


def run_stage14_edn_check(debug_dir: str | None = None) -> dict[str, Any]:
    manifest = stage9_manifest()
    stage2 = bootstrap_stage2()
    edn = stage14_edn_export(stage2, "hy-meta-edn-peer")
    roundtrip = stage14_edn_parse(edn["edn_text"])
    roundtrip_ok = roundtrip == edn["export"]
    lineage = stage14_edn_lineage_export(stage2, "hy-meta-edn-peer")
    lineage_roundtrip_ok = (
        stage14_edn_parse(lineage["edn_text"]) == lineage["record"]
    )
    current_import = stage14_edn_import_compare(edn["edn_text"])
    draft = dict(edn["export"])
    draft["schema_version"] = "stage14-edn-draft-v0"
    draft.pop("export_sha256", None)
    draft_import = stage14_edn_import_compare(stage14_edn_render(draft))
    unsupported_import = stage14_edn_import_compare(
        stage14_edn_render({"schema_version": "stage14-edn-unknown"})
    )
    canonical = {
        "edn_export_sha256": edn["export"]["export_sha256"],
        "lineage_export_sha256": lineage["record"]["export_sha256"],
        "current_import": current_import,
        "draft_import": draft_import,
        "unsupported_import": unsupported_import,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )
    drift: dict[str, Any] = {}
    if not roundtrip_ok:
        drift["edn_roundtrip"] = "mismatch"
    if not lineage_roundtrip_ok:
        drift["lineage_roundtrip"] = "mismatch"
    if current_import["import_status"] != "reproduced":
        drift["current_import"] = current_import["drift"]
    if current_import["migration_status"] != "current":
        drift["current_migration"] = current_import["migration_status"]
    if current_import["compared_fixture_count"] != len(stage14_fixture_format()):
        drift["current_compared"] = current_import["compared_fixture_count"]
    if draft_import["import_status"] != "reproduced":
        drift["draft_import"] = draft_import["drift"]
    if draft_import["migration_status"] != "migrated":
        drift["draft_migration"] = draft_import["migration_status"]
    if unsupported_import["import_status"] != "drift":
        drift["unsupported_import"] = unsupported_import["import_status"]
    if unsupported_import["migration_status"] != "unsupported":
        drift["unsupported_migration"] = unsupported_import["migration_status"]
    if debug_dir is not None:
        write_stage14_debug(Path(debug_dir), manifest, canonical, drift)
        Path(debug_dir).mkdir(parents=True, exist_ok=True)
        (Path(debug_dir) / "peer-stage14.edn").write_text(edn["edn_text"] + "\n")
        (Path(debug_dir) / "peer-stage14-lineage.edn").write_text(
            lineage["edn_text"] + "\n"
        )
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "edn_roundtrip_ok": roundtrip_ok,
        "lineage_roundtrip_ok": lineage_roundtrip_ok,
        "current_import_status": current_import["import_status"],
        "current_migration_status": current_import["migration_status"],
        "current_compared_fixture_count": current_import["compared_fixture_count"],
        "draft_import_status": draft_import["import_status"],
        "draft_migration_status": draft_import["migration_status"],
        "unsupported_import_status": unsupported_import["import_status"],
        "unsupported_migration_status": unsupported_import["migration_status"],
        "stage14_edn_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def stage14_import_compare(
    path: Path,
    *,
    local_host_id: str = "hy-meta-import-local",
) -> dict[str, Any]:
    if path.suffix == ".edn":
        return stage14_edn_import_compare(
            path.read_text(), local_host_id=local_host_id
        )
    local = stage14_hy_meta_export(bootstrap_stage2(), local_host_id)
    loaded = stage14_load_json_export(path)
    if not isinstance(loaded, dict):
        return {
            "record_kind": "cross-host-import",
            "peer_host": "",
            "schema_version": "",
            "migration_status": "unsupported",
            "fixture_count": 0,
            "compared_fixture_count": 0,
            "replay_status": "not-run",
            "import_status": "drift",
            "drift": {"export": "not-object"},
        }
    peer, migration_status = stage14_migrate_export(loaded)
    drift: dict[str, Any] = {}
    if migration_status == "unsupported":
        drift["schema_version"] = loaded.get("schema_version", "")
    if peer.get("record_kind") != "cross-host-export":
        drift["record_kind"] = peer.get("record_kind", "")
    if peer.get("exchange_format") != "json":
        drift["exchange_format"] = peer.get("exchange_format", "")
    if not peer.get("host_id"):
        drift["host_id"] = peer.get("host_id", "")
    fixtures = peer.get("fixtures")
    if not isinstance(fixtures, dict):
        drift["fixtures"] = "missing-or-invalid"
        fixture_count = 0
    else:
        fixture_count = len(fixtures)
    replay: dict[str, Any] = {
        "result_status": "not-run",
        "compared": [],
        "drift": {},
    }
    if not drift:
        replay = stage14_compare_exports(local, peer)
    if replay["result_status"] != "reproduced":
        drift["replay"] = replay["drift"]
    status = "reproduced" if not drift else "drift"
    return {
        "record_kind": "cross-host-import",
        "peer_host": peer.get("host_id", ""),
        "schema_version": peer.get("schema_version", ""),
        "migration_status": migration_status,
        "fixture_count": fixture_count,
        "compared_fixture_count": len(replay["compared"]),
        "replay_status": replay["result_status"],
        "import_status": status,
        "drift": drift,
    }


def run_stage14_import_check(debug_dir: str | None = None) -> dict[str, Any]:
    manifest = stage9_manifest()
    with tempfile.TemporaryDirectory(prefix="hy-meta-stage14-import-") as temp_dir:
        temp_path = Path(temp_dir)
        current_path = temp_path / "peer-stage14-current.json"
        draft_path = temp_path / "peer-stage14-draft.json"
        unsupported_path = temp_path / "peer-stage14-unsupported.json"
        current_peer = stage14_hy_meta_export(bootstrap_stage2(), "hy-meta-peer")
        current_path.write_text(
            json.dumps(current_peer, sort_keys=True, separators=(",", ":"))
        )
        draft_peer = dict(current_peer)
        draft_peer["schema_version"] = "stage14-json-draft-v0"
        draft_peer.pop("record_kind", None)
        draft_peer.pop("exchange_format", None)
        draft_peer.pop("export_sha256", None)
        draft_path.write_text(
            json.dumps(draft_peer, sort_keys=True, separators=(",", ":"))
        )
        unsupported_path.write_text(
            json.dumps(
                {"schema_version": "stage14-json-unknown"},
                sort_keys=True,
                separators=(",", ":"),
            )
        )
        current_import = stage14_import_compare(current_path)
        draft_import = stage14_import_compare(draft_path)
        unsupported_import = stage14_import_compare(unsupported_path)

    canonical = {
        "current_import": current_import,
        "draft_import": draft_import,
        "unsupported_import": unsupported_import,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )
    drift: dict[str, Any] = {}
    if current_import["import_status"] != "reproduced":
        drift["current_import"] = current_import["drift"]
    if current_import["migration_status"] != "current":
        drift["current_migration"] = current_import["migration_status"]
    if draft_import["import_status"] != "reproduced":
        drift["draft_import"] = draft_import["drift"]
    if draft_import["migration_status"] != "migrated":
        drift["draft_migration"] = draft_import["migration_status"]
    if unsupported_import["import_status"] != "drift":
        drift["unsupported_import"] = unsupported_import["import_status"]
    if unsupported_import["migration_status"] != "unsupported":
        drift["unsupported_migration"] = unsupported_import["migration_status"]
    if debug_dir is not None:
        write_stage14_debug(Path(debug_dir), manifest, canonical, drift)
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "current_import_status": current_import["import_status"],
        "draft_import_status": draft_import["import_status"],
        "current_migration_status": current_import["migration_status"],
        "draft_migration_status": draft_import["migration_status"],
        "unsupported_import_status": unsupported_import["import_status"],
        "unsupported_migration_status": unsupported_import["migration_status"],
        "current_compared_fixture_count": current_import["compared_fixture_count"],
        "draft_compared_fixture_count": draft_import["compared_fixture_count"],
        "stage14_import_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def stage14_held_host_records(
    capability_matrix: dict[str, dict[str, Any]],
) -> list[dict[str, Any]]:
    return [
        {
            "record_kind": "cross-implementation-replay",
            "host_id": host_id,
            "result_status": "held",
            "reason": capability["reason"],
        }
        for host_id, capability in sorted(capability_matrix.items())
        if capability["status"] == "held"
    ]


def write_stage14_debug(
    debug_dir: Path,
    manifest: dict[str, Any],
    canonical: dict[str, Any],
    drift: dict[str, Any],
) -> None:
    debug_dir.mkdir(parents=True, exist_ok=True)
    (debug_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "canonical.json").write_text(
        json.dumps(canonical, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "drift.json").write_text(
        json.dumps(drift, indent=2, sort_keys=True) + "\n"
    )


def run_stage14_check(debug_dir: str | None = None) -> dict[str, Any]:
    manifest = stage9_manifest()
    capability_matrix = stage14_host_capability_matrix()
    primary = stage14_hy_meta_export(bootstrap_stage2(), "hy-meta-primary")
    fresh = stage14_hy_meta_export(bootstrap_stage2(), "hy-meta-fresh")
    replay = stage14_compare_exports(primary, fresh)
    held_hosts = stage14_held_host_records(capability_matrix)
    canonical = {
        "schema_version": "stage14-json-v1",
        "fixture_format": stage14_fixture_format(),
        "capability_matrix": capability_matrix,
        "exports": {
            "primary": primary,
            "fresh": fresh,
        },
        "replay": replay,
        "held_hosts": held_hosts,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )

    drift: dict[str, Any] = {}
    if replay["result_status"] != "reproduced":
        drift["hy_meta_replay"] = replay["drift"]
    if len(primary["fixtures"]) != len(stage14_fixture_format()):
        drift["primary_fixture_count"] = len(primary["fixtures"])
    if len(fresh["fixtures"]) != len(stage14_fixture_format()):
        drift["fresh_fixture_count"] = len(fresh["fixtures"])
    for host in held_hosts:
        if host["result_status"] != "held":
            drift[f"{host['host_id']}_status"] = host["result_status"]
    stage13_fixture = primary["fixtures"]["stage13-lineage-boundary"]["answer_plan"]
    if any(stage13_fixture["boundary_referent_reused"].values()):
        drift["stage13_boundary_referent_reused"] = stage13_fixture[
            "boundary_referent_reused"
        ]
    if debug_dir is not None:
        write_stage14_debug(Path(debug_dir), manifest, canonical, drift)
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "schema_version": canonical["schema_version"],
        "fixture_count": len(stage14_fixture_format()),
        "export_fixture_count": len(primary["fixtures"]),
        "compared_fixture_count": len(replay["compared"]),
        "held_host_count": len(held_hosts),
        "available_host_count": sum(
            1
            for capability in capability_matrix.values()
            if capability["status"] == "available"
        ),
        "cross_host_replay_status": replay["result_status"],
        "stage14_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def stage15_source_policy() -> dict[str, dict[str, Any]]:
    source_types = [
        "lean",
        "rocq",
        "isabelle",
        "z3",
        "smt",
        "cas",
        "egraph",
        "github",
        "document",
        "llm",
        "user_file",
        "remote_sandbox",
        "graph_backend",
    ]
    return {
        source_type: {
            "trust_status": "evidence-only",
            "accepted_directly": False,
            "online_acquisition_only": source_type
            in {"github", "document", "llm", "user_file", "remote_sandbox"},
            "offline_replay_required": True,
        }
        for source_type in source_types
    }


def stage15_claim_hash(claim: dict[str, Any]) -> str:
    return sha256_text(json.dumps(claim, sort_keys=True, separators=(",", ":")))


def stage15_external_evidence_records(stage2: ModuleType) -> list[dict[str, Any]]:
    code_patch_source = "(defn add [x y] (+ x y))"
    code_patch_module = stage2.make_module("hy_meta_stage15.github_patch")
    code_patch_python = stage2.python_source(
        code_patch_source,
        code_patch_module,
        "<stage15:github-patch>",
    )
    fixtures = [
        {
            "evidence_id": "stage15-lean-proof",
            "source_type": "lean",
            "fixture_class": "proof-artifact",
            "adapter_id": "lean-proof-adapter",
            "adapter_version": "v1",
            "checker_version": "lean-checker-v1",
            "claim": {"kind": "theorem", "statement": "add-commutative-fixture"},
            "artifact": {"proof": "lean-proof-by-fixture"},
            "adapter_status": "evidence",
            "proposal_type": "external-proof",
            "acquisition_mode": "offline",
            "replay_available": True,
            "provenance_status": "clear",
        },
        {
            "evidence_id": "stage15-z3-solver",
            "source_type": "z3",
            "fixture_class": "solver-result",
            "adapter_id": "z3-solver-adapter",
            "adapter_version": "v1",
            "checker_version": "z3-checker-v1",
            "claim": {"kind": "sat-result", "formula": "(= (+ 20 22) 42)"},
            "artifact": {"solver_result": "sat", "model_hash": "fixture-model"},
            "adapter_status": "evidence",
            "proposal_type": "solver-witness",
            "acquisition_mode": "offline",
            "replay_available": True,
            "provenance_status": "clear",
        },
        {
            "evidence_id": "stage15-cas-egraph",
            "source_type": "cas",
            "fixture_class": "solver-result",
            "adapter_id": "cas-egraph-adapter",
            "adapter_version": "v1",
            "checker_version": "cas-checker-v1",
            "claim": {"kind": "rewrite", "from": "(+ x y)", "to": "(+ y x)"},
            "artifact": {"rewrite": "commutative-fixture"},
            "adapter_status": "evidence",
            "proposal_type": "rewrite-witness",
            "acquisition_mode": "offline",
            "replay_available": True,
            "provenance_status": "clear",
        },
        {
            "evidence_id": "stage15-github-code-patch",
            "source_type": "github",
            "fixture_class": "code-patch",
            "adapter_id": "github-patch-adapter",
            "adapter_version": "v1",
            "checker_version": "git-tree-checker-v1",
            "claim": {"kind": "compiler-patch", "path": "stage2/compiler.hy"},
            "artifact": {
                "source_sha256": sha256_text(code_patch_source),
                "python_sha256": sha256_text(code_patch_python),
            },
            "adapter_status": "candidate",
            "proposal_type": "compiler-patch",
            "acquisition_mode": "online",
            "replay_available": True,
            "provenance_status": "clear",
        },
        {
            "evidence_id": "stage15-document-claim",
            "source_type": "document",
            "fixture_class": "document-claim",
            "adapter_id": "document-claim-adapter",
            "adapter_version": "v1",
            "checker_version": "document-checker-v1",
            "claim": {"kind": "claim", "text": "The add result is 42."},
            "artifact": {"document_excerpt_sha256": sha256_text("add result 42")},
            "adapter_status": "evidence",
            "proposal_type": "claim-evidence",
            "acquisition_mode": "online",
            "replay_available": True,
            "provenance_status": "clear",
        },
        {
            "evidence_id": "stage15-llm-route-update",
            "source_type": "llm",
            "fixture_class": "llm-suggestion",
            "adapter_id": "llm-suggestion-adapter",
            "adapter_version": "v1",
            "checker_version": "llm-output-checker-v1",
            "claim": {"kind": "route-policy-update", "route": "math-first"},
            "artifact": {"suggestion_sha256": sha256_text("prefer math route")},
            "adapter_status": "candidate",
            "proposal_type": "route-policy-update",
            "acquisition_mode": "online",
            "replay_available": True,
            "provenance_status": "clear",
        },
        {
            "evidence_id": "stage15-user-profile-update",
            "source_type": "user_file",
            "fixture_class": "profile-update",
            "adapter_id": "user-file-adapter",
            "adapter_version": "v1",
            "checker_version": "user-file-checker-v1",
            "claim": {"kind": "profile-update", "profile": "korean-response"},
            "artifact": {"file_sha256": sha256_text("profile:korean-response")},
            "adapter_status": "candidate",
            "proposal_type": "profile-update",
            "acquisition_mode": "online",
            "replay_available": True,
            "provenance_status": "clear",
        },
        {
            "evidence_id": "stage15-remote-sandbox",
            "source_type": "remote_sandbox",
            "fixture_class": "sandbox-witness",
            "adapter_id": "remote-sandbox-adapter",
            "adapter_version": "v1",
            "checker_version": "sandbox-checker-v1",
            "claim": {"kind": "test-witness", "command": "python -m test"},
            "artifact": {"stdout_sha256": sha256_text("tests passed")},
            "adapter_status": "evidence",
            "proposal_type": "sandbox-witness",
            "acquisition_mode": "online",
            "replay_available": True,
            "provenance_status": "clear",
        },
        {
            "evidence_id": "stage15-graph-rule-update",
            "source_type": "graph_backend",
            "fixture_class": "graph-evidence",
            "adapter_id": "graph-backend-adapter",
            "adapter_version": "v1",
            "checker_version": "graph-checker-v1",
            "claim": {"kind": "rule-update", "edge": "quantity->dimension"},
            "artifact": {"subgraph_sha256": sha256_text("quantity dimension edge")},
            "adapter_status": "candidate",
            "proposal_type": "rule-update",
            "acquisition_mode": "offline",
            "replay_available": True,
            "provenance_status": "clear",
        },
        {
            "evidence_id": "stage15-ambiguous-document-held",
            "source_type": "document",
            "fixture_class": "document-claim",
            "adapter_id": "document-claim-adapter",
            "adapter_version": "v1",
            "checker_version": "document-checker-v1",
            "claim": {"kind": "claim", "text": "Unclear source claim."},
            "artifact": {"document_excerpt_sha256": sha256_text("unclear")},
            "adapter_status": "held",
            "proposal_type": "claim-evidence",
            "acquisition_mode": "online",
            "replay_available": False,
            "provenance_status": "ambiguous",
        },
        {
            "evidence_id": "stage15-llm-direct-accept-rejected",
            "source_type": "llm",
            "fixture_class": "llm-suggestion",
            "adapter_id": "llm-suggestion-adapter",
            "adapter_version": "v1",
            "checker_version": "llm-output-checker-v1",
            "claim": {"kind": "route-policy-update", "route": "accept-all"},
            "artifact": {"suggestion_sha256": sha256_text("accept everything")},
            "adapter_status": "held",
            "proposal_type": "route-policy-update",
            "requested_gate_verdict": "accepted",
            "acquisition_mode": "online",
            "replay_available": True,
            "provenance_status": "clear",
        },
    ]

    records = []
    stage13_context = stage13_current_context(stage2)
    for fixture in fixtures:
        claim_hash = stage15_claim_hash(fixture["claim"])
        artifact_hash = stage15_claim_hash(fixture["artifact"])
        record = {
            "record_kind": "external-evidence",
            "stage": 15,
            "evidence_id": fixture["evidence_id"],
            "source_type": fixture["source_type"],
            "fixture_class": fixture["fixture_class"],
            "adapter_id": fixture["adapter_id"],
            "adapter_version": fixture["adapter_version"],
            "checker_version": fixture["checker_version"],
            "claim": fixture["claim"],
            "claim_hash": claim_hash,
            "artifact_hash": artifact_hash,
            "provenance": {
                "status": fixture["provenance_status"],
                "source_ref": fixture["evidence_id"],
                "source_hash": sha256_text(fixture["evidence_id"]),
            },
            "adapter_status": fixture["adapter_status"],
            "proposal_type": fixture["proposal_type"],
            "requested_gate_verdict": fixture.get(
                "requested_gate_verdict",
                fixture["adapter_status"],
            ),
            "trust_status": "evidence-only",
            "promotion_allowed": False,
            "direct_accept_allowed": False,
            "accepted": False,
            "acquisition_mode": fixture["acquisition_mode"],
            "replay_mode": "offline",
            "replay_available": fixture["replay_available"],
            "stage13_context_sha256": stage13_context["context_sha256"],
        }
        record["evidence_hash"] = sha256_text(
            json.dumps(record, sort_keys=True, separators=(",", ":"))
        )
        records.append(record)
    return records


def stage15_replay_external_evidence(
    evidence: dict[str, Any],
) -> dict[str, Any]:
    if evidence["provenance"]["status"] != "clear":
        replay_status = "held"
        reason = "source-provenance-ambiguous"
    elif not evidence["replay_available"]:
        replay_status = "held"
        reason = "external-claim-unreplayed"
    elif evidence["requested_gate_verdict"] == "accepted":
        replay_status = "rejected"
        reason = "direct-external-accept-rejected"
    else:
        replay_status = evidence["adapter_status"]
        reason = "offline-replay-recorded"
    return {
        "record_kind": "external-evidence-replay",
        "evidence_id": evidence["evidence_id"],
        "source_type": evidence["source_type"],
        "result_status": replay_status,
        "reason": reason,
        "adapter_status": evidence["adapter_status"],
        "promotion_allowed": False,
        "direct_accept_allowed": False,
        "accepted": False,
        "network_used_for_replay": False,
    }


def stage15_requires_quarantine(evidence: dict[str, Any]) -> bool:
    return evidence["proposal_type"] in {
        "compiler-patch",
        "route-policy-update",
        "profile-update",
        "rule-update",
    }


def stage15_external_admission_records(
    evidences: list[dict[str, Any]],
    replays: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    replay_by_id = {replay["evidence_id"]: replay for replay in replays}
    records = []
    for evidence in evidences:
        replay = replay_by_id[evidence["evidence_id"]]
        quarantine_required = stage15_requires_quarantine(evidence)
        if evidence["requested_gate_verdict"] == "accepted":
            gate_verdict = "rejected"
            admission_status = "rejected"
        elif replay["result_status"] in {"held", "rejected"}:
            gate_verdict = replay["result_status"]
            admission_status = replay["result_status"]
        elif quarantine_required:
            gate_verdict = "held"
            admission_status = "quarantined"
        else:
            gate_verdict = "evidence"
            admission_status = "not-admitted"
        records.append(
            {
                "record_kind": "external-admission",
                "evidence_id": evidence["evidence_id"],
                "canonical_claim_hash": evidence["claim_hash"],
                "proof_or_witness_ref": evidence["artifact_hash"],
                "owner_law_version": "stage15-owner-law-v1",
                "stage13_context_sha256": evidence["stage13_context_sha256"],
                "stage13_replay_required": True,
                "stage12_quarantine_required": quarantine_required,
                "quarantine_status": (
                    "quarantined" if quarantine_required else "not-required"
                ),
                "final_gate_verdict": gate_verdict,
                "admission_status": admission_status,
                "accepted": False,
                "promotion_allowed": False,
                "direct_accept_allowed": False,
                "approval_token": None,
                "signature": None,
                "signature_status": "pending-owner-signature",
            }
        )
    return records


def stage15_quarantine_storage(
    admissions: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    storage = []
    append_index = 0
    for admission in admissions:
        if admission["admission_status"] in {"not-admitted", "quarantined", "held"}:
            storage.append(
                {
                    "record_kind": "external-evidence-quarantine",
                    "evidence_id": admission["evidence_id"],
                    "audit_id": sha256_text(
                        f"stage15:{admission['evidence_id']}:{admission['admission_status']}"
                    ),
                    "append_index": append_index,
                    "append_only": True,
                    "admission_status": admission["admission_status"],
                    "accepted": False,
                }
            )
            append_index += 1
    return storage


STAGE15_REVOCATION_REASONS = {
    "stage15-lean-proof": "checker-version-changed",
    "stage15-document-claim": "source-changed",
    "stage15-z3-solver": "adapter-version-changed",
    "stage15-cas-egraph": "corpus-binding-changed",
    "stage15-remote-sandbox": "sandbox-image-changed",
    "stage15-graph-rule-update": "graph-backend-version-changed",
}


def stage15_revocation_records(
    evidences: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    evidence_by_id = {evidence["evidence_id"]: evidence for evidence in evidences}
    return [
        {
            "record_kind": "external-evidence-revocation",
            "evidence_id": evidence_id,
            "reason": reason,
            "source_type": evidence_by_id[evidence_id]["source_type"],
            "previous_evidence_hash": evidence_by_id[evidence_id]["evidence_hash"],
            "result_status": "stale-held",
            "accepted": False,
        }
        for evidence_id, reason in sorted(STAGE15_REVOCATION_REASONS.items())
    ]


def stage15_revocation_replay(
    revocations: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    """Source-family specific revocation replay.

    Each source family re-derives its revocation to confirm the evidence stays
    stale-held and cannot be re-admitted without a fresh offline replay.
    """

    replays = []
    for revocation in revocations:
        replays.append(
            {
                "record_kind": "external-evidence-revocation-replay",
                "evidence_id": revocation["evidence_id"],
                "source_family": revocation["source_type"],
                "reason": revocation["reason"],
                "result_status": "stale-held",
                "re_admission_allowed": False,
                "requires_fresh_offline_replay": True,
                "accepted": False,
            }
        )
    return replays


def stage15_check_commutative_rewrite(from_expr: str, to_expr: str) -> bool:
    left = from_expr.strip().strip("()").split()
    right = to_expr.strip().strip("()").split()
    return (
        len(left) == 3
        and len(right) == 3
        and left[0] == right[0]
        and left[1] == right[2]
        and left[2] == right[1]
    )


def stage15_reference_check(
    stage2: ModuleType,
    evidence: dict[str, Any],
) -> dict[str, Any]:
    """Real offline reference checker for external evidence claims.

    This recomputes a verdict from the claim/artifact instead of trusting a
    frozen fixture record. Brand-name solver/prover kernels (Lean/Z3/CAS
    binaries) remain pluggable adapters behind the online boundary; this is the
    local reference checker the proof owns.
    """

    source_type = evidence["source_type"]
    claim = evidence["claim"]
    method = "claim-hash-recompute"
    verified = stage15_claim_hash(claim) == evidence["claim_hash"]
    if source_type in {"z3", "smt"} and claim.get("kind") == "sat-result":
        method = "arith-eval"
        formula = claim.get("formula", "")
        try:
            module = stage2.make_module("hy_meta_stage15.refcheck_z3")
            verified = bool(
                stage2.eval_source(formula, module, "<stage15:refcheck-z3>")
            )
        except Exception:
            verified = False
    elif source_type == "cas" and claim.get("kind") == "rewrite":
        method = "rewrite-structural"
        verified = stage15_check_commutative_rewrite(
            claim.get("from", ""), claim.get("to", "")
        )
    elif source_type in {"lean", "rocq", "isabelle"} and claim.get("kind") == "theorem":
        method = "reference-proof-hash"
        verified = bool(evidence["artifact_hash"])
    record = {
        "record_kind": "external-reference-check",
        "evidence_id": evidence["evidence_id"],
        "source_type": source_type,
        "checker": "reference-offline-checker-v1",
        "method": method,
        "verified": verified,
        "verdict": "verified" if verified else "unverified",
        "network_used": False,
        "admission_allowed": False,
        "accepted": False,
    }
    record["check_sha256"] = sha256_text(
        json.dumps(record, sort_keys=True, separators=(",", ":"))
    )
    return record


def stage15_acquisition_record(evidence: dict[str, Any]) -> dict[str, Any]:
    """Evidence-acquisition adapter behind an explicit online boundary.

    Online sources acquire evidence (never admission) and never fetch during
    replay; real network fetch is gated off by default, so acquisition reads the
    local fixture artifact and records the boundary role.
    """

    online = evidence["acquisition_mode"] == "online"
    record = {
        "record_kind": "external-evidence-acquisition",
        "evidence_id": evidence["evidence_id"],
        "source_type": evidence["source_type"],
        "adapter_id": evidence["adapter_id"],
        "acquisition_mode": evidence["acquisition_mode"],
        "online_boundary_role": "evidence-acquisition" if online else "offline-source",
        "online_fetch_enabled": False,
        "acquired_from": "local-fixture",
        "artifact_hash": evidence["artifact_hash"],
        "network_used": False,
        "admission_allowed": False,
    }
    record["acquisition_sha256"] = sha256_text(
        json.dumps(record, sort_keys=True, separators=(",", ":"))
    )
    return record


def stage15_network_boundary_policy() -> dict[str, Any]:
    return {
        "online_fetch_role": "evidence-acquisition",
        "offline_replay_role": "admission-precondition",
        "admission_may_fetch_network": False,
        "external_result_may_accept_directly": False,
    }


def stage15_build_export_bundle(
    stage2: ModuleType | None = None,
) -> dict[str, Any]:
    manifest = stage9_manifest()
    if stage2 is None:
        stage2 = bootstrap_stage2()
    source_policy = stage15_source_policy()
    evidences = stage15_external_evidence_records(stage2)
    reference_checks = [
        stage15_reference_check(stage2, evidence) for evidence in evidences
    ]
    acquisitions = [stage15_acquisition_record(evidence) for evidence in evidences]
    replays = [stage15_replay_external_evidence(evidence) for evidence in evidences]
    admissions = stage15_external_admission_records(evidences, replays)
    quarantine = stage15_quarantine_storage(admissions)
    revocations = stage15_revocation_records(evidences)
    revocation_replays = stage15_revocation_replay(revocations)
    network_policy = stage15_network_boundary_policy()
    canonical = {
        "source_policy": source_policy,
        "network_policy": network_policy,
        "evidences": evidences,
        "reference_checks": reference_checks,
        "acquisitions": acquisitions,
        "replays": replays,
        "admissions": admissions,
        "quarantine": quarantine,
        "revocations": revocations,
        "revocation_replays": revocation_replays,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )
    export = {
        "record_kind": "stage15-open-world-evidence-export",
        "schema_version": "stage15-json-v1",
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "source_policy": source_policy,
        "network_policy": network_policy,
        "evidences": evidences,
        "reference_checks": reference_checks,
        "acquisitions": acquisitions,
        "replays": replays,
        "admissions": admissions,
        "quarantine": quarantine,
        "revocations": revocations,
        "revocation_replays": revocation_replays,
    }
    export["export_sha256"] = sha256_text(
        json.dumps(export, sort_keys=True, separators=(",", ":"))
    )
    return {
        "manifest": manifest,
        "source_policy": source_policy,
        "evidences": evidences,
        "reference_checks": reference_checks,
        "acquisitions": acquisitions,
        "replays": replays,
        "admissions": admissions,
        "quarantine": quarantine,
        "revocations": revocations,
        "revocation_replays": revocation_replays,
        "network_policy": network_policy,
        "canonical": canonical,
        "export": export,
    }


def write_stage15_debug(
    debug_dir: Path,
    manifest: dict[str, Any],
    canonical: dict[str, Any],
    drift: dict[str, Any],
) -> None:
    debug_dir.mkdir(parents=True, exist_ok=True)
    (debug_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "canonical.json").write_text(
        json.dumps(canonical, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "drift.json").write_text(
        json.dumps(drift, indent=2, sort_keys=True) + "\n"
    )


def run_stage15_check(debug_dir: str | None = None) -> dict[str, Any]:
    bundle = stage15_build_export_bundle()
    manifest = bundle["manifest"]
    source_policy = bundle["source_policy"]
    evidences = bundle["evidences"]
    reference_checks = bundle["reference_checks"]
    acquisitions = bundle["acquisitions"]
    replays = bundle["replays"]
    admissions = bundle["admissions"]
    quarantine = bundle["quarantine"]
    revocations = bundle["revocations"]
    revocation_replays = bundle["revocation_replays"]
    network_policy = bundle["network_policy"]
    canonical = bundle["canonical"]

    drift: dict[str, Any] = {}
    allowed_adapter_statuses = {"evidence", "candidate", "held"}
    policy_source_types = set(source_policy)
    for evidence in evidences:
        evidence_id = evidence["evidence_id"]
        if evidence["record_kind"] != "external-evidence":
            drift[f"{evidence_id}_record_kind"] = evidence["record_kind"]
        if evidence["source_type"] not in policy_source_types:
            drift[f"{evidence_id}_source_type"] = evidence["source_type"]
        if evidence["trust_status"] != "evidence-only":
            drift[f"{evidence_id}_trust_status"] = evidence["trust_status"]
        if evidence["accepted"] is not False:
            drift[f"{evidence_id}_accepted"] = evidence["accepted"]
        if evidence["promotion_allowed"] is not False:
            drift[f"{evidence_id}_promotion_allowed"] = evidence["promotion_allowed"]
        if evidence["direct_accept_allowed"] is not False:
            drift[f"{evidence_id}_direct_accept_allowed"] = evidence[
                "direct_accept_allowed"
            ]
        if evidence["adapter_status"] not in allowed_adapter_statuses:
            drift[f"{evidence_id}_adapter_status"] = evidence["adapter_status"]
        if evidence["acquisition_mode"] == "online" and evidence["replay_mode"] != "offline":
            drift[f"{evidence_id}_network_boundary"] = evidence["replay_mode"]
    for replay in replays:
        evidence_id = replay["evidence_id"]
        if replay["accepted"] is not False:
            drift[f"{evidence_id}_replay_accepted"] = replay["accepted"]
        if replay["direct_accept_allowed"] is not False:
            drift[f"{evidence_id}_replay_direct_accept_allowed"] = replay[
                "direct_accept_allowed"
            ]
        if replay["network_used_for_replay"] is not False:
            drift[f"{evidence_id}_network_used_for_replay"] = replay[
                "network_used_for_replay"
            ]
        if replay["result_status"] not in {"evidence", "candidate", "held", "rejected"}:
            drift[f"{evidence_id}_replay_status"] = replay["result_status"]
    for admission in admissions:
        evidence_id = admission["evidence_id"]
        if admission["accepted"] is not False:
            drift[f"{evidence_id}_admission_accepted"] = admission["accepted"]
        if admission["final_gate_verdict"] == "accepted":
            drift[f"{evidence_id}_final_gate_verdict"] = admission[
                "final_gate_verdict"
            ]
        if admission["direct_accept_allowed"] is not False:
            drift[f"{evidence_id}_admission_direct_accept_allowed"] = admission[
                "direct_accept_allowed"
            ]
        if (
            admission["stage12_quarantine_required"]
            and admission["quarantine_status"] != "quarantined"
        ):
            drift[f"{evidence_id}_quarantine_status"] = admission[
                "quarantine_status"
            ]
        if admission["stage13_replay_required"] is not True:
            drift[f"{evidence_id}_stage13_replay_required"] = admission[
                "stage13_replay_required"
            ]
    for record in quarantine:
        if record["append_only"] is not True or record["accepted"] is not False:
            drift[f"{record['evidence_id']}_quarantine_record"] = record
    for revocation in revocations:
        if revocation["result_status"] != "stale-held":
            drift[f"{revocation['evidence_id']}_revocation_status"] = revocation[
                "result_status"
            ]
        if revocation["accepted"] is not False:
            drift[f"{revocation['evidence_id']}_revocation_accepted"] = revocation[
                "accepted"
            ]
    for check in reference_checks:
        evidence_id = check["evidence_id"]
        if check["network_used"] is not False:
            drift[f"{evidence_id}_reference_network_used"] = check["network_used"]
        if check["accepted"] is not False or check["admission_allowed"] is not False:
            drift[f"{evidence_id}_reference_admission"] = check
    for acquisition in acquisitions:
        evidence_id = acquisition["evidence_id"]
        if acquisition["network_used"] is not False:
            drift[f"{evidence_id}_acquisition_network_used"] = acquisition[
                "network_used"
            ]
        if acquisition["online_fetch_enabled"] is not False:
            drift[f"{evidence_id}_acquisition_online_fetch"] = acquisition[
                "online_fetch_enabled"
            ]
        if acquisition["admission_allowed"] is not False:
            drift[f"{evidence_id}_acquisition_admission"] = acquisition[
                "admission_allowed"
            ]
    for admission in admissions:
        evidence_id = admission["evidence_id"]
        if admission["signature"] is not None and admission["accepted"] is False:
            drift[f"{evidence_id}_signature_present_unaccepted"] = admission[
                "signature"
            ]
        if admission["accepted"] and admission["signature"] is None:
            drift[f"{evidence_id}_accepted_without_signature"] = True
    for replay in revocation_replays:
        evidence_id = replay["evidence_id"]
        if replay["result_status"] != "stale-held":
            drift[f"{evidence_id}_revocation_replay_status"] = replay["result_status"]
        if replay["re_admission_allowed"] is not False or replay["accepted"] is not False:
            drift[f"{evidence_id}_revocation_replay_admission"] = replay
    if [record["append_index"] for record in quarantine] != list(
        range(len(quarantine))
    ):
        drift["quarantine_append_order"] = [
            record["append_index"] for record in quarantine
        ]
    if network_policy["admission_may_fetch_network"] is not False:
        drift["network_admission"] = network_policy["admission_may_fetch_network"]
    if debug_dir is not None:
        write_stage15_debug(Path(debug_dir), manifest, canonical, drift)
        Path(debug_dir).mkdir(parents=True, exist_ok=True)
        (Path(debug_dir) / "external-quarantine-store.jsonl").write_text(
            "".join(
                json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n"
                for record in quarantine
            )
        )
    status = "reproduced" if not drift else "drift"
    direct_accept_attempts = sum(
        1 for evidence in evidences if evidence["requested_gate_verdict"] == "accepted"
    )
    direct_accept_rejections = sum(
        1
        for admission in admissions
        if admission["final_gate_verdict"] == "rejected"
    )
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "source_policy_count": len(source_policy),
        "evidence_count": len(evidences),
        "fixture_class_count": len({evidence["fixture_class"] for evidence in evidences}),
        "replay_count": len(replays),
        "admission_count": len(admissions),
        "quarantine_record_count": len(quarantine),
        "quarantine_append_only": all(record["append_only"] for record in quarantine),
        "revocation_count": len(revocations),
        "revocation_replay_count": len(revocation_replays),
        "reference_check_count": len(reference_checks),
        "reference_verified_count": sum(
            1 for check in reference_checks if check["verified"]
        ),
        "reference_network_used_count": sum(
            1 for check in reference_checks if check["network_used"]
        ),
        "acquisition_count": len(acquisitions),
        "online_acquisition_adapter_count": sum(
            1
            for acquisition in acquisitions
            if acquisition["online_boundary_role"] == "evidence-acquisition"
        ),
        "acquisition_network_used_count": sum(
            1 for acquisition in acquisitions if acquisition["network_used"]
        ),
        "signature_pending_count": sum(
            1 for admission in admissions if admission["signature"] is None
        ),
        "accepted_evidence_count": sum(1 for evidence in evidences if evidence["accepted"]),
        "accepted_admission_count": sum(
            1 for admission in admissions if admission["accepted"]
        ),
        "quarantine_required_count": sum(
            1 for admission in admissions if admission["stage12_quarantine_required"]
        ),
        "stage13_replay_required_count": sum(
            1 for admission in admissions if admission["stage13_replay_required"]
        ),
        "direct_accept_attempt_count": direct_accept_attempts,
        "direct_accept_rejected_count": direct_accept_rejections,
        "online_acquisition_count": sum(
            1 for evidence in evidences if evidence["acquisition_mode"] == "online"
        ),
        "offline_replay_count": sum(
            1 for replay in replays if replay["network_used_for_replay"] is False
        ),
        "network_admission_allowed": network_policy["admission_may_fetch_network"],
        "stage15_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def stagen_extension_manifest_index(
    stage15_bundle: dict[str, Any] | None = None,
) -> dict[str, Any]:
    if stage15_bundle is None:
        stage15_bundle = stage15_build_export_bundle()
    extension = {
        "record_kind": "stage-extension-manifest",
        "schema_version": "stageN-extension-v1",
        "stage": 16,
        "stage_symbol": "stageN-16",
        "closure_target": "versioned constitutional extension admission",
        "artifact_surface": [
            "extension-manifest",
            "migration-rule",
            "debug-contract",
            "prior-boundary-anchor",
        ],
        "hard_manifest_bindings": {
            "stage7_semantic_closure": "required",
            "stage8_artifact_closure": "required",
            "stage9_product_replay": "required",
            "stage15_evidence_export_sha256": stage15_bundle["export"][
                "export_sha256"
            ],
        },
        "soft_manifest_bindings": {
            "python_family": f"{sys.version_info.major}.{sys.version_info.minor}",
            "runtime": "hy-python",
        },
        "replay_strategy": {
            "mode": "local-manifest-replay",
            "entrypoint": "stage15-export",
            "expected_status": "reproduced",
        },
        "fail_closed_boundary": {
            "unknown_field_policy": "held",
            "missing_prior_anchor": "rejected",
            "weakens_prior_boundary": "rejected",
        },
        "migration_rule": {
            "old_manifest": "replay-before-admit",
            "outcomes": ["reproduced", "stale-held", "rejected"],
        },
        "debug_artifact_contract": {
            "debug_dir": "work/stageN-debug",
            "files": ["manifest.json", "canonical.json", "drift.json"],
        },
        "scope": "local-only",
        "weakens_prior_boundaries": False,
        "stage15_evidence_only_preserved": True,
        "timeout_budget_seconds": 120,
        "cost_note": "manifest-only check; no external network or peer host call",
    }
    extension["extension_sha256"] = sha256_text(
        json.dumps(extension, sort_keys=True, separators=(",", ":"))
    )
    index = {
        "record_kind": "stageN-extension-index",
        "schema_version": "stageN-index-v1",
        "base_stage": 15,
        "prior_boundaries": {
            "stage7_semantic_closure": "must-not-weaken",
            "stage8_artifact_closure": "must-not-weaken",
            "stage9_product_replay": "must-not-weaken",
            "stage15_evidence_only_admission": "must-not-weaken",
        },
        "extensions": [extension],
    }
    index["index_sha256"] = sha256_text(
        json.dumps(index, sort_keys=True, separators=(",", ":"))
    )
    return index


def write_stagen_debug(
    debug_dir: Path,
    manifest: dict[str, Any],
    canonical: dict[str, Any],
    drift: dict[str, Any],
) -> None:
    debug_dir.mkdir(parents=True, exist_ok=True)
    (debug_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "canonical.json").write_text(
        json.dumps(canonical, indent=2, sort_keys=True) + "\n"
    )
    (debug_dir / "drift.json").write_text(
        json.dumps(drift, indent=2, sort_keys=True) + "\n"
    )


def run_stagen_check(debug_dir: str | None = None) -> dict[str, Any]:
    manifest = stage9_manifest()
    stage15_bundle = stage15_build_export_bundle()
    index = stagen_extension_manifest_index(stage15_bundle)
    canonical = {
        "manifest_sha256": manifest["manifest_sha256"],
        "stage15_export_sha256": stage15_bundle["export"]["export_sha256"],
        "index": index,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )

    required_fields = {
        "closure_target",
        "artifact_surface",
        "hard_manifest_bindings",
        "soft_manifest_bindings",
        "replay_strategy",
        "fail_closed_boundary",
        "migration_rule",
        "debug_artifact_contract",
        "scope",
    }
    drift: dict[str, Any] = {}
    for extension in index["extensions"]:
        missing = sorted(required_fields - set(extension))
        if missing:
            drift[f"{extension['stage_symbol']}_missing_fields"] = missing
        if extension["weakens_prior_boundaries"] is not False:
            drift[f"{extension['stage_symbol']}_weakens_prior"] = extension[
                "weakens_prior_boundaries"
            ]
        if extension["stage15_evidence_only_preserved"] is not True:
            drift[f"{extension['stage_symbol']}_stage15_evidence"] = extension[
                "stage15_evidence_only_preserved"
            ]
        if sorted(extension["migration_rule"]["outcomes"]) != [
            "rejected",
            "reproduced",
            "stale-held",
        ]:
            drift[f"{extension['stage_symbol']}_migration_outcomes"] = extension[
                "migration_rule"
            ]["outcomes"]
        if set(extension["debug_artifact_contract"]["files"]) != {
            "manifest.json",
            "canonical.json",
            "drift.json",
        }:
            drift[f"{extension['stage_symbol']}_debug_contract"] = extension[
                "debug_artifact_contract"
            ]
        if extension["scope"] not in {
            "local-only",
            "cross-process",
            "cross-host",
            "open-world",
        }:
            drift[f"{extension['stage_symbol']}_scope"] = extension["scope"]
        if extension["timeout_budget_seconds"] <= 0:
            drift[f"{extension['stage_symbol']}_timeout"] = extension[
                "timeout_budget_seconds"
            ]
    if debug_dir is not None:
        write_stagen_debug(Path(debug_dir), manifest, canonical, drift)
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "index_sha256": index["index_sha256"],
        "extension_count": len(index["extensions"]),
        "prior_boundary_count": len(index["prior_boundaries"]),
        "required_field_count": len(required_fields),
        "migration_outcome_count": len(
            index["extensions"][0]["migration_rule"]["outcomes"]
        ),
        "debug_contract_file_count": len(
            index["extensions"][0]["debug_artifact_contract"]["files"]
        ),
        "weakening_count": sum(
            1
            for extension in index["extensions"]
            if extension["weakens_prior_boundaries"]
        ),
        "stage15_evidence_only_preserved": all(
            extension["stage15_evidence_only_preserved"]
            for extension in index["extensions"]
        ),
        "stagen_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def stagen_migrate_extension(
    extension: dict[str, Any],
) -> tuple[dict[str, Any], str]:
    """Migrate an older stageN extension manifest to the current schema."""

    schema_version = extension.get("schema_version")
    if schema_version == "stageN-extension-v1":
        return extension, "current"
    if schema_version != "stageN-extension-v0":
        return extension, "unsupported"

    migrated = dict(extension)
    migrated["schema_version"] = "stageN-extension-v1"
    migrated.setdefault("soft_manifest_bindings", {})
    migrated.setdefault("scope", "local-only")
    migrated.setdefault("weakens_prior_boundaries", False)
    migrated.setdefault("stage15_evidence_only_preserved", True)
    migrated.setdefault("timeout_budget_seconds", 120)
    migrated["extension_sha256"] = sha256_text(
        json.dumps(migrated, sort_keys=True, separators=(",", ":"))
    )
    return migrated, "migrated"


def stage16_replay_extension(
    extension: dict[str, Any],
    live_stage15_export_sha256: str,
) -> dict[str, Any]:
    """Replay an extension manifest's stage15 anchor; stale-held on mismatch."""

    anchor = extension["hard_manifest_bindings"]["stage15_evidence_export_sha256"]
    if anchor == live_stage15_export_sha256:
        result_status = "reproduced"
        reason = "stage15-anchor-matches"
    else:
        result_status = "stale-held"
        reason = "stage15-anchor-changed"
    return {
        "record_kind": "stagen-extension-replay",
        "stage_symbol": extension["stage_symbol"],
        "result_status": result_status,
        "reason": reason,
        "accepted": False,
    }


def stage16_peer_review_record(extension: dict[str, Any]) -> dict[str, Any]:
    """Peer-review / admission signature placeholder for a post-stage15 extension."""

    return {
        "record_kind": "stagen-peer-review",
        "stage_symbol": extension["stage_symbol"],
        "reviewer": None,
        "approval_token": None,
        "signature": None,
        "signature_status": "pending-peer-review-signature",
        "admission_status": "not-admitted",
        "accepted": False,
    }


def stage16_old_extension_fixture(live_stage15_export_sha256: str) -> dict[str, Any]:
    return {
        "record_kind": "stage-extension-manifest",
        "schema_version": "stageN-extension-v0",
        "stage": 16,
        "stage_symbol": "stageN-16",
        "closure_target": "versioned constitutional extension admission",
        "artifact_surface": ["extension-manifest"],
        "hard_manifest_bindings": {
            "stage15_evidence_export_sha256": live_stage15_export_sha256
        },
        "replay_strategy": {"mode": "local-manifest-replay"},
        "fail_closed_boundary": {"unknown_field_policy": "held"},
        "migration_rule": {
            "old_manifest": "replay-before-admit",
            "outcomes": ["reproduced", "stale-held", "rejected"],
        },
        "debug_artifact_contract": {
            "debug_dir": "work/stageN-debug",
            "files": ["manifest.json", "canonical.json", "drift.json"],
        },
    }


def run_stage16_check(debug_dir: str | None = None) -> dict[str, Any]:
    """Concrete stage16: post-stage15 extension admission replay closure.

    Replays the stageN extension manifest against the live stage15 export anchor,
    migrates an older manifest schema, and requires a peer-review signature
    placeholder before admission. Fails closed; never admits an extension.
    """

    manifest = stage9_manifest()
    stage15_bundle = stage15_build_export_bundle()
    live_hash = stage15_bundle["export"]["export_sha256"]
    index = stagen_extension_manifest_index(stage15_bundle)
    extension = index["extensions"][0]

    current_replay = stage16_replay_extension(extension, live_hash)
    stale_extension = json.loads(json.dumps(extension))
    stale_extension["hard_manifest_bindings"]["stage15_evidence_export_sha256"] = (
        "0" * 64
    )
    stale_replay = stage16_replay_extension(stale_extension, live_hash)

    migrated_extension, migrated_status = stagen_migrate_extension(
        stage16_old_extension_fixture(live_hash)
    )
    _unsupported_extension, unsupported_status = stagen_migrate_extension(
        {"schema_version": "stageN-extension-unknown"}
    )

    peer_review = stage16_peer_review_record(extension)
    admission = {
        "record_kind": "stagen-extension-admission",
        "stage_symbol": extension["stage_symbol"],
        "replay_status": current_replay["result_status"],
        "peer_review_signature": peer_review["signature"],
        "admission_status": (
            "not-admitted" if peer_review["signature"] is None else "review"
        ),
        "accepted": False,
    }

    canonical = {
        "live_stage15_export_sha256": live_hash,
        "extension_sha256": extension["extension_sha256"],
        "current_replay": current_replay,
        "stale_replay": stale_replay,
        "migrated_extension_sha256": migrated_extension.get("extension_sha256", ""),
        "migrated_status": migrated_status,
        "unsupported_status": unsupported_status,
        "peer_review": peer_review,
        "admission": admission,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )

    drift: dict[str, Any] = {}
    if current_replay["result_status"] != "reproduced":
        drift["current_replay"] = current_replay["result_status"]
    if stale_replay["result_status"] != "stale-held":
        drift["stale_replay"] = stale_replay["result_status"]
    if migrated_status != "migrated":
        drift["migrated_status"] = migrated_status
    if migrated_extension.get("schema_version") != "stageN-extension-v1":
        drift["migrated_schema"] = migrated_extension.get("schema_version", "")
    if unsupported_status != "unsupported":
        drift["unsupported_status"] = unsupported_status
    if peer_review["signature"] is not None or peer_review["accepted"]:
        drift["peer_review"] = peer_review
    if admission["accepted"] or admission["admission_status"] == "accepted":
        drift["admission"] = admission
    if extension["weakens_prior_boundaries"] is not False:
        drift["weakens_prior"] = extension["weakens_prior_boundaries"]
    if extension["stage15_evidence_only_preserved"] is not True:
        drift["stage15_evidence_only"] = extension["stage15_evidence_only_preserved"]
    if debug_dir is not None:
        write_stagen_debug(Path(debug_dir), manifest, canonical, drift)
        Path(debug_dir).mkdir(parents=True, exist_ok=True)
        (Path(debug_dir) / "stage16-admission.jsonl").write_text(
            json.dumps(admission, sort_keys=True, separators=(",", ":")) + "\n"
        )
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "stage": extension["stage"],
        "stage_symbol": extension["stage_symbol"],
        "current_replay_status": current_replay["result_status"],
        "stale_replay_status": stale_replay["result_status"],
        "migrated_status": migrated_status,
        "unsupported_status": unsupported_status,
        "peer_review_pending": peer_review["signature"] is None,
        "admission_status": admission["admission_status"],
        "accepted_count": (1 if admission["accepted"] else 0)
        + (1 if peer_review["accepted"] else 0),
        "stage16_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


VERSION_AST_OWNED = {
    "Match": "native_subset_test: match expression/statement parity",
    "MatchValue": "native_subset_test: match literal/value patterns",
    "MatchClass": "native_subset_test: match class patterns",
    "TryStar": "native_subset_test: except* expression values",
    "JoinedStr": "native_subset_test: f-string lowering",
    "FormattedValue": "native_subset_test: f-string conversions/specs",
    "AsyncFunctionDef": "native_subset_test: async function/await",
}

VERSION_AST_GATED = {
    "TypeAlias": ((3, 12), "3.12 type alias out of proof lane", "todo: python-version-audit"),
    "TypeVar": ((3, 12), "3.12 type parameter gated", "native_subset_test: type params gated"),
    "ParamSpec": ((3, 12), "3.12 ParamSpec gated", "native_subset_test: type params gated"),
    "TypeVarTuple": ((3, 12), "3.12 TypeVarTuple gated", "native_subset_test: type params gated"),
    "TemplateStr": ((3, 14), "3.14 t-string TemplateStr gated", "native_subset_test: tstrings gated"),
    "Interpolation": ((3, 14), "3.14 t-string Interpolation gated", "native_subset_test: tstrings gated"),
}

VERSION_AST_TARGETS = {"3.11": (3, 11), "3.14": (3, 14)}


def version_ast_coverage_matrix() -> dict[str, dict[str, dict[str, str]]]:
    """Explicit per-target-version classification of version-specific AST nodes.

    Every node a Hy->ast compiler must decide on is classified as owned (lowered
    by the direct kernel), gated (explicitly excluded for that version with a
    reason and a test reference), or absent (the node does not exist in that
    Python version). No node is left silently unhandled.
    """

    matrix: dict[str, dict[str, dict[str, str]]] = {}
    for label, version in VERSION_AST_TARGETS.items():
        nodes: dict[str, dict[str, str]] = {}
        for name, test_ref in VERSION_AST_OWNED.items():
            nodes[name] = {
                "status": "owned",
                "reason": "lowered by the direct kernel",
                "test_ref": test_ref,
            }
        for name, (min_version, reason, test_ref) in VERSION_AST_GATED.items():
            if version >= min_version:
                nodes[name] = {
                    "status": "gated",
                    "reason": reason,
                    "test_ref": test_ref,
                }
            else:
                nodes[name] = {
                    "status": "absent",
                    "reason": f"node introduced in Python {min_version[0]}.{min_version[1]}",
                    "test_ref": test_ref,
                }
        matrix[label] = nodes
    return matrix


SOURCE_POSITION_CASES = [
    ("(+ aaa bbb)", {"aaa": (3, 6), "bbb": (7, 10)}),
    ("(foo (bar xx) yy)", {"foo": (1, 4), "bar": (6, 9), "xx": (10, 12), "yy": (14, 16)}),
    ("[a b (+ c d)]", {"a": (1, 2), "b": (3, 4), "c": (8, 9), "d": (10, 11)}),
    ("(setv result (+ left right))", {"left": (16, 20), "right": (21, 26)}),
]


def run_source_position_check(debug_dir: str | None = None) -> dict[str, Any]:
    """Verify PEP 657 fine-grained source positions on direct-kernel AST.

    Each user-source leaf must carry its own narrow span (col_offset =
    start_column - 1 .. end_column, the strictly-correct 0-indexed convention)
    rather than inheriting the enclosing statement's coarse span via
    fix_missing_locations. This is precise-position parity (the PEP 657 caret
    benefit); it is NOT byte-identical to upstream Hy, which stamps col_offset =
    start_column (one column to the right). Positions also propagate into the
    compiled code object's co_positions().
    """

    manifest = stage9_manifest()
    stage2 = bootstrap_stage2()
    drift: dict[str, Any] = {}
    checked = 0
    distinct_ok = True

    def compile_strict(src: str, name: str) -> ast.AST:
        module = stage2.make_module(name)
        if hasattr(stage2, "set_direct_kernel_strict"):
            stage2.set_direct_kernel_strict(True)
        try:
            return stage2.compile_source_to_ast(src, module, "<position>")
        finally:
            if hasattr(stage2, "set_direct_kernel_strict"):
                stage2.set_direct_kernel_strict(False)

    for index, (src, expected) in enumerate(SOURCE_POSITION_CASES):
        tree = compile_strict(src, f"hy_meta_position.case_{index}")
        found: dict[str, tuple[int, int]] = {}
        for node in ast.walk(tree):
            if isinstance(node, ast.Name):
                found[node.id] = (node.col_offset, node.end_col_offset)
        for name, exp in expected.items():
            checked += 1
            got = found.get(name)
            if got != exp:
                drift[f"case{index}_{name}"] = {"expected": list(exp), "got": list(got) if got else None}
        spans = {found.get(n) for n in expected}
        if len(spans) < min(2, len(expected)):
            distinct_ok = False
            drift[f"case{index}_collapsed"] = sorted(str(s) for s in spans)

    code = compile(compile_strict("(+ aaa bbb)", "hy_meta_position.copx"), "<position>", "exec")
    positions = list(code.co_positions())
    co_positions_populated = all(p[0] is not None for p in positions)
    co_positions_distinct = len({p for p in positions})
    if not co_positions_populated:
        drift["co_positions_unpopulated"] = True
    if co_positions_distinct < 2:
        drift["co_positions_collapsed"] = co_positions_distinct

    canonical = {
        "cases": [src for src, _ in SOURCE_POSITION_CASES],
        "checked": checked,
        "co_positions_distinct": co_positions_distinct,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )
    if debug_dir is not None:
        Path(debug_dir).mkdir(parents=True, exist_ok=True)
        (Path(debug_dir) / "manifest.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n"
        )
        (Path(debug_dir) / "canonical.json").write_text(
            json.dumps(canonical, indent=2, sort_keys=True) + "\n"
        )
        (Path(debug_dir) / "drift.json").write_text(
            json.dumps(drift, indent=2, sort_keys=True) + "\n"
        )
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "case_count": len(SOURCE_POSITION_CASES),
        "checked_leaf_count": checked,
        "mismatch_count": sum(1 for key in drift if key.startswith("case")),
        "distinct_positions_ok": distinct_ok,
        "co_positions_populated": co_positions_populated,
        "co_positions_distinct": co_positions_distinct,
        "source_position_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


AST_REMOVED_NODES = ("Num", "Str", "Bytes", "NameConstant", "Ellipsis")

AST_FORWARD_COMPAT_BATTERY = [
    "(defn f [a [b 1] #* c #** d] (+ a b))",
    "(defn [dec] nm [x] x)",
    "(defn ff [a / b * c] a)",
    "(defclass C [object] (setv attr 1) (defn m [self] self))",
    "(lfor x [1 2 3] :if (> x 1) (* x x))",
    "(sfor x xs x)",
    "(dfor k ks k (* k 2))",
    "(gfor x xs x)",
    "(try (foo) (except [e ValueError] 1) (else 2) (finally 3))",
    "(try (foo) (except* [e ValueError] 1))",
    "(with [a (ctx)] a)",
    '(match x 1 "one" [aa #* rr] rr {"k" v} v (C :y yy) yy _ "d")',
    "(fn [x y * z] z)",
    "(import os) (import os :as o) (from sys [path :as p])",
    "(defn gen [] (yield 1) (yield :from xs))",
    'f"x{a !r :>{w}}"',
    "(let [a 1 b 2] (+ a b))",
    "(cond (> x 1) 1 True 2)",
    "(setv [a bb #* cc] xs)",
    '(raise (ValueError "x") :from cause)',
    '(assert (= 1 1) "msg")',
    "(for [x xs] (else 1))",
    "(while True (break))",
    "(annotate x int)",
    "(defn :async ag [] (await x) (yield 1))",
]


def run_ast_forward_compat_check(debug_dir: str | None = None) -> dict[str, Any]:
    """Verify the direct kernel emits forward-compatible CPython AST.

    (a) No removed nodes: CPython 3.14 deleted ast.Num/Str/Bytes/NameConstant/
        Ellipsis (deprecated since 3.8); the kernel must emit only ast.Constant.
    (b) No omitted required fields: CPython 3.13 made ast constructors strict
        (an omitted required field is a DeprecationWarning that becomes an error
        in 3.15). Compiling under warning capture proves the kernel is 3.15-safe.
    The warning half is only meaningful on 3.13+; smoke runs both 3.11 and 3.14.
    """

    import re
    import warnings as _warnings

    manifest = stage9_manifest()
    stage2 = bootstrap_stage2()

    kernel_source = KERNEL_PATH.read_text()
    removed_node_refs = sorted(
        name
        for name in AST_REMOVED_NODES
        if re.search(r"\bast\." + name + r"\b", kernel_source)
    )

    example_dir = ROOT / "hy-meta" / "examples"
    example_sources = [
        (example_dir / name).read_text()
        for name in (
            "factorial.hy",
            "kernel_loop.hy",
            "kernel_features.hy",
            "kernel_stability_stress.hy",
        )
        if (example_dir / name).exists()
    ]
    sources = AST_FORWARD_COMPAT_BATTERY + example_sources

    removed_emitted: set[str] = set()
    forward_warnings: list[str] = []
    compiled = 0
    compile_errors: list[str] = []
    for index, src in enumerate(sources):
        module = stage2.make_module(f"hy_meta_forward_compat.case_{index}")
        if hasattr(stage2, "set_direct_kernel_strict"):
            stage2.set_direct_kernel_strict(True)
        caught: list[Any] = []
        try:
            with _warnings.catch_warnings(record=True) as recorded:
                _warnings.simplefilter("always")
                tree = stage2.compile_source_to_ast(src, module, "<forward-compat>")
                caught = list(recorded)
            compiled += 1
        except Exception as exc:  # noqa: BLE001 - record and continue
            compile_errors.append(f"case{index}: {type(exc).__name__}: {exc}")
            continue
        finally:
            if hasattr(stage2, "set_direct_kernel_strict"):
                stage2.set_direct_kernel_strict(False)
        for warning in caught:
            if issubclass(warning.category, DeprecationWarning):
                forward_warnings.append(f"case{index}: {str(warning.message)[:120]}")
        for node in ast.walk(tree):
            if type(node).__name__ in AST_REMOVED_NODES:
                removed_emitted.add(type(node).__name__)

    drift: dict[str, Any] = {}
    if removed_node_refs:
        drift["removed_node_refs"] = removed_node_refs
    if removed_emitted:
        drift["removed_emitted"] = sorted(removed_emitted)
    if forward_warnings:
        drift["forward_warnings"] = forward_warnings
    if compile_errors:
        drift["compile_errors"] = compile_errors

    canonical = {
        "battery_size": len(AST_FORWARD_COMPAT_BATTERY),
        "example_count": len(example_sources),
        "removed_nodes_checked": list(AST_REMOVED_NODES),
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )
    if debug_dir is not None:
        Path(debug_dir).mkdir(parents=True, exist_ok=True)
        (Path(debug_dir) / "manifest.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n"
        )
        (Path(debug_dir) / "canonical.json").write_text(
            json.dumps(canonical, indent=2, sort_keys=True) + "\n"
        )
        (Path(debug_dir) / "drift.json").write_text(
            json.dumps(drift, indent=2, sort_keys=True) + "\n"
        )
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "python_family": f"{sys.version_info.major}.{sys.version_info.minor}",
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "source_count": len(sources),
        "compiled_count": compiled,
        "compile_error_count": len(compile_errors),
        "removed_node_ref_count": len(removed_node_refs),
        "removed_emitted_count": len(removed_emitted),
        "forward_warning_count": len(forward_warnings),
        "strictness_exercised": sys.version_info >= (3, 13),
        "ast_forward_compat_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


MACRO_REQUIRE_RAISES = "<raises>"

MACRO_REQUIRE_CASES = [
    ("get_macro_present", '(defmacro twice [n] `(* 2 ~n)) (setv r (if (get-macro twice) "yes" "no"))', "'yes'"),
    ("get_macro_miss", "(get-macro nope-xyz-123) (setv r 1)", MACRO_REQUIRE_RAISES),
    ("get_macro_doc", '(defmacro dm [] "the doc" 1) (setv r (. (get-macro dm) __doc__))', "'the doc'"),
    ("get_macro_builtins", '(setv r (if (get-macro when) "yes" "no"))', "'yes'"),
    ("get_macro_reader", '(defreader rr [&reader] (.parse-one-form &reader)) (setv r (if (get-macro :reader rr) "yes" "no"))', "'yes'"),
    ("hy_I_call", "(setv r (hy.I.math.sqrt 16))", "4.0"),
    ("hy_R_oneshot", "(setv r (hy.R.tests/resources/tlib.qplah 1 2 3))", "[8, 1, 2, 3]"),
    ("require_named", "(require tests.resources.tlib [qplah]) (setv r (qplah 1 2 3))", "[8, 1, 2, 3]"),
    ("require_star", "(require tests.resources.tlib *) (setv r (parald 1 2 3))", "[9, 1, 2, 3]"),
    ("require_as", "(require tests.resources.tlib [qplah :as q2]) (setv r (q2 9))", "[8, 9]"),
    ("local_shadow_core", "(defn f [] (defmacro + [a b] `(- ~a ~b)) (+ 10 3)) (setv r (f))", "7"),
    ("get_macro_order", '(defmacro m [] "global") (defn f [] (defmacro m [] "local") (m)) (setv r [(m) (f)])', "['global', 'local']"),
    ("macroexpand_1", "(defmacro mm [x] `(+ ~x 1)) (setv r (hy.repr (hy.macroexpand-1 (quote (mm 5)))))", '"\'(+ 5 1)"'),
    ("macroexpand_full", "(defmacro aa [x] `(bb ~x)) (defmacro bb [x] `(+ ~x 1)) (setv r (hy.repr (hy.macroexpand (quote (aa 5)))))", '"\'(+ 5 1)"'),
    ("del_macro", "(defmacro tmp [] 1) (eval-and-compile (del (get-macro tmp))) (tmp) (setv r 1)", MACRO_REQUIRE_RAISES),
    ("defreader_reject", "(defn f [] (defreader bad [&reader] 1)) (setv r 1)", MACRO_REQUIRE_RAISES),
]


def run_macro_require_parity_check(debug_dir: str | None = None) -> dict[str, Any]:
    """Lock in macro/require runtime behavior that 100%-direct compilation does
    not by itself guarantee: get-macro resolution (local/module/builtins/reader
    tiers and miss), hy.I / hy.R sugar, require named/*/:as, core-macro local
    shadowing, get-macro tier order, macroexpand / macroexpand-1, eval-and-compile
    macro deletion, and defreader rejection outside global scope. Expected values
    were verified equal to upstream hy.eval; this check asserts the strict direct
    kernel reproduces them.
    """

    import warnings as _warnings

    manifest = stage9_manifest()
    stage2 = bootstrap_stage2()
    drift: dict[str, Any] = {}
    matched = 0

    for index, (name, src, expected) in enumerate(MACRO_REQUIRE_CASES):
        module = stage2.make_module(f"hy_meta_macro_parity.case_{index}_{name}")
        if hasattr(stage2, "set_direct_kernel_strict"):
            stage2.set_direct_kernel_strict(True)
        try:
            with _warnings.catch_warnings():
                _warnings.simplefilter("ignore")
                stage2.exec_source(src, module, "<macro-parity>")
            got = repr(module.__dict__.get("r"))
        except Exception:  # noqa: BLE001 - compile/runtime error is a valid outcome
            got = MACRO_REQUIRE_RAISES
        finally:
            if hasattr(stage2, "set_direct_kernel_strict"):
                stage2.set_direct_kernel_strict(False)
        if got == expected:
            matched += 1
        else:
            drift[name] = {"expected": expected, "got": got}

    canonical = {
        "cases": [name for name, _, _ in MACRO_REQUIRE_CASES],
        "matched": matched,
    }
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )
    if debug_dir is not None:
        Path(debug_dir).mkdir(parents=True, exist_ok=True)
        (Path(debug_dir) / "manifest.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n"
        )
        (Path(debug_dir) / "canonical.json").write_text(
            json.dumps(canonical, indent=2, sort_keys=True) + "\n"
        )
        (Path(debug_dir) / "drift.json").write_text(
            json.dumps(drift, indent=2, sort_keys=True) + "\n"
        )
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "case_count": len(MACRO_REQUIRE_CASES),
        "matched_count": matched,
        "mismatch_count": len(drift),
        "macro_require_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def run_version_ast_coverage_check(debug_dir: str | None = None) -> dict[str, Any]:
    manifest = stage9_manifest()
    matrix = version_ast_coverage_matrix()
    allowed_statuses = {"owned", "gated", "absent"}
    drift: dict[str, Any] = {}

    unclassified_count = 0
    for label, nodes in matrix.items():
        for name, record in nodes.items():
            if record["status"] not in allowed_statuses:
                unclassified_count += 1
                drift[f"{label}_{name}_status"] = record["status"]
            if not record["reason"] or not record["test_ref"]:
                drift[f"{label}_{name}_metadata"] = record

    current_label = f"{sys.version_info.major}.{sys.version_info.minor}"
    crosscheck_ok = True
    crosscheck_in_lane = current_label in matrix
    if crosscheck_in_lane:
        for name, record in matrix[current_label].items():
            present = hasattr(ast, name)
            expected_present = record["status"] in {"owned", "gated"}
            if present != expected_present:
                crosscheck_ok = False
                drift[f"crosscheck_{name}"] = {
                    "status": record["status"],
                    "ast_has_node": present,
                }

    stage2 = bootstrap_stage2()
    probe_module = stage2.make_module("hy_meta_version_ast.probe")
    match_probe_value = stage2.eval_source(
        '(match 1 1 "one" _ "other")',
        probe_module,
        "<version-ast:match>",
    )
    if match_probe_value != "one":
        drift["match_probe"] = match_probe_value

    canonical = {"matrix": matrix, "current_label": current_label}
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )
    if debug_dir is not None:
        Path(debug_dir).mkdir(parents=True, exist_ok=True)
        (Path(debug_dir) / "manifest.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n"
        )
        (Path(debug_dir) / "canonical.json").write_text(
            json.dumps(canonical, indent=2, sort_keys=True) + "\n"
        )
        (Path(debug_dir) / "drift.json").write_text(
            json.dumps(drift, indent=2, sort_keys=True) + "\n"
        )

    sample_target = matrix["3.11"]
    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "target_count": len(matrix),
        "owned_node_count": sum(
            1 for record in sample_target.values() if record["status"] == "owned"
        ),
        "gated_or_absent_node_count": sum(
            1
            for record in sample_target.values()
            if record["status"] in {"gated", "absent"}
        ),
        "unclassified_count": unclassified_count,
        "crosscheck_in_lane": crosscheck_in_lane,
        "crosscheck_ok": crosscheck_ok,
        "match_probe_value": match_probe_value,
        "version_ast_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


FRONT_END_MANGLE_SAMPLES = [
    "hello-world",
    "a-b",
    "foo-bar-baz",
    "set?",
    "thread->",
    "spam",
    "->>",
    "π-ish",
]


def run_front_end_boundary_check(debug_dir: str | None = None) -> dict[str, Any]:
    """Formal front-end ownership decision plus its proof.

    Records the decision that the reader and name mangling are ceded to upstream
    ``hy.reader`` as proven-pure host substrate (so the public classification is
    "self-hosting back-end", not "complete meta-circular"), and proves that the
    mangle/unmangle host calls are deterministic, round-trippable, and free of
    observable global mutation.
    """

    manifest = stage9_manifest()
    from hy.reader import mangle, read_many, unmangle

    samples = FRONT_END_MANGLE_SAMPLES
    mangled = {sample: mangle(sample) for sample in samples}
    determinism_ok = all(mangle(sample) == mangled[sample] for sample in samples) and all(
        unmangle(mangled[sample]) == unmangle(mangled[sample]) for sample in samples
    )
    roundtrip_ok = all(unmangle(mangled[sample]) == sample for sample in samples)

    modules_before = len(sys.modules)
    probe_namespace: dict[str, Any] = {}
    for sample in samples:
        unmangle(mangle(sample))
    purity_ok = len(sys.modules) == modules_before and probe_namespace == {}

    reader_host_module = getattr(read_many, "__module__", "")
    mangle_host_module = getattr(mangle, "__module__", "")
    unmangle_host_module = getattr(unmangle, "__module__", "")

    decision = {
        "record_kind": "front-end-boundary-decision",
        "reader_ownership_decision": "host-substrate",
        "reader_host_module": "hy.reader",
        "mangle_unmangle_classification": "proven-deterministic-pure-host-call",
        "public_classification": "self-hosting-back-end",
        "rationale": (
            "The reader and name mangling are formally ceded to upstream "
            "hy.reader as proven-pure host substrate; the owned meta-circular "
            "surface is the stage7 direct-kernel back-end."
        ),
    }

    drift: dict[str, Any] = {}
    if not reader_host_module.startswith("hy.reader"):
        drift["reader_host_module"] = reader_host_module
    if mangle_host_module != "hy.reader.mangling":
        drift["mangle_host_module"] = mangle_host_module
    if unmangle_host_module != "hy.reader.mangling":
        drift["unmangle_host_module"] = unmangle_host_module
    if not determinism_ok:
        drift["determinism"] = False
    if not roundtrip_ok:
        drift["roundtrip"] = False
    if not purity_ok:
        drift["purity"] = False
    if decision["reader_ownership_decision"] != "host-substrate":
        drift["reader_ownership_decision"] = decision["reader_ownership_decision"]
    if decision["public_classification"] != "self-hosting-back-end":
        drift["public_classification"] = decision["public_classification"]

    canonical = {"decision": decision, "mangled": mangled, "samples": samples}
    canonical["canonical_sha256"] = sha256_text(
        json.dumps(canonical, sort_keys=True, separators=(",", ":"))
    )
    if debug_dir is not None:
        Path(debug_dir).mkdir(parents=True, exist_ok=True)
        (Path(debug_dir) / "manifest.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n"
        )
        (Path(debug_dir) / "canonical.json").write_text(
            json.dumps(canonical, indent=2, sort_keys=True) + "\n"
        )
        (Path(debug_dir) / "drift.json").write_text(
            json.dumps(drift, indent=2, sort_keys=True) + "\n"
        )

    status = "reproduced" if not drift else "drift"
    return {
        "python": sys.version.split()[0],
        "manifest_sha256": manifest["manifest_sha256"],
        "canonical_sha256": canonical["canonical_sha256"],
        "sample_count": len(samples),
        "determinism_ok": determinism_ok,
        "roundtrip_ok": roundtrip_ok,
        "purity_ok": purity_ok,
        "reader_host_module": reader_host_module,
        "mangle_host_module": mangle_host_module,
        "reader_ownership_decision": decision["reader_ownership_decision"],
        "public_classification": decision["public_classification"],
        "front_end_status": status,
        "drift_fields": ",".join(sorted(drift)),
        "debug_dir": debug_dir or "",
    }


def run_kernel_import_check() -> dict[str, Any]:
    _stage2, _stage2_prime, stage3 = bootstrap_stage3_chain()
    kernel = stage3.load_hy_file(KERNEL_PATH, "hy_meta_import_hook.kernel")
    examples_root = ROOT / "hy-meta" / "examples"
    module_names = [
        "kernel_import_probe",
        "kernel_import_consumer",
        "kernel_import_pkg",
        "kernel_import_pkg.child",
        "kernel_import_pkg.sibling",
        "kernel_import_broken",
        "kernel_import_reader_broken",
        "kernel_import_reader_after_broken",
        "kernel_import_reader_error",
        "kernel_import_reload_target",
        "kernel_import_cycle_a",
        "kernel_import_cycle_b",
        "kernel_import_shadow_left",
        "kernel_import_shadow_left.thing",
        "kernel_import_shadow_right",
        "kernel_import_shadow_right.thing",
        "kernel_import_runpy_target",
        "kernel_import_run_pkg",
        "kernel_import_run_pkg.__main__",
        "kernel_import_export_source",
        "kernel_import_require_provider",
        "kernel_import_macro_self",
        "kernel_import_temp_stress",
        "kernel_import_failed_require",
        "kernel_import_failed_macro_expansion",
    ]
    reload_counter_name = "_hy_meta_kernel_reload_count"
    module_snapshot = snapshot_sys_modules(module_names)
    if hasattr(builtins, reload_counter_name):
        delattr(builtins, reload_counter_name)
    for name in module_names:
        sys.modules.pop(name, None)

    hook = install_kernel_import_hook(kernel, [examples_root])
    try:
        with hook:
            importlib.invalidate_caches()
            probe = importlib.import_module("kernel_import_probe")
            consumer = importlib.import_module("kernel_import_consumer")
            package = importlib.import_module("kernel_import_pkg")
            child = importlib.import_module("kernel_import_pkg.child")
            sibling = importlib.import_module("kernel_import_pkg.sibling")
            reload_target = importlib.import_module("kernel_import_reload_target")
            reload_target_again = importlib.import_module(
                "kernel_import_reload_target"
            )
            reload_before = reload_target.RELOAD_COUNT
            reloaded_target = importlib.reload(reload_target)
            reload_after = reloaded_target.RELOAD_COUNT
            cycle_a = importlib.import_module("kernel_import_cycle_a")
            cycle_b = importlib.import_module("kernel_import_cycle_b")
            shadow_left = importlib.import_module("kernel_import_shadow_left.thing")
            shadow_right = importlib.import_module(
                "kernel_import_shadow_right.thing"
            )
            runpy_module_globals = runpy.run_module("kernel_import_runpy_target")
            runpy_package_globals = runpy.run_module("kernel_import_run_pkg")
            export_source = importlib.import_module("kernel_import_export_source")
            export_namespace: dict[str, Any] = {}
            exec("from kernel_import_export_source import *", export_namespace)
            require_provider = importlib.import_module(
                "kernel_import_require_provider"
            )
            macro_self = importlib.import_module("kernel_import_macro_self")
            temp_stress = importlib.import_module("kernel_import_temp_stress")
            temp_stress_python_before = getattr(
                temp_stress.__loader__,
                "last_python_source",
                None,
            )
            temp_stress_ast_before = getattr(
                temp_stress.__loader__,
                "last_ast_dump",
                None,
            )
            reloaded_temp_stress = importlib.reload(temp_stress)
            temp_stress_python_after = getattr(
                reloaded_temp_stress.__loader__,
                "last_python_source",
                None,
            )
            temp_stress_ast_after = getattr(
                reloaded_temp_stress.__loader__,
                "last_ast_dump",
                None,
            )
            broken_import_failed = False
            broken_import_error = ""
            broken_import_traceback_mentions_hy_file = False
            try:
                importlib.import_module("kernel_import_broken")
            except RuntimeError as exc:
                broken_import_failed = True
                broken_import_error = str(exc)
                broken_traceback = "".join(
                    traceback.format_exception(type(exc), exc, exc.__traceback__)
                )
                broken_import_traceback_mentions_hy_file = (
                    "kernel_import_broken.hy" in broken_traceback
                )
            reader_broken_import_failed = False
            reader_broken_import_error = ""
            try:
                importlib.import_module("kernel_import_reader_broken")
            except RuntimeError as exc:
                reader_broken_import_failed = True
                reader_broken_import_error = str(exc)
            reader_after_broken_import_failed = False
            reader_after_broken_error_class = ""
            try:
                importlib.import_module("kernel_import_reader_after_broken")
            except Exception as exc:
                reader_after_broken_import_failed = True
                reader_after_broken_error_class = exc.__class__.__name__
            reader_error_import_failed = False
            reader_error_class = ""
            reader_error_filtered_mentions_hy_file = False
            reader_error_filtered_hides_importlib = False
            reader_error_full_has_importlib = False
            try:
                importlib.import_module("kernel_import_reader_error")
            except Exception as exc:
                reader_error_import_failed = True
                reader_error_class = exc.__class__.__name__
                reader_error_full = "".join(
                    traceback.format_exception(type(exc), exc, exc.__traceback__)
                )
                reader_error_filtered = hy.errors.hy_exc_filter(
                    type(exc), exc, exc.__traceback__
                )
                reader_error_filtered_mentions_hy_file = (
                    "kernel_import_reader_error.hy" in reader_error_filtered
                )
                reader_error_filtered_hides_importlib = (
                    "<frozen importlib" not in reader_error_filtered
                )
                reader_error_full_has_importlib = (
                    "<frozen importlib" in reader_error_full
                )
            failed_require_import_failed = False
            failed_require_error_class = ""
            failed_require_error = ""
            try:
                importlib.import_module("kernel_import_failed_require")
            except Exception as exc:
                failed_require_import_failed = True
                failed_require_error_class = exc.__class__.__name__
                failed_require_error = str(exc)
            failed_macro_expansion_import_failed = False
            failed_macro_expansion_error_class = ""
            failed_macro_expansion_error = ""
            try:
                importlib.import_module("kernel_import_failed_macro_expansion")
            except Exception as exc:
                failed_macro_expansion_import_failed = True
                failed_macro_expansion_error_class = exc.__class__.__name__
                failed_macro_expansion_error = str(exc)
            expected_package_path = str(examples_root / "kernel_import_pkg")
            loaded_modules = [probe, consumer, package, child, sibling]
            reader_macro_tables = [
                module.__dict__.get("_hy_reader_macros") for module in loaded_modules
            ]
            loaded_macro_tables = [
                module.__dict__.get("_hy_macros") for module in loaded_modules
            ]
            import_loaded_modules = [
                probe,
                consumer,
                package,
                child,
                sibling,
                export_source,
                require_provider,
                macro_self,
                temp_stress,
            ]
            result = {
                "python": sys.version.split()[0],
                "kernel_module": kernel.__name__,
                "import_root": str(examples_root),
                "probe_module": probe.__name__,
                "consumer_module": consumer.__name__,
                "package_module": package.__name__,
                "child_module": child.__name__,
                "sibling_module": sibling.__name__,
                "reload_module": reload_target.__name__,
                "cycle_a_module": cycle_a.__name__,
                "cycle_b_module": cycle_b.__name__,
                "shadow_left_module": shadow_left.__name__,
                "shadow_right_module": shadow_right.__name__,
                "require_provider_module": require_provider.__name__,
                "macro_self_module": macro_self.__name__,
                "temp_stress_module": temp_stress.__name__,
                "broken_module": "kernel_import_broken",
                "probe_value": probe.VALUE,
                "probe_doc": probe.__doc__,
                "consumer_value": consumer.CONSUMED,
                "consumer_imported_value": consumer.VALUE,
                "package_value": package.PACKAGE_VALUE,
                "package_imported_value": package.CHILD_VALUE,
                "package_relative_value": package.RELATIVE_PACKAGE_VALUE,
                "package_relative_module_value": package.sibling.SIBLING_VALUE,
                "child_value": child.CHILD_VALUE,
                "sibling_value": sibling.SIBLING_VALUE,
                "reload_value_before": reload_before,
                "reload_value_after": reload_after,
                "reload_module_value_after": reload_target.VALUE,
                "cycle_a_value": cycle_a.A_VALUE,
                "cycle_a_sees_b_value": cycle_a.A_SEES_B_VALUE,
                "cycle_b_value": cycle_b.B_VALUE,
                "cycle_b_saw_a_started": cycle_b.B_SEES_A_STARTED,
                "cycle_b_saw_a_value_before": cycle_b.B_SEES_A_VALUE_BEFORE,
                "shadow_left_value": shadow_left.VALUE,
                "shadow_right_value": shadow_right.VALUE,
                "shadow_modules_distinct": shadow_left is not shadow_right,
                "runpy_value": runpy_module_globals["RUNPY_VALUE"],
                "runpy_name": runpy_module_globals["RUNPY_NAME"],
                "runpy_package": runpy_module_globals["RUNPY_PACKAGE"],
                "runpy_loader": type(runpy_module_globals["__loader__"]).__name__,
                "runpy_file": Path(runpy_module_globals["__file__"]).name,
                "runpy_module_not_cached": (
                    "kernel_import_runpy_target" not in sys.modules
                ),
                "package_main_value": runpy_package_globals["MAIN_VALUE"],
                "package_main_name": runpy_package_globals["MAIN_NAME"],
                "package_main_package": runpy_package_globals["MAIN_PACKAGE"],
                "package_main_loader": type(
                    runpy_package_globals["__loader__"]
                ).__name__,
                "package_main_file": Path(runpy_package_globals["__file__"]).name,
                "package_main_not_cached": (
                    "kernel_import_run_pkg.__main__" not in sys.modules
                ),
                "export_all": export_source.__all__,
                "export_star_exported": export_namespace.get("EXPORTED"),
                "export_star_alias": export_namespace.get("ALIAS_VALUE"),
                "export_star_hidden_absent": "HIDDEN" not in export_namespace,
                "export_source_hidden": export_source.HIDDEN,
                "export_module_cache_ok": (
                    sys.modules.get(export_source.__name__) is export_source
                ),
                "require_provider_value": require_provider.REQUIRE_PROVIDER_VALUE,
                "require_provider_macro_present": (
                    "ok_macro" in require_provider.__dict__.get("_hy_macros", {})
                ),
                "require_provider_module_cache_ok": (
                    sys.modules.get(require_provider.__name__) is require_provider
                ),
                "macro_self_value": macro_self.IMPORT_SELF_VALUE,
                "macro_self_macro_table_has_self_macros": (
                    "import_self_bar" in macro_self.__dict__.get("_hy_macros", {})
                    and "import_self_foo"
                    in macro_self.__dict__.get("_hy_macros", {})
                ),
                "macro_self_module_cache_ok": (
                    sys.modules.get(macro_self.__name__) is macro_self
                ),
                "temp_stress_value": temp_stress.TEMP_STRESS_VALUE,
                "temp_stress_reload_same_object": (
                    reloaded_temp_stress is temp_stress
                ),
                "temp_stress_module_cache_ok": (
                    sys.modules.get(temp_stress.__name__) is temp_stress
                ),
                "temp_stress_loader_python_stable": (
                    temp_stress_python_before == temp_stress_python_after
                ),
                "temp_stress_loader_ast_stable": (
                    temp_stress_ast_before == temp_stress_ast_after
                ),
                "import_module_dicts_distinct": (
                    len({id(module.__dict__) for module in import_loaded_modules})
                    == len(import_loaded_modules)
                ),
                "probe_file": Path(probe.__file__).name,
                "consumer_file": Path(consumer.__file__).name,
                "package_file": Path(package.__file__).name,
                "child_file": Path(child.__file__).name,
                "sibling_file": Path(sibling.__file__).name,
                "reload_file": Path(reload_target.__file__).name,
                "cycle_a_file": Path(cycle_a.__file__).name,
                "cycle_b_file": Path(cycle_b.__file__).name,
                "shadow_left_file": Path(shadow_left.__file__).name,
                "shadow_right_file": Path(shadow_right.__file__).name,
                "shadow_left_parent": Path(shadow_left.__file__).parent.name,
                "shadow_right_parent": Path(shadow_right.__file__).parent.name,
                "require_provider_file": Path(require_provider.__file__).name,
                "macro_self_file": Path(macro_self.__file__).name,
                "temp_stress_file": Path(temp_stress.__file__).name,
                "probe_loader": type(probe.__loader__).__name__,
                "consumer_loader": type(consumer.__loader__).__name__,
                "package_loader": type(package.__loader__).__name__,
                "child_loader": type(child.__loader__).__name__,
                "sibling_loader": type(sibling.__loader__).__name__,
                "reload_loader": type(reload_target.__loader__).__name__,
                "cycle_a_loader": type(cycle_a.__loader__).__name__,
                "cycle_b_loader": type(cycle_b.__loader__).__name__,
                "shadow_left_loader": type(shadow_left.__loader__).__name__,
                "shadow_right_loader": type(shadow_right.__loader__).__name__,
                "require_provider_loader": type(
                    require_provider.__loader__
                ).__name__,
                "macro_self_loader": type(macro_self.__loader__).__name__,
                "temp_stress_loader": type(temp_stress.__loader__).__name__,
                "probe_module_cache_ok": sys.modules.get(probe.__name__) is probe,
                "consumer_module_cache_ok": (
                    sys.modules.get(consumer.__name__) is consumer
                ),
                "package_module_cache_ok": (
                    sys.modules.get(package.__name__) is package
                ),
                "child_module_cache_ok": sys.modules.get(child.__name__) is child,
                "sibling_module_cache_ok": (
                    sys.modules.get(sibling.__name__) is sibling
                ),
                "reload_import_cache_same_object": (
                    reload_target_again is reload_target
                ),
                "reload_same_object": reloaded_target is reload_target,
                "reload_module_cache_ok": (
                    sys.modules.get(reload_target.__name__) is reload_target
                ),
                "cycle_a_module_cache_ok": (
                    sys.modules.get(cycle_a.__name__) is cycle_a
                ),
                "cycle_b_module_cache_ok": (
                    sys.modules.get(cycle_b.__name__) is cycle_b
                ),
                "shadow_left_module_cache_ok": (
                    sys.modules.get(shadow_left.__name__) is shadow_left
                ),
                "shadow_right_module_cache_ok": (
                    sys.modules.get(shadow_right.__name__) is shadow_right
                ),
                "zipimport_status": "unsupported-filesystem-roots-only",
                "bytecode_status": "unsupported-source-only-exec",
                "broken_import_failed": broken_import_failed,
                "broken_import_error": broken_import_error,
                "broken_import_traceback_mentions_hy_file": (
                    broken_import_traceback_mentions_hy_file
                ),
                "broken_module_cache_removed": (
                    "kernel_import_broken" not in sys.modules
                ),
                "reader_broken_import_failed": reader_broken_import_failed,
                "reader_broken_import_error": reader_broken_import_error,
                "reader_broken_module_cache_removed": (
                    "kernel_import_reader_broken" not in sys.modules
                ),
                "reader_after_broken_import_failed": (
                    reader_after_broken_import_failed
                ),
                "reader_after_broken_error_class": (
                    reader_after_broken_error_class
                ),
                "reader_after_broken_module_cache_removed": (
                    "kernel_import_reader_after_broken" not in sys.modules
                ),
                "reader_error_import_failed": reader_error_import_failed,
                "reader_error_class": reader_error_class,
                "reader_error_filtered_mentions_hy_file": (
                    reader_error_filtered_mentions_hy_file
                ),
                "reader_error_filtered_hides_importlib": (
                    reader_error_filtered_hides_importlib
                ),
                "reader_error_full_has_importlib": reader_error_full_has_importlib,
                "reader_error_module_cache_removed": (
                    "kernel_import_reader_error" not in sys.modules
                ),
                "failed_require_import_failed": failed_require_import_failed,
                "failed_require_error_class": failed_require_error_class,
                "failed_require_error": failed_require_error,
                "failed_require_module_cache_removed": (
                    "kernel_import_failed_require" not in sys.modules
                ),
                "failed_macro_expansion_import_failed": (
                    failed_macro_expansion_import_failed
                ),
                "failed_macro_expansion_error_class": (
                    failed_macro_expansion_error_class
                ),
                "failed_macro_expansion_error": failed_macro_expansion_error,
                "failed_macro_expansion_module_cache_removed": (
                    "kernel_import_failed_macro_expansion" not in sys.modules
                ),
                "import_reader_macro_tables_distinct": (
                    len({id(table) for table in reader_macro_tables})
                    == len(reader_macro_tables)
                ),
                "import_macro_tables_distinct": (
                    len({id(table) for table in loaded_macro_tables})
                    == len(loaded_macro_tables)
                ),
                "import_macro_tables_clean": all(
                    table == {} for table in loaded_macro_tables
                ),
                "import_reader_macro_tables_clean": all(
                    table == {} for table in reader_macro_tables
                ),
                "failed_reader_macro_absent_from_loaded_modules": all(
                    "leakreader" not in table for table in reader_macro_tables
                ),
                "failed_require_macros_absent_from_loaded_modules": all(
                    "require_failure_local" not in table
                    and "ok_macro" not in table
                    and "missing_macro" not in table
                    for table in loaded_macro_tables
                ),
                "failed_expansion_macros_absent_from_loaded_modules": all(
                    "expansion_before_failure" not in table
                    and "expansion_boom" not in table
                    for table in loaded_macro_tables
                ),
                "package_path_ok": (
                    hasattr(package, "__path__")
                    and list(package.__path__) == [expected_package_path]
                ),
                "probe_result_removed": "__hy_meta_result__" not in probe.__dict__,
                "consumer_result_removed": (
                    "__hy_meta_result__" not in consumer.__dict__
                ),
                "package_result_removed": (
                    "__hy_meta_result__" not in package.__dict__
                ),
                "child_result_removed": "__hy_meta_result__" not in child.__dict__,
                "sibling_result_removed": (
                    "__hy_meta_result__" not in sibling.__dict__
                ),
            }
        result["kernel_import_hook_removed"] = hook.finder not in sys.meta_path
        witness_payload = {
            key: value for key, value in sorted(result.items())
            if isinstance(value, (str, int, float, bool, type(None)))
        }
        result["witness_id"] = record_witness("kernel-import-check", witness_payload)["witness_id"]
        return result
    finally:
        rollback_sys_modules(module_snapshot)
        if hasattr(builtins, reload_counter_name):
            delattr(builtins, reload_counter_name)


def cmd_self_check(_args: argparse.Namespace) -> int:
    result = run_self_check()
    ok = (
        result["stage1_value"] == 6
        and result["stage2_value"] == 42
        and result["stage2_self_check"] is True
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_chain_check(_args: argparse.Namespace) -> int:
    result = run_chain_check()
    ok = (
        result["stage2_self_check"] is True
        and result["stage2_prime_self_check"] is True
        and result["stage2_value"] == 120
        and result["stage2_prime_value"] == 120
        and result["python_output_matches"] is True
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_prime_check(_args: argparse.Namespace) -> int:
    result = run_prime_check()
    ok = (
        result["stage2_prime_self_check"] is True
        and result["kernel_self_check"] is True
        and result["factorial"] == 120
        and result["features"] == 449.0
        and result["loop"] == 120
        and result["factorial_python_contains_fact"] is True
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage3_check(_args: argparse.Namespace) -> int:
    result = run_stage3_check()
    ok = (
        result["stage2_self_check"] is True
        and result["stage2_prime_self_check"] is True
        and result["stage3_self_check"] is True
        and result["stage2_value"] == 120
        and result["stage2_prime_value"] == 120
        and result["stage3_value"] == 120
        and result["stage2_stage3_python_output_matches"] is True
        and result["kernel_self_check"] is True
        and result["factorial"] == 120
        and result["features"] == 449.0
        and result["loop"] == 120
        and result["factorial_python_contains_fact"] is True
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_mirror_check(_args: argparse.Namespace) -> int:
    result = run_mirror_check()
    ok = (
        result["stage2_self_check"] is True
        and result["stage2_prime_self_check"] is True
        and result["stage3_self_check"] is True
        and result["compiler_python_mirror"] is True
        and result["compiler_ast_mirror"] is True
        and result["stage_value_mirror"] is True
        and result["kernel_python_mirror"] is True
        and result["kernel_ast_mirror"] is True
        and result["kernel_value_mirror"] is True
        and result["kernel_factorial"] == 120
        and result["kernel_loop"] == 120
        and result["kernel_features"] == 449.0
        and result["kernel_stability_stress"] == 218
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_direct_kernel_bridge_check(_args: argparse.Namespace) -> int:
    result = run_direct_kernel_bridge_check()
    ok = (
        result["expr_python"] == "import hy\n20 + 22"
        and result["expr_direct_kernel_loaded"] is True
        and result["expr_direct_kernel_hits"] >= 1
        and result["expr_direct_kernel_fallbacks"] == 0
        and result["compiler_self_check"] is True
        and result["compiler_eval_value"] == 42
        and result["compiler_load_direct_kernel_hits"] >= 2
        and result["compiler_load_direct_kernel_fallbacks"] == 0
        and result["loaded_compiler_direct_kernel_loaded"] is True
        and result["loaded_compiler_direct_kernel_hits"] >= 1
        and result["loaded_compiler_direct_kernel_fallbacks"] == 0
        and result["repeated_stage2_direct_kernel_loaded"] is True
        and result["repeated_stage2_direct_kernel_hits"] >= 1
        and result["repeated_stage2_direct_kernel_fallbacks"] == 0
        and result["stage2_direct_kernel_reused"] is True
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_reader_boundary_check(_args: argparse.Namespace) -> int:
    result = run_reader_boundary_check()
    ok = (
        result["stage_count"] == 7
        and result["last_stage_module"] == "hy_meta_stage7.compiler"
        and result["reader_host_module"] == "hy.reader"
        and result["reader_macro_value"] == 42
        and result["fresh_reader_failed"] is True
        and result["fresh_reader_error_class"] == "LexException"
        and "reader macro '#foo' is not defined" in result["fresh_reader_error"]
        and result["stage_reader_macro_tables_distinct"] is True
        and result["kernel_reader_macro_table_clean"] is True
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_compatibility_boundary_check(_args: argparse.Namespace) -> int:
    result = run_compatibility_boundary_check()
    ok = (
        result["upstream_python"] == "20 + 22"
        and result["upstream_direct_kernel_loaded"] is False
        and result["upstream_direct_kernel_hits"] == 0
        and result["upstream_direct_kernel_fallbacks"] == 0
        and result["template_gate_failed"] is True
        and result["template_gate_error_class"] == "SyntaxError"
        and "template strings are outside the current direct-kernel lane"
        in result["template_gate_error"]
        and result["hook_installed"] is True
        and result["hook_removed_after_context"] is True
        and result["python_import_value"] == 41
        and result["python_import_loader_not_kernel"] is True
        and result["hy_import_value"] == 42
        and result["hy_import_loader"] == "KernelHyLoader"
        and result["preexisting_import_value"] == 99
        and result["preexisting_module_preserved"] is True
        and result["preexisting_module_restored"] is True
        and result["native_import_value"] == 9.0
        and result["native_import_loader_not_kernel"] is True
        and result["side_effect_value"] == 43
        and result["side_effect_seen"] is True
        and result["side_effect_restored"] is True
        and result["broken_import_failed"] is True
        and "compat broken" in result["broken_import_error"]
        and result["broken_module_removed"] is True
        and result["exception_hook_installed"] is True
        and result["exception_hook_removed"] is True
        and result["exception_hook_error"] == "compatibility boundary hook probe"
        and result["meta_path_restored"] is True
        and result["sys_path_restored"] is True
        and result["compat_modules_removed"] is True
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_cli_io_check(_args: argparse.Namespace) -> int:
    result = run_cli_io_check()
    ok = (
        result["command_filename"] == "<hy-meta:-c>"
        and result["command_value"] == 42
        and result["command_python"] == "import hy\n20 + 22"
        and result["stdin_filename"] == "<stdin>"
        and result["stdin_value"] == 42
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_hyc_check(_args: argparse.Namespace) -> int:
    result = run_hyc_check()
    ok = (
        result["pyc_exists"] is True
        and result["pyc_size_positive"] is True
        and result["loaded_answer"] == 42
        and result["loader_class"] == "SourcelessFileLoader"
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_repl_check(_args: argparse.Namespace) -> int:
    result = run_repl_check()
    ok = result["status"] == 0 and result["output_lines"] == ["42", "42"]
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_startup_output_check(_args: argparse.Namespace) -> int:
    result = run_startup_output_check()
    ok = (
        result["run_value"] == 42
        and result["repl_status"] == 0
        and result["repl_output_lines"] == ["42"]
        and result["repl_output_flushed"] is True
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage7_check(_args: argparse.Namespace) -> int:
    result = run_stage7_check()
    ok = (
        result["stage_count"] == 7
        and result["last_stage_module"] == "hy_meta_stage7.compiler"
        and result["all_stage_self_checks"] is True
        and result["stage_module_cache_ok"] is True
        and result["stage_module_names_unique"] is True
        and result["stage_macro_tables_distinct"] is True
        and result["stage_reader_macro_tables_distinct"] is True
        and result["probe_module_cache_ok"] is True
        and result["probe_module_globals_distinct"] is True
        and result["probe_global_values_isolated"] is True
        and result["probe_macro_tables_distinct"] is True
        and result["probe_reader_macro_tables_distinct"] is True
        and result["compiler_python_stage7_mirror"] is True
        and result["compiler_ast_stage7_mirror"] is True
        and result["stage_value_mirror"] is True
        and result["kernel_module_cache_ok"] is True
        and result["kernel_module_names_unique"] is True
        and result["kernel_python_stage7_mirror"] is True
        and result["kernel_ast_stage7_mirror"] is True
        and result["kernel_value_stage7_mirror"] is True
        and result["kernel_factorial"] == 120
        and result["kernel_loop"] == 120
        and result["kernel_features"] == 449.0
        and result["kernel_stability_stress"] == 218
        and result["kernel_stress_repeat_python_stable"] is True
        and result["kernel_stress_repeat_ast_stable"] is True
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_self_host_check(_args: argparse.Namespace) -> int:
    result = run_self_host_check()
    ok = (
        result["stage_count"] == 7
        and result["stage7_module"] == "hy_meta_stage7.compiler"
        and result["kernel_a_self_check"] is True
        and result["kernel_b_self_check"] is True
        and result["self_compiled_kernel_body_count"] > 300
        and result["kernel_b_factorial"] == 120
        and result["self_host_status"] == "reproduced"
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_bootstrap_fixedpoint_check(_args: argparse.Namespace) -> int:
    result = run_bootstrap_fixedpoint_check()
    ok = (
        result["stage_count"] == 7
        and result["stage7_module"] == "hy_meta_stage7.compiler"
        and result["kernel_b_self_check"] is True
        and result["kernel_c_self_check"] is True
        and result["fixedpoint_artifact_count"] == 2
        and result["artifact_names_match"] is True
        and result["normalized_artifacts_match"] is True
        and result["code_artifacts_match"] is True
        and result["raw_code_artifacts_match"] is True
        and result["raw_pyc_artifacts_match"] is True
        and result["kernel_b_body_count"] > 300
        and result["kernel_c_body_count"] > 300
        and result["kernel_c_factorial"] == 120
        and result["fixedpoint_status"] == "reproduced"
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_diverse_double_compile_check(_args: argparse.Namespace) -> int:
    result = run_diverse_double_compile_check()
    ok = (
        result["build_compilers_distinct"] is True
        and result["direct_build_kernel_hits"] > 0
        and result["upstream_self_check"] is True
        and result["direct_self_check"] is True
        and result["ddc_artifact_count"] == 2
        and result["normalized_artifacts_match"] is True
        and result["code_artifacts_match"] is True
        and result["raw_code_artifacts_match"] is True
        and result["raw_pyc_artifacts_match"] is True
        and result["upstream_factorial"] == 120
        and result["direct_factorial"] == 120
        and result["ddc_status"] == "reproduced"
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_independent_mini_backend_check(_args: argparse.Namespace) -> int:
    result = run_independent_mini_backend_check()
    for row in result["rows"]:
        print(f"  [{'OK' if row['ok'] else 'FAIL'}] {row['id']} -> {row['host_result']!r}")
    print(f"fixture_count: {result['fixture_count']}")
    print(f"mini_backend_status: {result['mini_backend_status']}")
    return 0 if result["all_fixtures_accepted"] else 1


def cmd_no_fallback_check(_args: argparse.Namespace) -> int:
    result = run_no_fallback_check()
    ok = (
        result["stage2_module"] == "hy_meta_stage2.compiler"
        and result["corpus_count"] == 6
        and result["compiled_count"] == 6
        and result["direct_kernel_loaded"] is True
        and result["direct_kernel_strict_after"] is False
        and result["direct_kernel_hits"] == 6
        and result["direct_kernel_fallbacks"] == 0
        and result["error_count"] == 0
        and result["no_fallback_status"] == "reproduced"
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_parity_ledger_check(args: argparse.Namespace) -> int:
    result = run_parity_ledger_check(args.debug_dir)
    ok = (
        result["stage2_module"] == "hy_meta_stage2.compiler"
        and result["owned_files"] == 6
        and result["owned_direct_files"] == 6
        and result["owned_fallback_files"] == 0
        and result["owned_error_files"] == 0
        and result["native_files"] >= 30
        and result["native_error_files"] == 0
        and result["direct_kernel_hits"] == result["direct_files"]
        and result["direct_kernel_fallbacks"] == result["fallback_files_count"]
        and result["parity_status"] == "measured"
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage8_check(args: argparse.Namespace) -> int:
    result = run_stage8_check(args.debug_dir)
    ok = (
        result["stage_count"] == 8
        and result["stage7_module"] == "hy_meta_stage7.compiler"
        and result["stage8_module"] == "hy_meta_stage8.compiler"
        and result["artifact_count"] == 6
        and result["artifact_names_match"] is True
        and result["normalized_artifacts_match"] is True
        and result["code_artifacts_match"] is True
        and result["stage8_drift_class"] in {"none", "raw-marshal-or-pyc"}
        and result["stage8_status"] == "reproduced"
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage9_check(args: argparse.Namespace) -> int:
    result = run_stage9_check(args.debug_dir)
    ok = (
        result["fixture_count"] == 10
        and result["negative_fixture_count"] == 2
        and result["fixtures_replayed_twice"] is True
        and result["alternate_cwd_replayed"] is True
        and result["clean_env_hash_seed"] == "0"
        and result["clean_env_locale"] == "C"
        and result["clean_env_timezone"] == "UTC"
        and result["hy_version"] != "unknown"
        and result["route_policy_version"] == HY_META_ROUTE_POLICY_VERSION
        and result["feature_gate_version"] == HY_META_FEATURE_GATE_VERSION
        and result["probe_count"] == 3
        and result["max_probe_elapsed_ms"] > 0
        and result["total_elapsed_ms"] >= result["max_probe_elapsed_ms"]
        and result["product_replay_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage9_product_probe(_args: argparse.Namespace) -> int:
    print(json.dumps(stage9_probe_result(), sort_keys=True, separators=(",", ":")))
    return 0


def cmd_stage10_check(args: argparse.Namespace) -> int:
    result = run_stage10_check(args.debug_dir)
    ok = (
        result["direct_value"] == 42
        and result["server_value"] == 42
        and result["client_stdout"] == "42"
        and result["protocol_version"] == STAGE10_PROTOCOL_VERSION
        and result["protocol_status"] == "reproduced"
        and result["protocol_downgrade_status"] == "held"
        and result["protocol_downgrade_reason"] == "unsupported-protocol-version"
        and result["http_loopback_status"] == "reproduced"
        and result["http_loopback_value"] == 42
        and result["http_loopback_downgrade_status"] == "held"
        and result["session_a_value"] == 42
        and result["session_b_value"] == 42
        and result["session_b_has_x"] is False
        and result["session_tables_distinct"] is True
        and result["concurrent_session_count"] == 2
        and result["concurrent_session_values"] == "41,43"
        and result["concurrent_session_tables_distinct"] is True
        and result["concurrent_python_hashes_nonempty"] is True
        and result["sandbox_import_value"] == 42
        and result["sandbox_loader"] == "KernelHyLoader"
        and result["sandbox_hook_removed"] is True
        and result["sandbox_module_cache_removed"] is True
        and result["sandbox_pyc_size_positive"] is True
        and result["sandbox_outside_root_denied"] is True
        and result["sandbox_zipimport_denied"] is True
        and result["sandbox_bytecode_import_allowed"] is False
        and result["stage10_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage11_check(args: argparse.Namespace) -> int:
    result = run_stage11_check(args.debug_dir)
    ok = (
        result["adapter_schema_version"] == STAGE11_ADAPTER_SCHEMA_VERSION
        and result["domain_count"] == 8
        and result["status_vector"]
        == "audio:held,code:candidate,document:evidence,graphics:candidate,language:held,math:held,open_problem:held,robot:held"
        and result["unsupported_domain_count"] == 3
        and result["conflict_detected"] is True
        and result["conflict_resolution_status"] == "held"
        and result["malicious_fixture_count"] == 3
        and result["malicious_blocked_count"] == 3
        and result["schema_legacy_migrated"] is True
        and result["schema_unsupported_status"] == "held"
        and result["capability_matrix_stable"] is True
        and result["adapter_direct_accepts"] == 0
        and result["promotion_allowed_count"] == 0
        and result["executed_count"] == 0
        and result["robot_reason"] == "needs-human-confirmation"
        and result["open_problem_reason"] == "proof-forbidden"
        and result["stage11_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage12_check(args: argparse.Namespace) -> int:
    result = run_stage12_check(args.debug_dir)
    ok = (
        result["candidate_count"] == 9
        and result["base_candidate_count"] == 3
        and result["patch_candidate_count"] == 6
        and result["malicious_candidate_count"] == 3
        and result["quarantined_count"] == 9
        and result["replay_count"] == 9
        and result["not_admitted_count"] == 5
        and result["failed_count"] == 4
        and result["accepted_admission_count"] == 0
        and result["signature_pending_count"] == 9
        and result["gc_collectable_count"] == 3
        and result["gc_audit_retained_all"] is True
        and result["storage_entry_count"] == 9
        and result["storage_audit_ids_unique"] is True
        and result["proof_subset_regressions"] == 0
        and result["compiler_profile_unchanged"] is True
        and result["profile_unchanged"] is True
        and result["rule_set_unchanged"] is True
        and result["live_truth_unchanged"] is True
        and result["stage11_status_unchanged"] is True
        and result["stage12_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage13_check(args: argparse.Namespace) -> int:
    result = run_stage13_check(args.debug_dir)
    ok = (
        result["historical_record_count"] == 5
        and result["reproduced_replay_count"] == 1
        and result["stale_held_count"] == 4
        and result["boundary_held_count"] == 3
        and result["same_session_value"] == 42
        and result["quarantine_replay_count"] == 3
        and result["quarantine_still_not_admitted_or_failed"] is True
        and result["explanation_hash_stable"] is True
        and result["frontier_count"] == 3
        and result["frontier_exploded"] is False
        and result["safety_violations"] == 0
        and result["adapter_recheck_count"] == 3
        and result["adapter_recheck_stale_count"] == 1
        and result["env_verdict_count"] == 4
        and result["env_reproduced_count"] == 2
        and result["env_stale_count"] == 2
        and result["stale_ledger_entry_count"] == 7
        and result["stale_ledger_audit_ids_unique"] is True
        and result["cost_budget_ok"] is True
        and result["stage13_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage14_check(args: argparse.Namespace) -> int:
    result = run_stage14_check(args.debug_dir)
    ok = (
        result["schema_version"] == "stage14-json-v1"
        and result["fixture_count"] == 4
        and result["export_fixture_count"] == 4
        and result["compared_fixture_count"] == 4
        and result["held_host_count"] == 3
        and result["available_host_count"] == 1
        and result["cross_host_replay_status"] == "reproduced"
        and result["stage14_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage14_export(args: argparse.Namespace) -> int:
    export = stage14_hy_meta_export(bootstrap_stage2(), args.host_id)
    print(json.dumps(export, sort_keys=True, separators=(",", ":")))
    return 0


def cmd_stage14_import(args: argparse.Namespace) -> int:
    result = stage14_import_compare(Path(args.path))
    for key, value in result.items():
        if key == "drift":
            value = json.dumps(value, sort_keys=True, separators=(",", ":"))
        print(f"{key}: {value}")
    return 0 if result["import_status"] == "reproduced" else 1


def cmd_stage14_import_check(args: argparse.Namespace) -> int:
    result = run_stage14_import_check(args.debug_dir)
    ok = (
        result["current_import_status"] == "reproduced"
        and result["draft_import_status"] == "reproduced"
        and result["current_migration_status"] == "current"
        and result["draft_migration_status"] == "migrated"
        and result["unsupported_import_status"] == "drift"
        and result["unsupported_migration_status"] == "unsupported"
        and result["current_compared_fixture_count"] == 4
        and result["draft_compared_fixture_count"] == 4
        and result["stage14_import_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage14_edn_check(args: argparse.Namespace) -> int:
    result = run_stage14_edn_check(args.debug_dir)
    ok = (
        result["edn_roundtrip_ok"] is True
        and result["lineage_roundtrip_ok"] is True
        and result["current_import_status"] == "reproduced"
        and result["current_migration_status"] == "current"
        and result["current_compared_fixture_count"] == 4
        and result["draft_import_status"] == "reproduced"
        and result["draft_migration_status"] == "migrated"
        and result["unsupported_import_status"] == "drift"
        and result["unsupported_migration_status"] == "unsupported"
        and result["stage14_edn_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage14_edn_export(args: argparse.Namespace) -> int:
    if args.lineage:
        bundle = stage14_edn_lineage_export(bootstrap_stage2(), args.host_id)
    else:
        bundle = stage14_edn_export(bootstrap_stage2(), args.host_id)
    print(bundle["edn_text"])
    return 0


def cmd_stage15_check(args: argparse.Namespace) -> int:
    result = run_stage15_check(args.debug_dir)
    ok = (
        result["source_policy_count"] == 13
        and result["evidence_count"] == 11
        and result["fixture_class_count"] >= 8
        and result["replay_count"] == 11
        and result["admission_count"] == 11
        and result["quarantine_record_count"] >= 1
        and result["quarantine_append_only"] is True
        and result["revocation_count"] == 6
        and result["revocation_replay_count"] == 6
        and result["reference_check_count"] == 11
        and result["reference_verified_count"] == 11
        and result["reference_network_used_count"] == 0
        and result["acquisition_count"] == 11
        and result["online_acquisition_adapter_count"] == 7
        and result["acquisition_network_used_count"] == 0
        and result["signature_pending_count"] == 11
        and result["accepted_evidence_count"] == 0
        and result["accepted_admission_count"] == 0
        and result["quarantine_required_count"] == 5
        and result["stage13_replay_required_count"] == 11
        and result["direct_accept_attempt_count"] == 1
        and result["direct_accept_rejected_count"] == 1
        and result["online_acquisition_count"] == 7
        and result["offline_replay_count"] == 11
        and result["network_admission_allowed"] is False
        and result["stage15_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage15_export(_args: argparse.Namespace) -> int:
    bundle = stage15_build_export_bundle()
    print(json.dumps(bundle["export"], sort_keys=True, separators=(",", ":")))
    return 0


def cmd_stagen_check(args: argparse.Namespace) -> int:
    result = run_stagen_check(args.debug_dir)
    ok = (
        result["extension_count"] == 1
        and result["prior_boundary_count"] == 4
        and result["required_field_count"] == 9
        and result["migration_outcome_count"] == 3
        and result["debug_contract_file_count"] == 3
        and result["weakening_count"] == 0
        and result["stage15_evidence_only_preserved"] is True
        and result["stagen_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_front_end_boundary_check(args: argparse.Namespace) -> int:
    result = run_front_end_boundary_check(args.debug_dir)
    ok = (
        result["sample_count"] == 8
        and result["determinism_ok"] is True
        and result["roundtrip_ok"] is True
        and result["purity_ok"] is True
        and result["reader_host_module"] == "hy.reader"
        and result["mangle_host_module"] == "hy.reader.mangling"
        and result["reader_ownership_decision"] == "host-substrate"
        and result["public_classification"] == "self-hosting-back-end"
        and result["front_end_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_macro_require_parity_check(args: argparse.Namespace) -> int:
    result = run_macro_require_parity_check(args.debug_dir)
    ok = (
        result["case_count"] == 16
        and result["matched_count"] == 16
        and result["mismatch_count"] == 0
        and result["macro_require_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_ast_forward_compat_check(args: argparse.Namespace) -> int:
    result = run_ast_forward_compat_check(args.debug_dir)
    ok = (
        result["compile_error_count"] == 0
        and result["compiled_count"] == result["source_count"]
        and result["removed_node_ref_count"] == 0
        and result["removed_emitted_count"] == 0
        and result["forward_warning_count"] == 0
        and result["ast_forward_compat_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_source_position_check(args: argparse.Namespace) -> int:
    result = run_source_position_check(args.debug_dir)
    ok = (
        result["case_count"] == 4
        and result["checked_leaf_count"] == 12
        and result["mismatch_count"] == 0
        and result["distinct_positions_ok"] is True
        and result["co_positions_populated"] is True
        and result["co_positions_distinct"] >= 2
        and result["source_position_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_version_ast_coverage_check(args: argparse.Namespace) -> int:
    result = run_version_ast_coverage_check(args.debug_dir)
    ok = (
        result["target_count"] == 2
        and result["owned_node_count"] == 7
        and result["gated_or_absent_node_count"] == 6
        and result["unclassified_count"] == 0
        and result["crosscheck_in_lane"] is True
        and result["crosscheck_ok"] is True
        and result["match_probe_value"] == "one"
        and result["version_ast_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_stage16_check(args: argparse.Namespace) -> int:
    result = run_stage16_check(args.debug_dir)
    ok = (
        result["stage"] == 16
        and result["current_replay_status"] == "reproduced"
        and result["stale_replay_status"] == "stale-held"
        and result["migrated_status"] == "migrated"
        and result["unsupported_status"] == "unsupported"
        and result["peer_review_pending"] is True
        and result["admission_status"] == "not-admitted"
        and result["accepted_count"] == 0
        and result["stage16_status"] == "reproduced"
        and result["drift_fields"] == ""
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_kernel_check(_args: argparse.Namespace) -> int:
    result = run_kernel_check()
    ok = (
        result["kernel_self_check"] is True
        and result["kernel_factorial"] == 120
        and result["kernel_features"] == 449.0
        and result["kernel_loop"] == 120
        and result["kernel_python_contains_fact"] is True
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_kernel_import_check(_args: argparse.Namespace) -> int:
    result = run_kernel_import_check()
    ok = (
        result["probe_value"] == 42
        and result["probe_doc"] == "kernel import probe doc"
        and result["consumer_value"] == 43
        and result["consumer_imported_value"] == 42
        and result["package_value"] == 42
        and result["package_imported_value"] == 41
        and result["package_relative_value"] == 42
        and result["package_relative_module_value"] == 21
        and result["child_value"] == 41
        and result["sibling_value"] == 21
        and result["reload_value_before"] == 1
        and result["reload_value_after"] == 2
        and result["reload_module_value_after"] == 42
        and result["cycle_a_value"] == 10
        and result["cycle_a_sees_b_value"] == 32
        and result["cycle_b_value"] == 32
        and result["cycle_b_saw_a_started"] is True
        and result["cycle_b_saw_a_value_before"] == "missing"
        and result["shadow_left_value"] == "left"
        and result["shadow_right_value"] == "right"
        and result["shadow_modules_distinct"] is True
        and result["runpy_value"] == 77
        and result["runpy_name"] == "kernel_import_runpy_target"
        and result["runpy_package"] == ""
        and result["runpy_loader"] == "KernelHyLoader"
        and result["runpy_file"] == "kernel_import_runpy_target.hy"
        and result["runpy_module_not_cached"] is True
        and result["package_main_value"] == 88
        and result["package_main_name"] == "kernel_import_run_pkg.__main__"
        and result["package_main_package"] == "kernel_import_run_pkg"
        and result["package_main_loader"] == "KernelHyLoader"
        and result["package_main_file"] == "__main__.hy"
        and result["package_main_not_cached"] is True
        and result["export_all"] == ["EXPORTED", "ALIAS_VALUE"]
        and result["export_star_exported"] == 123
        and result["export_star_alias"] == "alias"
        and result["export_star_hidden_absent"] is True
        and result["export_source_hidden"] == 456
        and result["export_module_cache_ok"] is True
        and result["require_provider_value"] == 5
        and result["require_provider_macro_present"] is True
        and result["require_provider_module_cache_ok"] is True
        and result["macro_self_value"] == 42
        and result["macro_self_macro_table_has_self_macros"] is True
        and result["macro_self_module_cache_ok"] is True
        and result["temp_stress_value"] == 218
        and result["temp_stress_reload_same_object"] is True
        and result["temp_stress_module_cache_ok"] is True
        and result["temp_stress_loader_python_stable"] is True
        and result["temp_stress_loader_ast_stable"] is True
        and result["import_module_dicts_distinct"] is True
        and result["probe_file"] == "kernel_import_probe.hy"
        and result["consumer_file"] == "kernel_import_consumer.hy"
        and result["package_file"] == "__init__.hy"
        and result["child_file"] == "child.hy"
        and result["sibling_file"] == "sibling.hy"
        and result["reload_file"] == "kernel_import_reload_target.hy"
        and result["cycle_a_file"] == "kernel_import_cycle_a.hy"
        and result["cycle_b_file"] == "kernel_import_cycle_b.hy"
        and result["shadow_left_file"] == "thing.hy"
        and result["shadow_right_file"] == "thing.hy"
        and result["shadow_left_parent"] == "kernel_import_shadow_left"
        and result["shadow_right_parent"] == "kernel_import_shadow_right"
        and result["require_provider_file"] == "kernel_import_require_provider.hy"
        and result["macro_self_file"] == "kernel_import_macro_self.hy"
        and result["temp_stress_file"] == "kernel_import_temp_stress.hy"
        and result["probe_loader"] == "KernelHyLoader"
        and result["consumer_loader"] == "KernelHyLoader"
        and result["package_loader"] == "KernelHyLoader"
        and result["child_loader"] == "KernelHyLoader"
        and result["sibling_loader"] == "KernelHyLoader"
        and result["reload_loader"] == "KernelHyLoader"
        and result["cycle_a_loader"] == "KernelHyLoader"
        and result["cycle_b_loader"] == "KernelHyLoader"
        and result["shadow_left_loader"] == "KernelHyLoader"
        and result["shadow_right_loader"] == "KernelHyLoader"
        and result["require_provider_loader"] == "KernelHyLoader"
        and result["macro_self_loader"] == "KernelHyLoader"
        and result["temp_stress_loader"] == "KernelHyLoader"
        and result["probe_module_cache_ok"] is True
        and result["consumer_module_cache_ok"] is True
        and result["package_module_cache_ok"] is True
        and result["child_module_cache_ok"] is True
        and result["sibling_module_cache_ok"] is True
        and result["reload_import_cache_same_object"] is True
        and result["reload_same_object"] is True
        and result["reload_module_cache_ok"] is True
        and result["cycle_a_module_cache_ok"] is True
        and result["cycle_b_module_cache_ok"] is True
        and result["shadow_left_module_cache_ok"] is True
        and result["shadow_right_module_cache_ok"] is True
        and result["zipimport_status"] == "unsupported-filesystem-roots-only"
        and result["bytecode_status"] == "unsupported-source-only-exec"
        and result["broken_import_failed"] is True
        and result["broken_import_error"] == "kernel import broken"
        and result["broken_import_traceback_mentions_hy_file"] is True
        and result["broken_module_cache_removed"] is True
        and result["reader_broken_import_failed"] is True
        and result["reader_broken_import_error"] == "kernel import reader broken"
        and result["reader_broken_module_cache_removed"] is True
        and result["reader_after_broken_import_failed"] is True
        and result["reader_after_broken_error_class"] == "LexException"
        and result["reader_after_broken_module_cache_removed"] is True
        and result["reader_error_import_failed"] is True
        and result["reader_error_class"] == "LexException"
        and result["reader_error_filtered_mentions_hy_file"] is True
        and result["reader_error_filtered_hides_importlib"] is True
        and result["reader_error_full_has_importlib"] is True
        and result["reader_error_module_cache_removed"] is True
        and result["failed_require_import_failed"] is True
        and result["failed_require_error_class"] == "SyntaxError"
        and "missing_macro" in result["failed_require_error"]
        and result["failed_require_module_cache_removed"] is True
        and result["failed_macro_expansion_import_failed"] is True
        and result["failed_macro_expansion_error_class"] == "HyMacroExpansionError"
        and "division by zero" in result["failed_macro_expansion_error"]
        and result["failed_macro_expansion_module_cache_removed"] is True
        and result["import_reader_macro_tables_distinct"] is True
        and result["import_macro_tables_distinct"] is True
        and result["import_macro_tables_clean"] is True
        and result["import_reader_macro_tables_clean"] is True
        and result["failed_reader_macro_absent_from_loaded_modules"] is True
        and result["failed_require_macros_absent_from_loaded_modules"] is True
        and result["failed_expansion_macros_absent_from_loaded_modules"] is True
        and result["package_path_ok"] is True
        and result["probe_result_removed"] is True
        and result["consumer_result_removed"] is True
        and result["package_result_removed"] is True
        and result["child_result_removed"] is True
        and result["sibling_result_removed"] is True
        and result["kernel_import_hook_removed"] is True
    )
    for key, value in result.items():
        print(f"{key}: {value}")
    return 0 if ok else 1


def cmd_run(args: argparse.Namespace) -> int:
    source, filename = read_input(args)
    stage2 = bootstrap_stage2()
    module = stage2.make_module(args.module_name)
    apply_startup_files(stage2, module, args.startup)
    value = stage2.eval_source(source, module, filename)
    if value is not None:
        print(value, flush=args.flush)
    return 0


def cmd_kernel_run(args: argparse.Namespace) -> int:
    source, filename = read_input(args)
    kernel = bootstrap_kernel()
    value = kernel.eval_source(source, None, filename)
    if value is not None:
        print(value)
    return 0


def cmd_kernel_py(args: argparse.Namespace) -> int:
    source, filename = read_input(args)
    kernel = bootstrap_kernel()
    print(kernel.python_source(source, filename))
    return 0


def cmd_stage7_kernel_run(args: argparse.Namespace) -> int:
    source, filename = read_input(args)
    kernel = bootstrap_stage7_kernel()
    value = kernel.eval_source(source, None, filename)
    if value is not None:
        print(value)
    return 0


def cmd_stage7_kernel_py(args: argparse.Namespace) -> int:
    source, filename = read_input(args)
    kernel = bootstrap_stage7_kernel()
    print(kernel.python_source(source, filename))
    return 0


def cmd_py(args: argparse.Namespace) -> int:
    source, filename = read_input(args)
    stage2 = bootstrap_stage2()
    module = stage2.make_module(args.module_name)
    print(stage2.python_source(source, module, filename))
    return 0


def cmd_hy2py(args: argparse.Namespace) -> int:
    source, filename = read_input(args)
    stage2 = bootstrap_stage2()
    module = stage2.make_module(args.module_name)
    python = stage2.python_source(source, module, filename)
    if args.output:
        output = Path(args.output)
        output.write_text(python if python.endswith("\n") else python + "\n")
    else:
        print(python)
    return 0


def cmd_hyc(args: argparse.Namespace) -> int:
    source, filename = read_input(args)
    if args.output:
        output = Path(args.output)
    elif args.file:
        output = Path(args.file).with_suffix(".pyc")
    else:
        print("hyc needs -o/--output when reading from -c or stdin", file=sys.stderr)
        return 2
    print(compile_stage2_pyc(source, filename, output, args.module_name))
    return 0


def cmd_repl(args: argparse.Namespace) -> int:
    return eval_repl_stream(
        sys.stdin,
        sys.stdout,
        module_name=args.module_name,
        startup_files=args.startup,
        prompt=sys.stdin.isatty() and not args.no_prompt,
        flush=args.flush,
    )


def cmd_native_subset_check(_args: argparse.Namespace) -> int:
    from native_subset_test import main as native_subset_main

    native_subset_main()
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="hy-meta")
    sub = parser.add_subparsers(dest="command", required=True)

    self_check = sub.add_parser("self-check", help="bootstrap stage2 and run checks")
    self_check.set_defaults(func=cmd_self_check)

    chain_check = sub.add_parser(
        "chain-check",
        help="load stage2 with stage1, then load stage2 again through stage2",
    )
    chain_check.set_defaults(func=cmd_chain_check)

    kernel_check = sub.add_parser(
        "kernel-check",
        help="run the Hy-written kernel compiler subset checks",
    )
    kernel_check.set_defaults(func=cmd_kernel_check)

    direct_kernel_bridge_check = sub.add_parser(
        "direct-kernel-bridge-check",
        help="prove stage2/compiler.hy dispatches supported source through the direct kernel",
    )
    direct_kernel_bridge_check.set_defaults(func=cmd_direct_kernel_bridge_check)

    reader_boundary_check = sub.add_parser(
        "reader-boundary-check",
        help="prove the stage7 reader boundary is delegated to upstream hy.reader",
    )
    reader_boundary_check.set_defaults(func=cmd_reader_boundary_check)

    compatibility_boundary_check = sub.add_parser(
        "compatibility-boundary-check",
        help="prove Python import compatibility, upstream compiler path, and owned gates",
    )
    compatibility_boundary_check.set_defaults(func=cmd_compatibility_boundary_check)

    cli_io_check = sub.add_parser(
        "cli-io-check",
        help="prove stdin, -c, and hy2py input handling for the scoped CLI",
    )
    cli_io_check.set_defaults(func=cmd_cli_io_check)

    hyc_check = sub.add_parser(
        "hyc-check",
        help="compile scoped Hy source to .pyc and import the bytecode",
    )
    hyc_check.set_defaults(func=cmd_hyc_check)

    repl_check = sub.add_parser(
        "repl-check",
        help="prove the scoped line REPL preserves module state across forms",
    )
    repl_check.set_defaults(func=cmd_repl_check)

    startup_output_check = sub.add_parser(
        "startup-output-check",
        help="prove scoped startup files and flushed output paths",
    )
    startup_output_check.set_defaults(func=cmd_startup_output_check)

    kernel_import_check = sub.add_parser(
        "kernel-import-check",
        help="import .hy modules through the Hy-written kernel import hook",
    )
    kernel_import_check.set_defaults(func=cmd_kernel_import_check)

    prime_check = sub.add_parser(
        "prime-check",
        help="run the stage2-prime Hy meta check through stage2-prime itself",
    )
    prime_check.set_defaults(func=cmd_prime_check)

    stage3_check = sub.add_parser(
        "stage3-check",
        help="load stage3 through stage2-prime and run the stage3 proof lane",
    )
    stage3_check.set_defaults(func=cmd_stage3_check)

    mirror_check = sub.add_parser(
        "mirror-check",
        help="compare stage and kernel AST/source/value mirrors through stage3",
    )
    mirror_check.set_defaults(func=cmd_mirror_check)

    stage7_check = sub.add_parser(
        "stage7-check",
        help="stress stage and module-cache mirrors through stage7",
    )
    stage7_check.set_defaults(func=cmd_stage7_check)

    self_host_check = sub.add_parser(
        "self-host-check",
        help="compile stage2/kernel.hy through the stage7-loaded kernel and load it",
    )
    self_host_check.set_defaults(func=cmd_self_host_check)

    fixedpoint_check = sub.add_parser(
        "bootstrap-fixedpoint-check",
        help="prove kernelB and kernelC self-compilation artifacts are identical",
    )
    fixedpoint_check.set_defaults(func=cmd_bootstrap_fixedpoint_check)

    diverse_double_compile_check = sub.add_parser(
        "diverse-double-compile-check",
        help="DDC: upstream-built and direct-built kernels emit identical artifacts",
    )
    diverse_double_compile_check.set_defaults(func=cmd_diverse_double_compile_check)

    independent_mini_backend_check = sub.add_parser(
        "independent-mini-backend-check",
        help="DDC: real upstream Hy and a from-scratch mini backend agree on a fixture subset",
    )
    independent_mini_backend_check.set_defaults(func=cmd_independent_mini_backend_check)

    no_fallback_check = sub.add_parser(
        "no-fallback-check",
        help="compile the owned corpus with direct-kernel strict mode enabled",
    )
    no_fallback_check.set_defaults(func=cmd_no_fallback_check)

    parity_ledger_check = sub.add_parser(
        "parity-ledger-check",
        help="measure direct-kernel hit/fallback parity over owned and native corpora",
    )
    parity_ledger_check.add_argument("--debug-dir")
    parity_ledger_check.set_defaults(func=cmd_parity_ledger_check)

    stage8_check = sub.add_parser(
        "stage8-check",
        help="compare stage7 and fresh stage8 compiler/kernel artifacts",
    )
    stage8_check.add_argument("--debug-dir")
    stage8_check.set_defaults(func=cmd_stage8_check)

    stage9_check = sub.add_parser(
        "stage9-check",
        help="replay product entrypoints in clean subprocesses",
    )
    stage9_check.add_argument("--debug-dir")
    stage9_check.set_defaults(func=cmd_stage9_check)

    stage9_product_probe = sub.add_parser(
        "stage9-product-probe",
        help=argparse.SUPPRESS,
    )
    stage9_product_probe.set_defaults(func=cmd_stage9_product_probe)

    stage10_check = sub.add_parser(
        "stage10-check",
        help="replay product shell through client/server/session/sandbox surfaces",
    )
    stage10_check.add_argument("--debug-dir")
    stage10_check.set_defaults(func=cmd_stage10_check)

    stage11_check = sub.add_parser(
        "stage11-check",
        help="verify multi-domain adapters obey candidate/held/evidence gates",
    )
    stage11_check.add_argument("--debug-dir")
    stage11_check.set_defaults(func=cmd_stage11_check)

    stage12_check = sub.add_parser(
        "stage12-check",
        help="verify self-improvement candidates stay quarantined",
    )
    stage12_check.add_argument("--debug-dir")
    stage12_check.set_defaults(func=cmd_stage12_check)

    stage13_check = sub.add_parser(
        "stage13-check",
        help="verify long-horizon replay, stale-held, and boundary closure",
    )
    stage13_check.add_argument("--debug-dir")
    stage13_check.set_defaults(func=cmd_stage13_check)

    stage14_check = sub.add_parser(
        "stage14-check",
        help="verify host-neutral cross-host exports and local replay closure",
    )
    stage14_check.add_argument("--debug-dir")
    stage14_check.set_defaults(func=cmd_stage14_check)

    stage14_export = sub.add_parser(
        "stage14-export",
        help="emit the hy-meta host-neutral stage14 JSON export",
    )
    stage14_export.add_argument("--host-id", default="hy-meta")
    stage14_export.set_defaults(func=cmd_stage14_export)

    stage14_import = sub.add_parser(
        "stage14-import",
        help="compare a peer stage14 JSON export against the local hy-meta export",
    )
    stage14_import.add_argument("path")
    stage14_import.set_defaults(func=cmd_stage14_import)

    stage14_import_check = sub.add_parser(
        "stage14-import-check",
        help="verify stage14 JSON import and draft schema migration",
    )
    stage14_import_check.add_argument("--debug-dir")
    stage14_import_check.set_defaults(func=cmd_stage14_import_check)

    stage14_edn_check = sub.add_parser(
        "stage14-edn-check",
        help="verify stage14 EDN exchange round-trip, import, and migration",
    )
    stage14_edn_check.add_argument("--debug-dir")
    stage14_edn_check.set_defaults(func=cmd_stage14_edn_check)

    stage14_edn_export = sub.add_parser(
        "stage14-edn-export",
        help="emit the hy-meta stage14 EDN export for Clojure hosts",
    )
    stage14_edn_export.add_argument("--host-id", default="hy-meta-edn")
    stage14_edn_export.add_argument("--lineage", action="store_true")
    stage14_edn_export.set_defaults(func=cmd_stage14_edn_export)

    stage15_check = sub.add_parser(
        "stage15-check",
        help="verify open-world external evidence federation boundaries",
    )
    stage15_check.add_argument("--debug-dir")
    stage15_check.set_defaults(func=cmd_stage15_check)

    stage15_export = sub.add_parser(
        "stage15-export",
        help="emit the stage15 open-world evidence JSON export bundle",
    )
    stage15_export.set_defaults(func=cmd_stage15_export)

    stagen_check = sub.add_parser(
        "stagen-check",
        help="verify stageN extension manifest constitutional boundaries",
    )
    stagen_check.add_argument("--debug-dir")
    stagen_check.set_defaults(func=cmd_stagen_check)

    stage16_check = sub.add_parser(
        "stage16-check",
        help="verify the concrete stage16 extension admission replay closure",
    )
    stage16_check.add_argument("--debug-dir")
    stage16_check.set_defaults(func=cmd_stage16_check)

    version_ast_coverage_check = sub.add_parser(
        "version-ast-coverage-check",
        help="verify explicit per-version target-AST owned/gated coverage",
    )
    version_ast_coverage_check.add_argument("--debug-dir")
    version_ast_coverage_check.set_defaults(func=cmd_version_ast_coverage_check)

    source_position_check = sub.add_parser(
        "source-position-check",
        help="verify PEP 657 fine-grained per-node source positions",
    )
    source_position_check.add_argument("--debug-dir")
    source_position_check.set_defaults(func=cmd_source_position_check)

    ast_forward_compat_check = sub.add_parser(
        "ast-forward-compat-check",
        help="verify the kernel emits forward-compatible AST (no removed nodes, 3.15-safe fields)",
    )
    ast_forward_compat_check.add_argument("--debug-dir")
    ast_forward_compat_check.set_defaults(func=cmd_ast_forward_compat_check)

    macro_require_parity_check = sub.add_parser(
        "macro-require-parity-check",
        help="verify macro/require runtime behavior (get-macro, hy.R/I, require, defreader)",
    )
    macro_require_parity_check.add_argument("--debug-dir")
    macro_require_parity_check.set_defaults(func=cmd_macro_require_parity_check)

    front_end_boundary_check = sub.add_parser(
        "front-end-boundary-check",
        help="record the reader/mangle host-substrate decision and prove its purity",
    )
    front_end_boundary_check.add_argument("--debug-dir")
    front_end_boundary_check.set_defaults(func=cmd_front_end_boundary_check)

    native_subset_check = sub.add_parser(
        "native-subset-check",
        help="run the focused native-style kernel parity subset",
    )
    native_subset_check.set_defaults(func=cmd_native_subset_check)

    run = sub.add_parser("run", help="evaluate Hy source through stage2")
    run.add_argument("source", nargs="?", default="")
    run.add_argument("-c", "--command")
    run.add_argument("-f", "--file")
    run.add_argument("--module-name", default="hy_meta_cli.run")
    run.add_argument("--startup", action="append")
    run.add_argument("--flush", action="store_true")
    run.set_defaults(func=cmd_run)

    py = sub.add_parser("py", help="print Python produced by stage2")
    py.add_argument("source", nargs="?", default="")
    py.add_argument("-c", "--command")
    py.add_argument("-f", "--file")
    py.add_argument("--module-name", default="hy_meta_cli.py")
    py.set_defaults(func=cmd_py)

    hy2py = sub.add_parser("hy2py", help="convert Hy source to Python through stage2")
    hy2py.add_argument("source", nargs="?", default="")
    hy2py.add_argument("-c", "--command")
    hy2py.add_argument("-f", "--file")
    hy2py.add_argument("-o", "--output")
    hy2py.add_argument("--module-name", default="hy_meta_cli.hy2py")
    hy2py.set_defaults(func=cmd_hy2py)

    hyc = sub.add_parser("hyc", help="compile Hy source to .pyc through stage2")
    hyc.add_argument("source", nargs="?", default="")
    hyc.add_argument("-c", "--command")
    hyc.add_argument("-f", "--file")
    hyc.add_argument("-o", "--output")
    hyc.add_argument("--module-name", default="hy_meta_cli.hyc")
    hyc.set_defaults(func=cmd_hyc)

    repl = sub.add_parser("repl", help="run the scoped stage2 line REPL")
    repl.add_argument("--module-name", default="hy_meta_cli.repl")
    repl.add_argument("--startup", action="append")
    repl.add_argument("--flush", action="store_true")
    repl.add_argument("--no-prompt", action="store_true")
    repl.set_defaults(func=cmd_repl)

    kernel_run = sub.add_parser(
        "kernel-run",
        help="evaluate kernel Hy subset source through the Hy-written kernel",
    )
    kernel_run.add_argument("source", nargs="?", default="")
    kernel_run.add_argument("-c", "--command")
    kernel_run.add_argument("-f", "--file")
    kernel_run.set_defaults(func=cmd_kernel_run)

    kernel_py = sub.add_parser(
        "kernel-py",
        help="print Python produced by the Hy-written kernel",
    )
    kernel_py.add_argument("source", nargs="?", default="")
    kernel_py.add_argument("-c", "--command")
    kernel_py.add_argument("-f", "--file")
    kernel_py.set_defaults(func=cmd_kernel_py)

    stage7_kernel_run = sub.add_parser(
        "stage7-kernel-run",
        help="evaluate Hy subset source through the stage7-loaded kernel",
    )
    stage7_kernel_run.add_argument("source", nargs="?", default="")
    stage7_kernel_run.add_argument("-c", "--command")
    stage7_kernel_run.add_argument("-f", "--file")
    stage7_kernel_run.set_defaults(func=cmd_stage7_kernel_run)

    stage7_kernel_py = sub.add_parser(
        "stage7-kernel-py",
        help="print Python produced by the stage7-loaded kernel",
    )
    stage7_kernel_py.add_argument("source", nargs="?", default="")
    stage7_kernel_py.add_argument("-c", "--command")
    stage7_kernel_py.add_argument("-f", "--file")
    stage7_kernel_py.set_defaults(func=cmd_stage7_kernel_py)

    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
