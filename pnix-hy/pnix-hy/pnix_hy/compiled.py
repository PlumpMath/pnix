"""pnix_hy.compiled -- a COMPILED runtime for the pnix core subset (proposal 0028 Phase 1).

Compiles a pnix core-subset AST to a tree of Python closures (linked environments, memoized
thunks, native operators), giving a fast execution path that BYPASSES the reference tree-walker.
It is a CONTRAST lane, never the canonical evaluator: `pnix_runtime.eval_source` stays the source
of truth (SACRED, untouched), and `compiled_runtime_report` gates the compiled results against it
over a core-subset corpus.

Supported subset: int/float/bool/string/null, var, binary (+ - * // == != < > && || ++), if,
lambda (curried), apply, let (recursive+lazy), attrset, list, select, and a fixed set of pure
builtins (head/tail/length/elemAt/attrNames/toString/seq/isInt/isBool/isString/hasAttr/getAttr/
listToAttrs/map). Anything else raises -- the compiled lane is deliberately partial.

NOTE (0028): this runtime was built to also make cogen re-derive a full compiler in-budget, but
three experiments confirmed the cogen wall is the NAIVE cogen's own algorithmic inefficiency
(a bloated generating extension), not runtime speed -- so the compiled runtime stands on its own
merit (fast, verified core eval) and the cogen speedup (0028 P2) needs an efficient/optimal cogen,
a research-grade item.
"""

from __future__ import annotations

import sys
import threading
from typing import Any, Callable

from . import pnix_runtime as rt


class _Thunk:
    __slots__ = ("f", "v", "done")

    def __init__(self, f: Callable[[], Any]) -> None:
        self.f = f
        self.v = None
        self.done = False

    def force(self) -> Any:
        if not self.done:
            self.v = self.f()
            self.done = True
            self.f = None  # type: ignore[assignment]
        return self.v


def _force(x: Any) -> Any:
    while type(x) is _Thunk:
        x = x.force()
    return x


def _deep(x: Any) -> Any:
    x = _force(x)
    if isinstance(x, dict):
        return {k: _deep(v) for k, v in x.items()}
    if isinstance(x, list):
        return [_deep(v) for v in x]
    return x


class _Clo:
    __slots__ = ("p", "b", "e")

    def __init__(self, p: str, b: Callable, e: Any) -> None:
        self.p, self.b, self.e = p, b, e


_B_MARK = ("__b__",)


def _bi(name: str, args: tuple) -> tuple:
    return ("__bi__", name, args)


def _look(env: Any, nm: str) -> Any:
    while env is not None:
        d, env = env
        if nm in d:
            return d[nm]
    raise rt.PnixError(f"undefined variable '{nm}'")


def _ts(v: Any) -> str:
    v = _force(v)
    return "true" if v is True else "false" if v is False else str(v)


def _eq(a: Any, b: Any) -> bool:
    return _deep(a) == _deep(b)


_BIN = {
    "+": lambda a, b: _force(a) + _force(b),
    "-": lambda a, b: _force(a) - _force(b),
    "*": lambda a, b: _force(a) * _force(b),
    "//": lambda a, b: {**_force(a), **_force(b)},
    "==": _eq,
    "!=": lambda a, b: not _eq(a, b),
    "<": lambda a, b: _force(a) < _force(b),
    ">": lambda a, b: _force(a) > _force(b),
    "++": lambda a, b: list(_force(a)) + list(_force(b)),
}


def _hasattr_strict(name, base):
    if not isinstance(base, dict):
        raise ValueError("hasAttr base must be an attrset")
    return name in base

