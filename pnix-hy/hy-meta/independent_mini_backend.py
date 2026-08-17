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
recursion, calling between top-level `defn`s, boolean/`None` literals,
string/list/dict/keyword literals, `get` subscript access, dot-prefixed
method calls (`(.method target args...)`), `setv`/`while` mutation, and
genuine closures via `fn` (single-expression body only)), not the Hy
language.
"""

from __future__ import annotations

import ast
import re
from typing import Any

# `hy.models.Keyword` is a plain VALUE type (like Python's own `ast` module
# is treated as trusted substrate elsewhere in this file) -- importing it is
# not the same as reusing `hy.reader`/`hy.compiler`'s READING or COMPILING
# logic (the actual thing this witness independently re-derives). It's
# needed so a keyword literal compiled by this backend constructs the exact
# same runtime value real Hy's own compiled output does (`Keyword("a") ==
# Keyword("a")` by `.name`), so `mini_result == expected` comparisons in the
# 3-way check are meaningful for keyword-keyed fixtures.
from hy.models import Keyword

TOKEN_RE = re.compile(r'\s*("(?:[^"\\]|\\.)*"|\(|\)|\[|\]|\{|\}|-?\d+|[^\s()\[\]{}]+)')


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
    if tok == "{":
        items = []
        i += 1
        while tokens[i] != "}":
            item, i = _parse_one(tokens, i)
            items.append(item)
        if len(items) % 2 != 0:
            raise SyntaxError("tiny reader: malformed dict literal")
        pairs = [(items[k], items[k + 1]) for k in range(0, len(items), 2)]
        return ("__dict__", pairs), i + 1
    if tok in (")", "]", "}"):
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
    if tok.startswith(":") and len(tok) > 1:
        return ("__kw__", tok[1:]), i + 1
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


def _is_dict(form: Any) -> bool:
    return isinstance(form, tuple) and len(form) == 2 and form[0] == "__dict__"


def _is_kw(form: Any) -> bool:
    return isinstance(form, tuple) and len(form) == 2 and form[0] == "__kw__"


_KEYWORD_NAME = "__mini_backend_Keyword__"


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
    if _is_kw(form):
        # `:name` -- real Hy reads this as a `hy.models.Keyword("name")`
        # value; compiled code constructs one at runtime the same way real
        # Hy's own compiled output does (see the `hy.models` import note at
        # the top of this file).
        _, name = form
        return ast.Call(
            func=ast.Name(id=_KEYWORD_NAME, ctx=ast.Load()),
            args=[ast.Constant(value=name)],
            keywords=[],
        )
    if _is_dict(form):
        # Keys are string OR keyword literals -- `_emit_expr` on the key
        # form handles both uniformly now.
        _, pairs = form
        keys = []
        values = []
        for key_f, value_f in pairs:
            if not (isinstance(key_f, str) or _is_kw(key_f)):
                raise SyntaxError(
                    "tiny analyzer: dict literal key must be a string or keyword literal"
                )
            keys.append(_emit_expr(key_f))
            values.append(_emit_expr(value_f))
        return ast.Dict(keys=keys, values=values)
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
        if _is_sym(head, "fn"):
            # `(fn [params...] EXPR)` -- a genuine closure, single-expression
            # body only (real Hy's `fn` also accepts multiple body forms
            # with setv/while like `defn`, but that needs a statement-
            # capable function value, which Python's `ast.Lambda` can't
            # express; no fixture needs it, so it's not attempted here).
            # Compiles straight to `ast.Lambda`, which gets Python's own
            # closure semantics (capture-by-reference to the enclosing
            # scope, late-binding) for free -- no env bookkeeping needed on
            # this backend's side, unlike the clr-meta/rs-meta hosts' mini
            # backends, which had to build their own closure value
            # representation from scratch. Calling a closure-bound name is
            # ALSO already handled by the existing bare `_is_sym(head)` ->
            # `ast.Call` case below: Python's own name resolution doesn't
            # care whether a name is bound to a `def`-made function or a
            # lambda, so no separate dispatch is needed the way the other
            # hosts' mini backends required.
            if len(args) != 2:
                raise SyntaxError(
                    "tiny analyzer: fn (closure) needs exactly [params] and one body expression"
                )
            params_f, body_f = args
            if not isinstance(params_f, list):
                raise SyntaxError("tiny analyzer: fn params must be a vector")
            params = [_sym_name(p) for p in params_f]
            return ast.Lambda(
                args=ast.arguments(
                    posonlyargs=[],
                    args=[ast.arg(arg=p) for p in params],
                    vararg=None,
                    kwonlyargs=[],
                    kw_defaults=[],
                    kwarg=None,
                    defaults=[],
                ),
                body=_emit_expr(body_f),
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
        if _is_sym(head, "get"):
            # `(get target index)` -- Hy special form for subscript access
            # (`target[index]`), NOT a Python builtin (there is no free
            # function named `get`), so it needs its own case here rather
            # than falling through to the generic call dispatch below.
            if len(args) != 2:
                raise SyntaxError("tiny analyzer: get arity")
            return ast.Subscript(
                value=_emit_expr(args[0]),
                slice=_emit_expr(args[1]),
                ctx=ast.Load(),
            )
        if _is_sym(head) and _sym_name(head).startswith("."):
            # `(.method target args...)` -- Hy's dot-prefixed method-call
            # sugar, compiling to `target.method(args...)`. Needed for
            # mutation methods like `(.append lst x)` (no dedicated
            # `setv`-like list-mutation special form exists in Hy; methods
            # are how it's done), but works for any zero-or-more-arg method
            # call generally.
            method = _sym_name(head)[1:]
            if not method or not args:
                raise SyntaxError("tiny analyzer: .method call needs a method name and a target")
            target_f, method_args = args[0], args[1:]
            return ast.Call(
                func=ast.Attribute(value=_emit_expr(target_f), attr=method, ctx=ast.Load()),
                args=[_emit_expr(a) for a in method_args],
                keywords=[],
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
    namespace: dict[str, Any] = {_KEYWORD_NAME: Keyword}
    exec(code, namespace)  # noqa: S102 - trusted, hand-built AST, no eval of untrusted strings
    return namespace[_RESULT_NAME]
