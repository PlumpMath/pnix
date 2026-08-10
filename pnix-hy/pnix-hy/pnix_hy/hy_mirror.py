"""Bridge to the sibling hy-meta proof ladder.

hy-meta is now closed far past the old stage7 eval mirror. Its proof ladder runs
stage7 (semantic/eval closure) -> stage8 (artifact reproducibility) -> stage9
(clean product runtime replay) -> stage10 (client/server/session/sandbox) ->
stage11 (multi-domain adapters) -> stage12 (self-improvement quarantine) ->
stage13 (long-horizon organism) -> stage14 (cross-host pnix law) -> stage15
(open-world evidence federation) -> stageN (constitutional extension; stage16 is
the first concrete instance). Only 3 cross-repo Clojure-host stage14 items remain
open upstream.

pnix-hy still EXECUTES inside the stage7-verified kernel: stages 8..N are
proof/federation lanes layered on top, not different execution engines, so the
kernel ('stage2/kernel.hy', verified through the 7-deep mirror tower) stays the
only place Hy runs. What upgrades to "stage15+" is the EVIDENCE pnix-hy federates
from hy-meta -- the kernel eval probe PLUS the stage15/stageN closure verdict
(see `closure_probe`).
"""

from __future__ import annotations

from dataclasses import dataclass
from functools import lru_cache as _lru_cache
from pathlib import Path
import json
import os
import subprocess
import sys
from typing import Any



PROJECT_ROOT = Path(__file__).resolve().parents[1]
# HY_ROOT holds the vendored Hy + hy-meta tree. Default = the repo root (sibling of hy-meta/),
# so in-repo / flake / editable behavior is UNCHANGED. `PNIX_HY_HOME` overrides it so an
# installed (pip/nix) pnix_hy can point at a checkout and get the projection/proof tiers too.
_PNIX_HY_HOME = os.environ.get("PNIX_HY_HOME")
HY_ROOT = Path(_PNIX_HY_HOME).expanduser().resolve() if _PNIX_HY_HOME else PROJECT_ROOT.parent
HY_META_BOOTSTRAP = HY_ROOT / "hy-meta" / "bootstrap.py"


class HyMirrorError(RuntimeError):
    """Raised when the hy-meta proof-ladder bridge cannot run."""


class _Stage7EvalError(HyMirrorError):
    """Raised when stage7 worker evaluates source and returns an error."""


class _Stage7TimeoutError(HyMirrorError):
    """Raised when a stage7 worker eval exceeds its deadline. NOT retried one-shot -- the same
    source would just burn the one-shot timeout again (A10)."""


@dataclass(frozen=True)
class HyMetaResult:
    command: tuple[str, ...]
    returncode: int
    stdout: str
    stderr: str
    data: dict[str, Any]


def _candidate_pythons() -> list[Path]:
    env_value = os.environ.get("PNIX_HY_PYTHON")
    candidates: list[Path] = []
    if env_value:
        candidates.append(Path(env_value))
    candidates.append(Path(sys.executable))
    candidates.extend(
        [
            Path("/tmp/pnix-hy-py311-venv/bin/python"),
            Path("/tmp/pnix-hy-py314-venv/bin/python"),
            Path("/usr/local/bin/python3.11"),
            Path("/opt/homebrew/bin/python3.11"),
            Path("/usr/local/opt/python@3.14/bin/python3.14"),
            Path("/opt/homebrew/opt/python@3.14/bin/python3.14"),
        ]
    )
    return candidates


def _candidate_has_hy(candidate: Path) -> bool:
    if not candidate.exists():
        return False
    try:
        proc = subprocess.run(
            [str(candidate), "-c", "import hy"],  # noqa: S603 - internal probe
            cwd=str(HY_ROOT),
            text=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
            timeout=15,
        )
    except Exception:  # noqa: BLE001
        return False
    return proc.returncode == 0


@_lru_cache(maxsize=1)
def hy_python() -> Path:
    # B3: the proof Python path doesn't change within a process, but this is hit on
    # every subprocess facility call -- resolve the candidate list once and cache it.
    env_value = os.environ.get("PNIX_HY_PYTHON")
    if env_value:
        env_candidate = Path(env_value)
        if not _candidate_has_hy(env_candidate):
            raise HyMirrorError(
                "PNIX_HY_PYTHON is set but does not provide a working hy import: "
                + str(env_candidate)
            )
        return env_candidate
    for candidate in _candidate_pythons():
        if _candidate_has_hy(candidate):
            return candidate
    raise HyMirrorError(
        "no supported Hy proof Python found; set PNIX_HY_PYTHON to a "
        "Python 3.11 or Homebrew Python 3.14 venv"
    )


def _coerce_value(value: str) -> Any:
    value = value.strip()
    if value == "True":
        return True
    if value == "False":
        return False
    if value == "None":
        return None
    try:
        return int(value)
    except ValueError:
        pass
    try:
        return float(value)
    except ValueError:
        return value


def parse_key_output(stdout: str) -> dict[str, Any]:
    data: dict[str, Any] = {}
    for line in stdout.splitlines():
        if ": " not in line:
            continue
        key, value = line.split(": ", 1)
        if key:
            data[key] = _coerce_value(value)
    return data


