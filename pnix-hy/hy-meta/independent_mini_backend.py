"""Tiny independent Hy-subset-to-Python-AST compiler.

This module is a Trusting-Trust (Diverse Double-Compiling) witness: it parses
a small Hy source subset with its own hand-written tokenizer/reader, builds
Python `ast` nodes directly, and hands them to the stdlib `compile()` builtin.
It shares zero code with `hy.reader`, `hy.compiler`, `stage1/compiler.py`
(the upstream-seeded path), and `stage2/kernel.hy` (the direct-kernel
self-hosted path). Python's own `ast`/`compile()`/`exec()` remain trusted host
substrate here, the same way the JVM classfile format and `clojure.asm`
remain trusted substrate for clj-meta's analogous `frontend_selfhost.clj`.

It is a frontier witness, not a replacement for the production frontend: it
covers a bounded fixture set (arithmetic, comparisons, `if`, `defn`,
recursion, boolean/`None` literals, string/list literals, and `setv`/`while`
mutation), not the Hy language.
"""

from __future__ import annotations

import ast
import re
from typing import Any

TOKEN_RE = re.compile(r'\s*("(?:[^"\\]|\\.)*"|\(|\)|\[|\]|-?\d+|[^\s()\[\]]+)')


def _tokenize(source: str) -> list[str]:
    tokens = []
    pos = 0
    while pos < len(source):
        m = TOKEN_RE.match(source, pos)
        if m is None:
            if source[pos:].strip() == "":
                break
            raise SyntaxError(f"tiny reader: unexpected input at {pos!r}")
        pos = m.end()
        tok = m.group(1)
        if tok is not None:
            tokens.append(tok)
    return tokens


def _parse_one(tokens: list[str], i: int) -> tuple[Any, int]:
    tok = tokens[i]
    if tok == "(":
        items = []
        i += 1
        while tokens[i] != ")":
            item, i = _parse_one(tokens, i)
            items.append(item)
        return tuple(items), i + 1
    if tok == "[":
        items = []
        i += 1
        while tokens[i] != "]":
            item, i = _parse_one(tokens, i)
            items.append(item)
        return list(items), i + 1
    if tok in (")", "]"):
        raise SyntaxError(f"tiny reader: unexpected closing delimiter {tok!r}")
    if tok == "True":
        return True, i + 1
    if tok == "False":
        return False, i + 1
    if tok == "None":
        return None, i + 1
    if tok.startswith('"') and tok.endswith('"'):
        return ast.literal_eval(tok), i + 1
    if re.fullmatch(r"-?\d+", tok):
        return int(tok), i + 1
    return ("__sym__", tok), i + 1


def tiny_read_all(source: str) -> list[Any]:
    """Read every top-level form in `source`. No dependency on hy.reader."""
    tokens = _tokenize(source)
    forms = []
    i = 0
    while i < len(tokens):
        form, i = _parse_one(tokens, i)
        forms.append(form)
    return forms


_BINOPS = {"+": ast.Add(), "-": ast.Sub(), "*": ast.Mult()}
_CMPOPS = {
    "<": ast.Lt(),
    ">": ast.Gt(),
    "<=": ast.LtE(),
    ">=": ast.GtE(),
    "=": ast.Eq(),
}


def _is_sym(form: Any, name: str | None = None) -> bool:
    return (
        isinstance(form, tuple)
        and len(form) == 2
        and form[0] == "__sym__"
        and (name is None or form[1] == name)
    )


def _sym_name(form: Any) -> str:
    if not _is_sym(form):
        raise SyntaxError(f"tiny analyzer: expected symbol, got {form!r}")
    return form[1]


