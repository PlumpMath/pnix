"""Host (CPython/Hy) introspection facilities — the Hy/Python meta-circular
HOST artifact lane. Relocated here from pnix_hy/hy_mirror.py (SEP1 / hy-meta SR1):
these compile/code-object/bytecode/marshal/AST/symtable/tokenize/inspect/gc/sys/
traceback/module + execution-trace facilities are host compiler/evaluator artifacts,
NOT pnix runtime semantics. pnix-hy path-imports and re-exports them.

Pure stdlib; no `hy` and no pnix dependency. Eventually fold with bootstrap.py's
artifact_from_ast / stable_code_payload / ast_data / pyc_bytes_for_code so there is one
host-artifact lane (hy-meta SR2).
"""

from __future__ import annotations

import ast as _ast
import dis as _dis
import gc as _gc
import importlib
import inspect as _inspect
import io as _io
import marshal as _marshal
import sys
import traceback as _traceback
import types as _types
from typing import Any

try:  # symbol-table introspection
    import symtable as _symtable
except ImportError:  # pragma: no cover
    _symtable = None
try:  # token-stream introspection
    import tokenize as _tokenize
except ImportError:  # pragma: no cover
    _tokenize = None


def _try(fn: Any) -> Any:
    try:
        return fn()
    except Exception as exc:  # noqa: BLE001
        return f"<{type(exc).__name__}>"


def _json_safe(value: Any) -> Any:
    if isinstance(value, _types.CodeType):
        return {"__code__": value.co_name, "co_code_hex": value.co_code.hex()}
    if isinstance(value, (str, bool, int, float)) or value is None:
        return value
    if isinstance(value, (list, tuple)):
        return [_json_safe(v) for v in value]
    if isinstance(value, (set, frozenset)):
        return sorted(_json_safe(v) for v in value)
    if isinstance(value, dict):
        return {str(k): _json_safe(v) for k, v in value.items()}
    return repr(value)


def compile_source(source: str, mode: str = "exec", filename: str = "<pnix-introspect>") -> Any:
    """compile() -> code object, the unit every facility below introspects."""
    return compile(source, filename, mode)


def code_object_info(code: Any) -> dict[str, Any]:
    """Every introspectable field of a code object."""
    info: dict[str, Any] = {
        "co_name": code.co_name,
        "co_filename": code.co_filename,
        "co_firstlineno": code.co_firstlineno,
        "co_argcount": code.co_argcount,
        "co_posonlyargcount": getattr(code, "co_posonlyargcount", 0),
        "co_kwonlyargcount": code.co_kwonlyargcount,
        "co_nlocals": code.co_nlocals,
        "co_stacksize": code.co_stacksize,
        "co_flags": code.co_flags,
        "co_consts": [_json_safe(c) for c in code.co_consts],
        "co_names": list(code.co_names),
        "co_varnames": list(code.co_varnames),
        "co_freevars": list(code.co_freevars),
        "co_cellvars": list(code.co_cellvars),
        "co_code_len": len(code.co_code),
        "co_code_hex": code.co_code.hex(),
    }
    if hasattr(code, "co_qualname"):
        info["co_qualname"] = code.co_qualname
    if hasattr(code, "co_exceptiontable"):
        info["co_exceptiontable_hex"] = code.co_exceptiontable.hex()
    if hasattr(code, "co_lines"):
        info["co_lines"] = [list(t) for t in code.co_lines()]
    if hasattr(code, "co_positions"):
        info["co_positions_count"] = sum(1 for _ in code.co_positions())
    return info


def disassemble(code: Any) -> list[dict[str, Any]]:
    """Structured bytecode: one dict per dis.Instruction."""
    out: list[dict[str, Any]] = []
    for i in _dis.get_instructions(code):
        out.append(
            {
                "offset": i.offset,
                "opname": i.opname,
                "opcode": i.opcode,
                "arg": i.arg,
                "argval": _json_safe(i.argval),
                "argrepr": i.argrepr,
                "is_jump_target": i.is_jump_target,
                "starts_line": getattr(i, "starts_line", None),
            }
        )
    return out


def _own_code_objects(code: Any) -> set:
    """`code` plus every nested code object reachable through its co_consts."""
    seen: set = set()
    stack = [code]
    while stack:
        c = stack.pop()
        if c in seen:
            continue
        seen.add(c)
        for k in getattr(c, "co_consts", ()):  # type: ignore[union-attr]
            if hasattr(k, "co_code"):
                stack.append(k)
    return seen


