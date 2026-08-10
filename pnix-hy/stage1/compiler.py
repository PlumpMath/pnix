"""Python seed for a Hy meta-circular compiler.

Stage 1 deliberately stays thin: it exposes a stable compiler/interpreter
protocol while delegating parsing, macro expansion, and Python AST generation
to the checked-in Hy 1.3.0 implementation. Stage 2 can then re-express the
same protocol in Hy and gradually move more behavior out of this seed.
"""

from __future__ import annotations

import ast
import sys
import types
from pathlib import Path
from typing import Any

SUPPORTED_PYTHON_FAMILIES = {(3, 11), (3, 14)}

if sys.version_info[:2] not in SUPPORTED_PYTHON_FAMILIES:
    raise RuntimeError(
        "hy-meta stage1 supports exactly Python 3.11 and Homebrew Python 3.14 "
        "in this proof lane; run it with /tmp/pnix-hy-py311-venv/bin/python "
        "or /tmp/pnix-hy-py314-venv/bin/python."
    )

import hy  # noqa: F401  # Import side effects install Hy import/runtime hooks.
from hy.compiler import hy_compile, hy_eval
from hy.reader import read_many


DEFAULT_MODULE = "hy_meta_stage1.session"


def make_module(name: str = DEFAULT_MODULE, filename: str | None = None) -> types.ModuleType:
    """Create and register an execution module for Hy compilation."""

    module = types.ModuleType(name)
    module.__file__ = filename or f"<{name}>"
    module.__package__ = name.rpartition(".")[0]
    module.hy = hy
    module.__dict__.setdefault("_hy_macros", {})
    module.__dict__.setdefault("_hy_reader_macros", {})
    sys.modules[name] = module
    return module


def read_source(source: str, filename: str = "<string>") -> Any:
    """Read Hy source into Hy model objects."""

    return read_many(source, filename=filename)


class Stage1Compiler:
    """A small compiler/interpreter facade over Hy's Python implementation."""

    def __init__(
        self,
        module_name: str = DEFAULT_MODULE,
        module: types.ModuleType | None = None,
        filename: str | None = None,
    ) -> None:
        self.module = module or make_module(module_name, filename)

    def read_source(self, source: str, filename: str = "<string>") -> Any:
        return read_source(source, filename)

    def compile_model_to_ast(
        self,
        model: Any,
        *,
        filename: str = "<string>",
        source: str | None = None,
        import_stdlib: bool = True,
    ) -> ast.Module:
        tree = hy_compile(
            model,
            self.module,
            filename=filename,
            source=source,
            import_stdlib=import_stdlib,
        )
        return ast.fix_missing_locations(tree)

    def compile_source_to_ast(
        self,
        source: str,
        *,
        filename: str = "<string>",
        import_stdlib: bool = True,
    ) -> ast.Module:
        return self.compile_model_to_ast(
            self.read_source(source, filename),
            filename=filename,
            source=source,
            import_stdlib=import_stdlib,
        )

    def compile_source_to_code(
        self,
        source: str,
        *,
        filename: str = "<string>",
        import_stdlib: bool = True,
    ) -> types.CodeType:
        tree = self.compile_source_to_ast(
            source,
            filename=filename,
            import_stdlib=import_stdlib,
        )
        return compile(tree, filename, "exec")

    def python_source(
        self,
        source: str,
        *,
        filename: str = "<string>",
        import_stdlib: bool = True,
    ) -> str:
        tree = self.compile_source_to_ast(
            source,
            filename=filename,
            import_stdlib=import_stdlib,
        )
        return ast.unparse(tree)

    def exec_source(
        self,
        source: str,
        *,
        filename: str = "<string>",
        import_stdlib: bool = True,
    ) -> types.ModuleType:
        code = self.compile_source_to_code(
            source,
            filename=filename,
            import_stdlib=import_stdlib,
        )
        exec(code, self.module.__dict__)
        return self.module

    def eval_source(
        self,
        source: str,
        *,
        filename: str = "<string>",
        import_stdlib: bool = True,
    ) -> Any:
        model = self.read_source(source, filename)
        return hy_eval(
            model,
            self.module.__dict__,
            module=self.module,
            filename=filename,
            source=source,
            import_stdlib=import_stdlib,
        )

    def load_hy_file(
        self,
        path: str | Path,
        *,
        module_name: str | None = None,
        import_stdlib: bool = True,
    ) -> types.ModuleType:
        path = Path(path)
        name = module_name or path.with_suffix("").name
        module = make_module(name, str(path))
        loader = Stage1Compiler(module_name=name, module=module, filename=str(path))
        loader.exec_source(
            path.read_text(),
            filename=str(path),
            import_stdlib=import_stdlib,
        )
        return module


def compile_source_to_ast(
    source: str,
    module: types.ModuleType | None = None,
    *,
    filename: str = "<string>",
    import_stdlib: bool = True,
) -> ast.Module:
    compiler = Stage1Compiler(module=module) if module else Stage1Compiler()
    return compiler.compile_source_to_ast(
        source,
        filename=filename,
        import_stdlib=import_stdlib,
    )


def python_source(
    source: str,
    module: types.ModuleType | None = None,
    *,
    filename: str = "<string>",
    import_stdlib: bool = True,
) -> str:
    compiler = Stage1Compiler(module=module) if module else Stage1Compiler()
    return compiler.python_source(
        source,
        filename=filename,
        import_stdlib=import_stdlib,
    )


def exec_source(
    source: str,
    module: types.ModuleType | None = None,
    *,
    filename: str = "<string>",
    import_stdlib: bool = True,
) -> types.ModuleType:
    compiler = Stage1Compiler(module=module) if module else Stage1Compiler()
    return compiler.exec_source(
        source,
        filename=filename,
        import_stdlib=import_stdlib,
    )


def eval_source(
    source: str,
    module: types.ModuleType | None = None,
    *,
    filename: str = "<string>",
    import_stdlib: bool = True,
) -> Any:
    compiler = Stage1Compiler(module=module) if module else Stage1Compiler()
    return compiler.eval_source(
        source,
        filename=filename,
        import_stdlib=import_stdlib,
    )


def load_hy_file(
    path: str | Path,
    *,
    module_name: str | None = None,
    import_stdlib: bool = True,
) -> types.ModuleType:
    return Stage1Compiler().load_hy_file(
        path,
        module_name=module_name,
        import_stdlib=import_stdlib,
    )