_BUILTINS: dict[str, tuple[int, Callable]] = {
    "head": (1, lambda xs: _force(xs)[0]),
    "tail": (1, lambda xs: _force(xs)[1:]),
    "length": (1, lambda xs: len(_force(xs))),
    "elemAt": (2, lambda xs, i: _force(xs)[_force(i)]),
    "attrNames": (1, lambda s: sorted(_force(s).keys())),
    "toString": (1, _ts),
    "seq": (2, lambda a, b: (_force(a), b)[1]),
    "isInt": (1, lambda v: isinstance(_force(v), int) and not isinstance(_force(v), bool)),
    "isBool": (1, lambda v: isinstance(_force(v), bool)),
    "isString": (1, lambda v: isinstance(_force(v), str)),
    # hasAttr must ERROR on a non-attrset (canonical semantics); Python `in`
    # would silently answer membership on lists/strings — raise so the
    # out-of-subset input falls back to the canonical evaluator (audit 2026-07-08).
    "hasAttr": (2, lambda n, s: _hasattr_strict(_force(n), _force(s))),
    "getAttr": (2, lambda n, s: _force(s)[_force(n)]),
    "listToAttrs": (1, lambda xs: {_force(_force(e)["name"]): _force(e)["value"] for e in _force(xs)}),
    "map": (2, lambda f, xs: [_apply(f, x) for x in _force(xs)]),
}


def _apply(fn: Any, arg: Any) -> Any:
    fn = _force(fn)
    if isinstance(fn, _Clo):
        return fn.b(({fn.p: arg}, fn.e))
    if isinstance(fn, tuple) and fn and fn[0] == "__bi__":
        name, args = fn[1], fn[2] + (arg,)
        ar, impl = _BUILTINS[name]
        return impl(*args) if len(args) >= ar else _bi(name, args)
    raise rt.PnixError("compiled: call of a non-function")


def compile_node(node: dict[str, Any]) -> Callable[[Any], Any]:
    """Compile a core-subset AST node to a Python closure `fn(env) -> value` (values are thunks
    or realized). Dispatch happens ONCE at compile time, not per evaluation step."""
    t = node["tag"]
    if t in ("int", "float", "string", "bool"):
        v = node["value"]
        return lambda env: v
    if t == "null":
        return lambda env: None
    if t == "var":
        nm = node["name"]
        if nm == "builtins":
            return lambda env: _B_MARK
        return lambda env: _look(env, nm)
    if t == "binary":
        op = node["op"]
        lf, rf = compile_node(node["lhs"]), compile_node(node["rhs"])
        if op == "&&":
            return lambda env: (_force(lf(env)) and _force(rf(env)))
        if op == "||":
            return lambda env: (_force(lf(env)) or _force(rf(env)))
        fn = _BIN[op]
        return lambda env: fn(lf(env), rf(env))
    if t == "if":
        cf, tf, ef = compile_node(node["cond"]), compile_node(node["then"]), compile_node(node["else"])
        return lambda env: (tf(env) if _force(cf(env)) else ef(env))
    if t == "lambda":
        p, bf = node["param"], compile_node(node["body"])
        return lambda env: _Clo(p, bf, env)
    if t == "apply":
        ff, af = compile_node(node["func"]), compile_node(node["arg"])
        return lambda env: _apply(ff(env), _Thunk(lambda env=env: af(env)))
    if t == "select":
        bf, attr = compile_node(node["base"]), node["attr"]
        def sel(env: Any, bf: Any = bf, attr: str = attr) -> Any:
            b = _force(bf(env))
            if isinstance(b, tuple) and b and b[0] == "__b__":
                return _bi(attr, ())
            return b[attr]
        return sel
    if t == "attrset":
        if node.get("recursive"):
            raise rt.PnixError("compiled: rec attrset unsupported")
        items = [(b["path"][0], compile_node(b["value"])) for b in node["bindings"]
                 if len(b.get("path", [])) == 1]
        if len(items) != len(node["bindings"]):
            raise rt.PnixError("compiled: multi-path attrset unsupported")
        return lambda env: {nm: _Thunk(lambda env=env, vf=vf: vf(env)) for nm, vf in items}
    if t == "list":
        fns = [compile_node(x) for x in node["items"]]
        return lambda env: [_Thunk(lambda env=env, vf=vf: vf(env)) for vf in fns]
    if t == "let":
        binds = [(b["path"][0], compile_node(b["value"])) for b in node["bindings"]
                 if len(b.get("path", [])) == 1]
        if len(binds) != len(node["bindings"]):
            raise rt.PnixError("compiled: multi-path let unsupported")
        bf = compile_node(node["body"])
        def dl(env: Any, binds: Any = binds, bf: Any = bf) -> Any:
            d: dict[str, Any] = {}
            frame = (d, env)
            for nm, vf in binds:
                d[nm] = _Thunk(lambda vf=vf, frame=frame: vf(frame))
            return bf(frame)
        return dl
    raise rt.PnixError(f"compiled: unsupported tag '{t}'")