def execution_trace(source: str, mode: str = "eval", max_events: int = 400) -> dict[str, Any]:
    """The DYNAMIC counterpart of `disassemble`: compile `source`, then actually
    run it under an opcode-level tracer (sys.settrace + f_trace_opcodes) and report
    the bytecode that genuinely EXECUTES (vs. the static instruction listing).

    Tracing is restricted to the code objects we compiled (the module body and its
    nested functions/lambdas) so library internals never enter the trace. This turns
    the static Python-ecosystem meta-circular view (ast/dis/marshal) into a live one:
    which opcodes a Hy/Python fragment's lowering actually drives, in order.

    schema pnix-hy.execution-trace.v0
    """
    code = compile(source, "<pnix-trace>", mode)
    own = _own_code_objects(code)
    instr_cache: dict[Any, dict[int, str]] = {}

    def _instrs(c: Any) -> dict[int, str]:
        m = instr_cache.get(c)
        if m is None:
            m = {i.offset: i.opname for i in _dis.get_instructions(c)}
            instr_cache[c] = m
        return m

    seq: list[dict[str, Any]] = []
    hist: dict[str, int] = {}
    state = {"count": 0, "truncated": False}

    def tracer(frame, event, arg):  # noqa: ANN001
        if frame.f_code not in own:
            return None
        frame.f_trace_opcodes = True
        if event == "opcode":
            op = _instrs(frame.f_code).get(frame.f_lasti, "?")
            hist[op] = hist.get(op, 0) + 1
            state["count"] += 1
            if len(seq) < max_events:
                seq.append({"code": frame.f_code.co_name, "offset": frame.f_lasti, "opname": op})
            else:
                state["truncated"] = True
        return tracer

    glb: dict[str, Any] = {}
    result_repr = None
    error = None
    prev = sys.gettrace()
    sys.settrace(tracer)
    try:
        if mode == "eval":
            result_repr = repr(eval(code, glb))  # noqa: S307
        else:
            exec(code, glb)  # noqa: S102
    except Exception as exc:  # noqa: BLE001
        error = f"{type(exc).__name__}: {exc}"
    finally:
        sys.settrace(prev)

    static_ops = [i.opname for i in _dis.get_instructions(code)]
    static_distinct = set(static_ops)
    executed_distinct = set(hist)
    return {
        "schema": "pnix-hy.execution-trace.v0",
        "source": source,
        "mode": mode,
        "result_repr": result_repr,
        "error": error,
        "executed_opcode_count": state["count"],
        "executed_distinct": len(executed_distinct),
        "truncated": state["truncated"],
        "opcode_histogram": dict(sorted(hist.items(), key=lambda kv: (-kv[1], kv[0]))),
        "static_opcode_count": len(static_ops),
        "static_distinct": len(static_distinct),
        "top_module_coverage": sorted(static_distinct & executed_distinct),
        "static_only_opcodes": sorted(static_distinct - executed_distinct),
        "event_sequence": seq,
    }


def execution_trace_report() -> dict[str, Any]:
    """Self-check: a closed expression traces to real, ordered bytecode."""
    try:
        t = execution_trace("(lambda x: x * 10 + 2)(4)", "eval")
        ready = (
            t.get("result_repr") == "42"
            and t.get("error") is None
            and t.get("executed_opcode_count", 0) > 0
            and "BINARY_OP" in t.get("opcode_histogram", {})
            and t.get("event_sequence")
        )
        return {"ready": bool(ready), "executed": t.get("executed_opcode_count"),
                "result": t.get("result_repr"), "distinct": t.get("executed_distinct")}
    except Exception as exc:  # noqa: BLE001
        return {"ready": False, "error": f"{type(exc).__name__}: {exc}"}


def opcode_tables() -> dict[str, Any]:
    """The opcode metadata tables dis exposes."""
    return {
        "opname": list(_dis.opname),
        "opmap": dict(_dis.opmap),
        "cmp_op": list(_dis.cmp_op),
        "hasconst": list(_dis.hasconst),
        "hasname": list(_dis.hasname),
        "hasjrel": list(_dis.hasjrel),
        "hasjabs": list(_dis.hasjabs),
        "haslocal": list(_dis.haslocal),
        "hascompare": list(_dis.hascompare),
        "hasfree": list(_dis.hasfree),
        "HAVE_ARGUMENT": _dis.HAVE_ARGUMENT,
        "EXTENDED_ARG": _dis.EXTENDED_ARG,
    }


