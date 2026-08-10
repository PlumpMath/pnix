"""pnix_hy.repl -- context-retaining REPL front-ends (proposal 0008).

A REPL is a LONG-LIVED WARM process: the pnix runtime (and, for the hy/python modes, the
interpreter + `import hy`) stay hot, so this is FASTER than repeated CLI calls, not slower. The
"REPL is slow" concern only applies to a REPL that re-spawns / cold-imports per line, which this
avoids by keeping everything in one process.

This is a thin FRONT-END over the existing library; it is never imported by the eval hot path.
Only the pnix REPL is new logic (an accumulating pnix env threaded through
`rt.eval_source_raw(..., ctx)` so bindings persist across inputs -- context retention for a
pure/lazy language). The python/hy modes just launch the stdlib / Hy REPL.
"""

from __future__ import annotations

import re
import sys
from typing import Any, TextIO

from . import pnix_runtime as rt
from .pnix_mirror import diagnose

_BANNER = "pnix REPL -- pure/lazy, context-retaining. :help for commands, :quit (or Ctrl-D) to exit."
_HELP = (
    "  <expr>            evaluate a pnix expression in the current env; result bound to _\n"
    "  name = <expr>     bind `name` (persists across inputs); also `:let name = <expr>`\n"
    "  :env              list current bindings\n"
    "  :reset            clear all bindings\n"
    "  :help             this help\n"
    "  :quit / Ctrl-D    exit"
)

# `name = expr` where name is a pnix-style identifier and it is not `==` (avoid attrset/let exprs,
# which begin with `{` / `let ` / `rec ` and are handled as plain expressions).
_BIND_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_']*)\s*=(?!=)\s*(.+)\Z", re.S)


def _parse_binding(line: str) -> tuple[str | None, str]:
    s = line.strip()
    if s.startswith(":let "):
        s = s[len(":let "):].strip()
    elif s.startswith("{") or s.startswith("let ") or s.startswith("rec ") or s.startswith("with "):
        return None, line  # a pnix expression, not a REPL binding
    m = _BIND_RE.match(s)
    if m:
        return m.group(1), m.group(2)
    return None, line


def _format(value: Any) -> str:
    try:
        return repr(rt.stable_data(value))
    except Exception:  # noqa: BLE001 - unrepresentable (e.g. a function) -> a marker
        return repr(value)[:200]


def run_pnix_repl(inp: TextIO | None = None, out: TextIO | None = None) -> int:
    """The pnix REPL loop. Bindings accumulate in `ctx['env']` so context is retained across
    inputs; a bad line is reported (via diagnose) and the session survives."""
    inp = inp or sys.stdin
    out = out or sys.stdout

    def emit(text: str = "") -> None:
        out.write(text + "\n")
        out.flush()

    ctx = rt.runtime_context(None)
    ctx["env"] = {}
    env = ctx["env"]
    emit(_BANNER)
    while True:
        out.write("pnix> ")
        out.flush()
        line = inp.readline()
        if not line:  # EOF
            emit("")
            break
        stripped = line.strip()
        if not stripped:
            continue
        if stripped in (":quit", ":q"):
            break
        if stripped in (":help", ":h"):
            emit(_HELP)
            continue
        if stripped == ":env":
            emit(", ".join(sorted(k for k in env if k != "_")) or "(empty)")
            continue
        if stripped == ":reset":
            env.clear()
            emit("(env reset)")
            continue
        name, expr = _parse_binding(line.rstrip("\n"))
        try:
            if name is not None:
                env[name] = rt.eval_source_raw(expr, ctx, realize=False)  # lazy, persists
                emit(f"{name} bound")
            else:
                raw = rt.eval_source_raw(line.rstrip("\n"), ctx, realize=False)
                emit(_format(rt.realize_value(raw)))
                env["_"] = raw  # last result (lazy) only after successful realize
        except Exception as exc:  # noqa: BLE001 - keep the session alive; show a diagnostic
            emit(f"error: {type(exc).__name__}: {exc}")
            diag = diagnose(line.rstrip("\n"))
            excerpt = diag.get("excerpt") if isinstance(diag, dict) else None
            if excerpt:
                emit(excerpt)
    return 0


def run_repl(mode: str = "pnix") -> int:
    """Launch one of the context-retaining REPL modes. `python`/`hy` are stdlib/Hy REPL
    launchers (already-existing interpreters, kept warm); `pnix` is the loop above."""
    if mode == "pnix":
        return run_pnix_repl()
    if mode == "python":
        import code  # noqa: PLC0415
        import pnix_hy as ph  # noqa: PLC0415
        code.interact(
            banner="pnix-hy python REPL -- `ph` / `pnix_hy` is the toolkit; Ctrl-D to exit.",
            local={"ph": ph, "pnix_hy": ph},
            exitmsg="",
        )
        return 0
    if mode == "hy":
        import subprocess  # noqa: PLC0415
        from .hy_mirror import hy_python  # noqa: PLC0415
        try:
            py = str(hy_python())
        except Exception as exc:  # noqa: BLE001
            print(f"hy REPL unavailable: {exc}", file=sys.stderr)
            return 2
        return subprocess.call([py, "-m", "hy"])
    raise ValueError(f"unknown repl mode: {mode!r} (expected python|hy|pnix)")


def repl_report() -> dict[str, Any]:
    """Self-check: the pnix REPL retains context across inputs (a=20; b=a+22; b -> 42) and a bad
    line does not end the session."""
    import io  # noqa: PLC0415
    try:
        script = "a = 20\nb = a + 22\nb\n1 +\nb + 0\n:quit\n"  # includes a deliberate bad line
        out = io.StringIO()
        run_pnix_repl(io.StringIO(script), out)
        text = out.getvalue()
        ready = (
            "a bound" in text and "b bound" in text
            and "42" in text                      # context retained: b = a + 22 = 42
            and "error:" in text                  # the bad line "1 +" was reported...
            and text.count("42") >= 2             # ...and the session survived to eval "b + 0"
        )
        return {"schema": "pnix-hy.repl.report.v0", "ready": bool(ready), "available": True,
                "context_retained": "42" in text, "survives_error": "error:" in text}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.repl.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}