def run_bootstrap(*args: str, timeout: int = 180) -> HyMetaResult:
    if not HY_META_BOOTSTRAP.exists():
        raise HyMirrorError(f"missing hy-meta bootstrap: {HY_META_BOOTSTRAP}")
    python = hy_python()
    command = (str(python), str(HY_META_BOOTSTRAP), *args)
    try:
        proc = subprocess.run(
            command,
            cwd=str(HY_ROOT),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    except (subprocess.TimeoutExpired, OSError) as exc:  # surface as HyMirrorError, not raw
        raise HyMirrorError(
            "hy-meta command timed out/failed: " + " ".join(command)
            + f"\n{type(exc).__name__}: {exc}"
        ) from exc
    result = HyMetaResult(
        command=command,
        returncode=proc.returncode,
        stdout=proc.stdout,
        stderr=proc.stderr,
        data=parse_key_output(proc.stdout),
    )
    if proc.returncode != 0:
        raise HyMirrorError(
            "hy-meta command failed: "
            + " ".join(command)
            + ("\n" + proc.stderr if proc.stderr else "")
        )
    return result


def stage7_light_probe() -> dict[str, Any]:
    """Cheapest liveness probe: run `(+ 20 22)` through the stage7 eval kernel
    (the execution substrate, base of the ladder)."""
    result = run_bootstrap("stage7-kernel-run", "(+ 20 22)", timeout=120)
    value = result.stdout.strip()
    return {
        "schema": "pnix-hy.hy-meta.light-probe.v0",
        "ready": value == "42",
        "python": str(hy_python()),
        "bootstrap": str(HY_META_BOOTSTRAP),
        "value": _coerce_value(value),
        "stderr": result.stderr.strip(),
    }


def stage7_check() -> dict[str, Any]:
    """Stage7 semantic/eval mirror closure -- the base of the ladder and the lane
    that proves the kernel pnix-hy runs inside is self-consistent. Higher closure
    (stage8..stageN) is federated separately via `closure_probe`."""
    result = run_bootstrap("stage7-check", timeout=240)
    data = dict(result.data)
    ready = (
        data.get("stage_count") == 7
        and data.get("last_stage_module") == "hy_meta_stage7.compiler"
        and data.get("all_stage_self_checks") is True
        and data.get("compiler_python_stage7_mirror") is True
        and data.get("compiler_ast_stage7_mirror") is True
        and data.get("kernel_python_stage7_mirror") is True
        and data.get("kernel_ast_stage7_mirror") is True
        and data.get("kernel_value_stage7_mirror") is True
        and data.get("kernel_stress_repeat_python_stable") is True
        and data.get("kernel_stress_repeat_ast_stable") is True
    )
    return {
        "schema": "pnix-hy.hy-meta.stage7-check.v0",
        "ready": ready,
        "python": str(hy_python()),
        "bootstrap": str(HY_META_BOOTSTRAP),
        "data": data,
        "stderr": result.stderr.strip(),
    }


# ============================================================================
# Persistent stage7 worker — the dominant speed lever.
#
# bootstrap_stage7_kernel() (what `stage7-kernel-run` calls) costs ~27s to build the
# Hy-written kernel, but reusing that kernel for further eval_source() calls is ~0s.
# The one-shot run_bootstrap path paid the ~27s rebuild on EVERY stage7 eval. This keeps
# one long-lived worker process that builds the kernel once, then services fragments over
# a JSON-line protocol on stdin/stdout. Falls back to the one-shot path on any failure;
# set PNIX_HY_NO_WORKER=1 to force the one-shot path. The kernel is functional (repeated
# evals are independent — verified against the 4-lane mirror, which must stay 4x identical).
# ============================================================================

_STAGE7_WORKER_SCRIPT = r'''
import contextlib
import io
import json
import os
import sys
import traceback
sys.path.insert(0, os.path.join(os.getcwd(), "hy-meta"))
import bootstrap
kernel = bootstrap.bootstrap_stage7_kernel()
sys.stdout.write("__PNIX_STAGE7_READY__\n"); sys.stdout.flush()
for line in sys.stdin:
    line = line.rstrip("\n")
    if not line:
        continue
    try:
        req = json.loads(line)
    except Exception:
        continue
    if req.get("cmd") == "shutdown":
        break
    try:
        with io.StringIO() as _stdout:
            with contextlib.redirect_stdout(_stdout):
                value = kernel.eval_source(req["source"], None, req.get("filename", "<stage7-worker>"))
            out = _stdout.getvalue()
        resp = {
            "ok": True,
            "value": ("" if value is None else str(value)),
            "stdout": out,
        }
    except Exception as exc:
        resp = {"ok": False, "error": "%s: %s" % (type(exc).__name__, exc),
                "tb": traceback.format_exc()}
    sys.stdout.write(json.dumps(resp) + "\n")
    sys.stdout.flush()
'''

import atexit as _atexit
import tempfile as _tempfile
import threading as _threading

_stage7_worker_proc: Any = None
_stage7_worker_lock = _threading.Lock()
_stage7_worker_script_path: str | None = None


def _stage7_worker_script_file() -> str:
    global _stage7_worker_script_path
    if _stage7_worker_script_path is None:
        path = os.path.join(_tempfile.gettempdir(), "pnix_hy_stage7_worker.py")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(_STAGE7_WORKER_SCRIPT)
        _stage7_worker_script_path = path
    return _stage7_worker_script_path


def _stage7_kill_worker() -> None:
    global _stage7_worker_proc
    proc = _stage7_worker_proc
    _stage7_worker_proc = None
    if proc is not None:
        try:
            proc.kill()
        except Exception:  # noqa: BLE001
            pass


def _stage7_ensure_worker(timeout: int = 120):
    """Return a live worker process (building the kernel on first use), or None if the
    worker is disabled or failed to start (caller then uses the one-shot path)."""
    global _stage7_worker_proc
    if os.environ.get("PNIX_HY_NO_WORKER"):
        return None
    proc = _stage7_worker_proc
    if proc is not None and proc.poll() is None:
        return proc
    python = hy_python()
    env = dict(os.environ)
    env.pop("PYTHONPATH", None)
    try:
        proc = subprocess.Popen(
            (str(python), _stage7_worker_script_file()),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            cwd=str(HY_ROOT),
            env=env,
            text=True,
            bufsize=1,
        )
    except Exception:  # noqa: BLE001
        return None
    # Drain stdout until the kernel is built and the worker signals ready.
    import select as _select
    import time as _time
    deadline = _time.time() + timeout
    while _time.time() < deadline:
        remaining = deadline - _time.time()
        if remaining <= 0:
            _stage7_kill_worker()
            return None
        ready, _, _ = _select.select([proc.stdout], [], [], remaining)
        if not ready:
            _stage7_kill_worker()
            return None
        line = proc.stdout.readline()
        if not line:
            return None  # worker died during build -> fall back to one-shot
        if line.strip() == "__PNIX_STAGE7_READY__":
            _stage7_worker_proc = proc
            return proc
        if line.strip():
            # protocol noise during startup is non-fatal; treat it as a soft failure
            _stage7_kill_worker()
            return None
    _stage7_kill_worker()
    return None


def _stage7_worker_eval(source: str, filename: str = "<stage7-kernel-run>") -> str:
    """Eval one fragment on the persistent worker. Raises on protocol/worker failure so
    the caller can fall back."""
    with _stage7_worker_lock:
        proc = _stage7_ensure_worker()
        if proc is None:
            raise HyMirrorError("stage7 worker unavailable")
        proc.stdin.write(json.dumps({"source": source, "filename": filename}) + "\n")
        proc.stdin.flush()
        timeout = float(os.environ.get("PNIX_HY_WORKER_TIMEOUT", "120"))
        import time as _time
        deadline = _time.time() + timeout
        import select as _select
        line = ""
        while _time.time() < deadline and not line:
            remaining = deadline - _time.time()
            ready, _, _ = _select.select([proc.stdout], [], [], remaining)
            if not ready:
                _stage7_kill_worker()
                raise _Stage7TimeoutError(f"stage7 worker timeout after {timeout}s")
            line = proc.stdout.readline()
        if not line:
            _stage7_kill_worker()
            raise HyMirrorError("stage7 worker died")
        try:
            resp = json.loads(line)
        except json.JSONDecodeError as exc:  # noqa: BLE001
            _stage7_kill_worker()
            raise HyMirrorError(f"stage7 worker protocol violation: {line!r}") from exc
    if not resp.get("ok"):
        raise _Stage7EvalError(f"stage7 worker eval error: {resp.get('error')}")
    return resp["value"]


@_atexit.register
def _stage7_shutdown_worker() -> None:
    proc = _stage7_worker_proc
    if proc is not None and proc.poll() is None:
        try:
            proc.stdin.write('{"cmd": "shutdown"}\n')
            proc.stdin.flush()
            proc.wait(timeout=2)
        except Exception:  # noqa: BLE001
            _stage7_kill_worker()


def stage7_eval(source: str) -> str:
    """Evaluate Hy source on the stage7 kernel. Uses the persistent worker (kernel built
    once) when available, else the one-shot bootstrap path."""
    if not os.environ.get("PNIX_HY_NO_WORKER"):
        try:
            return _stage7_worker_eval(source)
        except (_Stage7EvalError, _Stage7TimeoutError):
            raise  # eval errors keep the worker; timeouts must NOT re-run the source one-shot
        except HyMirrorError:
            _stage7_kill_worker()  # fall through to the robust one-shot path
    timeout = int(float(os.environ.get("PNIX_HY_WORKER_TIMEOUT", "120")))
    result = run_bootstrap("stage7-kernel-run", source, timeout=timeout)
    return result.stdout.strip()


def stage7_eval_json(source: str) -> Any:
    output = stage7_eval(source)
    try:
        return json.loads(output)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(f"stage7 Hy source did not return JSON: {output!r}") from exc


# ============================================================================
# PP3 -- Persistent Hy PROJECTION worker.
#
# Every projection facility (hy_form_projection / macroexpand / module / macro-step /
# quasiquote / defmacro / reader-macro / import) spawns a fresh `python -` that pays the
# `import hy` startup each call. This worker imports hy ONCE, then runs each facility's
# script on demand: it receives {script, source}, sets PNIX_HY_FORM_SRC, execs the script
# with stdout captured, and returns that captured JSON. Per-call `import hy` inside a script
# is then a cache hit. Falls back to the one-shot path on ANY worker hiccup (so behavior is
# identical); set PNIX_HY_NO_PROJECTION_WORKER=1 to force one-shot.
# ============================================================================

_PROJECTION_WORKER_SCRIPT = r'''
import sys, os, json, io, contextlib, traceback
# run as `python /tmp/worker.py`, so sys.path[0] is the script dir, NOT cwd -- put cwd
# (HY_ROOT, where the `hy` symlink lives) on the path so `import hy` resolves, exactly as
# the one-shot `python -` path gets it via sys.path[0]=''.
sys.path.insert(0, os.getcwd())
import hy
from hy.reader import read_many, HyReader
import hy.models as M
import hy.compiler as C
sys.stdout.write("__PNIX_PROJ_READY__\n"); sys.stdout.flush()
for line in sys.stdin:
    line = line.rstrip("\n")
    if not line:
        continue
    try:
        req = json.loads(line)
    except Exception:
        continue
    if req.get("cmd") == "shutdown":
        break
    try:
        os.environ["PNIX_HY_FORM_SRC"] = req.get("source", "")
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            exec(req["script"], {"__name__": "__pnix_proj__"})
        resp = {"ok": True, "out": buf.getvalue()}
    except Exception as exc:
        resp = {"ok": False, "error": "%s: %s" % (type(exc).__name__, exc),
                "tb": traceback.format_exc()}
    sys.stdout.write(json.dumps(resp) + "\n")
    sys.stdout.flush()
'''

_proj_worker_proc: Any = None
_proj_worker_lock = _threading.Lock()
_proj_worker_script_path: str | None = None


def _proj_worker_script_file() -> str:
    global _proj_worker_script_path
    if _proj_worker_script_path is None:
        path = os.path.join(_tempfile.gettempdir(), "pnix_hy_projection_worker.py")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(_PROJECTION_WORKER_SCRIPT)
        _proj_worker_script_path = path
    return _proj_worker_script_path


def _proj_kill_worker() -> None:
    global _proj_worker_proc
    proc = _proj_worker_proc
    _proj_worker_proc = None
    if proc is not None:
        try:
            proc.kill()
        except Exception:  # noqa: BLE001
            pass


def _proj_ensure_worker(timeout: int = 60):
    global _proj_worker_proc
    if os.environ.get("PNIX_HY_NO_PROJECTION_WORKER"):
        return None
    proc = _proj_worker_proc
    if proc is not None and proc.poll() is None:
        return proc
    python = hy_python()
    env = dict(os.environ)
    env.pop("PYTHONPATH", None)
    try:
        proc = subprocess.Popen(
            (str(python), _proj_worker_script_file()),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            cwd=str(HY_ROOT),
            env=env,
            text=True,
            bufsize=1,
        )
    except Exception:  # noqa: BLE001
        return None
    import time as _time
    deadline = _time.time() + timeout
    while _time.time() < deadline:
        line = proc.stdout.readline()
        if not line:
            return None
        if line.strip() == "__PNIX_PROJ_READY__":
            _proj_worker_proc = proc
            return proc
    _proj_kill_worker()
    return None


def _proj_worker_run(script: str, source: str) -> str | None:
    """Run one projection script on the persistent worker; return its captured stdout, or
    None to signal the caller should fall back to the one-shot path."""
    with _proj_worker_lock:
        proc = _proj_ensure_worker()
        if proc is None:
            return None
        try:
            proc.stdin.write(json.dumps({"script": script, "source": source}) + "\n")
            proc.stdin.flush()
            import select as _select  # noqa: PLC0415
            timeout = float(os.environ.get("PNIX_HY_WORKER_TIMEOUT", "120"))
            ready, _, _ = _select.select([proc.stdout], [], [], timeout)
            if not ready:  # A10: a silent worker must not block readline forever
                _proj_kill_worker()
                return None  # bounded fallback: the one-shot path has its own subprocess timeout
            line = proc.stdout.readline()
        except Exception:  # noqa: BLE001
            _proj_kill_worker()
            return None
        if not line:
            _proj_kill_worker()
            return None
        try:
            resp = json.loads(line)
        except Exception:  # noqa: BLE001
            _proj_kill_worker()
            return None
    return resp["out"] if resp.get("ok") else None


def _one_shot_hy_script(script: str, source: str, *, timeout: int, label: str) -> str:
    python = hy_python()
    env = dict(os.environ)
    env.pop("PYTHONPATH", None)
    env["PNIX_HY_FORM_SRC"] = source
    try:
        proc = subprocess.run(
            (str(python), "-"),
            input=script,
            cwd=str(HY_ROOT),
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    except (subprocess.TimeoutExpired, OSError) as exc:  # surface as HyMirrorError, not raw
        raise HyMirrorError(f"{label} timed out/failed: {type(exc).__name__}: {exc}") from exc
    if proc.returncode != 0:
        raise HyMirrorError(
            f"{label} failed" + ("\n" + proc.stderr if proc.stderr else "")
        )
    return proc.stdout


def _run_hy_script(script: str, source: str, *, timeout: int = 60, label: str = "hy projection") -> str:
    """Run a projection script through the persistent worker (fast: warm `import hy`),
    falling back to a one-shot `python -` on any worker miss/error so behavior is identical."""
    out = None
    if not os.environ.get("PNIX_HY_NO_PROJECTION_WORKER"):
        try:
            out = _proj_worker_run(script, source)
        except Exception:  # noqa: BLE001
            _proj_kill_worker()
            out = None
    if out is not None:
        return out
    return _one_shot_hy_script(script, source, timeout=timeout, label=label)


@_atexit.register
def _proj_shutdown_worker() -> None:
    proc = _proj_worker_proc
    if proc is not None and proc.poll() is None:
        try:
            proc.stdin.write('{"cmd": "shutdown"}\n')
            proc.stdin.flush()
            proc.wait(timeout=2)
        except Exception:  # noqa: BLE001
            _proj_kill_worker()


# ============================================================================
# Hy reader-form projection (the FRONT of the Hy -> pnix expressiveness pipeline)
#
# pnix-hy's mission is to project language expressiveness between Hy <-> pnix so a
# developer can study meta-circular evaluation. The existing CPython introspection
# (ast_info / disassemble / symtable_info ...) covers the back half of the pipeline
# (Python AST -> bytecode). This covers the FRONT half: take Hy source, surface its
# reader-form model tree (Expression / Symbol / Keyword / List / Dict / ...), and
# show exactly which Python AST that Hy form lowers to. `hy` is only importable with
# the repo root on the path (the `hy` symlink lives at HY_ROOT), so this runs in the
# Hy proof Python with cwd=HY_ROOT, mirroring the stage7 bridge.
# ============================================================================

_HY_MODEL_TO_JSON_SNIPPET = r'''
def model_to_json(m):
    cls = type(m).__name__
    if isinstance(m, M.Keyword):
        return {"model": "Keyword", "name": m.name}
    if isinstance(m, M.Symbol):
        return {"model": "Symbol", "name": str(m)}
    if isinstance(m, M.Dict):
        return {"model": "Dict", "items": [model_to_json(c) for c in m]}
    if isinstance(m, (M.Expression, M.List, M.Tuple, M.Set, M.FString, M.FComponent)):
        return {"model": cls, "items": [model_to_json(c) for c in m]}
    if isinstance(m, M.String):
        return {"model": "String", "value": str(m)}
    if isinstance(m, M.Bytes):
        return {"model": "Bytes", "value": bytes(m).decode("latin-1")}
    if isinstance(m, M.Integer):
        return {"model": "Integer", "value": int(m)}
    if isinstance(m, M.Float):
        return {"model": "Float", "value": float(m)}
    if isinstance(m, M.Complex):
        return {"model": "Complex", "value": str(complex(m))}
    return {"model": cls, "repr": repr(m)}
'''

_HY_FORM_PROJECTION_SCRIPT = r'''
import os, sys, json, ast, types
import hy
from hy.reader import read_many
import hy.models as M
import hy.compiler as C

src = os.environ["PNIX_HY_FORM_SRC"]
''' + _HY_MODEL_TO_JSON_SNIPPET + r'''
forms = list(read_many(src))
reader_forms = [model_to_json(f) for f in forms]

mod = types.ModuleType("_pnix_hy_form_probe")
lowered = C.hy_compile(read_many(src), mod, filename="<pnix-hy-form>", source=src)

out = {
    "schema": "pnix-hy.hy-form-projection.v0",
    "source": src,
    "hy_version": getattr(hy, "__version__", "?"),
    "reader_form_count": len(forms),
    "reader_forms": reader_forms,
    "python_ast": ast.dump(lowered, indent=2),
    "python_source": ast.unparse(lowered),
}
print(json.dumps(out))
'''


def hy_form_projection(source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Project a Hy source fragment: its reader-form model tree + the Python AST /
    source it lowers to. The concrete `Hy form -> Python AST` half of the Hy <-> pnix
    expressiveness mirror. Returns a JSON-safe dict (schema
    `pnix-hy.hy-form-projection.v0`)."""
    stdout = _run_hy_script(_HY_FORM_PROJECTION_SCRIPT, source, timeout=timeout,
                            label=f"hy form projection for {source!r}")
    try:
        return json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(
            f"hy form projection did not return JSON: {stdout!r}"
        ) from exc


def hy_form_projection_report() -> dict[str, Any]:
    """Self-check for the Hy reader-form projection: a representative form must
    surface its reader models AND lower to the expected Python construct."""
    probe = "(defn f [x] (+ x 1))"
    try:
        proj = hy_form_projection(probe)
    except HyMirrorError as exc:
        return {
            "schema": "pnix-hy.hy-form-projection.report.v0",
            "ready": False,
            "available": False,
            "error": str(exc),
        }
    top = (proj.get("reader_forms") or [{}])[0]
    models = [c.get("model") for c in top.get("items", [])]
    py = proj.get("python_source", "")
    ready = (
        top.get("model") == "Expression"
        and models[:3] == ["Symbol", "Symbol", "List"]
        and "def f(x):" in py
        and "return x + 1" in py
    )
    return {
        "schema": "pnix-hy.hy-form-projection.report.v0",
        "ready": ready,
        "available": True,
        "probe": probe,
        "top_model": top.get("model"),
        "top_child_models": models,
        "python_source": py,
    }


# B2: one spawn for form + macro. The capstone hy_pnix_projection needed both the
# reader-form/lowering view AND the macro-layer view; it used to pay TWO Hy-proof
# subprocess starts (each re-imports hy). This combined script returns both from a
# single process, halving the capstone's subprocess cost.
_HY_FORM_AND_MACRO_SCRIPT = r'''
import os, sys, json, ast, types
import hy
from hy.reader import read_many
import hy.models as M
import hy.compiler as C

src = os.environ["PNIX_HY_FORM_SRC"]
''' + _HY_MODEL_TO_JSON_SNIPPET + r'''
forms = list(read_many(src))
reader_forms = [model_to_json(f) for f in forms]

mod = types.ModuleType("_pnix_hy_formmacro_probe")
sys.modules[mod.__name__] = mod

original = forms[0]
expanded_1 = hy.macroexpand_1(original, mod)
expanded_full = hy.macroexpand(original, mod)

lowered = C.hy_compile(read_many(src), mod, filename="<pnix-hy-formmacro>", source=src)
python_source = ast.unparse(lowered)

out = {
    "schema": "pnix-hy.hy-form-and-macro-projection.v0",
    "source": src,
    "hy_version": getattr(hy, "__version__", "?"),
    "reader_form_count": len(forms),
    "reader_forms": reader_forms,
    "python_ast": ast.dump(lowered, indent=2),
    "python_source": python_source,
    "top_form_count": len(forms),
    "original": model_to_json(original),
    "expanded_1": model_to_json(expanded_1),
    "expanded_full": model_to_json(expanded_full),
    "changed_1": repr(original) != repr(expanded_1),
    "is_macro": repr(original) != repr(expanded_full),
    "fully_expanded_python": python_source,
}
print(json.dumps(out))
'''


def hy_form_and_macro_projection(source: str, *, timeout: int = 60) -> dict[str, Any]:
    """B2: reader-form/lowering view AND macro-layer view of a Hy fragment in ONE
    Hy-proof subprocess (the union of hy_form_projection + hy_macroexpand_projection).
    Schema `pnix-hy.hy-form-and-macro-projection.v0`."""
    stdout = _run_hy_script(_HY_FORM_AND_MACRO_SCRIPT, source, timeout=timeout,
                            label=f"hy form+macro projection for {source!r}")
    try:
        return json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(
            f"hy form+macro projection did not return JSON: {stdout!r}"
        ) from exc


def hy_meta_circular_projection(hy_source: str, *, timeout: int = 60) -> dict[str, Any]:
    """End-to-end Python-ecosystem meta-circular view of a Hy fragment, in one call:
    Hy reader forms -> Python AST -> bytecode / code-object / symtable / marshal.

    Chains the FRONT (`hy_form_projection`, runs in the Hy proof Python so `hy` is
    importable) with the BACK (`full_introspection`, runs in-process: `compile()` lowers
    the Hy-emitted Python without executing its `import hy`). The single artifact a
    developer can read to study how a Hy form traverses the whole Python meta-circular
    substrate. Schema `pnix-hy.hy-meta-circular-projection.v0`."""
    front = hy_form_projection(hy_source, timeout=timeout)
    python_source = front.get("python_source", "")
    back: dict[str, Any] = {}
    back_error = None
    try:
        back = full_introspection(python_source, "exec")
    except Exception as exc:  # noqa: BLE001 - report, don't crash the projection
        back_error = f"{type(exc).__name__}: {exc}"
    return {
        "schema": "pnix-hy.hy-meta-circular-projection.v0",
        "hy_source": hy_source,
        "reader_forms": front.get("reader_forms"),
        "reader_form_count": front.get("reader_form_count"),
        "python_ast": front.get("python_ast"),
        "python_source": python_source,
        "python_introspection": back,
        "python_introspection_error": back_error,
    }


def hy_meta_circular_projection_report() -> dict[str, Any]:
    """Self-check: a `defn` form must traverse the whole pipeline -- reader Expression,
    lowered `def`, and a compiled code object carrying the function as a const."""
    probe = "(defn f [x] (+ x 1))"
    try:
        proj = hy_meta_circular_projection(probe)
    except HyMirrorError as exc:
        return {
            "schema": "pnix-hy.hy-meta-circular-projection.report.v0",
            "ready": False,
            "available": False,
            "error": str(exc),
        }
    back = proj.get("python_introspection") or {}
    consts = ((back.get("code_object") or {}).get("co_consts")) or []
    opnames = [i.get("opname") for i in (back.get("bytecode") or [])]
    func_consts = [c.get("__code__") for c in consts if isinstance(c, dict) and "__code__" in c]
    ready = (
        (proj.get("reader_forms") or [{}])[0].get("model") == "Expression"
        and "def f(x):" in (proj.get("python_source") or "")
        and proj.get("python_introspection_error") is None
        and "f" in func_consts
        and "MAKE_FUNCTION" in opnames
    )
    return {
        "schema": "pnix-hy.hy-meta-circular-projection.report.v0",
        "ready": ready,
        "available": True,
        "probe": probe,
        "stages": ["reader_forms", "python_ast", "python_source", "python_introspection"],
        "func_consts": func_consts,
        "opnames": opnames,
    }


# ============================================================================
# Hy macroexpansion projection (the macro layer of Hy's meta-circular)
#
# Macros are Hy's defining meta-circular capability: a form is rewritten to another
# form before lowering to Python. This surfaces that layer for a single top-level form:
# the original reader model, the 1-step expansion, the full expansion, and the Python
# the fully-expanded form lowers to. e.g. (when c x) -> (if c (do x) None);
# (cond a 1 b 2) -> (if a 1 (if b 2 None)). Runs in the Hy proof Python (cwd=HY_ROOT).
# ============================================================================

_HY_MACROEXPAND_SCRIPT = r'''
import os, sys, json, ast, types
import hy
from hy.reader import read_many
import hy.models as M
import hy.compiler as C

src = os.environ["PNIX_HY_FORM_SRC"]
''' + _HY_MODEL_TO_JSON_SNIPPET + r'''
mod = types.ModuleType("_pnix_hy_macro_probe")
sys.modules["_pnix_hy_macro_probe"] = mod

forms = list(read_many(src))
original = forms[0]
expanded_1 = hy.macroexpand_1(original, mod)
expanded_full = hy.macroexpand(original, mod)
lowered = C.hy_compile(read_many(src), mod, filename="<pnix-hy-macro>", source=src)

out = {
    "schema": "pnix-hy.hy-macroexpand-projection.v0",
    "source": src,
    "top_form_count": len(forms),
    "original": model_to_json(original),
    "expanded_1": model_to_json(expanded_1),
    "expanded_full": model_to_json(expanded_full),
    "changed_1": repr(original) != repr(expanded_1),
    "is_macro": repr(original) != repr(expanded_full),
    "fully_expanded_python": ast.unparse(lowered),
}
print(json.dumps(out))
'''


def hy_macroexpand_projection(source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Project the macro layer of a single top-level Hy form: original model, 1-step
    expansion, full expansion, and the Python the fully-expanded form lowers to. Schema
    `pnix-hy.hy-macroexpand-projection.v0`."""
    stdout = _run_hy_script(_HY_MACROEXPAND_SCRIPT, source, timeout=timeout,
                            label=f"hy macroexpand projection for {source!r}")
    try:
        return json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(
            f"hy macroexpand projection did not return JSON: {stdout!r}"
        ) from exc


def hy_macroexpand_projection_report() -> dict[str, Any]:
    """Self-check: `(when c x)` must expand (is_macro) to an `if` form."""
    probe = "(when c x)"
    try:
        proj = hy_macroexpand_projection(probe)
    except HyMirrorError as exc:
        return {"schema": "pnix-hy.hy-macroexpand-projection.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    expanded_head = (proj.get("expanded_full") or {}).get("items", [{}])[0]
    ready = (
        proj.get("is_macro") is True
        and proj.get("changed_1") is True
        and expanded_head.get("name") == "if"
    )
    return {
        "schema": "pnix-hy.hy-macroexpand-projection.report.v0",
        "ready": ready,
        "available": True,
        "probe": probe,
        "is_macro": proj.get("is_macro"),
        "expanded_head": expanded_head.get("name"),
        "fully_expanded_python": proj.get("fully_expanded_python"),
    }


# ============================================================================
# Hy macro STEP trace — the macro tower unfolding, one macroexpand_1 at a time
#
# hy_macroexpand_projection shows only the endpoints (1-step + fully-expanded) of the
# FIRST form. This records the WHOLE expansion sequence for EVERY top-level form:
# original -> macroexpand_1 -> ... -> fixpoint, with each intermediate form's head and
# Python lowering. That surfaces Hy's compile-time meta-circular depth (a macro that
# expands into another macro shows >1 non-trivial step), and it is multi-form (closes
# the forms[0]-only scope gap). Runs in the Hy proof Python (cwd=HY_ROOT).
# ============================================================================

_HY_MACRO_STEP_SCRIPT = r'''
import os, sys, json, ast, types
import hy
from hy.reader import read_many
import hy.models as M
import hy.compiler as C

src = os.environ["PNIX_HY_FORM_SRC"]
MAX_STEPS = 64
''' + _HY_MODEL_TO_JSON_SNIPPET + r'''
def head_name(m):
    if isinstance(m, M.Expression) and len(m) and isinstance(m[0], M.Symbol):
        return str(m[0])
    return None

def lower(form):
    try:
        return ast.unparse(C.hy_compile([form], mod, filename="<pnix-hy-macrostep>", source=src)), None
    except Exception as exc:
        return None, "%s: %s" % (type(exc).__name__, exc)

mod = types.ModuleType("_pnix_hy_macrostep_probe")
sys.modules[mod.__name__] = mod

forms = list(read_many(src))
out_forms = []
for f in forms:
    steps = []
    cur = f
    seen = set()
    n = 0
    while True:
        py, lerr = lower(cur)
        step = {"step": n, "head": head_name(cur), "reader_form": model_to_json(cur), "python_source": py}
        if lerr:
            step["lower_error"] = lerr
        r = repr(cur)
        if r in seen:
            step["cycle"] = True
            steps.append(step)
            break
        seen.add(r)
        try:
            nxt = hy.macroexpand_1(cur, mod)
        except Exception as exc:
            step["expand_error"] = "%s: %s" % (type(exc).__name__, exc)
            steps.append(step)
            break
        steps.append(step)
        if repr(nxt) == r or n >= MAX_STEPS:
            break
        cur = nxt
        n += 1
    out_forms.append({
        "head": head_name(f),
        "step_count": len(steps),
        "is_macro": len(steps) > 1,
        "fixpoint": len(steps) <= MAX_STEPS + 1,
        "steps": steps,
    })

print(json.dumps({
    "schema": "pnix-hy.hy-macro-step-trace.v0",
    "form_count": len(forms),
    "forms": out_forms,
}))
'''


def hy_macro_step_trace(source: str, *, timeout: int = 90) -> dict[str, Any]:
    """Trace the macro tower unfolding for EVERY top-level form: the full
    `original -> macroexpand_1 -> ... -> fixpoint` sequence, with each step's head and
    Python lowering. Schema `pnix-hy.hy-macro-step-trace.v0`."""
    stdout = _run_hy_script(_HY_MACRO_STEP_SCRIPT, source, timeout=timeout,
                            label="hy macro step trace")
    try:
        return json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(
            f"hy macro step trace did not return JSON: {stdout!r}"
        ) from exc


def hy_macro_step_trace_report() -> dict[str, Any]:
    """Self-check: a 3-form module traces per-form macro towers — `cond`/`when` are
    macros (>1 step, expand toward `if`), `setv` is not (1 step, fixpoint)."""
    probe = "(cond a 1 b 2)\n(when c (print x))\n(setv y 5)"
    try:
        tr = hy_macro_step_trace(probe)
    except HyMirrorError as exc:
        return {"schema": "pnix-hy.hy-macro-step-trace.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    forms = tr.get("forms") or []
    by_head = {f.get("head"): f for f in forms}
    cond = by_head.get("cond") or {}
    setv = by_head.get("setv") or {}
    cond_last = (cond.get("steps") or [{}])[-1]
    ready = (
        tr.get("form_count") == 3
        and cond.get("is_macro") is True
        and cond_last.get("head") == "if"
        and setv.get("is_macro") is False
        and setv.get("step_count") == 1
    )
    return {
        "schema": "pnix-hy.hy-macro-step-trace.report.v0",
        "ready": bool(ready),
        "available": True,
        "form_count": tr.get("form_count"),
        "cond_steps": cond.get("step_count"),
        "cond_final_head": cond_last.get("head"),
        "setv_is_macro": setv.get("is_macro"),
    }


# ============================================================================
# Hy quasiquote / unquote projection — code-as-data, the staging view
#
# `(...) is a CODE TEMPLATE: the quoted skeleton lowers to code that CONSTRUCTS an
# hy.models tree, with ~x (unquote) / ~@xs (unquote-splice) left as DYNAMIC HOLES.
# e.g. `(+ 1 ~x ~@ys) -> Expression([Symbol('+'), Integer(1), x, *(ys or [])]).
# That static-skeleton + dynamic-holes shape is exactly the binding-time structure
# pnix-hy's P5 partial evaluator exploits (static folds away, dynamic residualizes) --
# quasiquote is MANUAL staging; specialize_pnix is AUTOMATIC staging. This surfaces
# the holes and the model-building lowering. Runs in the Hy proof Python (cwd=HY_ROOT).
# ============================================================================

_HY_QUASIQUOTE_SCRIPT = r'''
import os, sys, json, ast, types
import hy
from hy.reader import read_many
import hy.models as M
import hy.compiler as C

src = os.environ["PNIX_HY_FORM_SRC"]
''' + _HY_MODEL_TO_JSON_SNIPPET + r'''
def head_name(m):
    if isinstance(m, M.Expression) and len(m) and isinstance(m[0], M.Symbol):
        return str(m[0])
    return None

UNQUOTES = ("unquote", "unquote-splice", "unquote_splice")

def find_holes(m, holes):
    h = head_name(m)
    if h in UNQUOTES:
        inner = m[1] if len(m) > 1 else None
        holes.append({
            "kind": "unquote-splice" if h != "unquote" else "unquote",
            "inserted": model_to_json(inner) if inner is not None else None,
            "repr": (repr(inner) if inner is not None else None),
        })
        return  # the hole's content is dynamic, not part of the static skeleton
    if isinstance(m, (M.Expression, M.List, M.Tuple, M.Set)):
        for c in m:
            find_holes(c, holes)

def count_static(m, holes_heads=UNQUOTES):
    h = head_name(m)
    if h in holes_heads:
        return 0
    if isinstance(m, (M.Expression, M.List, M.Tuple, M.Set)):
        return 1 + sum(count_static(c) for c in m)
    return 1

forms = list(read_many(src))
top = forms[0]
kind = head_name(top)
template_kind = kind if kind in ("quote", "quasiquote") else "none"
body = top[1] if (template_kind in ("quote", "quasiquote") and len(top) > 1) else top

holes = []
if template_kind == "quasiquote":
    find_holes(body, holes)

mod = types.ModuleType("_pnix_hy_quasiquote_probe")
sys.modules[mod.__name__] = mod
lowered = C.hy_compile([top], mod, filename="<pnix-hy-quasiquote>", source=src)

out = {
    "schema": "pnix-hy.hy-quasiquote-projection.v0",
    "source": src,
    "template_kind": template_kind,
    "reader_form": model_to_json(top),
    "template_body": model_to_json(body),
    "python_source": ast.unparse(lowered),
    "holes": holes,
    "hole_count": len(holes),
    "static_skeleton_nodes": count_static(body),
}
print(json.dumps(out))
'''

# The pnix side of quasiquote: pnix is not homoiconic -- programs are not first-class
# model values, so there is NO quasiquote/unquote construct. The HONEST analogue is the
# binding-time structure, which pnix DOES express two ways.
_QUASIQUOTE_PNIX_NOTE = {
    "direct_pnix_construct": None,
    "reason": (
        "pnix is not homoiconic: programs are not first-class data values, so it has no "
        "quote/quasiquote/unquote. Code-as-data has no direct pnix term."
    ),
    "binding_time_analogue": (
        "a quasiquote is a STATIC code skeleton with DYNAMIC holes (unquote). That is the "
        "same binding-time structure pnix expresses as (a) string interpolation "
        '"...${expr}..." (static text + dynamic holes), and (b) the residual code emitted '
        "by specialize_pnix (static folded skeleton + dynamic free vars). Quasiquote is "
        "MANUAL staging; specialize_pnix is AUTOMATIC staging."
    ),
    "see": ["specialize_pnix", "str_interp"],
}


def hy_quasiquote_projection(source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Project a quoted/quasiquoted Hy form as code-as-data: its model tree, the Python
    that CONSTRUCTS that tree, the unquote/unquote-splice holes (dynamic insertion
    points), and the pnix binding-time analogue (no quasiquote, but the same
    static-skeleton/dynamic-hole staging structure). Schema
    `pnix-hy.hy-quasiquote-projection.v0`."""
    stdout = _run_hy_script(_HY_QUASIQUOTE_SCRIPT, source, timeout=timeout,
                            label=f"hy quasiquote projection for {source!r}")
    try:
        out = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(
            f"hy quasiquote projection did not return JSON: {stdout!r}"
        ) from exc
    out["pnix_correspondence"] = _QUASIQUOTE_PNIX_NOTE
    return out


def hy_quasiquote_projection_report() -> dict[str, Any]:
    """Self-check: `(+ 1 ~x ~@ys) is a quasiquote with two holes (unquote + splice)
    whose lowering both builds the model skeleton and inserts the dynamic vars."""
    probe = "`(+ 1 ~x ~@ys)"
    try:
        proj = hy_quasiquote_projection(probe)
    except HyMirrorError as exc:
        return {"schema": "pnix-hy.hy-quasiquote-projection.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    kinds = [h.get("kind") for h in proj.get("holes") or []]
    py = proj.get("python_source", "")
    ready = (
        proj.get("template_kind") == "quasiquote"
        and proj.get("hole_count") == 2
        and "unquote" in kinds
        and "unquote-splice" in kinds
        and "hy.models.Expression" in py
        and "*(ys or [])" in py
        and proj.get("pnix_correspondence", {}).get("direct_pnix_construct") is None
    )
    return {
        "schema": "pnix-hy.hy-quasiquote-projection.report.v0",
        "ready": bool(ready),
        "available": True,
        "template_kind": proj.get("template_kind"),
        "hole_count": proj.get("hole_count"),
        "hole_kinds": kinds,
        "python_source": py,
    }


_HY_EVAL_FORM_SCRIPT = r'''
import os, json
import hy
from hy.reader import read_many
import hy.models as M

src = os.environ["PNIX_HY_FORM_SRC"]
''' + _HY_MODEL_TO_JSON_SNIPPET + r'''
result = None
for f in read_many(src):
    result = hy.eval(f)
out = {
    "schema": "pnix-hy.hy-eval-form.v0",
    "source": src,
    "result": model_to_json(result),
    "result_repr": repr(result),
}
print(json.dumps(out))
'''


def hy_eval_form(source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Evaluate Hy source in the Hy proof Python and project the resulting VALUE as a model
    tree. Used to feed pnix values into a Hy quasiquote and read the constructed form back.
    Schema `pnix-hy.hy-eval-form.v0`."""
    stdout = _run_hy_script(_HY_EVAL_FORM_SCRIPT, source, timeout=timeout,
                            label=f"hy eval form for {source!r}")
    try:
        return json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(
            f"hy eval form did not return JSON: {stdout!r}"
        ) from exc


_HY_PNIX_READER_SCRIPT = r'''
import os, json
import hy
from hy.reader import read_many
from hy.reader.hy_reader import HyReader
import hy.models as M

src = os.environ["PNIX_HY_FORM_SRC"]
''' + _HY_MODEL_TO_JSON_SNIPPET + r'''
def px_handler(reader, key):
    # `#px <form>` -> (pnix-eval <form>): read-time embedding of pnix in Hy
    return M.Expression([M.Symbol("pnix-eval"), reader.parse_one_form()])

reader = HyReader()
reader.reader_macros["px"] = px_handler
forms = list(read_many(src, reader=reader))

embedded = []
def walk(m):
    if (isinstance(m, M.Expression) and len(m) >= 2 and isinstance(m[0], M.Symbol)
            and str(m[0]) == "pnix-eval" and isinstance(m[1], M.String)):
        embedded.append(str(m[1]))
    if isinstance(m, (M.Expression, M.List, M.Tuple, M.Set, M.Dict)):
        for c in m:
            walk(c)
for f in forms:
    walk(f)

out = {
    "schema": "pnix-hy.hy-read-with-pnix-reader.v0",
    "source": src,
    "reader_macro": "px",
    "read_forms": [model_to_json(f) for f in forms],
    "embedded_pnix": embedded,
}
print(json.dumps(out))
'''


def hy_read_with_pnix_reader(source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Read Hy source with a `#px "..."` reader macro active: at READ time each `#px <form>`
    becomes `(pnix-eval <form>)`, embedding pnix in Hy. Returns the read model forms and the
    embedded pnix strings. This is Hy's reader machinery embedding pnix -- NOT a pnix
    reader-macro. Schema `pnix-hy.hy-read-with-pnix-reader.v0`."""
    stdout = _run_hy_script(_HY_PNIX_READER_SCRIPT, source, timeout=timeout,
                            label=f"hy #px reader for {source!r}")
    try:
        return json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(
            f"hy #px reader did not return JSON: {stdout!r}"
        ) from exc


# ============================================================================
# Hy defmacro DEFINITION projection — a macro is a compile-time form->form function
#
# Quasiquote showed code-as-data; defmacro shows the meta-circular peak: a USER macro is
# a compile-time function over syntax. (defmacro inc [x] `(+ ~x 1)) lowers to
#   hy.macros.macro('inc')(lambda x: Expression([Symbol('+'), x, Integer(1)]))
# i.e. a TRANSFORMER lambda (whose body is the quasiquote code template) REGISTERED by
# name in the module macro table, run BEFORE compilation. This projects the definition
# (name/params/body/lowering/registration) and, defensively, demonstrates the
# form->form rewrite on any use forms in the same source. Runs in the Hy proof Python.
# ============================================================================

_HY_DEFMACRO_SCRIPT = r'''
import os, sys, json, ast, types
import hy
from hy.reader import read_many
import hy.models as M
import hy.compiler as C

src = os.environ["PNIX_HY_FORM_SRC"]
''' + _HY_MODEL_TO_JSON_SNIPPET + r'''
def head_name(m):
    if isinstance(m, M.Expression) and len(m) and isinstance(m[0], M.Symbol):
        return str(m[0])
    return None

mod = types.ModuleType("_pnix_hy_defmacro_probe")
sys.modules[mod.__name__] = mod

forms = list(read_many(src))
defmacros = []
uses = []
for f in forms:
    if head_name(f) == "defmacro":
        params = []
        if len(f) > 2 and isinstance(f[2], (M.List, M.Expression)):
            params = [str(s) if isinstance(s, M.Symbol) else repr(s) for s in f[2]]
        body_forms = list(f[3:]) if len(f) > 3 else []
        entry = {
            "name": str(f[1]) if len(f) > 1 else None,
            "params": params,
            "body_form_count": len(body_forms),
            "body_reader_forms": [model_to_json(b) for b in body_forms],
            "body_is_quasiquote": (len(body_forms) == 1 and head_name(body_forms[0]) == "quasiquote"),
        }
        try:
            lowered = C.hy_compile([f], mod, filename="<pnix-hy-defmacro>", source=src)
            py = ast.unparse(lowered)
            entry["python_source"] = py
            entry["registers"] = ("hy.macros.macro(" in py)
            try:
                exec(compile(lowered, "<pnix-hy-defmacro>", "exec"), mod.__dict__)
                entry["registered_ok"] = True
            except Exception as exc:
                entry["registered_ok"] = False
                entry["register_error"] = "%s: %s" % (type(exc).__name__, exc)
        except Exception as exc:
            entry["python_source"] = None
            entry["lower_error"] = "%s: %s" % (type(exc).__name__, exc)
        defmacros.append(entry)
    else:
        uses.append(f)

expansions = []
for u in uses:
    e = {"call": model_to_json(u), "call_head": head_name(u)}
    try:
        exp = hy.macroexpand(u, mod)
        e["expanded"] = model_to_json(exp)
        e["changed"] = repr(u) != repr(exp)
    except Exception as exc:
        e["expand_error"] = "%s: %s" % (type(exc).__name__, exc)
        e["changed"] = None
    expansions.append(e)

print(json.dumps({
    "schema": "pnix-hy.hy-defmacro-projection.v0",
    "source": src,
    "defmacro_count": len(defmacros),
    "defmacros": defmacros,
    "expansions": expansions,
}))
'''

# The pnix side of defmacro: pnix has NO macro system. The honest analogue is staging.
_DEFMACRO_PNIX_NOTE = {
    "direct_pnix_construct": None,
    "reason": (
        "pnix has no macro system: there is no compile-time form->form transformer and no "
        "macro table. Evaluation is the only stage, and it operates on VALUES, not syntax."
    ),
    "staging_analogue": (
        "a Hy macro is a COMPILE-TIME function over SYNTAX (hy.models form -> form), "
        "registered by name and run before compilation. The nearest pnix term is an "
        "ordinary function (lambda), but it maps VALUES->VALUES at EVAL time, not "
        "CODE->CODE at COMPILE time. The distinguishing axes are STAGING (compile vs "
        "runtime) and DOMAIN (syntax vs values). A macro body is itself a code template "
        "(quasiquote), and the compile-vs-runtime split is the same staging axis "
        "specialize_pnix automates."
    ),
    "see": ["hy_quasiquote_projection", "specialize_pnix", "lambda"],
}


def hy_defmacro_projection(source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Project user macro DEFINITIONS: for each `(defmacro ...)`, its name/params, the
    body (the code template), the Python lowering (transformer lambda + macro-table
    registration), and -- defensively -- the form->form rewrite demonstrated on any use
    forms in the same source. Schema `pnix-hy.hy-defmacro-projection.v0`."""
    stdout = _run_hy_script(_HY_DEFMACRO_SCRIPT, source, timeout=timeout,
                            label=f"hy defmacro projection for {source!r}")
    try:
        out = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(
            f"hy defmacro projection did not return JSON: {stdout!r}"
        ) from exc
    out["pnix_correspondence"] = _DEFMACRO_PNIX_NOTE
    return out


def hy_defmacro_projection_report() -> dict[str, Any]:
    """Self-check: `(defmacro inc [x] `(+ ~x 1))` defines a registered transformer with a
    quasiquote body, and using `(inc 41)` rewrites (form->form) to `(+ 41 1)`."""
    probe = "(defmacro inc [x] `(+ ~x 1))\n(inc 41)"
    try:
        proj = hy_defmacro_projection(probe)
    except HyMirrorError as exc:
        return {"schema": "pnix-hy.hy-defmacro-projection.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    dms = proj.get("defmacros") or []
    dm = dms[0] if dms else {}
    exps = proj.get("expansions") or []
    exp = exps[0] if exps else {}
    exp_head = ((exp.get("expanded") or {}).get("items") or [{}])[0]
    ready = (
        proj.get("defmacro_count") == 1
        and dm.get("name") == "inc"
        and dm.get("params") == ["x"]
        and dm.get("body_is_quasiquote") is True
        and dm.get("registers") is True
        and dm.get("registered_ok") is True
        and exp.get("changed") is True
        and exp_head.get("name") == "+"
        and proj.get("pnix_correspondence", {}).get("direct_pnix_construct") is None
    )
    return {
        "schema": "pnix-hy.hy-defmacro-projection.report.v0",
        "ready": bool(ready),
        "available": True,
        "defmacro_count": proj.get("defmacro_count"),
        "name": dm.get("name"),
        "params": dm.get("params"),
        "body_is_quasiquote": dm.get("body_is_quasiquote"),
        "expansion_head": exp_head.get("name"),
    }


# ============================================================================
# Hy reader-macro (defreader / #name) projection — the READ-TIME stage of the tower
#
# This completes the meta-circular STAGING TOWER. A reader macro runs at READ time,
# before any model tree exists: (defreader double (let [x (.parse-one-form &reader)]
# `[~x ~x])) lowers to a function (reader -> model) registered via
# hy.macros.reader_macro('double', fn); then `#double 21` is expanded BY THE READER into
# the model [21 21] before compilation even begins. So Hy has THREE meta-stages --
# read-time (reader macros, extend SYNTAX) -> compile-time (defmacro, rewrite FORMS) ->
# run-time (eval, values). pnix collapses all three to a single eval stage; that collapse
# is exactly what Futamura/specialize_pnix (P5) is about. Runs in the Hy proof Python.
# ============================================================================

_HY_READER_MACRO_SCRIPT = r'''
import os, sys, json, ast, types, io
import hy
from hy.reader import HyReader, read_many
import hy.models as M
import hy.compiler as C

src = os.environ["PNIX_HY_FORM_SRC"]
''' + _HY_MODEL_TO_JSON_SNIPPET + r'''
def head_name(m):
    if isinstance(m, M.Expression) and len(m) and isinstance(m[0], M.Symbol):
        return str(m[0])
    return None

mod = types.ModuleType("_pnix_hy_reader_probe")
sys.modules[mod.__name__] = mod

reader = HyReader(use_current_readers=True)
builtin_dispatch = sorted(reader.reader_macros.keys())

defreaders = []
produced_forms = []
error = None
try:
    with reader.as_current_reader():
        for f in read_many(io.StringIO(src), filename="<pnix-hy-reader>", reader=reader):
            is_defreader = head_name(f) == "defreader"
            if is_defreader:
                entry = {"name": str(f[1]) if len(f) > 1 else None}
                try:
                    low = C.hy_compile([f], mod, filename="<pnix-hy-reader>", source=src)
                    py = ast.unparse(low)
                    entry["python_source"] = py
                    entry["registers"] = ("hy.macros.reader_macro(" in py)
                    exec(compile(low, "<pnix-hy-reader>", "exec"), mod.__dict__)
                    entry["registered_ok"] = True
                except Exception as exc:
                    entry["registered_ok"] = False
                    entry["register_error"] = "%s: %s" % (type(exc).__name__, exc)
                defreaders.append(entry)
            # record the model the reader produced (a #name use is already expanded here)
            produced_forms.append({
                "is_defreader": is_defreader,
                "head": head_name(f),
                "form": model_to_json(f),
            })
except Exception as exc:
    error = "%s: %s" % (type(exc).__name__, exc)

user_reader_macros = sorted(k for k in reader.reader_macros.keys() if k not in builtin_dispatch)

print(json.dumps({
    "schema": "pnix-hy.hy-reader-macro-projection.v0",
    "source": src,
    "builtin_dispatch": builtin_dispatch,
    "defreaders": defreaders,
    "user_reader_macros": user_reader_macros,
    "produced_forms": produced_forms,
    "error": error,
}))
'''

# The pnix side of reader macros: pnix has no read-time stage at all.
_READER_MACRO_PNIX_NOTE = {
    "direct_pnix_construct": None,
    "reason": (
        "pnix has no reader/tag macros and no read-time stage: its surface grammar is "
        "fixed and cannot be extended from within a program. Read-time syntax extension "
        "has no pnix analogue."
    ),
    "staging_tower": (
        "this completes the meta-circular staging tower pnix-hy projects from Hy: "
        "READ-time (reader macros: hy.macros.reader_macro, stream->model, EXTEND SYNTAX) "
        "-> COMPILE-time (defmacro: hy.macros.macro, form->form, REWRITE FORMS) -> "
        "RUN-time (eval, value->value). pnix collapses all three into a SINGLE eval stage "
        "over values; that read->compile->run collapse is exactly the tower-collapse "
        "Futamura/specialize_pnix (P5) realizes."
    ),
    "see": ["hy_defmacro_projection", "hy_quasiquote_projection", "specialize_pnix"],
}


def hy_reader_macro_projection(source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Project the READ-TIME stage: `defreader` definitions (reader -> model, registered
    via hy.macros.reader_macro) and the models the reader PRODUCES (a `#name` use is
    already expanded at read time). Surfaces Hy's built-in dispatch table too. Schema
    `pnix-hy.hy-reader-macro-projection.v0`."""
    stdout = _run_hy_script(_HY_READER_MACRO_SCRIPT, source, timeout=timeout,
                            label=f"hy reader macro projection for {source!r}")
    try:
        out = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(
            f"hy reader macro projection did not return JSON: {stdout!r}"
        ) from exc
    out["pnix_correspondence"] = _READER_MACRO_PNIX_NOTE
    return out


def hy_reader_macro_projection_report() -> dict[str, Any]:
    """Self-check: `(defreader double ...)` registers a read-time reader macro, and
    `#double 21` is expanded BY THE READER into the model `[21 21]` before compilation."""
    probe = "(defreader double (let [x (.parse-one-form &reader)] `[~x ~x]))\n#double 21"
    try:
        proj = hy_reader_macro_projection(probe)
    except HyMirrorError as exc:
        return {"schema": "pnix-hy.hy-reader-macro-projection.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    drs = proj.get("defreaders") or []
    dr = drs[0] if drs else {}
    forms = proj.get("produced_forms") or []
    used = forms[1].get("form") if len(forms) > 1 else {}
    items = (used or {}).get("items") or []
    item_vals = [it.get("value") for it in items]
    ready = (
        proj.get("error") is None
        and dr.get("name") == "double"
        and dr.get("registers") is True
        and dr.get("registered_ok") is True
        and "double" in (proj.get("user_reader_macros") or [])
        and (used or {}).get("model") == "List"
        and item_vals == [21, 21]
        and proj.get("pnix_correspondence", {}).get("direct_pnix_construct") is None
    )
    return {
        "schema": "pnix-hy.hy-reader-macro-projection.report.v0",
        "ready": bool(ready),
        "available": True,
        "user_reader_macros": proj.get("user_reader_macros"),
        "read_time_produced": used,
        "builtin_dispatch": proj.get("builtin_dispatch"),
    }


# ============================================================================
# Hy import / require projection — the cross-module dimension of the staging tower
#
# `import` and `require` are the two cross-module scopes: `import` binds a Python MODULE
# OBJECT at RUN time (import json -> import json); `require` loads another module's MACROS
# at COMPILE time (so its macros are usable in THIS module's forms). So the read->compile
# ->run tower also has a module axis: require feeds the compile stage, import feeds the run
# stage. pnix's `import ./f.px` is an EXPRESSION evaluating a file to a pure VALUE at eval
# time (closest to Hy import, but value vs module-object); pnix has no `require` (no macro
# stage). Runs in the Hy proof Python (cwd=HY_ROOT).
# ============================================================================

_HY_IMPORT_SCRIPT = r'''
import os, sys, json, ast, types
import hy
from hy.reader import read_many
import hy.models as M
import hy.compiler as C

src = os.environ["PNIX_HY_FORM_SRC"]
''' + _HY_MODEL_TO_JSON_SNIPPET + r'''
def head_name(m):
    if isinstance(m, M.Expression) and len(m) and isinstance(m[0], M.Symbol):
        return str(m[0])
    return None

def is_scaffold(st):
    # drop the compiler's `import hy*` and the trailing None/[None] result expr
    if isinstance(st, ast.Import) and all(a.name == "hy" or a.name.startswith("hy.") for a in st.names):
        return True
    if isinstance(st, ast.Expr):
        v = st.value
        if isinstance(v, ast.Constant) and v.value is None:
            return True
        if isinstance(v, ast.List) and all(isinstance(e, ast.Constant) and e.value is None for e in v.elts):
            return True
    return False

def imports_of(stmts):
    out = []
    for st in stmts:
        if isinstance(st, ast.Import):
            out.append({"from": None, "names": [{"name": a.name, "asname": a.asname} for a in st.names]})
        elif isinstance(st, ast.ImportFrom):
            out.append({"from": st.module, "level": st.level,
                        "names": [{"name": a.name, "asname": a.asname} for a in st.names]})
    return out

mod = types.ModuleType("_pnix_hy_import_probe")
sys.modules[mod.__name__] = mod

forms = list(read_many(src))
entries = []
for f in forms:
    head = head_name(f)
    if head not in ("import", "require"):
        continue
    entry = {
        "kind": head,
        "stage": "run-time" if head == "import" else "compile-time",
        "reader_form": model_to_json(f),
    }
    try:
        low = C.hy_compile([f], mod, filename="<pnix-hy-import>", source=src)
        cleaned = [st for st in low.body if not is_scaffold(st)]
        entry["python_source"] = "\n".join(ast.unparse(st) for st in cleaned) or "(no statements)"
        entry["imports"] = imports_of(cleaned)
    except Exception as exc:
        entry["python_source"] = None
        entry["error"] = "%s: %s" % (type(exc).__name__, exc)
        entry["needs_module"] = "Require" in type(exc).__name__
    entries.append(entry)

print(json.dumps({
    "schema": "pnix-hy.hy-import-projection.v0",
    "source": src,
    "entry_count": len(entries),
    "entries": entries,
}))
'''

_IMPORT_PNIX_NOTE = {
    "hy_import": (
        "Hy `import` is a run-time STATEMENT that binds a Python MODULE OBJECT in the "
        "namespace (import json -> `import json`). It is impure (code-bearing module)."
    ),
    "hy_require": (
        "Hy `require` is a COMPILE-TIME load of another module's MACROS, so they expand in "
        "this module's forms. It feeds the compile stage, not the run stage."
    ),
    "pnix_import": (
        "pnix `import ./f.px` is an EXPRESSION that evaluates a file to a pure VALUE "
        "(usually an attrset) at eval time -- closest to Hy import, but it yields a pnix "
        "VALUE, not a module object, and binds nothing implicitly."
    ),
    "pnix_require": None,
    "pnix_require_reason": (
        "pnix has no `require`: there is no compile-time macro stage to import into "
        "(consistent with the no-macro-system gap of hy_defmacro_projection)."
    ),
    "see": ["hy_defmacro_projection", "hy_reader_macro_projection"],
}


def hy_import_projection(source: str, *, timeout: int = 60) -> dict[str, Any]:
    """Project `import` / `require` forms: kind, stage (run-time vs compile-time), the
    Python lowering, structured imports, and the pnix correspondence (import = value-at-
    eval; no require). `require` of a missing module is recorded gracefully, not raised.
    Schema `pnix-hy.hy-import-projection.v0`."""
    stdout = _run_hy_script(_HY_IMPORT_SCRIPT, source, timeout=timeout,
                            label=f"hy import projection for {source!r}")
    try:
        out = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(
            f"hy import projection did not return JSON: {stdout!r}"
        ) from exc
    out["pnix_correspondence"] = _IMPORT_PNIX_NOTE
    return out


def hy_import_projection_report() -> dict[str, Any]:
    """Self-check: `import`s lower (run-time) with structured names; a `require` is
    classified compile-time and a missing module is recorded gracefully, not raised."""
    probe = ("(import json)\n(import os.path [join])\n(import math :as m)\n"
             "(require nonexistent-pnix-hy-macros)")
    try:
        proj = hy_import_projection(probe)
    except HyMirrorError as exc:
        return {"schema": "pnix-hy.hy-import-projection.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    entries = proj.get("entries") or []
    by_kind = {}
    for e in entries:
        by_kind.setdefault(e["kind"], []).append(e)
    imports = by_kind.get("import") or []
    requires = by_kind.get("require") or []
    from_join = next((e for e in imports if e.get("python_source") == "from os.path import join"), None)
    req = requires[0] if requires else {}
    ready = (
        proj.get("entry_count") == 4
        and len(imports) == 3
        and all(e.get("stage") == "run-time" for e in imports)
        and from_join is not None
        and req.get("stage") == "compile-time"
        and ("error" in req or req.get("needs_module"))
        and proj.get("pnix_correspondence", {}).get("pnix_require") is None
    )
    return {
        "schema": "pnix-hy.hy-import-projection.report.v0",
        "ready": bool(ready),
        "available": True,
        "entry_count": proj.get("entry_count"),
        "import_lowerings": [e.get("python_source") for e in imports],
        "require_stage": req.get("stage"),
        "require_graceful": "error" in req or bool(req.get("needs_module")),
    }


# ============================================================================
# Hy module (multi-form) projection — every top-level form, not just the first
#
# hy_form_projection lowers a whole fragment to one Python block and only macroexpands
# forms[0]. A real .hy module has many top-level forms (defn/defmacro/setv/...). This
# projects EACH top-level form independently: reader model, its own lowering, and its own
# macro layer — the per-form breakdown needed to study a whole module. Runs in the Hy
# proof Python (cwd=HY_ROOT).
# ============================================================================

_HY_MODULE_PROJECTION_SCRIPT = r'''
import os, sys, json, ast, types
import hy
from hy.reader import read_many
import hy.models as M
import hy.compiler as C

src = os.environ["PNIX_HY_FORM_SRC"]
''' + _HY_MODEL_TO_JSON_SNIPPET + r'''
def head_name(m):
    if isinstance(m, M.Expression) and len(m) and isinstance(m[0], M.Symbol):
        return str(m[0])
    return None

mod = types.ModuleType("_pnix_hy_module_probe")
sys.modules["_pnix_hy_module_probe"] = mod

forms = list(read_many(src))
out_forms = []
for f in forms:
    entry = {"head": head_name(f), "reader_form": model_to_json(f)}
    try:
        lowered = C.hy_compile([f], mod, filename="<pnix-hy-module>", source=src)
        entry["python_source"] = ast.unparse(lowered)
    except Exception as exc:
        entry["python_source"] = None
        entry["lower_error"] = "%s: %s" % (type(exc).__name__, exc)
    try:
        expanded = hy.macroexpand(f, mod)
        entry["is_macro"] = repr(f) != repr(expanded)
        entry["expanded_head"] = head_name(expanded)
    except Exception as exc:
        entry["is_macro"] = None
        entry["macro_error"] = "%s: %s" % (type(exc).__name__, exc)
    out_forms.append(entry)

print(json.dumps({
    "schema": "pnix-hy.hy-module-projection.v0",
    "form_count": len(forms),
    "forms": out_forms,
}))
'''


def hy_module_projection(source: str, *, timeout: int = 90) -> dict[str, Any]:
    """Project EVERY top-level Hy form of a module independently: per-form reader model,
    its own Python lowering, and its own macro layer. Schema
    `pnix-hy.hy-module-projection.v0`."""
    stdout = _run_hy_script(_HY_MODULE_PROJECTION_SCRIPT, source, timeout=timeout,
                            label="hy module projection")
    try:
        return json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise HyMirrorError(
            f"hy module projection did not return JSON: {stdout!r}"
        ) from exc


def hy_module_projection_report() -> dict[str, Any]:
    """Self-check: a 2-form module projects to 2 per-form entries (a defn + a setv)."""
    probe = "(defn f [x] (+ x 1))\n(setv y (f 41))"
    try:
        proj = hy_module_projection(probe)
    except HyMirrorError as exc:
        return {"schema": "pnix-hy.hy-module-projection.report.v0", "ready": False,
                "available": False, "error": str(exc)}
    forms = proj.get("forms") or []
    heads = [f.get("head") for f in forms]
    ready = (
        proj.get("form_count") == 2
        and heads == ["defn", "setv"]
        and all(f.get("python_source") for f in forms)
    )
    return {
        "schema": "pnix-hy.hy-module-projection.report.v0",
        "ready": ready,
        "available": True,
        "probe": probe,
        "form_count": proj.get("form_count"),
        "heads": heads,
    }


# ============================================================================
# Closure ladder (stage8 -> stage15 -> stageN)
#
# These are proof/federation lanes layered ON TOP of the stage7 eval kernel, not
# execution engines. Each `<stage>-check` emits a single verdict field that reads
# "reproduced" when the lane is closed (else "held"/drift). pnix-hy federates
# that verdict as host evidence; the top of the ladder (stage15 + stageN) is the
# cheap representative probe because the stageN lane fails closed if any lower
# stage regressed (it asserts no stage7/8/9/15 weakening).
# ============================================================================

STAGE_VERDICT_KEYS: dict[str, str] = {
    "stage8": "stage8_status",
    "stage9": "product_replay_status",
    "stage10": "stage10_status",
    "stage11": "stage11_status",
    "stage12": "stage12_status",
    "stage13": "stage13_status",
    "stage14": "stage14_status",
    "stage15": "stage15_status",
    "stagen": "stagen_status",
    "stage16": "stage16_status",
}

# Top of the ladder: fast (~3s each) and representative. stage15 is the
# open-world federation closure; stageN/stage16 the constitutional extension.
CLOSURE_DEFAULT_STAGES: tuple[str, ...] = ("stage15", "stagen")


def stage_status_check(stage: str, timeout: int = 300) -> dict[str, Any]:
    """Run one hy-meta `<stage>-check` and read its reproduced/held verdict."""
    key = STAGE_VERDICT_KEYS.get(stage)
    if key is None:
        raise HyMirrorError(f"unknown hy-meta stage: {stage!r}")
    result = run_bootstrap(f"{stage}-check", timeout=timeout)
    status = result.data.get(key)
    return {
        "schema": "pnix-hy.hy-meta.stage-status.v0",
        "stage": stage,
        "command": f"{stage}-check",
        "verdict_key": key,
        "ready": status == "reproduced",
        "status": status,
        "data": dict(result.data),
        "stderr": result.stderr.strip(),
    }


def stage15_check() -> dict[str, Any]:
    """Stage15 open-world external-evidence federation closure."""
    return stage_status_check("stage15")


def stagen_check() -> dict[str, Any]:
    """StageN constitutional-extension closure (post-stage15)."""
    return stage_status_check("stagen")


def closure_probe(stages: tuple[str, ...] = CLOSURE_DEFAULT_STAGES) -> dict[str, Any]:
    """Federate hy-meta's higher-stage closure as host evidence: run each
    `<stage>-check` and report whether the chosen ladder slice is `reproduced`."""
    checks = [stage_status_check(stage) for stage in stages]
    return {
        "schema": "pnix-hy.hy-meta.closure.v0",
        "ready": all(check["ready"] for check in checks),
        "python": str(hy_python()),
        "bootstrap": str(HY_META_BOOTSTRAP),
        "stages": list(stages),
        "status": {check["stage"]: check["status"] for check in checks},
        "details": checks,
    }


def host_summary(light: bool = True, closure: bool = False) -> dict[str, Any]:
    summary = {
        "schema": "pnix-hy.hy-host.v0",
        "runtime_python": sys.version.split()[0],
        "hy_python": str(hy_python()),
        "hy_meta_bootstrap": str(HY_META_BOOTSTRAP),
        "hy_meta_exists": HY_META_BOOTSTRAP.exists(),
    }
    if light:
        summary["kernel_probe"] = stage7_light_probe()
    if closure:
        summary["closure"] = closure_probe()
    return summary


def hy_meta_host_api_report() -> dict[str, Any]:
    """Run hy-meta's SR2-SR6 host API report under the supported hy-meta Python."""
    script = (
        "import json, sys; "
        f"sys.path.insert(0, {json.dumps(str(HY_ROOT / 'hy-meta'))}); "
        "import hy_meta; "
        "print(json.dumps(hy_meta.host_api_report(), sort_keys=True))"
    )
    # Budget is generous + env-configurable: a slow-but-correct host API (cold Hy import can
    # exceed 60s on some boxes) must read as PASS, not a timeout crash. TimeoutExpired/OSError
    # are captured symmetrically with the rc!=0 / JSONDecodeError paths below.
    try:
        budget = int(os.environ.get("PNIX_HY_HOST_API_TIMEOUT", "300"))
    except ValueError:
        budget = 300
    try:
        proc = subprocess.run(
            [str(hy_python()), "-c", script],
            cwd=str(HY_ROOT),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=budget,
            check=False,
        )
    except (subprocess.TimeoutExpired, OSError) as exc:
        return {
            "schema": "pnix-hy.hy-meta.host-api.v0",
            "ready": False,
            "available": False,
            "python": str(hy_python()),
            "error": f"{type(exc).__name__}: {exc}",
        }
    if proc.returncode != 0:
        return {
            "schema": "pnix-hy.hy-meta.host-api.v0",
            "ready": False,
            "available": False,
            "python": str(hy_python()),
            "stderr": proc.stderr.strip(),
        }
    try:
        report = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        return {
            "schema": "pnix-hy.hy-meta.host-api.v0",
            "ready": False,
            "available": False,
            "python": str(hy_python()),
            "error": f"JSONDecodeError: {exc}",
            "stdout": proc.stdout,
            "stderr": proc.stderr.strip(),
        }
    return {
        "schema": "pnix-hy.hy-meta.host-api.v0",
        "ready": bool(report.get("ready")),
        "available": True,
        "python": str(hy_python()),
        "report": report,
        "stderr": proc.stderr.strip(),
    }


# ============================================================================
# CPython (C-level) introspection
#
# Two forms, per the performance rule:
#   * host-direct (fast, no stage7) -- the PRIMARY path. Introspection is a side
#     channel: compile/eval stay native, so there is zero hot-path cost.
#   * stage7-mirror -- re-runs the SAME introspection inside the kernel, as the
#     spec / verification lane, cross-checked against host-direct.
# ============================================================================


# ============================================================================
# CPython introspection -- SEP1: HOST machinery, relocated to hy-meta.
# compile/code-object/bytecode/marshal/ast/symtable/tokenize/inspect/gc/sys/
# traceback/module + execution_trace are Hy/Python host artifacts, not pnix runtime.
# They now live in hy-meta/host_introspect.py; path-import + re-export so the
# projection facilities and the stage7 mirror seam below keep working.
# (Plan: pnix-hy/docs/SEPARATION.md SEP1 / hy-meta/todo.md SR1.)
# ============================================================================

import importlib.util as _ilu

host_introspect = None
_host_introspect_error = None


def _load_host_introspect() -> Any:
    path = HY_ROOT / "hy-meta" / "host_introspect.py"
    spec = _ilu.spec_from_file_location("pnix_hy_host_introspect", str(path))
    if spec is None or spec.loader is None:
        raise HyMirrorError(f"cannot load hy-meta host_introspect: {path}")
    mod = _ilu.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


# Names hy-meta/host_introspect.py exports; kept here so the host-introspection facilities
# degrade GRACEFULLY (import-able; callable raises) when the host tree is absent -- e.g. when
# pnix_hy is pip/nix-installed WITHOUT the repo at HY_ROOT. `import pnix_hy` must never fail.
_HOST_INTROSPECT_EXPORTS = (
    "_json_safe", "_own_code_objects", "_try", "ast_info", "code_object_info",
    "compile_source", "disassemble", "execution_trace", "execution_trace_report",
    "frame_info", "full_introspection", "function_info", "gc_info", "gc_referrers",
    "jump_labels", "line_starts", "marshal_code", "module_info", "object_info",
    "opcode_tables", "rebuild_code", "stack_effect", "symtable_info", "sys_info",
    "tokenize_info", "trace_run", "traceback_info",
)


def _host_introspect_unavailable(name: str):
    def _stub(*_args: Any, **_kwargs: Any) -> Any:
        raise HyMirrorError(
            f"host introspection '{name}' unavailable: needs the repo tree at HY_ROOT "
            f"(hy-meta/host_introspect.py). {_host_introspect_error or ''}".strip()
        )
    _stub.__name__ = name
    _stub.__qualname__ = name
    return _stub


try:
    host_introspect = _load_host_introspect()
    for _name in host_introspect.__all__:
        globals()[_name] = getattr(host_introspect, _name)
    del _name
except Exception as _exc:  # noqa: BLE001 - host module unavailable; facilities degrade
    _host_introspect_error = repr(_exc)

# Backfill any missing export as a call-time-raising stub, so a tree-less install still imports.
for _hname in _HOST_INTROSPECT_EXPORTS:
    if _hname not in globals():
        globals()[_hname] = _host_introspect_unavailable(_hname)
del _hname


# --- stage7 mirror: the SAME introspection performed inside the kernel --------

_MIRROR_INTROSPECT_HY = r'''
(do
  (import dis marshal json)
  (setv code (compile __SRC__ "<pnix-introspect>" __MODE__))
  (json.dumps
    {"co_name" code.co_name
     "co_argcount" code.co_argcount
     "co_stacksize" code.co_stacksize
     "co_flags" code.co_flags
     "co_nlocals" code.co_nlocals
     "co_names" (list code.co_names)
     "co_varnames" (list code.co_varnames)
     "co_consts_repr" (lfor c code.co_consts (repr c))
     "co_code_len" (len code.co_code)
     "co_code_hex" (.hex code.co_code)
     "opnames" (lfor i (dis.get_instructions code) i.opname)
     "marshal_len" (len (marshal.dumps code))}))
'''


def mirror_full_introspection(source: str, mode: str = "exec") -> dict[str, Any]:
    """Run code-object + bytecode + marshal introspection INSIDE stage7."""
    hy = _MIRROR_INTROSPECT_HY.replace("__SRC__", json.dumps(source)).replace(
        "__MODE__", json.dumps(mode)
    )
    return stage7_eval_json(hy)


def introspection_parity(source: str, mode: str = "exec") -> dict[str, Any]:
    """Cross-check host-direct introspection against the stage7 mirror."""
    host = full_introspection(source, mode)
    mirror = mirror_full_introspection(source, mode)
    hco = host["code_object"]
    checks = {
        "co_code_hex": hco["co_code_hex"] == mirror["co_code_hex"],
        "co_names": hco["co_names"] == mirror["co_names"],
        "co_varnames": hco["co_varnames"] == mirror["co_varnames"],
        "co_stacksize": hco["co_stacksize"] == mirror["co_stacksize"],
        "co_flags": hco["co_flags"] == mirror["co_flags"],
        "opnames": [i["opname"] for i in host["bytecode"]] == mirror["opnames"],
        "marshal_len": host["marshal"]["blob_len"] == mirror["marshal_len"],
    }
    return {
        "schema": "pnix-hy.cpython-introspection.parity.v0",
        "ready": all(checks.values()),
        "source": source,
        "mode": mode,
        "checks": checks,
        "host": host,
        "mirror": mirror,
    }