def stack_effect(opcode: int, arg: Any = None) -> int:
    return _dis.stack_effect(opcode, arg)


def line_starts(code: Any) -> list[list[int]]:
    return [[off, ln] for off, ln in _dis.findlinestarts(code)]


def marshal_code(code: Any) -> dict[str, Any]:
    """Serialize a code object and round-trip it (marshal owns the bytes)."""
    blob = _marshal.dumps(code)
    code2 = _marshal.loads(blob)
    return {
        "marshal_version": _marshal.version,
        "blob_len": len(blob),
        "blob_hex": blob.hex(),
        "roundtrip_ok": code.co_code == code2.co_code and list(code.co_names) == list(code2.co_names),
    }


def rebuild_code(source: str, mode: str = "eval") -> dict[str, Any]:
    """Own the code object via code.replace()/types.CodeType, then run it -- the
    fast 'we own compile output, host still executes' route."""
    code = compile(source, "<pnix-rebuild>", mode)
    replaced = code.replace(co_name="pnix_rebuilt")
    result: dict[str, Any] = {
        "original_name": code.co_name,
        "replaced_name": replaced.co_name,
        "same_bytecode": code.co_code == replaced.co_code,
        "codetype_available": hasattr(_types, "CodeType"),
    }
    if mode == "eval":
        result["eval_result"] = _json_safe(eval(replaced))
    return result


def function_info(func: Any) -> dict[str, Any]:
    code = func.__code__
    return {
        "name": func.__name__,
        "qualname": func.__qualname__,
        "module": func.__module__,
        "doc": func.__doc__,
        "defaults": [_json_safe(d) for d in (func.__defaults__ or ())],
        "kwdefaults": {k: _json_safe(v) for k, v in (func.__kwdefaults__ or {}).items()},
        "closure": [_json_safe(c.cell_contents) for c in (func.__closure__ or ())],
        "annotations": {k: repr(v) for k, v in getattr(func, "__annotations__", {}).items()},
        "globals_keys_count": len(func.__globals__),
        "signature": str(_inspect.signature(func)),
        "code": code_object_info(code),
    }


def object_info(obj: Any) -> dict[str, Any]:
    return {
        "type": type(obj).__name__,
        "type_mro": [t.__name__ for t in type(obj).__mro__],
        "type_bases": [t.__name__ for t in type(obj).__bases__],
        "id": id(obj),
        "hash": _try(lambda: hash(obj)),
        "dir": dir(obj),
        "dict_keys": sorted(vars(obj).keys()) if hasattr(obj, "__dict__") else None,
        "slots": list(getattr(type(obj), "__slots__", []) or []),
        "sizeof": sys.getsizeof(obj),
        "refcount": sys.getrefcount(obj),
        "is_routine": _inspect.isroutine(obj),
        "is_class": _inspect.isclass(obj),
    }


def ast_info(source: str, mode: str = "exec") -> dict[str, Any]:
    tree = _ast.parse(source, mode=mode)
    body = getattr(tree, "body", None)
    return {
        "dump": _ast.dump(tree),
        "node_types": [type(n).__name__ for n in _ast.walk(tree)],
        "body_len": len(body) if isinstance(body, list) else (1 if body is not None else 0),
    }


def symtable_info(source: str, mode: str = "exec") -> dict[str, Any]:
    if _symtable is None:
        return {"available": False}
    st = _symtable.symtable(source, "<pnix-symtable>", mode)
    return {
        "available": True,
        "type": st.get_type(),
        "identifiers": list(st.get_identifiers()),
        "symbols": [
            {
                "name": s.get_name(),
                "is_global": s.is_global(),
                "is_local": s.is_local(),
                "is_free": s.is_free(),
                "is_parameter": s.is_parameter(),
            }
            for s in st.get_symbols()
        ],
    }


def tokenize_info(source: str) -> dict[str, Any]:
    if _tokenize is None:
        return {"available": False}
    toks: list[dict[str, Any]] = []
    try:
        for tok in _tokenize.generate_tokens(_io.StringIO(source).readline):
            toks.append({"type": _tokenize.tok_name.get(tok.type, tok.type), "string": tok.string})
    except _tokenize.TokenError:
        pass
    return {"available": True, "tokens": toks}


def frame_info(depth: int = 0) -> dict[str, Any]:
    f = sys._getframe(depth + 1)
    return {
        "function": f.f_code.co_name,
        "filename": f.f_code.co_filename,
        "lineno": f.f_lineno,
        "lasti": f.f_lasti,
        "local_keys": sorted(f.f_locals.keys()),
        "global_keys_count": len(f.f_globals),
        "has_back": f.f_back is not None,
        "stack_depth": len(_inspect.stack()),
    }


