"""Clean subprocess replay API for hy-meta SR3."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

from host_exec import _load_bootstrap
from witness import drift_witness, record_witness, replay_witness

HOST_EFFECTS = ("subprocess", "import", "file-read", "file-write", "network")


def clean_env() -> dict[str, str]:
    """Return the canonical stage9 clean subprocess environment."""
    return _load_bootstrap().stage9_clean_env()


def run_clean_probe(
    command: list[str] | tuple[str, ...] | None = None,
    *,
    cwd: str | Path | None = None,
    timeout: int = 240,
    parse_json: bool = True,
) -> dict[str, Any]:
    """Run a subprocess in the stage9 clean environment.

    With no command, this delegates to the existing stage9 product probe implementation.
    """
    bootstrap = _load_bootstrap()
    env = bootstrap.stage9_clean_env()
    probe_cwd = Path(cwd) if cwd is not None else bootstrap.ROOT
    if command is None:
        result = bootstrap.run_stage9_probe_subprocess(env=env, cwd=probe_cwd)
        summary = {key: value for key, value in result.items() if key not in {"stdout", "stderr", "parsed"}}
        witness = replay_witness({"command": "stage9-product-probe", **summary})
        return {**result, "witness_id": witness["witness_id"]}

    result = bootstrap.run_stage9_clean_subprocess(
        list(command),
        env=env,
        cwd=probe_cwd,
        timeout=timeout,
        parse_json=parse_json,
    )
    summary = {key: value for key, value in result.items() if key not in {"stdout", "stderr", "parsed"}}
    witness = replay_witness(summary)
    return {**result, "witness_id": witness["witness_id"]}


def compare_clean_probes(*probes: dict[str, Any]) -> dict[str, Any]:
    """Compare clean probe summaries and emit a drift witness."""
    drift: dict[str, Any] = {}
    for field in ("returncode", "stdout_sha256", "stderr_sha256"):
        values = [probe.get(field) for probe in probes]
        if len(set(values)) > 1:
            drift[field] = values
    canonical_hashes = [
        probe.get("parsed", {}).get("canonical_sha256")
        for probe in probes
        if isinstance(probe.get("parsed"), dict) and "canonical_sha256" in probe["parsed"]
    ]
    if canonical_hashes and len(set(canonical_hashes)) > 1:
        drift["canonical_sha256"] = canonical_hashes
    witness = drift_witness({"probe_count": len(probes), "drift": drift})
    return {
        "schema": "hy-meta.clean-replay.compare.v0",
        "ready": not drift,
        "probe_count": len(probes),
        "drift": drift,
        "witness_id": witness["witness_id"],
    }


def gate_check(
    *,
    required: tuple[str, ...] | list[str] = (),
    granted: tuple[str, ...] | list[str] = (),
    source: str = "host-sandbox",
) -> dict[str, Any]:
    """Capability-aware host admission record mirroring pnix_hy.gate.gate_check shape."""
    required_effects = sorted(set(required))
    granted_set = set(granted)
    required_records = [
        {"name": effect, "kind": "host-capability", "effect": effect}
        for effect in required_effects
    ]
    denials = [record for record in required_records if record["effect"] not in granted_set]
    unknown = [effect for effect in required_effects if effect not in HOST_EFFECTS]
    return {
        "schema": "hy-meta.host-gate-check.v0",
        "source": source,
        "pure": not required_effects,
        "required_effects": required_effects,
        "granted": sorted(granted_set),
        "denials": denials,
        "uncertain": [{"kind": "unknown-effect", "effect": effect} for effect in unknown],
        "allowed": not denials and not unknown,
    }


def sandbox_run(
    command: list[str] | tuple[str, ...] | None = None,
    *,
    required: tuple[str, ...] | list[str] = ("subprocess",),
    granted: tuple[str, ...] | list[str] = ("subprocess",),
    cwd: str | Path | None = None,
    timeout: int = 240,
    parse_json: bool = True,
) -> dict[str, Any]:
    """Gate and run a host clean-replay probe."""
    gate = gate_check(required=required, granted=granted, source="host-sandbox")
    if not gate["allowed"]:
        witness = record_witness("host-sandbox-denied", gate)
        return {
            "schema": "hy-meta.sandbox-run.v0",
            "ok": False,
            "gate": gate,
            "witness_id": witness["witness_id"],
        }
    probe = run_clean_probe(command, cwd=cwd, timeout=timeout, parse_json=parse_json)
    ok = probe.get("returncode") == 0
    witness = record_witness("host-sandbox-run", {
        "ok": ok,
        "required_effects": gate["required_effects"],
        "returncode": probe.get("returncode"),
        "stdout_sha256": probe.get("stdout_sha256"),
        "stderr_sha256": probe.get("stderr_sha256"),
    })
    return {
        "schema": "hy-meta.sandbox-run.v0",
        "ok": ok,
        "gate": gate,
        "probe": probe,
        "witness_id": witness["witness_id"],
    }


def sandbox_report() -> dict[str, Any]:
    try:
        command = [
            sys.executable,
            "-c",
            "import json; print(json.dumps({'value': 42}, sort_keys=True))",
        ]
        denied = sandbox_run(command, granted=())
        allowed = sandbox_run(command, granted=("subprocess",))
        ready = (
            denied.get("ok") is False
            and (denied.get("gate") or {}).get("allowed") is False
            and allowed.get("ok") is True
            and ((allowed.get("probe") or {}).get("parsed") or {}).get("value") == 42
            and bool(denied.get("witness_id"))
            and bool(allowed.get("witness_id"))
        )
        return {"schema": "hy-meta.sandbox-run.report.v0", "ready": bool(ready),
                "available": True, "denied": (denied.get("gate") or {}).get("denials"),
                "allowed_value": ((allowed.get("probe") or {}).get("parsed") or {}).get("value")}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "hy-meta.sandbox-run.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def clean_replay_report() -> dict[str, Any]:
    try:
        command = [
            sys.executable,
            "-c",
            "import json, os; print(json.dumps({'hash': os.environ.get('PYTHONHASHSEED'), 'value': 42}, sort_keys=True))",
        ]
        first = run_clean_probe(command)
        second = run_clean_probe(command)
        comparison = compare_clean_probes(first, second)
        env = clean_env()
        ready = (
            comparison["ready"] is True
            and first["parsed"] == {"hash": "0", "value": 42}
            and second["parsed"] == {"hash": "0", "value": 42}
            and env["PYTHONHASHSEED"] == "0"
            and env["LC_ALL"] == "C"
            and env["TZ"] == "UTC"
        )
        return {
            "schema": "hy-meta.clean-replay.report.v0",
            "ready": ready,
            "available": True,
            "comparison": comparison,
            "first_witness_id": first["witness_id"],
            "second_witness_id": second["witness_id"],
        }
    except Exception as exc:  # noqa: BLE001
        return {
            "schema": "hy-meta.clean-replay.report.v0",
            "ready": False,
            "available": False,
            "error": f"{type(exc).__name__}: {exc}",
        }


__all__ = [
    "clean_env",
    "clean_replay_report",
    "compare_clean_probes",
    "gate_check",
    "run_clean_probe",
    "sandbox_report",
    "sandbox_run",
]


def env_diff(a: dict[str, str], b: dict[str, str]) -> dict[str, Any]:
    """0027/G5: a per-VARIABLE environment diff -- which names were added/removed/changed --
    complementing the existing hash-level env comparison with explanatory power."""
    added = sorted(set(b) - set(a))
    removed = sorted(set(a) - set(b))
    changed = {k: {"a": a[k], "b": b[k]} for k in sorted(set(a) & set(b)) if a[k] != b[k]}
    return {"schema": "hy-meta.env-diff.v0", "equal": not (added or removed or changed),
            "added": added, "removed": removed, "changed": changed}