def compiled_eval(source: str, *, big_stack: bool = True) -> Any:
    """Evaluate pnix core-subset `source` on the compiled runtime; returns the realized value.
    Deep force chains run in a large-stack worker thread (the compiled recursion IS the work)."""
    if not big_stack:
        return _deep(compile_node(rt.ast_stable(rt.parse(source)))(None))
    out: dict[str, Any] = {}

    def go() -> None:
        old = sys.getrecursionlimit()
        sys.setrecursionlimit(8_000_000)
        try:
            out["v"] = _deep(compile_node(rt.ast_stable(rt.parse(source)))(None))
        except BaseException as exc:  # noqa: BLE001 - surface in caller
            out["e"] = exc
        finally:
            sys.setrecursionlimit(old)

    old_stack = threading.stack_size()
    threading.stack_size(1024 * 1024 * 1024)
    try:
        worker = threading.Thread(target=go, name="pnix-compiled")
        worker.start()
        worker.join()
    finally:
        threading.stack_size(old_stack)
    if "e" in out:
        raise out["e"]
    return out["v"]


_CORPUS = [
    "2 * 3 + 4", "(10 - 3) * 2", "if 1 < 2 then 10 else 20", "if 2 < 1 then 10 else 20",
    "let a = 5; b = a + 1; in a * b", "let f = x: x + 1; in f 41", "(a: b: a + b) 3 4",
    "let f = n: if n == 0 then 0 else 1 + (f (n - 1)); in f 25",
    "let fac = n: if n == 0 then 1 else n * (fac (n - 1)); in fac 6",
    '{ a = 1; b = 2; }', '{ a = 1; b = [ (2) (3) ]; }.b',
    "[ (1) (2 + 1) (if true then 3 else 9) ]",
    'builtins.length [ (1) (2) (3) ]', 'builtins.head [ (7) (8) ]', 'builtins.tail [ (7) (8) (9) ]',
    'builtins.attrNames { b = 1; a = 2; }', 'builtins.toString 42',
    'builtins.hasAttr "a" { a = 1; }', 'builtins.getAttr "a" { a = 9; }',
    'builtins.map (x: x * 2) [ (1) (2) (3) ]',
    'builtins.listToAttrs [ { name = "x"; value = 1; } { name = "y"; value = 2; } ]',
    '{ a = 1; } // { b = 2; }', 'true && false', 'true || false', '1 == 1', '1 != 2',
    '[ (1) ] ++ [ (2) (3) ]',
]


_BENCH_CASES = [
    ("fib 22", "let fib = n: if n < 2 then n else (fib (n - 1)) + (fib (n - 2)); in fib 22", 1),
    ("countdown 3000", "let f = n: if n == 0 then 0 else 1 + (f (n - 1)); in f 3000", 1),
    ("map/length x2000", "let g = xs: builtins.length (builtins.map (x: x * x) xs); "
                         "in g [ (1)(2)(3)(4)(5)(6)(7)(8)(9)(10) ]", 2000),
    ("small arith x4000", "2 * 3 + 4 * 5 - 6", 4000),
]