def trace_run(source: str) -> dict[str, Any]:
    """Run via sys.settrace and record the line/opcode trace. Fast side channel:
    execution stays native; the tracer only records."""
    code = compile(source, "<pnix-trace>", "exec")
    events: list[dict[str, Any]] = []

    def tracer(frame: Any, event: str, arg: Any) -> Any:
        if frame.f_code is code:
            events.append({"event": event, "lineno": frame.f_lineno, "lasti": frame.f_lasti})
        return tracer

    old = sys.gettrace()
    sys.settrace(tracer)
    ns: dict[str, Any] = {}
    try:
        exec(code, ns)
    finally:
        sys.settrace(old)
    return {"event_count": len(events), "events": events[:50]}


def gc_info() -> dict[str, Any]:
    return {
        "counts": list(_gc.get_count()),
        "threshold": list(_gc.get_threshold()),
        "stats": _gc.get_stats(),
        "tracked_objects": len(_gc.get_objects()),
        "enabled": _gc.isenabled(),
    }


def gc_referrers(obj: Any, limit: int = 20) -> dict[str, Any]:
    """gc.get_referrers / get_referents / is_tracked for one object."""
    return {
        "referrers_count": len(_gc.get_referrers(obj)),
        "referents_types": sorted({type(r).__name__ for r in _gc.get_referents(obj)})[:limit],
        "is_tracked": _gc.is_tracked(obj),
    }


def sys_info() -> dict[str, Any]:
    """Interpreter-level introspection via sys (intern, allocated blocks, ...)."""
    return {
        "version": sys.version.split()[0],
        "version_info": list(sys.version_info[:3]),
        "implementation": sys.implementation.name,
        "maxsize": sys.maxsize,
        "byteorder": sys.byteorder,
        "recursionlimit": sys.getrecursionlimit(),
        "allocatedblocks": sys.getallocatedblocks(),
        "intern_identity": sys.intern("pnix-hy") is sys.intern("pnix-hy"),
        "modules_count": len(sys.modules),
        "flags_optimize": sys.flags.optimize,
    }


def traceback_info() -> dict[str, Any]:
    """Call-stack introspection via traceback (extract_stack / walk_stack)."""
    stack = _traceback.extract_stack()
    return {
        "depth": len(stack),
        "frames": [{"name": f.name, "lineno": f.lineno} for f in stack[-5:]],
        "walk_depth": sum(1 for _ in _traceback.walk_stack(sys._getframe())),
    }


def module_info(name: str = "sys") -> dict[str, Any]:
    """Import-system introspection via sys.modules / importlib."""
    import importlib

    mod = sys.modules.get(name) or importlib.import_module(name)
    spec = getattr(mod, "__spec__", None)
    return {
        "name": mod.__name__,
        "file": getattr(mod, "__file__", None),
        "doc_present": mod.__doc__ is not None,
        "spec_name": getattr(spec, "name", None),
        "members_count": len(dir(mod)),
        "loaded_modules": len(sys.modules),
    }


def jump_labels(code: Any) -> list[int]:
    """dis.findlabels: bytecode offsets that are jump targets."""
    return _dis.findlabels(code.co_code)


def full_introspection(source: str, mode: str = "exec") -> dict[str, Any]:
    """One host-direct call: compile + every code/bytecode/marshal/ast/symtable
    facility on the result. Fast (no stage7)."""
    code = compile(source, "<pnix-introspect>", mode)
    return {
        "source": source,
        "mode": mode,
        "code_object": code_object_info(code),
        "bytecode": disassemble(code),
        "line_starts": line_starts(code),
        "marshal": marshal_code(code),
        "ast": ast_info(source, mode),
        "symtable": symtable_info(source, mode),
    }


__all__ = [
    "_json_safe",
    "_own_code_objects",
    "_try",
    "ast_info",
    "code_object_info",
    "compile_source",
    "disassemble",
    "execution_trace",
    "execution_trace_report",
    "frame_info",
    "full_introspection",
    "function_info",
    "gc_info",
    "gc_referrers",
    "jump_labels",
    "line_starts",
    "marshal_code",
    "module_info",
    "object_info",
    "opcode_tables",
    "rebuild_code",
    "stack_effect",
    "symtable_info",
    "sys_info",
    "tokenize_info",
    "trace_run",
    "traceback_info",
]
