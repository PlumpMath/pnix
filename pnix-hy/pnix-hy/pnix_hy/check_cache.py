"""pnix_hy.check_cache -- hash-keyed VERIFYING-TRACE cache for `--check` reports (proposal 0019).

Unison-style principle: a deterministic check whose inputs' content hashes have not changed
never needs re-running. Correctness rules (Build Systems a la Carte, verifying traces):
- the key is CONSERVATIVE: the content hash of EVERY pnix_hy source file + the proof-python
  identity. Any byte change anywhere in the package invalidates everything -- a spurious miss
  is safe, a wrong hit is not.
- a FAILING report is never cached: failures always re-run.
- opt-in only (`--check --cached`); the default `--check` path is byte-identical to before.
The sacred `--gate` lanes never use this cache.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
from pathlib import Path
from typing import Any, Callable

_PKG = Path(__file__).resolve().parent


def _cache_file() -> Path:
    base = os.environ.get("PNIX_HY_CACHE_DIR") or str(Path.home() / ".cache" / "pnix-hy")
    return Path(base) / "check-cache.json"


def package_state_hash() -> str:
    """Conservative input hash: every pnix_hy/*.py content hash + proof python + interpreter."""
    h = hashlib.sha256()
    for p in sorted(_PKG.glob("*.py")):
        h.update(p.name.encode())
        h.update(hashlib.sha256(p.read_bytes()).digest())
    h.update(os.environ.get("PNIX_HY_PYTHON", "").encode())
    h.update(sys.version.encode())
    return h.hexdigest()


def _load() -> dict[str, Any]:
    try:
        return json.loads(_cache_file().read_text(encoding="utf-8"))
    except Exception:  # noqa: BLE001 - missing/corrupt cache == empty cache
        return {}


def _save(data: dict[str, Any]) -> None:
    path = _cache_file()
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(data, ensure_ascii=False, default=repr), encoding="utf-8")
    tmp.replace(path)


def cached_run(name: str, fn: Callable[[], dict[str, Any]], *,
               state_hash: str | None = None) -> dict[str, Any]:
    """Run one report through the verifying-trace cache: replay iff the key matches AND the
    cached run was ready:True (failures are never cached). Cache hits carry `cached: True`."""
    key = state_hash or package_state_hash()
    data = _load()
    ent = data.get(name)
    if isinstance(ent, dict) and ent.get("key") == key and ent.get("ready") is True:
        report = dict(ent.get("report") or {})
        report["cached"] = True
        return report
    report = fn()
    if isinstance(report, dict) and report.get("ready") is True:
        data[name] = {"key": key, "ready": True, "report": report}
        try:
            _save(data)
        except Exception:  # noqa: BLE001 - an unwritable cache degrades to always-run
            pass
    return report


def check_cache_report() -> dict[str, Any]:
    """Self-check (proposal 0019): a second identical run replays from cache; a key change
    re-runs; a failing report is never cached. Runs against an isolated temp cache dir."""
    import tempfile  # noqa: PLC0415

    try:
        with tempfile.TemporaryDirectory() as tmp:
            prev = os.environ.get("PNIX_HY_CACHE_DIR")
            os.environ["PNIX_HY_CACHE_DIR"] = tmp
            try:
                calls = {"ok": 0, "bad": 0}

                def ok_report() -> dict[str, Any]:
                    calls["ok"] += 1
                    return {"ready": True, "n": calls["ok"]}

                def bad_report() -> dict[str, Any]:
                    calls["bad"] += 1
                    return {"ready": False}

                r1 = cached_run("probe", ok_report, state_hash="k1")
                r2 = cached_run("probe", ok_report, state_hash="k1")
                replay_ok = calls["ok"] == 1 and r2.get("cached") is True and not r1.get("cached")
                cached_run("probe", ok_report, state_hash="k2")  # key change -> re-run
                invalidate_ok = calls["ok"] == 2
                cached_run("broken", bad_report, state_hash="k1")
                cached_run("broken", bad_report, state_hash="k1")
                fail_rerun_ok = calls["bad"] == 2  # failures never cached
            finally:
                if prev is None:
                    os.environ.pop("PNIX_HY_CACHE_DIR", None)
                else:
                    os.environ["PNIX_HY_CACHE_DIR"] = prev
        state_ok = package_state_hash() == package_state_hash()  # deterministic key
        ready = bool(replay_ok and invalidate_ok and fail_rerun_ok and state_ok)
        return {"schema": "pnix-hy.check-cache.report.v0", "ready": ready, "available": True,
                "replay_on_hit": replay_ok, "invalidate_on_key_change": invalidate_ok,
                "failures_never_cached": fail_rerun_ok, "deterministic_key": state_ok}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.check-cache.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


__all__ = ["package_state_hash", "cached_run", "check_cache_report"]