def compiled_bench(now: Callable[[], float], cases: list | None = None) -> dict[str, Any]:
    """Benchmark the compiled runtime against the canonical tree-walker on core-subset programs.
    Timing is nondeterministic, so this is NOT a --check report; `now` is injected (e.g.
    time.perf_counter) to keep the module import-time-pure. Returns per-case wall times +
    speedup, and asserts result agreement."""
    import sys  # noqa: PLC0415

    cases = cases or _BENCH_CASES
    old = sys.getrecursionlimit()
    sys.setrecursionlimit(200000)
    rows = []
    try:
        for name, src, reps in cases:
            t0 = now()
            for _ in range(reps):
                ref = rt.stable_data(rt.eval_source(src))
            tw = now() - t0
            t0 = now()
            for _ in range(reps):
                got = compiled_eval(src, big_stack=False)
            cc = now() - t0
            rows.append({"case": name, "reps": reps, "tree_walker_s": round(tw, 4),
                         "compiled_s": round(cc, 4), "speedup": round(tw / cc, 1) if cc else None,
                         "agree": got == ref})
    finally:
        sys.setrecursionlimit(old)
    return {"schema": "pnix-hy.compiled-bench.v0", "rows": rows,
            "all_agree": all(r["agree"] for r in rows)}


def compiled_runtime_report() -> dict[str, Any]:
    """Self-check (proposal 0028 P1): the compiled runtime agrees with the canonical
    `pnix_runtime.eval_source` on every core-subset corpus program."""
    try:
        mismatches: list[dict[str, Any]] = []
        for src in _CORPUS:
            ref = rt.stable_data(rt.eval_source(src))
            got = compiled_eval(src, big_stack=False)
            if got != ref:
                mismatches.append({"source": src, "ref": ref, "compiled": got})
        ready = not mismatches
        return {"schema": "pnix-hy.compiled-runtime.report.v0", "ready": ready, "available": True,
                "corpus": len(_CORPUS), "agree": len(_CORPUS) - len(mismatches),
                "mismatches": mismatches[:8],
                "note": "compiled core-subset lane; canonical evaluator is pnix_runtime (SACRED)"}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.compiled-runtime.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


# --- 0028 P1 application: DIFFERENTIAL oracle (compiled lane vs canonical tree-walker) ---
# Two independent evaluators must converge -- the same principle as the 4-lane mirror, now
# extended to the compiled lane over a DETERMINISTICALLY GENERATED core-subset corpus. Every
# generated program is total (no division, well-typed conditions) so both evaluators succeed and
# must agree; a divergence would be a real bug in one of the two.

def differential_corpus() -> list[str]:
    """A deterministic, total core-subset corpus (no RNG): arithmetic, comparisons+if, let,
    lambda/apply, list+map+length, attrset+select -- systematically combined."""
    ints = ["0", "1", "2", "3", "5"]
    ops = ["+", "-", "*"]
    cmps = ["==", "!=", "<", ">"]
    # depth-1 arithmetic over the int base
    a1 = [f"({x} {op} {y})" for op in ops for x in ints[:3] for y in ints[:3]]
    # depth-2 arithmetic (nest a1 into another op)
    a2 = [f"({e} {op} {z})" for op in ops for e in a1[:9] for z in ints[:2]]
    progs: list[str] = []
    progs += ints + a1 + a2[:20]
    # comparisons + if
    for c in cmps:
        for e in a1[:4]:
            progs.append(f"if {e} {c} 3 then {e} else (0 - {e})")
    # let bindings (data + recursion via a named function)
    for e in a1[:6]:
        progs.append(f"let a = {e}; b = (a * 2); in (a + b)")
    progs.append("let f = n: if n == 0 then 0 else n + (f (n - 1)); in f 8")
    progs.append("let g = n: if n < 2 then n else (g (n - 1)) + (g (n - 2)); in g 10")
    # lambda / apply (unary + curried)
    for e in a1[:6]:
        progs.append(f"(x: x + {e}) 7")
    progs.append("(a: b: a * b + 1) 6 7")
    # list + map + length
    for k in ints[:4]:
        progs.append(f"builtins.length (builtins.map (x: x + {k}) [ (1) (2) (3) (4) ])")
    progs.append("builtins.head (builtins.map (x: x * x) [ (2) (3) (4) ])")
    progs.append("builtins.elemAt [ (10) (20) (30) ] 1")
    # attrset + select + //
    for e in a1[:5]:
        progs.append(f"{{ a = {e}; b = ({e} + 1); }}.b")
    progs.append('({ a = 1; } // { b = 2; }).b')
    progs.append('builtins.attrNames { c = 1; a = 2; b = 3; }')
    # booleans
    progs += ["true && false", "true || false", "(1 < 2) && (3 > 2)", "(1 == 2) || (2 == 2)"]
    # de-dup, keep order
    seen: set[str] = set()
    out: list[str] = []
    for p in progs:
        if p not in seen:
            seen.add(p)
            out.append(p)
    return out