def _emit_expr(form: Any) -> ast.expr:
    if isinstance(form, bool):
        return ast.Constant(value=form)
    if isinstance(form, int):
        return ast.Constant(value=form)
    if isinstance(form, str):
        return ast.Constant(value=form)
    if form is None:
        return ast.Constant(value=None)
    if isinstance(form, list):
        return ast.List(elts=[_emit_expr(item) for item in form], ctx=ast.Load())
    if _is_sym(form):
        return ast.Name(id=_sym_name(form), ctx=ast.Load())
    if isinstance(form, tuple):
        if not form:
            raise SyntaxError("tiny analyzer: empty call form")
        head = form[0]
        args = form[1:]
        if _is_sym(head, "if"):
            if len(args) != 3:
                raise SyntaxError("tiny analyzer: if arity")
            test_f, then_f, else_f = args
            return ast.IfExp(
                test=_emit_expr(test_f),
                body=_emit_expr(then_f),
                orelse=_emit_expr(else_f),
            )
        if _is_sym(head) and _sym_name(head) in _BINOPS:
            if len(args) != 2:
                raise SyntaxError("tiny analyzer: binary op arity")
            return ast.BinOp(
                left=_emit_expr(args[0]),
                op=_BINOPS[_sym_name(head)],
                right=_emit_expr(args[1]),
            )
        if _is_sym(head) and _sym_name(head) in _CMPOPS:
            if len(args) != 2:
                raise SyntaxError("tiny analyzer: compare op arity")
            return ast.Compare(
                left=_emit_expr(args[0]),
                ops=[_CMPOPS[_sym_name(head)]],
                comparators=[_emit_expr(args[1])],
            )
        if _is_sym(head):
            return ast.Call(
                func=ast.Name(id=_sym_name(head), ctx=ast.Load()),
                args=[_emit_expr(a) for a in args],
                keywords=[],
            )
        raise SyntaxError(f"tiny analyzer: unsupported call head {head!r}")
    raise SyntaxError(f"tiny analyzer: unsupported form {form!r}")


def _emit_stmt(form: Any) -> ast.stmt:
    """Statement-position forms: `setv` (mutation) and `while` (loop). Any
    other form is a bare expression statement (evaluated for side effects,
    if any, and discarded)."""
    if isinstance(form, tuple) and form and _is_sym(form[0], "setv"):
        if len(form) != 3:
            raise SyntaxError("tiny analyzer: setv arity")
        _, name_f, value_f = form
        return ast.Assign(
            targets=[ast.Name(id=_sym_name(name_f), ctx=ast.Store())],
            value=_emit_expr(value_f),
        )
    if isinstance(form, tuple) and form and _is_sym(form[0], "while"):
        if len(form) < 2:
            raise SyntaxError("tiny analyzer: while needs a test")
        _, test_f, *body_fs = form
        if not body_fs:
            raise SyntaxError("tiny analyzer: while needs a body")
        return ast.While(
            test=_emit_expr(test_f),
            body=[_emit_stmt(b) for b in body_fs],
            orelse=[],
        )
    return ast.Expr(value=_emit_expr(form))


def _emit_defn(form: tuple) -> ast.FunctionDef:
    # (defn name [params...] body...): every body form but the last is a
    # statement (setv/while/expression-for-effect); the last form's value
    # is returned.
    _, name_f, params_f, *body_fs = form
    name = _sym_name(name_f)
    if not isinstance(params_f, list):
        raise SyntaxError("tiny analyzer: defn params must be a vector")
    params = [_sym_name(p) for p in params_f]
    if not body_fs:
        raise SyntaxError("tiny analyzer: defn needs a body")
    body_stmts = [_emit_stmt(f) for f in body_fs[:-1]]
    body_stmts.append(ast.Return(value=_emit_expr(body_fs[-1])))
    return ast.FunctionDef(
        name=name,
        args=ast.arguments(
            posonlyargs=[],
            args=[ast.arg(arg=p) for p in params],
            vararg=None,
            kwonlyargs=[],
            kw_defaults=[],
            kwarg=None,
            defaults=[],
        ),
        body=body_stmts,
        decorator_list=[],
    )


_RESULT_NAME = "__mini_backend_result__"


def compile_and_eval(source: str) -> Any:
    """Read every top-level form; `defn` forms become real functions in the
    module namespace, and the value of the final form is returned."""
    forms = tiny_read_all(source)
    if not forms:
        raise SyntaxError("tiny reader: empty source")
    stmts: list[ast.stmt] = []
    for form in forms[:-1]:
        if isinstance(form, tuple) and form and _is_sym(form[0], "defn"):
            stmts.append(_emit_defn(form))
        else:
            stmts.append(ast.Expr(value=_emit_expr(form)))
    last = forms[-1]
    if isinstance(last, tuple) and last and _is_sym(last[0], "defn"):
        stmts.append(_emit_defn(last))
        stmts.append(
            ast.Assign(
                targets=[ast.Name(id=_RESULT_NAME, ctx=ast.Store())],
                value=ast.Constant(value=None),
            )
        )
    else:
        stmts.append(
            ast.Assign(
                targets=[ast.Name(id=_RESULT_NAME, ctx=ast.Store())],
                value=_emit_expr(last),
            )
        )
    module = ast.Module(body=stmts, type_ignores=[])
    ast.fix_missing_locations(module)
    code = compile(module, "<independent-mini-backend>", "exec")
    namespace: dict[str, Any] = {}
    exec(code, namespace)  # noqa: S102 - trusted, hand-built AST, no eval of untrusted strings
    return namespace[_RESULT_NAME]