def compiled_differential_report() -> dict[str, Any]:
    """Self-check (0028 application): the compiled lane and the canonical tree-walker AGREE on
    every program in a deterministic core-subset corpus -- a differential oracle across two
    independent evaluators (bug in either -> a divergence here)."""
    try:
        corpus = differential_corpus()
        mismatches: list[dict[str, Any]] = []
        for src in corpus:
            try:
                ref = rt.stable_data(rt.eval_source(src))
            except Exception as exc:  # noqa: BLE001 - a generated program must be total
                mismatches.append({"source": src, "kind": "canonical-error", "error": str(exc)[:80]})
                continue
            try:
                got = compiled_eval(src, big_stack=False)
            except Exception as exc:  # noqa: BLE001
                mismatches.append({"source": src, "kind": "compiled-error", "error": str(exc)[:80]})
                continue
            if got != ref:
                mismatches.append({"source": src, "ref": ref, "compiled": got})
        ready = len(corpus) >= 50 and not mismatches
        return {"schema": "pnix-hy.compiled-differential.report.v0", "ready": ready,
                "available": True, "corpus": len(corpus),
                "agree": len(corpus) - len(mismatches), "mismatches": mismatches[:8]}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.compiled-differential.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


# --- 0028 P3: usable fast path (subset-aware, auto-fallback to the canonical evaluator) ---

def subset_supported(source: str) -> bool:
    """True iff `source` is within the compiled runtime's core subset (compiles without hitting
    an 'unsupported' node). Compilation is cheap and side-effect-free, so this is a safe probe."""
    try:
        compile_node(rt.ast_stable(rt.parse(source)))
        return True
    except Exception:  # noqa: BLE001 - parse/unsupported -> not in the compiled subset
        return False


def evaluate(source: str) -> dict[str, Any]:
    """Evaluate via the FAST compiled runtime when `source` is in the core subset, else fall back
    to the canonical `pnix_runtime` -- always returning the canonical value either way. Reports
    which backend ran. (0028 P3: makes the compiled speedup usable while staying total.)"""
    if subset_supported(source):
        try:
            return {"schema": "pnix-hy.evaluate.v0", "backend": "compiled",
                    "value": compiled_eval(source)}
        except Exception:  # noqa: BLE001 - any compiled surprise -> canonical (soundness first)
            pass
    return {"schema": "pnix-hy.evaluate.v0", "backend": "canonical",
            "value": rt.stable_data(rt.eval_source(source))}


def evaluate_report() -> dict[str, Any]:
    """Self-check (0028 P3): the fast-path picks the compiled backend for in-subset programs and
    the canonical backend otherwise, and BOTH yield the canonical value."""
    try:
        subset = "let f = n: if n == 0 then 0 else 1 + (f (n - 1)); in f 30"
        out = "rec { x = 1; y = x + 41; }.y"   # rec attrset -> outside the compiled subset
        r1 = evaluate(subset)
        r2 = evaluate(out)
        ok = (r1["backend"] == "compiled" and r1["value"] == rt.stable_data(rt.eval_source(subset))
              and r2["backend"] == "canonical" and r2["value"] == rt.stable_data(rt.eval_source(out))
              and subset_supported(subset) and not subset_supported(out))
        return {"schema": "pnix-hy.evaluate.report.v0", "ready": bool(ok), "available": True,
                "subset_backend": r1["backend"], "fallback_backend": r2["backend"]}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.evaluate.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


__all__ = ["compile_node", "compiled_eval", "compiled_bench",
           "compiled_runtime_report", "differential_corpus", "compiled_differential_report",
           "subset_supported", "evaluate", "evaluate_report"]
